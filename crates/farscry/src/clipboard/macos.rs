#![cfg(target_os = "macos")]

use super::{check_clipboard_file_path, supported_image_extension};
use anyhow::{Context, Result};

pub fn read_clipboard_image_macos() -> Result<(Vec<u8>, String)> {
    use std::process::Command;

    let type_script = r#"
set cTypes to (clipboard info)
set typeList to {}
repeat with t in cTypes
    set end of typeList to (class of t) as string
end repeat
return typeList as string"#;

    let type_result = Command::new("osascript")
        .arg("-e")
        .arg(type_script)
        .output()?;
    let type_str = String::from_utf8_lossy(&type_result.stdout).to_lowercase();

    if type_str.contains("\u{ab}class utf8\u{bb}")
        || type_str.contains("\u{ab}class utxt\u{bb}")
        || type_str.contains("string")
    {
        let text_script = r#"return (the clipboard as string)"#;
        let text_result = Command::new("osascript")
            .arg("-e")
            .arg(text_script)
            .output()?;
        let clipboard_text = String::from_utf8_lossy(&text_result.stdout);
        let text = clipboard_text.trim();

        if text.is_empty() {
            anyhow::bail!("Clipboard is empty.");
        }

        if let Some(file_path) = check_clipboard_file_path(text) {
            supported_image_extension(&file_path)?;
            let data = std::fs::read(&file_path)?;
            let label = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            return Ok((data, label));
        }

        anyhow::bail!("Clipboard contains text, not an image.");
    }

    if type_str.contains("pdf") {
        anyhow::bail!("PDF not supported. Export as PNG first.");
    }

    let script = r#"
set out to "/tmp/farscry_clipboard.png"
set tiff to "/tmp/farscry_clipboard.tiff"
try
    set d to (the clipboard as «class PNGf»)
    set f to open for access POSIX file out with write permission
    set eof of f to 0
    write d to f
    close access f
    return out
on error
    try
        set d to (the clipboard as TIFF picture)
        set f to open for access POSIX file tiff with write permission
        set eof of f to 0
        write d to f
        close access f
        do shell script "sips -s format png " & quoted form of tiff & " --out " & quoted form of out
        return out
    on error
        return ""
    end try
end try"#;

    let result = Command::new("osascript").arg("-e").arg(script).output()?;

    if !result.status.success() || result.stdout.is_empty() {
        anyhow::bail!("Clipboard is empty.");
    }

    let out_path = String::from_utf8_lossy(&result.stdout).trim().to_string();
    if out_path.is_empty() {
        anyhow::bail!("Clipboard is empty.");
    }

    let data =
        std::fs::read("/tmp/farscry_clipboard.png").context("Failed to read clipboard image")?;
    Ok((data, "clipboard".to_string()))
}
