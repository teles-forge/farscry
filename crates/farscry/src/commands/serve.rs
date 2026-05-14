use anyhow::Result;
use chrono::Utc;
use farscry_core::vasf::{VasfFrame, VasfWriter};
use farscry_core::{DiffEngine, Pipeline, StateId, VaspDelta, VaspOutput};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn serve_mcp(
    mcp: bool,
    port: Option<u16>,
    record: Option<PathBuf>,
    hamming_threshold: u8,
) -> Result<()> {
    if !mcp {
        anyhow::bail!("Only MCP mode is currently supported");
    }
    let pipeline = crate::pipeline::get_or_build_pipeline()
        .map_err(|e| anyhow::anyhow!("Pipeline init failed: {e}"))?;
    let effective_threshold = effective_hamming_threshold(hamming_threshold);
    let record_path = resolve_record_path(record, effective_threshold).await?;
    let recorder = record_path
        .map(|p| SessionRecorder::new(p, effective_threshold))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Failed to create recorder: {e}"))?;
    let recorder_arc: Arc<Mutex<Option<SessionRecorder>>> = Arc::new(Mutex::new(recorder));
    let adapter = RecordingAdapter {
        pipeline,
        recorder: recorder_arc.clone(),
    };
    run_server(port, adapter).await;
    finalize_recorder(recorder_arc);
    Ok(())
}

fn effective_hamming_threshold(cli_value: u8) -> u8 {
    crate::config::read_farscry_config()
        .sessions
        .and_then(|s| s.hamming_threshold)
        .unwrap_or(cli_value)
}

async fn run_server<P: farscry_mcp::PipelineOps>(port: Option<u16>, adapter: P) {
    let task: tokio::task::JoinHandle<Result<(), String>> = if let Some(p) = port {
        tokio::spawn(async move { farscry_mcp::McpServer::serve_tcp_with(p, adapter).await })
    } else {
        #[cfg(unix)]
        {
            let socket_path = default_socket_path();
            tokio::spawn(async move {
                farscry_mcp::McpServer::serve_unix_with(&socket_path, adapter).await
            })
        }
        #[cfg(not(unix))]
        {
            eprintln!("[farscry] Unix sockets not supported on Windows. Use --port.");
            return;
        }
    };
    tokio::select! {
        r = task => { if let Err(e) = r { eprintln!("[farscry] server error: {e}"); } }
        _ = tokio::signal::ctrl_c() => { eprintln!("[farscry] shutting down"); }
    }
}

fn finalize_recorder(recorder_arc: Arc<Mutex<Option<SessionRecorder>>>) {
    if let Ok(mut guard) = recorder_arc.lock() {
        if let Some(mut rec) = guard.take() {
            rec.print_summary();
            let _ = rec.finalize();
        }
    }
}

async fn resolve_record_path(
    explicit: Option<PathBuf>,
    _hamming_threshold: u8,
) -> Result<Option<PathBuf>> {
    if let Some(p) = explicit {
        return Ok(Some(p));
    }
    let cfg = crate::config::read_farscry_config();
    if let Some(sessions) = &cfg.sessions {
        if sessions.record == Some(true) {
            let dir = sessions
                .output_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(default_sessions_dir);
            std::fs::create_dir_all(&dir)?;
            let path = dir.join(timestamp_filename());
            eprintln!("[farscry] session recording: auto (from config)");
            eprintln!("[farscry] recording to {}", path.display());
            return Ok(Some(path));
        }
    }
    tokio::task::spawn_blocking(prompt_record_session)
        .await
        .map_err(|e| anyhow::anyhow!("prompt task: {e}"))
        .and_then(|r| r)
}

fn prompt_record_session() -> Result<Option<PathBuf>> {
    use std::io::Write;
    let dir = default_sessions_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(timestamp_filename());
    eprint!("[farscry] session observability: disabled\nEnable recording for this session? (y/N): ");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    if line.trim().eq_ignore_ascii_case("y") {
        eprintln!("[farscry] recording to {}", path.display());
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn default_sessions_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".farscry")
        .join("sessions")
}

fn default_socket_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".farscry")
        .join("mcp.sock")
}

