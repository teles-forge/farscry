use anyhow::{Context, Result};
use image::GenericImageView;
use std::io::Read;
use std::path::PathBuf;

pub struct ExtractOpts {
    pub json: bool,
    pub affordances: bool,
    pub text_only: bool,
    pub context: bool,
    pub output: Option<PathBuf>,
}

pub fn write_output(content: &str, output_file: Option<&PathBuf>) -> Result<()> {
    match output_file {
        Some(path) => std::fs::write(path, content)
            .with_context(|| format!("Failed to write output to {}", path.display())),
        None => {
            print!("{content}");
            Ok(())
        }
    }
}

pub fn format_output(
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

pub fn extract_images(
    paths: Vec<PathBuf>,
    opts: ExtractOpts,
    _lang: &str,
    max_size: u64,
) -> Result<()> {
    for path in &paths {
        crate::pipeline::validate_image(path, max_size)?;
    }

    let pipeline = crate::pipeline::get_or_build_pipeline()?;
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

pub fn extract_from_clipboard(opts: ExtractOpts, _lang: &str, max_size: u64) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let (image_data, source_label) = crate::clipboard::macos::read_clipboard_image_macos()?;
        let temp_path = PathBuf::from("/tmp/farscry_clipboard.png");
        std::fs::write(&temp_path, image_data)?;

        let output = crate::pipeline::process_image(&temp_path, max_size)?;
        let (width, height) = image::open(&temp_path)
            .map(|img| img.dimensions())
            .unwrap_or((1920, 1080));
        let text = format_output(&output, &source_label, width, height, &opts);
        write_output(&text, opts.output.as_ref())
    }

    #[cfg(target_os = "linux")]
    {
        let image_data = crate::clipboard::linux::read_clipboard_png_linux()?;
        let temp_path = PathBuf::from("/tmp/farscry_clipboard.png");
        std::fs::write(&temp_path, image_data)?;

        let output = crate::pipeline::process_image(&temp_path, max_size)?;
        let (width, height) = image::open(&temp_path)
            .map(|img| img.dimensions())
            .unwrap_or((1920, 1080));
        let text = format_output(&output, "clipboard", width, height, &opts);
        write_output(&text, opts.output.as_ref())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (opts, max_size);
        anyhow::bail!("--from-clipboard not supported on this platform");
    }
}

pub fn extract_from_stdin(opts: ExtractOpts, _lang: &str, max_size: u64) -> Result<()> {
    let temp_path = PathBuf::from("/tmp/farscry_stdin.png");
    let mut stdin = std::io::stdin();
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer)?;

    std::fs::write(&temp_path, buffer)?;

    let output = crate::pipeline::process_image(&temp_path, max_size)?;
    let (width, height) = image::open(&temp_path)
        .map(|img| img.dimensions())
        .unwrap_or((1920, 1080));
    let text = format_output(&output, "stdin", width, height, &opts);
    write_output(&text, opts.output.as_ref())
}
