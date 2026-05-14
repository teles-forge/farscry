#![cfg(target_os = "windows")]

use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub fn read_clipboard_image_windows() -> Result<Vec<u8>> {
    use std::process::Command;

    // Try to get PNG data from clipboard via PowerShell
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$img = [System.Windows.Forms.Clipboard]::GetImage()
if ($img -eq $null) { exit 1 }
$ms = New-Object System.IO.MemoryStream
$img.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
[Convert]::ToBase64String($ms.ToArray())
"#;

    let result = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output();

    match result {
        Ok(out) if out.status.success() && !out.stdout.is_empty() => {
            let b64 = String::from_utf8_lossy(&out.stdout);
            let b64 = b64.trim();
            let bytes = STANDARD
                .decode(b64)
                .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))?;
            Ok(bytes)
        }
        _ => bail!("No image in clipboard or PowerShell not available"),
    }
}