fn timestamp_filename() -> String {
    let now = Utc::now();
    format!("{}.vasf", now.format("%Y-%m-%d-%H%M"))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn hamming(a: StateId, b: StateId) -> u8 {
    (a.to_bits() ^ b.to_bits()).count_ones() as u8
}

fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

struct SessionRecorder {
    writer: VasfWriter,
    last_state_id: Option<StateId>,
    last_vasp: Option<VaspOutput>,
    hamming_threshold: u8,
    output_path: PathBuf,
    start_time: i64,
    total_frames: u64,
    unique_frames: u64,
}

impl SessionRecorder {
    fn new(output_path: PathBuf, hamming_threshold: u8) -> std::io::Result<Self> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let writer = VasfWriter::create(&output_path)?;
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Ok(Self {
            writer,
            last_state_id: None,
            last_vasp: None,
            hamming_threshold,
            output_path,
            start_time,
            total_frames: 0,
            unique_frames: 0,
        })
    }

    fn record_frame(
        &mut self,
        state_id: StateId,
        vasp: &VaspOutput,
        delta: Option<&VaspDelta>,
    ) -> Result<()> {
        self.total_frames += 1;
        self.writer.total_input = self.total_frames as u32;
        let is_new = self
            .last_state_id
            .map(|last| hamming(state_id, last) > self.hamming_threshold)
            .unwrap_or(true);
        if !is_new {
            return Ok(());
        }
        self.unique_frames += 1;
        let computed = delta.is_none().then(|| {
            self.last_vasp.as_ref().map(|prev| {
                farscry_diff::DiffEngineImpl.diff(prev, vasp, None, None)
            })
        }).flatten();
        let eff = delta.or(computed.as_ref());
        let delta_bytes = eff.map(|d| farscry_formatter::VaspFormatter::format_diff(d).into_bytes());
        let vasp_text = farscry_formatter::VaspFormatter::format_vasp(vasp, "screen", 1920, 1080);
        let frame = VasfFrame {
            state_id,
            timestamp: now_ms(),
            vasp_data: vasp_text.into_bytes(),
            delta_data: delta_bytes,
        };
        self.writer
            .append_frame(&frame)
            .map_err(|e| anyhow::anyhow!("write frame: {e}"))?;
        self.writer
            .update_header()
            .map_err(|e| anyhow::anyhow!("update header: {e}"))?;
        self.last_state_id = Some(state_id);
        self.last_vasp = Some(vasp.clone());
        Ok(())
    }

    fn print_summary(&self) {
        let total = self.total_frames;
        let unique = self.unique_frames;
        let dupes = total.saturating_sub(unique);
        let pct = if total > 0 { dupes * 100 / total } else { 0 };
        let tokens_raw = total * 2765;
        let tokens_vasf = unique * 200;
        let ratio = if tokens_vasf > 0 { tokens_raw / tokens_vasf } else { 0 };
        eprintln!("[farscry] session complete");
        eprintln!("[farscry] unique states: {unique} of {total} frames ({pct}% deduplicated)");
        eprintln!("[farscry] tokens without farscry: ~{}", fmt_num(tokens_raw));
        eprintln!("[farscry] tokens with farscry: ~{}", fmt_num(tokens_vasf));
        eprintln!("[farscry] reduction: ~{ratio}x");
        eprintln!("[farscry] saved: {}", self.output_path.display());
        eprintln!("[farscry] replay: farscry timeline {}", self.output_path.display());
        let _ = self.start_time;
    }

    fn finalize(&mut self) -> std::io::Result<()> {
        self.writer.finalize()
    }
}

#[derive(Clone)]
struct RecordingAdapter {
    pipeline: Arc<Pipeline>,
    recorder: Arc<Mutex<Option<SessionRecorder>>>,
}

impl farscry_mcp::PipelineOps for RecordingAdapter {
    fn process(&self, image_path: &str) -> Result<VaspOutput, String> {
        let img = image::open(image_path).map_err(|e| format!("cannot open image: {e}"))?;
        let output = self.pipeline.process(img).map_err(|e| e.to_string())?;
        if let Ok(mut guard) = self.recorder.lock() {
            if let Some(rec) = guard.as_mut() {
                rec.record_frame(output.state_id, &output, None).ok();
            }
        }
        Ok(output)
    }

