use anyhow::{Context, Result};
use farscry_core::vasf::{VasfFile, VasfFrame};
use farscry_core::{DiffEngine, StateId};
use farscry_diff::DiffEngineImpl;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn pack_frames(input: PathBuf, output: PathBuf, hamming_threshold: u8) -> Result<()> {
    let paths = collect_image_paths(&input)?;
    let total = paths.len();
    if total == 0 {
        anyhow::bail!("no image files found in {}", input.display());
    }
    let pipeline = crate::pipeline::get_or_build_pipeline()?;
    let mut frames: Vec<VasfFrame> = Vec::new();
    let mut last_id: Option<StateId> = None;
    let mut last_vasp: Option<farscry_core::VaspOutput> = None;

    for path in &paths {
        let img =
            image::open(path).with_context(|| format!("cannot open image: {}", path.display()))?;
        let state_id = farscry_core::phash_image(&img);
        if let Some(lid) = last_id {
            if hamming(state_id, lid) <= hamming_threshold {
                continue;
            }
        }
        let (w, h) = (img.width(), img.height());
        let vasp = pipeline
            .process(img)
            .map_err(|e| anyhow::anyhow!("pipeline: {e}"))?;
        let vasp_text =
            farscry_formatter::VaspFormatter::format_vasp(&vasp, &path.to_string_lossy(), w, h);
        let delta_bytes = last_vasp.as_ref().map(|prev| {
            let delta = DiffEngineImpl.diff(prev, &vasp, None, None);
            farscry_formatter::VaspFormatter::format_diff(&delta).into_bytes()
        });
        frames.push(VasfFrame {
            state_id: vasp.state_id,
            timestamp: now_ms(),
            vasp_data: vasp_text.into_bytes(),
            delta_data: delta_bytes,
        });
        last_id = Some(state_id);
        last_vasp = Some(vasp);
    }

    let unique = frames.len();
    VasfFile::new(frames, total as u32)
        .write_to(&output)
        .with_context(|| format!("cannot write {}", output.display()))?;
    print_stats(total, unique, &output);
    Ok(())
}

fn collect_image_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read directory: {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| is_image_file(p))
        .collect();
    paths.sort();
    Ok(paths)
}

fn is_image_file(p: &Path) -> bool {
    p.is_file()
        && matches!(
            p.extension().and_then(|e| e.to_str()),
            Some("png" | "jpg" | "jpeg")
        )
}

fn hamming(a: StateId, b: StateId) -> u8 {
    (a.to_bits() ^ b.to_bits()).count_ones() as u8
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn print_stats(total: usize, unique: usize, output: &Path) {
    let dupes = total.saturating_sub(unique);
    let dedup_pct = if total > 0 { dupes * 100 / total } else { 0 };
    let tokens_raw = total * 1568;
    let tokens_vasf = unique * 175;
    let ratio = if tokens_vasf > 0 {
        tokens_raw / tokens_vasf
    } else {
        0
    };
    eprintln!(
        "[farscry] packed {} unique frames from {} total -> {}",
        unique,
        total,
        output.display()
    );
    eprintln!(
        "[farscry] deduplication: {}% of frames were duplicates",
        dedup_pct
    );
    eprintln!("[farscry] tokens without VASF: ~{tokens_raw} (at 1080p)");
    eprintln!("[farscry] tokens with VASF:    ~{tokens_vasf}");
    eprintln!("[farscry] reduction:           ~{ratio}x fewer tokens");
}
