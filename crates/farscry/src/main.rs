use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use farscry_core::{Pipeline, VaspOutput};
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, OnceLock};

#[derive(Parser)]
#[command(name = "farscry")]
#[command(version = "0.1.0")]
#[command(about = "Visual automation workflow Protocol CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    Extract {
        #[arg(required = false)]
        paths: Vec<PathBuf>,

        #[arg(long)]
        from_clipboard: bool,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        affordances: bool,

        #[arg(long)]
        text_only: bool,

        #[arg(long)]
        context: bool,

        #[arg(long, default_value = "eng")]
        lang: String,

        #[arg(long, default_value = "10")]
        max_size: u64,

        #[arg(short = 'o', long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    Diff {
        before: PathBuf,
        after: PathBuf,

        #[arg(long)]
        json: bool,
    },

    Serve {
        #[arg(long)]
        mcp: bool,

        #[arg(long)]
        port: Option<u16>,
    },

    InstallLang {
        #[arg(required = true)]
        lang: Vec<String>,
    },

    Setup,

    Paste {
        #[arg(long)]
        agent: Option<String>,

        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Extract {
            paths,
            from_clipboard,
            json,
            affordances,
            text_only,
            context,
            lang,
            max_size,
            output,
        } => {
            let max_size_bytes = max_size * 1024 * 1024;
            let opts = ExtractOpts {
                json,
                affordances,
                text_only,
                context,
                output,
            };
            if from_clipboard {
                extract_from_clipboard(opts, &lang, max_size_bytes)
            } else if paths.is_empty() {
                extract_from_stdin(opts, &lang, max_size_bytes)
            } else {
                extract_images(paths, opts, &lang, max_size_bytes)
            }
        }
        Commands::Diff { before, after, json } => diff_images(before, after, json),
        Commands::Serve { mcp, port } => serve_mcp(mcp, port).await,
        Commands::InstallLang { lang } => install_lang(lang),
        Commands::Setup => setup(),
        Commands::Paste { agent, prompt } => {
            let prompt_str = if prompt.is_empty() {
                None
            } else {
                Some(prompt.join(" "))
            };
            paste(agent.as_deref(), prompt_str.as_deref())
        }
    };

    match result {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            let exit_code = if e.to_string().contains("file not found")
                || e.to_string().contains("invalid input")
                || e.to_string().contains("not an image")
            {
                1
            } else if e.to_string().contains("OCR failed")
                || e.to_string().contains("model error")
            {
                2
            } else if e.to_string().contains("language not installed")
                || e.to_string().contains("configuration")
            {
                3
            } else {
                1
            };
            process::exit(exit_code);
        }
    }
}

struct ExtractOpts {
    json: bool,
    affordances: bool,
    text_only: bool,
    context: bool,
    output: Option<PathBuf>,
}

fn write_output(content: &str, output_file: Option<&PathBuf>) -> Result<()> {
    match output_file {
        Some(path) => std::fs::write(path, content)
            .with_context(|| format!("Failed to write output to {}", path.display())),
        None => {
            print!("{content}");
            Ok(())
        }
    }
}

fn format_output(
    output: &farscry_core::VaspOutput,
    source: &str,
    width: u32,
    height: u32,
    opts: &ExtractOpts,
) -> String {
    if opts.json {
        farscry_formatter::VaspFormatter::format_json(output, true)
    } else if opts.text_only {
        farscry_formatter::VaspFormatter::format_text_only(output)
    } else if opts.context {
        output.agent_context.clone()
    } else {
        farscry_formatter::VaspFormatter::format_vasp_with_options(
            output,
            source,
            width,
            height,
            opts.affordances,
        )
    }
}

fn extract_images(
    paths: Vec<PathBuf>,
    opts: ExtractOpts,
    _lang: &str,
    max_size: u64,
) -> Result<()> {
    for path in &paths {
        validate_image(path, max_size)?;
    }

    let pipeline = get_or_build_pipeline()?;
    let results = pipeline.process_batch(paths.clone());

    let mut combined = String::new();
    for (i, batch_result) in results.into_iter().enumerate() {
        if i > 0 {
            combined.push_str("---\n");
        }
        let path = &paths[i];
        let output = batch_result
            .output
            .map_err(|e| anyhow::anyhow!("{}: {}", path.display(), e))?;
        let (width, height) = image::open(path)
            .map(|img| img.dimensions())
            .unwrap_or((1920, 1080));
        let text = format_output(&output, &path.to_string_lossy(), width, height, &opts);
        combined.push_str(&text);
        if !text.ends_with('\n') {
            combined.push('\n');
        }
    }

    write_output(&combined, opts.output.as_ref())
}

