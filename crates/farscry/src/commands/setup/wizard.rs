use anyhow::Result;
use std::path::PathBuf;

pub fn readline_prompt(prompt: &str) -> String {
    use std::io::Write;
    print!("{}", prompt);
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

pub(crate) fn agent_in_path(binary: &str) -> bool {
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
        super::smart_paste::setup_smart_paste(&home)?;
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
