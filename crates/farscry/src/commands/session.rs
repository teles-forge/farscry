use anyhow::{Context, Result};
use farscry_core::vasf::VasfFile;
use std::path::PathBuf;

pub fn session_list() -> Result<()> {
    let dir = crate::util::sessions_dir();
    if !dir.exists() {
        println!("No sessions yet. Run: farscry setup --hook");
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vasf"))
        .collect();
    entries.sort_by(|a, b| b.cmp(a));
    if entries.is_empty() {
        println!("No sessions found in {}", dir.display());
        return Ok(());
    }
    for path in &entries {
        print_session_line(path);
    }
    Ok(())
}

pub fn session_latest() -> Result<()> {
    let dir = crate::util::sessions_dir();
    let latest = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vasf"))
        .max();
    let path = latest.with_context(|| "no sessions found")?;
    super::timeline::timeline(path)
}

fn print_session_line(path: &PathBuf) {
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let size_kb = std::fs::metadata(path).map(|m| m.len() / 1024).unwrap_or(0);

    match VasfFile::read_from(path) {
        Ok(vasf) => {
            let dur = format_duration(vasf.duration_ms());
            let total = vasf.total_frames();
            let unique = vasf.unique_states();
            let dedup = vasf.dedup_percentage();
            println!(
                "{filename}  {dur:>8}  {total:>6} frames  {unique:>4} unique  {dedup:.0}% dedup  {size_kb}KB"
            );
        }
        Err(_) => {
            println!("{filename}  (unreadable)  {size_kb}KB");
        }
    }
}

fn format_duration(ms: Option<i64>) -> String {
    let ms = match ms {
        Some(v) => v,
        None => return "?".to_string(),
    };
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    if mins > 0 {
        format!("{mins}m {secs:02}s")
    } else {
        format!("{secs}s")
    }
}
