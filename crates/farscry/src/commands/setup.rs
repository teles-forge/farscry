use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn readline_prompt(prompt: &str) -> String {
    use std::io::Write;
    print!("{}", prompt);
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

pub fn agent_in_path(binary: &str) -> bool {
    std::process::Command::new("which")
        .arg(binary)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn setup() -> Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let zshrc = home.join(".zshrc");

    println!("farscry v0.1.0 - setup\n");

    let agents: &[(&str, &str, &str)] = &[
        (
            "claude",
            "Claude Code",
            "farscry extract --from-clipboard | claude -p \"fix this\"",
        ),
        (
            "devin",
            "Devin",
            "devin -p \"$(farscry extract --from-clipboard) - fix this\"",
        ),
        (
            "codex",
            "Codex",
            "farscry extract --from-clipboard | codex exec \"fix this:\"",
        ),
        (
            "aider",
            "Aider",
            "aider --message \"$(farscry extract --from-clipboard)\"",
        ),
    ];

    let mut detected: Vec<usize> = Vec::new();
    for (i, (bin, _, _)) in agents.iter().enumerate() {
        if agent_in_path(bin) {
            detected.push(i);
        }
    }

    if detected.is_empty() {
        println!("No agent CLIs detected in PATH.");
        println!("Checked: claude, devin, codex, aider\n");
    } else {
        let names: Vec<&str> = detected.iter().map(|&i| agents[i].1).collect();
        println!("Detected agents: {}\n", names.join(", "));
    }

    println!("Which agent do you want to use with ffix?");
    for (i, (bin, name, _)) in agents.iter().enumerate() {
        let tag = if agent_in_path(bin) {
            "(detected)"
        } else {
            "(not installed)"
        };
        println!("  {}. {} {}", i + 1, name, tag);
    }
    println!("  {}. Configure multiple aliases", agents.len() + 1);
    println!("  {}. Skip\n", agents.len() + 2);

    let choice_str = readline_prompt("Choice: ");
    let choice: usize = choice_str.parse().unwrap_or(agents.len() + 2);

    let mcp_snippet = r#"{
  "mcpServers": {
    "farscry": {
      "command": "farscry",
      "args": ["serve", "--mcp"]
    }
  }
}"#;

    if choice >= 1 && choice <= agents.len() {
        let (_, name, alias_cmd) = agents[choice - 1];
        println!("\nRun this to add ffix to your shell:\n");
        println!(
            "  echo \"alias ffix='{alias_cmd}'\" >> {} && source {}",
            zshrc.display(),
            zshrc.display()
        );
        println!("\nThen: screenshot -> type ffix -> Enter\n");

        let preferred = agents[choice - 1].0;
        crate::config::write_farscry_config(preferred, "fix this")?;
        println!("Saved preferred agent: {name}");
    } else if choice == agents.len() + 1 {
        println!("\nAdd these aliases to your shell:\n");
        for (_, name, alias_cmd) in agents {
            if name == &"Claude Code" {
                println!("  echo \"alias ffix='{alias_cmd}'\" >> {}", zshrc.display());
            } else {
                let short = name.to_lowercase().replace(' ', "-");
                println!(
                    "  echo \"alias ffix-{short}='{alias_cmd}'\" >> {}",
                    zshrc.display()
                );
            }
        }
        println!("  source {}\n", zshrc.display());
    } else {
        println!("\nSkipped alias setup.\n");
    }

    println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("Zero-friction alias (recommended):\n");
    println!(
        "  echo \"alias fp='farscry paste'\" >> {} && source {}",
        zshrc.display(),
        zshrc.display()
    );
    println!("\nThen: screenshot -> fp -> done.\n");

    println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("Visual debug alias (opens annotated image):\n");
    println!(
        "  echo \"alias fannot='farscry annotate --from-clipboard -o /tmp/farscry_annotated.png && open /tmp/farscry_annotated.png'\" >> {}",
        zshrc.display()
    );
    println!("  source {}\n", zshrc.display());
    println!("Then: screenshot -> fannot -> annotated image opens.\n");

    println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("Smart paste - Cmd+V auto-detects images\n");
    println!("Configure Cmd+V to run farscry automatically");
    println!("when clipboard contains an image?\n");
    println!("  y = create script + show terminal instructions");
    println!("  n = skip\n");

    let sp = readline_prompt("Configure smart paste? [y/N]: ");
    if sp.eq_ignore_ascii_case("y") {
        setup_smart_paste(&home)?;
    }

    println!("\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("MCP integration (automatic, no alias needed):\n");
    println!("{mcp_snippet}\n");

    let mcp_agents: &[(&str, &str)] = &[
        ("Claude Code", ".claude/mcp.json"),
        ("Cursor", ".cursor/mcp.json"),
        ("Windsurf", ".windsurf/mcp.json"),
        ("Zed", ".config/zed/settings.json"),
    ];
    for (name, rel) in mcp_agents {
        let path = home.join(rel);
        let status = if path.exists() { "found" } else { "not found" };
        println!("  {name:12} {status:10} {}", path.display());
    }
    println!("\nfarscry never modifies your config files automatically.\n");

    println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("Setup complete.\n");
    println!("Summary:");
    println!("  ffix   -> farscry + your agent (one command)");
    println!("  fp     -> farscry paste (smart, uses saved config)");
    println!("  fannot -> annotate screenshot, opens image");
    println!("  Cmd+V  -> auto-detects images (if configured above)\n");

    let open = readline_prompt(&format!("Open {} in your editor? (y/N) ", zshrc.display()));
    if open.eq_ignore_ascii_case("y") {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "open".to_string());
        let _ = std::process::Command::new(&editor).arg(&zshrc).spawn();
    }

    Ok(())
}

