use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn run_annotate(
    img: image::DynamicImage,
    source: &Path,
    output: Option<PathBuf>,
) -> Result<()> {
    let vasp = {
        let pipeline = crate::pipeline::get_or_build_pipeline()?;
        let img_clone = img.clone();
        pipeline
            .process(img_clone)
            .map_err(|e| anyhow::anyhow!("pipeline failed: {e}"))?
    };

    let out_path = match output {
        Some(p) => p,
        None => {
            let stem = source.file_stem().unwrap_or_default().to_string_lossy();
            let ext = source
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_else(|| ".png".to_string());
            source
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(format!("{stem}_annotated{ext}"))
        }
    };

    let annotated = farscry_formatter::annotate::annotate_image(img, &vasp);
    annotated
        .save(&out_path)
        .with_context(|| format!("cannot save: {}", out_path.display()))?;

    eprintln!(
        "[farscry] annotated {} elements -> {}",
        vasp.ui_tree.len(),
        out_path.display()
    );
    Ok(())
}

pub fn annotate_images(paths: Vec<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    if paths.is_empty() {
        anyhow::bail!("invalid input: provide at least one image path or --from-clipboard");
    }
    if output.is_some() && paths.len() > 1 {
        anyhow::bail!("invalid input: -o/--output cannot be used with multiple input paths");
    }
    for path in &paths {
        crate::pipeline::validate_image(path, 100_000_000)?;
        let img =
            image::open(path).with_context(|| format!("cannot open image: {}", path.display()))?;
        run_annotate(img, path, output.clone())?;
    }
    Ok(())
}

pub fn annotate_from_clipboard(output: Option<PathBuf>) -> Result<()> {
    let tmp = PathBuf::from("/tmp/farscry_annotate_clip.png");
    let out = output.unwrap_or_else(|| PathBuf::from("/tmp/farscry_annotated.png"));

    #[cfg(target_os = "macos")]
    {
        let (data, _) = crate::clipboard::macos::read_clipboard_image_macos()?;
        std::fs::write(&tmp, data)?;
        let img = image::open(&tmp).context("cannot open clipboard image")?;
        run_annotate(img, &tmp, Some(out))
    }

    #[cfg(target_os = "linux")]
    {
        let data = crate::clipboard::linux::read_clipboard_png_linux()?;
        std::fs::write(&tmp, data)?;
        let img = image::open(&tmp).context("cannot open clipboard image")?;
        run_annotate(img, &tmp, Some(out))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (tmp, out);
        anyhow::bail!("--from-clipboard not supported on this platform");
    }
}