    fn diff(
        &self,
        before: &VaspOutput,
        after: &VaspOutput,
        before_dims: Option<(u32, u32)>,
        after_dims: Option<(u32, u32)>,
    ) -> VaspDelta {
        farscry_diff::DiffEngineImpl.diff(before, after, before_dims, after_dims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use farscry_core::vasf::VasfFile;
    use farscry_core::{Confidence, ScreenType, StateId, VaspOutput};

    fn make_vasp(id: u64) -> VaspOutput {
        VaspOutput::new(
            StateId::from_bits(id),
            ScreenType::Terminal,
            Confidence::High,
            "eng",
            "test ctx",
            vec![],
            vec![],
        )
    }

    #[test]
    fn test_recorder_creates_file() {
        let path = PathBuf::from("/tmp/_srv_recorder_creates.vasf");
        let _ = std::fs::remove_file(&path);
        let mut rec = SessionRecorder::new(path.clone(), 10).unwrap();
        let vasp = make_vasp(0x1111_0000_0000_0000);
        rec.record_frame(vasp.state_id, &vasp, None).unwrap();
        rec.finalize().unwrap();
        assert!(path.exists());
        let loaded = VasfFile::read_from(&path).unwrap();
        assert_eq!(loaded.frames.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_recorder_dedup() {
        let path = PathBuf::from("/tmp/_srv_recorder_dedup.vasf");
        let _ = std::fs::remove_file(&path);
        let mut rec = SessionRecorder::new(path.clone(), 10).unwrap();
        let vasp = make_vasp(0x1111_0000_0000_0000);
        rec.record_frame(vasp.state_id, &vasp, None).unwrap();
        rec.record_frame(vasp.state_id, &vasp, None).unwrap();
        rec.record_frame(vasp.state_id, &vasp, None).unwrap();
        rec.finalize().unwrap();
        assert_eq!(rec.total_frames, 3);
        assert_eq!(rec.unique_frames, 1);
        let loaded = VasfFile::read_from(&path).unwrap();
        assert_eq!(loaded.header.frame_count, 1);
        assert_eq!(loaded.header.total_input, 3);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_recorder_new_states() {
        let path = PathBuf::from("/tmp/_srv_recorder_new.vasf");
        let _ = std::fs::remove_file(&path);
        let mut rec = SessionRecorder::new(path.clone(), 10).unwrap();
        let v1 = make_vasp(0x0000_0000_0000_0000);
        let v2 = make_vasp(0xFFFF_0000_0000_0000);
        rec.record_frame(v1.state_id, &v1, None).unwrap();
        rec.record_frame(v1.state_id, &v1, None).unwrap();
        rec.record_frame(v2.state_id, &v2, None).unwrap();
        rec.finalize().unwrap();
        assert_eq!(rec.total_frames, 3);
        assert_eq!(rec.unique_frames, 2);
        let loaded = VasfFile::read_from(&path).unwrap();
        assert_eq!(loaded.frames.len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_recorder_incremental_readable() {
        let path = PathBuf::from("/tmp/_srv_recorder_incr.vasf");
        let _ = std::fs::remove_file(&path);
        let mut rec = SessionRecorder::new(path.clone(), 10).unwrap();
        let vasp = make_vasp(0x1111_0000_0000_0000);
        rec.record_frame(vasp.state_id, &vasp, None).unwrap();
        let loaded = VasfFile::read_from(&path).unwrap();
        assert_eq!(loaded.header.frame_count, 1, "header must be live before finalize");
        assert_eq!(loaded.header.total_input, 1);
        assert_eq!(loaded.frames.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_fmt_num_commas() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(999), "999");
        assert_eq!(fmt_num(1000), "1,000");
        assert_eq!(fmt_num(486000), "486,000");
        assert_eq!(fmt_num(1_234_567), "1,234,567");
    }

    #[test]
    fn test_hamming_same() {
        let id = StateId::from_bits(0xABCD_EF01_2345_6789);
        assert_eq!(hamming(id, id), 0);
    }

    #[test]
    fn test_recorder_delta_stored() {
        let path = PathBuf::from("/tmp/_srv_recorder_delta.vasf");
        let _ = std::fs::remove_file(&path);
        let mut rec = SessionRecorder::new(path.clone(), 10).unwrap();
        let v1 = make_vasp(0x0000_0000_0000_0000);
        let v2 = make_vasp(0xFFFF_0000_0000_0000);
        rec.record_frame(v1.state_id, &v1, None).unwrap();
        rec.record_frame(v2.state_id, &v2, None).unwrap();
        rec.finalize().unwrap();
        let loaded = VasfFile::read_from(&path).unwrap();
        assert_eq!(loaded.frames.len(), 2);
        assert!(loaded.frames[1].delta_data.is_some(), "second frame must carry delta");
        let _ = std::fs::remove_file(&path);
    }
}