fn extract_from_clipboard(opts: ExtractOpts, _lang: &str, max_size: u64) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let (image_data, source_label) = read_clipboard_image_macos()?;
        let temp_path = PathBuf::from("/tmp/farscry_clipboard.png");
        std::fs::write(&temp_path, image_data)?;

        let output = process_image(&temp_path, max_size)?;
        let (width, height) = image::open(&temp_path)
            .map(|img| img.dimensions())
            .unwrap_or((1920, 1080));
        let text = format_output(&output, &source_label, width, height, &opts);
        write_output(&text, opts.output.as_ref())
    }

    #[cfg(target_os = "linux")]
    {
        let image_data = read_clipboard_png_linux()?;
        let temp_path = PathBuf::from("/tmp/farscry_clipboard.png");
        std::fs::write(&temp_path, image_data)?;

        let output = process_image(&temp_path, max_size)?;
        let (width, height) = image::open(&temp_path)
            .map(|img| img.dimensions())
            .unwrap_or((1920, 1080));
        let text = format_output(&output, "clipboard", width, height, &opts);
        write_output(&text, opts.output.as_ref())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("--from-clipboard not supported on this platform");
    }
}

fn extract_from_stdin(opts: ExtractOpts, _lang: &str, max_size: u64) -> Result<()> {
    let temp_path = PathBuf::from("/tmp/farscry_stdin.png");
    let mut stdin = std::io::stdin();
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer)?;

    std::fs::write(&temp_path, buffer)?;

    let output = process_image(&temp_path, max_size)?;
    let (width, height) = image::open(&temp_path)
        .map(|img| img.dimensions())
        .unwrap_or((1920, 1080));
    let text = format_output(&output, "stdin", width, height, &opts);
    write_output(&text, opts.output.as_ref())
}

fn diff_images(before: PathBuf, after: PathBuf, json: bool) -> Result<()> {
    let before_dims = image::open(&before).ok().map(|i| (i.width(), i.height()));
    let after_dims = image::open(&after).ok().map(|i| (i.width(), i.height()));

    let before_output = process_image(&before, 10_000_000)?;
    let after_output = process_image(&after, 10_000_000)?;

    let engine = farscry_diff::DiffEngineImpl;
    use farscry_core::DiffEngine;
    let delta = engine.diff(&before_output, &after_output, before_dims, after_dims);

    if json {
        let json_output = serde_json::to_string_pretty(&delta)?;
        println!("{}", json_output);
    } else {
        let delta_text = farscry_formatter::VaspFormatter::format_diff(&delta);
        print!("{}", delta_text);
    }

    Ok(())
}

#[derive(Clone)]
struct FarscryPipelineAdapter {
    pipeline: Arc<Pipeline>,
}

impl farscry_mcp::PipelineOps for FarscryPipelineAdapter {
    fn process(&self, image_path: &str) -> Result<farscry_core::VaspOutput, String> {
        let img = image::open(image_path).map_err(|e| format!("cannot open image: {e}"))?;
        self.pipeline.process(img).map_err(|e| e.to_string())
    }

    fn diff(
        &self,
        before: &farscry_core::VaspOutput,
        after: &farscry_core::VaspOutput,
        before_dims: Option<(u32, u32)>,
        after_dims: Option<(u32, u32)>,
    ) -> farscry_core::VaspDelta {
        use farscry_core::DiffEngine;
        farscry_diff::DiffEngineImpl.diff(before, after, before_dims, after_dims)
    }
}

