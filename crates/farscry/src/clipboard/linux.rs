#![cfg(target_os = "linux")]

use anyhow::{bail, Result};

pub fn read_clipboard_png_linux() -> Result<Vec<u8>> {
    use std::process::Command;

    if let Ok(result) = Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "image/png", "-o"])
        .output()
    {
        if result.status.success() && !result.stdout.is_empty() {
            return Ok(result.stdout);
        }
    }

    if let Ok(result) = Command::new("wl-paste")
        .args(["--type", "image/png"])
        .output()
    {
        if result.status.success() && !result.stdout.is_empty() {
            return Ok(result.stdout);
        }
    }

    bail!("No image in clipboard (requires xclip or wl-paste)")
}