pub struct TerminalResult {
    pub name: &'static str,
    pub configured: bool,
    pub note: &'static str,
}

fn backup_file(path: &Path) -> Result<()> {
    if path.exists() {
        let backup = path.with_extension(format!(
            "{}.farscry-backup",
            path.extension().and_then(|e| e.to_str()).unwrap_or("")
        ));
        std::fs::copy(path, &backup)?;
    }
    Ok(())
}

fn restore_backup(path: &Path) -> bool {
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

fn cmd_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn configure_iterm2(script: &Path, home: &Path) -> TerminalResult {
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
fn configure_warp(script: &Path, home: &Path) -> TerminalResult {
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

fn configure_kitty(script: &Path, home: &Path) -> TerminalResult {
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

fn configure_alacritty(script: &Path, home: &Path) -> TerminalResult {
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
fn configure_bashrc_gnome(script: &Path, home: &Path) -> TerminalResult {
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
fn configure_windows_terminal(script: &Path, home: &Path) -> TerminalResult {
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

pub fn write_smart_paste_script(farscry_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(farscry_dir)?;

    #[cfg(not(target_os = "windows"))]
    let script_path = farscry_dir.join("smart-paste.sh");
    #[cfg(target_os = "windows")]
    let script_path = farscry_dir.join("smart-paste.ps1");

    #[cfg(target_os = "macos")]
    let content = r#"#!/bin/bash
HAS_IMAGE=$(osascript -e '
  try
    set img to the clipboard as «class PNGf»
    return "yes"
  end try
  try
    set img to the clipboard as TIFF picture
    return "yes"
  end try
  return "no"
')
if [ "$HAS_IMAGE" = "yes" ]; then
    farscry paste
else
    pbpaste
fi
"#;

    #[cfg(target_os = "linux")]
    let content = r#"#!/bin/bash
if command -v xclip &>/dev/null; then
    HAS_IMAGE=$(xclip -selection clipboard -t TARGETS -o 2>/dev/null | grep -c "image/")
    if [ "$HAS_IMAGE" -gt 0 ]; then
        farscry paste
    else
        xclip -selection clipboard -o
    fi
elif command -v wl-paste &>/dev/null; then
    HAS_IMAGE=$(wl-paste --list-types 2>/dev/null | grep -c "image/")
    if [ "$HAS_IMAGE" -gt 0 ]; then
        farscry paste
    else
        wl-paste
    fi
else
    echo "Install xclip or wl-clipboard for smart paste"
fi
"#;

    #[cfg(target_os = "windows")]
    let content = r#"$formats = [System.Windows.Forms.Clipboard]::GetDataObject().GetFormats()
$hasImage = $formats | Where-Object { $_ -match "Bitmap|PNG|image" }
if ($hasImage) {
    farscry paste
} else {
    Get-Clipboard
}
"#;

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let content = "#!/bin/sh\necho 'Platform not supported'\n";

    std::fs::write(&script_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(script_path)
}

pub fn setup_smart_paste(home: &Path) -> Result<()> {
    let farscry_dir = home.join(".farscry");
    let script = write_smart_paste_script(&farscry_dir)?;
    println!("\nCreated: {}\n", script.display());

    let mut detected: Vec<(&str, bool)> = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let iterm2 = Path::new("/Applications/iTerm.app").exists();
        let warp = Path::new("/Applications/Warp.app").exists();
        let kitty = cmd_exists("kitty") || home.join(".config/kitty/kitty.conf").exists();
        let alac = Path::new("/Applications/Alacritty.app").exists() || cmd_exists("alacritty");
        detected.push(("iTerm2", iterm2));
        detected.push(("Warp", warp));
        detected.push(("Kitty", kitty));
        detected.push(("Alacritty", alac));
        detected.push(("Terminal.app", true));
    }

    #[cfg(target_os = "linux")]
    {
        detected.push(("Kitty", cmd_exists("kitty")));
        detected.push(("Gnome Terminal", cmd_exists("gnome-terminal")));
        detected.push(("Alacritty", cmd_exists("alacritty")));
        detected.push(("Konsole", cmd_exists("konsole")));
        detected.push(("Tilix", cmd_exists("tilix")));
    }

    #[cfg(target_os = "windows")]
    {
        let wt = home.join("AppData/Local/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json");
        detected.push(("Windows Terminal", wt.exists()));
    }

    let found: Vec<&str> = detected
        .iter()
        .filter(|(_, d)| *d)
        .map(|(n, _)| *n)
        .collect();

    if found.is_empty() {
        println!("No terminals detected on your system.");
        return Ok(());
    }

    println!("Found terminals on your system:");
    for (name, det) in &detected {
        if *det {
            let limited = *name == "Terminal.app";
            if limited {
                println!("  \u{2713} {} (limited support)", name);
            } else {
                println!("  \u{2713} {}", name);
            }
        }
    }

    println!("\nConfigure smart Cmd+V in all of them? (y/N)");
    let all = readline_prompt("Configure all? [y/N]: ");

    let to_configure: Vec<&str> = if all.eq_ignore_ascii_case("y") {
        found.clone()
    } else {
        println!("\nWhich terminals to configure?");
        for (i, name) in found.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
        println!("  {}. All of the above", found.len() + 1);
        println!("  {}. Skip", found.len() + 2);

        let choice_str = readline_prompt("Choice: ");
        let choice: usize = choice_str.parse().unwrap_or(found.len() + 2);

        if choice == found.len() + 1 {
            found.clone()
        } else if choice >= 1 && choice <= found.len() {
            vec![found[choice - 1]]
        } else {
            println!("Skipped.");
            return Ok(());
        }
    };

    let mut results: Vec<TerminalResult> = Vec::new();

    for name in &to_configure {
        let result = match *name {
            #[cfg(target_os = "macos")]
            "iTerm2" => configure_iterm2(&script, home),
            #[cfg(target_os = "macos")]
            "Warp" => configure_warp(&script, home),
            "Kitty" => configure_kitty(&script, home),
            "Alacritty" => configure_alacritty(&script, home),
            #[cfg(target_os = "linux")]
            "Gnome Terminal" => configure_bashrc_gnome(&script, home),
            #[cfg(target_os = "windows")]
            "Windows Terminal" => configure_windows_terminal(&script, home),
            "Terminal.app" => TerminalResult {
                name: "Terminal.app",
                configured: false,
                note: "not supported - use fp alias instead",
            },
            other => TerminalResult {
                name: other,
                configured: false,
                note: "unknown terminal",
            },
        };
        results.push(result);
    }

    println!("\nSmart paste configured for:\n");
    for r in &results {
        if r.configured {
            println!("  \u{2713} {:16} -> {}", r.name, r.note);
        } else {
            println!("  \u{2717} {:16} -> {}", r.name, r.note);
        }
    }

    println!("\nTo undo all changes: farscry setup --undo-smart-paste\n");
    println!("Restart your terminal and Cmd+V will");
    println!("automatically detect images.\n");
    println!("Try it: take a screenshot -> press Cmd+V");

    Ok(())
}

pub fn undo_smart_paste_configs(home: &Path) -> Result<()> {
    let mut restored = 0usize;

    let candidates: &[(&str, &[&str])] = &[
        ("iTerm2 plist", &["Library/Preferences/com.googlecode.iterm2.plist"]),
        ("Warp keybindings", &[".warp/keybindings.yaml"]),
        ("kitty.conf", &[".config/kitty/kitty.conf"]),
        ("alacritty", &[".config/alacritty/alacritty.yml", ".config/alacritty/alacritty.toml"]),
        ("~/.bashrc", &[".bashrc"]),
        ("Windows Terminal settings", &["AppData/Local/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json"]),
    ];

    for (label, paths) in candidates {
        for rel in *paths {
            let p = home.join(rel);
            if restore_backup(&p) {
                println!("  \u{2713} Restored {label}");
                restored += 1;
                break;
            }
        }
    }

    if restored == 0 {
        println!("No backups found - nothing to restore.");
    } else {
        println!("\n\u{2713} All terminal configs restored to original.");
    }
    Ok(())
}