async fn serve_mcp(mcp: bool, port: Option<u16>) -> Result<()> {
    if !mcp {
        anyhow::bail!("Only MCP mode is currently supported");
    }

    let pipeline =
        get_or_build_pipeline().map_err(|e| anyhow::anyhow!("Pipeline init failed: {e}"))?;
    let adapter = FarscryPipelineAdapter { pipeline };

    if let Some(port) = port {
        farscry_mcp::McpServer::serve_tcp_with(port, adapter)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    } else {
        #[cfg(unix)]
        {
            let socket_path = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".farscry")
                .join("mcp.sock");
            farscry_mcp::McpServer::serve_unix_with(&socket_path, adapter)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        #[cfg(not(unix))]
        {
            anyhow::bail!(
                "Unix Domain Sockets are not supported on Windows. Use --port to specify a TCP port."
            );
        }
    }

    Ok(())
}

fn install_lang(langs: Vec<String>) -> Result<()> {
    let models_dir = resolve_models_dir();
    if let Some(lang) = langs.first() {
        eprintln!("[farscry] Installing language model: {lang}");
        eprintln!(
            "[farscry] Place model files manually at: {}",
            models_dir.display()
        );
        return Err(anyhow::anyhow!(
            "language not installed: {lang}. Multi-language support arrives in v0.2."
        ));
    }
    Ok(())
}

