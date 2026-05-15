use anyhow::Result;
use std::path::Path;

pub(crate) struct TerminalResult {
    pub(crate) name: &'static str,
    pub(crate) configured: bool,
    pub(crate) note: &'static str,
}

pub(super) fn backup_file(path: &Path) -> Result<()> {
    if path.exists() {
        let backup = path.with_extension(format!(
            "{}.farscry-backup",
            path.extension().and_then(|e| e.to_str()).unwrap_or("")
        ));
        std::fs::copy(path, &backup)?;
    }
    Ok(())
}

pub(crate) fn restore_backup(path: &Path) -> bool {
    let backup = path.with_extension(format!(
        "{}.farscry-backup",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    if backup.exists() {
        std::fs::copy(&backup, path).is_ok() && std::fs::remove_file(&backup).is_ok()
    } else {
        false
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn configure_iterm2(script: &Path, home: &Path) -> TerminalResult {
    let plist = home.join("Library/Preferences/com.googlecode.iterm2.plist");
    if !plist.exists() {
        return TerminalResult {
            name: "iTerm2",
            configured: false,
            note: "plist not found",
        };
    }
    if backup_file(&plist).is_err() {
        return TerminalResult {
            name: "iTerm2",
            configured: false,
            note: "backup failed",
        };
    }
    let script_str = script.to_string_lossy();
    let key = "0x76-0x100000";
    let ok = std::process::Command::new("defaults")
        .args([
            "write",
            "com.googlecode.iterm2",
            &format!("GlobalKeyMap:{key}:Action"),
            "13",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && std::process::Command::new("defaults")
            .args([
                "write",
                "com.googlecode.iterm2",
                &format!("GlobalKeyMap:{key}:Text"),
                &*script_str,
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    let _ = std::process::Command::new("killall")
        .args(["-HUP", "iTerm2"])
        .status();
    if ok {
        TerminalResult {
            name: "iTerm2",
            configured: true,
            note: "restart to apply",
        }
    } else {
        TerminalResult {
            name: "iTerm2",
            configured: false,
            note: "defaults write failed",
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn configure_warp(script: &Path, home: &Path) -> TerminalResult {
    let kb = home.join(".warp/keybindings.yaml");
    if let Some(p) = kb.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if backup_file(&kb).is_err() {
        return TerminalResult {
            name: "Warp",
            configured: false,
            note: "backup failed",
        };
    }
    let entry = format!("\n- key: cmd+v\n  command: {}\n", script.display());
    let ok = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&kb)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(entry.as_bytes())
        })
        .is_ok();
    if ok {
        TerminalResult {
            name: "Warp",
            configured: true,
            note: "active immediately",
        }
    } else {
        TerminalResult {
            name: "Warp",
            configured: false,
            note: "write failed",
        }
    }
}

pub(crate) fn configure_kitty(script: &Path, home: &Path) -> TerminalResult {
    let conf = home.join(".config/kitty/kitty.conf");
    if let Some(p) = conf.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if backup_file(&conf).is_err() {
        return TerminalResult {
            name: "Kitty",
            configured: false,
            note: "backup failed",
        };
    }
    let entry = format!("\nmap ctrl+v launch --type=overlay {}\n", script.display());
    let ok = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&conf)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(entry.as_bytes())
        })
        .is_ok();
    if ok {
        TerminalResult {
            name: "Kitty",
            configured: true,
            note: "restart to apply",
        }
    } else {
        TerminalResult {
            name: "Kitty",
            configured: false,
            note: "write failed",
        }
    }
}

pub(crate) fn configure_alacritty(script: &Path, home: &Path) -> TerminalResult {
    let yml = home.join(".config/alacritty/alacritty.yml");
    let toml = home.join(".config/alacritty/alacritty.toml");
    let (path, content) = if toml.exists() {
        (toml, format!(
            "\n[[keyboard.bindings]]\nkey = \"V\"\nmods = \"Control\"\ncommand = {{ program = \"{}\" }}\n",
            script.display()
        ))
    } else {
        (
            yml,
            format!(
                "\nkey_bindings:\n  - key: V\n    mods: Control\n    command:\n      program: {}\n",
                script.display()
            ),
        )
    };
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if backup_file(&path).is_err() {
        return TerminalResult {
            name: "Alacritty",
            configured: false,
            note: "backup failed",
        };
    }
    let ok = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(content.as_bytes())
        })
        .is_ok();
    if ok {
        TerminalResult {
            name: "Alacritty",
            configured: true,
            note: "restart to apply",
        }
    } else {
        TerminalResult {
            name: "Alacritty",
            configured: false,
            note: "write failed",
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn configure_bashrc_gnome(script: &Path, home: &Path) -> TerminalResult {
    let bashrc = home.join(".bashrc");
    if backup_file(&bashrc).is_err() {
        return TerminalResult {
            name: "Gnome Terminal",
            configured: false,
            note: "backup failed",
        };
    }
    let entry = format!("\nbind -x '\"\\C-v\": {}'\n", script.display());
    let ok = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&bashrc)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(entry.as_bytes())
        })
        .is_ok();
    if ok {
        TerminalResult {
            name: "Gnome Terminal",
            configured: true,
            note: "source ~/.bashrc to apply",
        }
    } else {
        TerminalResult {
            name: "Gnome Terminal",
            configured: false,
            note: "write failed",
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn configure_windows_terminal(script: &Path, home: &Path) -> TerminalResult {
    let settings = home.join(
        "AppData/Local/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json",
    );
    if !settings.exists() {
        return TerminalResult {
            name: "Windows Terminal",
            configured: false,
            note: "settings.json not found",
        };
    }
    if backup_file(&settings).is_err() {
        return TerminalResult {
            name: "Windows Terminal",
            configured: false,
            note: "backup failed",
        };
    }
    let raw = match std::fs::read_to_string(&settings) {
        Ok(s) => s,
        Err(_) => {
            return TerminalResult {
                name: "Windows Terminal",
                configured: false,
                note: "read failed",
            }
        }
    };
    let script_path = script.display().to_string().replace('\\', "\\\\");
    let new_action = format!(
        r#"{{ "command": {{ "action": "wt", "commandline": "powershell -Command \\"{}\\"" }}, "keys": "ctrl+v" }}"#,
        script_path
    );
    let updated = if raw.contains("\"actions\"") {
        raw.replacen(
            "\"actions\": [",
            &format!("\"actions\": [\n        {},", new_action),
            1,
        )
    } else {
        raw.replacen("}", &format!(", \"actions\": [ {} ] }}", new_action), 1)
    };
    let ok = std::fs::write(&settings, updated).is_ok();
    if ok {
        TerminalResult {
            name: "Windows Terminal",
            configured: true,
            note: "restart to apply",
        }
    } else {
        TerminalResult {
            name: "Windows Terminal",
            configured: false,
            note: "write failed",
        }
    }
}
