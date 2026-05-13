use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use farscry_core::{Pipeline, VaspOutput};
use image::GenericImageView;
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
        Commands::Diff {
            before,
            after,
            json,
        } => diff_images(before, after, json),
        Commands::Serve { mcp, port } => serve_mcp(mcp, port).await,
        Commands::InstallLang { lang } => install_lang(lang),
        Commands::Setup => setup(),
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
            } else if e.to_string().contains("OCR failed") || e.to_string().contains("model error")
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
        let image_data = read_clipboard_png_macos()?;
        let temp_path = PathBuf::from("/tmp/farscry_clipboard.png");
        std::fs::write(&temp_path, image_data)?;

        let output = process_image(&temp_path, max_size)?;
        let (width, height) = image::open(&temp_path)
            .map(|img| img.dimensions())
            .unwrap_or((1920, 1080));
        let text = format_output(&output, "clipboard", width, height, &opts);
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

fn setup() -> Result<()> {
    let snippet = r#"{
  "mcpServers": {
    "farscry": {
      "command": "farscry",
      "args": ["serve", "--mcp"]
    }
  }
}"#;

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    let agents: &[(&str, &str)] = &[
        ("Claude Code", ".claude/mcp.json"),
        ("Cursor",      ".cursor/mcp.json"),
        ("Windsurf",    ".windsurf/mcp.json"),
        ("Zed",         ".config/zed/settings.json"),
    ];

    let mut detected: Vec<&str> = Vec::new();
    for (name, rel) in agents {
        if home.join(rel).exists() {
            detected.push(name);
        }
    }

    println!("farscry v0.1.0\n");

    if detected.is_empty() {
        println!("No MCP-compatible agents detected.");
        println!("Checked: Claude Code, Cursor, Windsurf, Zed\n");
    } else {
        println!("Detected: {}\n", detected.join(", "));
    }

    println!("Add this to your agent's MCP config (paste manually):\n");
    println!("{snippet}\n");

    println!("Config file locations:");
    for (name, rel) in agents {
        let path = home.join(rel);
        let status = if path.exists() { "found" } else { "not found" };
        println!("  {name:12} {status:10} {}", path.display());
    }

    println!("\nfarscry never modifies your config files automatically.");

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

    let is_png = magic.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
    let is_jpg = magic.starts_with(&[0xFF, 0xD8, 0xFF]);
    let is_webp = magic.starts_with(&[0x52, 0x49, 0x46, 0x46]);
    let is_gif = magic.starts_with(&[0x47, 0x49, 0x46, 0x38]);

    if !is_png && !is_jpg && !is_webp && !is_gif {
        anyhow::bail!("not an image file: {}", path.display());
    }

    let img = image::open(path)?;
    let (width, height) = img.dimensions();
    if width < 50 || height < 50 {
        anyhow::bail!("image too small: {}x{} (minimum: 50x50)", width, height);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn read_clipboard_png_macos() -> Result<Vec<u8>> {
    use std::process::Command;

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
    set d to (the clipboard as TIFF picture)
    set f to open for access POSIX file tiff with write permission
    set eof of f to 0
    write d to f
    close access f
    do shell script "sips -s format png " & quoted form of tiff & " --out " & quoted form of out
    return out
end try"#;

    let result = Command::new("osascript").arg("-e").arg(script).output()?;

    if !result.status.success() {
        anyhow::bail!("No image in clipboard");
    }

    std::fs::read("/tmp/farscry_clipboard.png").context("Failed to read clipboard image")
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

use std::io::Read;