fn agent_in_path(binary: &str) -> bool {
    std::process::Command::new("which")
        .arg(binary)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn readline_prompt(prompt: &str) -> String {
    use std::io::Write;
    print!("{}", prompt);
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

fn setup() -> Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let zshrc = home.join(".zshrc");

    println!("farscry v0.1.0 — setup\n");

    let agents: &[(&str, &str, &str)] = &[
        ("claude", "Claude Code", "farscry extract --from-clipboard | claude -p \"fix this\""),
        ("devin",  "Devin",       "devin -p \"$(farscry extract --from-clipboard) — fix this\""),
        ("codex",  "Codex",       "farscry extract --from-clipboard | codex exec \"fix this:\""),
        ("aider",  "Aider",       "aider --message \"$(farscry extract --from-clipboard)\""),
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
        let tag = if agent_in_path(bin) { "(detected)" } else { "(not installed)" };
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
        println!("\nThen: screenshot → type ffix → Enter\n");

        let preferred = agents[choice - 1].0;
        write_farscry_config(preferred, "fix this")?;
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

    println!("─────────────────────────────────────────");
    println!("Zero-friction alias (recommended):\n");
    println!("  echo \"alias fp='farscry paste'\" >> {} && source {}", zshrc.display(), zshrc.display());
    println!("\nThen: screenshot → fp → done.\n");

    println!("─────────────────────────────────────────");
    println!("MCP integration (automatic, no alias needed):\n");
    println!("{mcp_snippet}\n");

    let mcp_agents: &[(&str, &str)] = &[
        ("Claude Code", ".claude/mcp.json"),
        ("Cursor",      ".cursor/mcp.json"),
        ("Windsurf",    ".windsurf/mcp.json"),
        ("Zed",         ".config/zed/settings.json"),
    ];
    for (name, rel) in mcp_agents {
        let path = home.join(rel);
        let status = if path.exists() { "found" } else { "not found" };
        println!("  {name:12} {status:10} {}", path.display());
    }
    println!("\nfarscry never modifies your config files automatically.\n");

    let open = readline_prompt(&format!("Open {} in your editor? (y/N) ", zshrc.display()));
    if open.eq_ignore_ascii_case("y") {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "open".to_string());
        let _ = std::process::Command::new(&editor).arg(&zshrc).spawn();
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct FarscryConfig {
    agent: Option<AgentConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentConfig {
    preferred: String,
    default_prompt: String,
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".farscry")
        .join("config.toml")
}

fn read_farscry_config() -> FarscryConfig {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_farscry_config(agent: &str, default_prompt: &str) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cfg = FarscryConfig {
        agent: Some(AgentConfig {
            preferred: agent.to_string(),
            default_prompt: default_prompt.to_string(),
        }),
    };
    let content = toml::to_string_pretty(&cfg)?;
    std::fs::write(&path, content)?;
    Ok(())
}

fn paste(agent_override: Option<&str>, prompt_override: Option<&str>) -> Result<()> {
    let cfg = read_farscry_config();

    let agent = if let Some(a) = agent_override {
        a.to_string()
    } else if let Some(ref a) = cfg.agent {
        a.preferred.clone()
    } else {
        let choice = readline_prompt(
            "Which agent? (claude / devin / codex) [claude]: "
        );
        let chosen = if choice.is_empty() {
            "claude".to_string()
        } else {
            choice
        };
        let prompt_default = prompt_override.unwrap_or("fix this").to_string();
        write_farscry_config(&chosen, &prompt_default)?;
        println!("Saved. Next time just run: farscry paste\n");
        chosen
    };

    let prompt = prompt_override
        .map(|s| s.to_string())
        .or_else(|| cfg.agent.as_ref().map(|a| a.default_prompt.clone()))
        .unwrap_or_else(|| "fix this".to_string());

    let vasp = capture_clipboard_vasp()?;

    dispatch_to_agent(&agent, &vasp, &prompt)
}

fn capture_clipboard_vasp() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        let (image_data, _) = read_clipboard_image_macos()?;
        let temp_path = PathBuf::from("/tmp/farscry_paste.png");
        std::fs::write(&temp_path, image_data)?;
        let output = process_image(&temp_path, 50_000_000)?;
        let (w, h) = image::open(&temp_path)
            .map(|i| i.dimensions())
            .unwrap_or((1920, 1080));
        Ok(farscry_formatter::VaspFormatter::format_vasp_with_options(
            &output, "clipboard", w, h, true,
        ))
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("farscry paste currently requires macOS");
    }
}

fn dispatch_to_agent(agent: &str, vasp: &str, prompt: &str) -> Result<()> {
    match agent {
        "claude" => {
            let mut child = std::process::Command::new("claude")
                .args(["-p", prompt])
                .stdin(std::process::Stdio::piped())
                .spawn()
                .context("claude not found in PATH")?;
            if let Some(stdin) = child.stdin.take() {
                use std::io::Write;
                let mut w = stdin;
                writeln!(w, "{vasp}")?;
            }
            child.wait()?;
        }
        "devin" => {
            let full_prompt = format!("{vasp}\n\n{prompt}");
            std::process::Command::new("devin")
                .args(["-p", &full_prompt])
                .status()
                .context("devin not found in PATH")?;
        }
        "codex" => {
            let mut child = std::process::Command::new("codex")
                .args(["exec", prompt])
                .stdin(std::process::Stdio::piped())
                .spawn()
                .context("codex not found in PATH")?;
            if let Some(stdin) = child.stdin.take() {
                use std::io::Write;
                let mut w = stdin;
                writeln!(w, "{vasp}")?;
            }
            child.wait()?;
        }
        other => {
            anyhow::bail!(
                "Unknown agent: {other}. Supported: claude, devin, codex"
            );
        }
    }
    Ok(())
}

static PIPELINE: OnceLock<Arc<Pipeline>> = OnceLock::new();

fn resolve_models_dir() -> PathBuf {
    if let Ok(p) = std::env::var("FARSCRY_MODELS_DIR") {
        let p = PathBuf::from(p);
        if p.exists() {
            return p;
        }
    }

    if let Some(home) = dirs::home_dir() {
        let p = home.join(".farscry").join("models");
        if p.exists() {
            return p;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        let p = exe.parent().unwrap_or(Path::new(".")).join("models");
        if p.exists() {
            return p;
        }
    }

    let dev = PathBuf::from("spike").join("models");
    if dev.exists() {
        return dev;
    }

    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".farscry")
        .join("models")
}

fn get_or_build_pipeline() -> Result<Arc<Pipeline>> {
    if let Some(p) = PIPELINE.get() {
        return Ok(Arc::clone(p));
    }

    let models_dir = resolve_models_dir();

    let ocr = farscry_ocr::build_ocr_engine(&models_dir).map_err(|e| {
        anyhow::anyhow!(
            "OCR engine init failed: {e}\n\
            Tip: run `farscry setup` or set FARSCRY_MODELS_DIR"
        )
    })?;

    let pipeline = Arc::new(Pipeline::new(
        Arc::new(IdentityPreprocessor),
        Arc::new(ocr),
        Arc::new(farscry_classifier::Classifier),
        Arc::new(farscry_classifier::Classifier),
        Arc::new(PHashStateHasher),
        Arc::new(DefaultVaspFormatter),
    ));

    let _ = PIPELINE.set(Arc::clone(&pipeline));
    Ok(pipeline)
}

struct IdentityPreprocessor;
impl farscry_core::Preprocessor for IdentityPreprocessor {
    fn process(&self, image: image::DynamicImage) -> image::DynamicImage {
        image
    }
}

struct PHashStateHasher;
impl farscry_core::StateHasher for PHashStateHasher {
    fn hash(&self, image: &image::DynamicImage) -> farscry_core::StateId {
        farscry_core::phash_image(image)
    }
}

struct DefaultVaspFormatter;
impl farscry_core::VaspFormatter for DefaultVaspFormatter {
    fn format(&self, screen: &farscry_core::ClassifiedScreen) -> VaspOutput {
        let ctx: String = screen
            .ui_tree
            .iter()
            .filter(|e| !e.text.is_empty())
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join(" • ");
        let ctx = if ctx.len() > 120 {
            format!("{}…", &ctx[..120])
        } else {
            ctx
        };

        VaspOutput::new(
            screen.state_id,
            screen.screen_type,
            screen.confidence,
            &screen.lang,
            ctx,
            screen.ui_tree.clone(),
            vec![],
        )
    }
}

fn process_image(path: &Path, max_size: u64) -> Result<VaspOutput> {
    validate_image(path, max_size)?;

    let pipeline = get_or_build_pipeline()?;
    let img =
        image::open(path).with_context(|| format!("cannot open image: {}", path.display()))?;

    pipeline
        .process(img)
        .map_err(|e| anyhow::anyhow!("pipeline failed: {e}"))
}

fn validate_image(path: &Path, max_size: u64) -> Result<()> {
    if !path.is_file() {
        anyhow::bail!("file not found: {}", path.display());
    }

    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();
    if file_size > max_size {
        anyhow::bail!(
            "file too large: {} bytes (max: {} bytes)",
            file_size,
            max_size
        );
    }

    let mut file = std::fs::File::open(path)?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;

    let is_png  = magic.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
    let is_jpg  = magic.starts_with(&[0xFF, 0xD8, 0xFF]);
    let is_webp = magic.starts_with(&[0x52, 0x49, 0x46, 0x46]);
    let is_gif  = magic.starts_with(&[0x47, 0x49, 0x46, 0x38]);
    let is_tiff = magic.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || magic.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]);
    let is_pdf  = magic.starts_with(b"%PDF");
    let is_svg  = magic.starts_with(b"<svg") || magic.starts_with(b"<?xm");

    if is_pdf {
        anyhow::bail!("PDF not supported. Export as PNG first.");
    }
    if is_svg {
        anyhow::bail!("SVG not supported. Export as PNG first.");
    }
    if !is_png && !is_jpg && !is_webp && !is_gif && !is_tiff {
        anyhow::bail!("not an image file: {}", path.display());
    }

    let img = image::open(path)?;
    let (width, height) = img.dimensions();
    if width < 50 || height < 50 {
        anyhow::bail!("image too small: {}x{} (minimum: 50x50)", width, height);
    }

    Ok(())
}

fn check_clipboard_file_path(text: &str) -> Option<PathBuf> {
    let path = PathBuf::from(text.trim());
    if path.exists() && path.is_file() {
        return Some(path);
    }
    None
}

fn supported_image_extension(path: &Path) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "tiff" | "tif" => Ok(()),
        "pdf" => anyhow::bail!("PDF not supported. Export as PNG first."),
        "svg" => anyhow::bail!("SVG not supported. Export as PNG first."),
        other => anyhow::bail!(
            "File type .{other} not supported. Use PNG or JPG."
        ),
    }
}

#[cfg(target_os = "macos")]
fn read_clipboard_image_macos() -> Result<(Vec<u8>, String)> {
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

    if type_str.contains("«class utf8»")
        || type_str.contains("«class utxt»")
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
            let label = file_path.file_name()
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

    let data = std::fs::read("/tmp/farscry_clipboard.png")
        .context("Failed to read clipboard image")?;
    Ok((data, "clipboard".to_string()))
}

#[cfg(target_os = "linux")]
fn read_clipboard_png_linux() -> Result<Vec<u8>> {
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

    anyhow::bail!("No image in clipboard (requires xclip or wl-paste)")
}
