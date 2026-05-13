


use oar_ocr::core::config::{OrtGraphOptimizationLevel, OrtSessionConfig};
use oar_ocr::domain::TextDetectionConfig;
use oar_ocr::prelude::*;
use oar_ocr::processors::LimitType;
use std::path::{Path, PathBuf};
use std::process::Command;


fn resolve_models_dir() -> PathBuf {
    let sentinel = "pp-ocrv5_mobile_det.onnx";


    if let Ok(val) = std::env::var("FARSCRY_MODELS") {
        return PathBuf::from(val);
    }


    if let Some(home) = std::env::var_os("HOME") {
        let candidate = PathBuf::from(home).join(".farscry").join("models");
        if candidate.join(sentinel).exists() {
            return candidate;
        }
    }


    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("models");
            if candidate.join(sentinel).exists() {
                return candidate;
            }
        }
    }


    PathBuf::from("models")
}


fn build_ocr(models: &Path) -> Result<OAROCR, Box<dyn std::error::Error>> {
    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let intra = if cfg!(target_arch = "x86_64") {
        (logical / 2).max(1)
    } else {
        logical
    };

    let ort_cfg = OrtSessionConfig::new()
        .with_intra_threads(intra)
        .with_inter_threads(1)
        .with_optimization_level(OrtGraphOptimizationLevel::Level2);

    Ok(OAROCRBuilder::new(
        models.join("pp-ocrv5_mobile_det.onnx"),
        models.join("en_pp-ocrv5_mobile_rec.onnx"),
        models.join("ppocrv5_en_dict.txt"),
    )

    .ort_session(ort_cfg)
    .text_detection_config(TextDetectionConfig {
        limit_side_len: Some(640),
        limit_type: Some(LimitType::Max),
        ..TextDetectionConfig::default()
    })
    .region_batch_size(32)
    .build()?)
}


fn position_label(cx: f32, cy: f32, img_w: u32, img_h: u32) -> String {
    let w = img_w as f32;
    let h = img_h as f32;
    let col = if cx < w / 3.0 {
        "left"
    } else if cx < 2.0 * w / 3.0 {
        "center"
    } else {
        "right"
    };
    let row = if cy < h / 3.0 {
        "top"
    } else if cy < 2.0 * h / 3.0 {
        "middle"
    } else {
        "bottom"
    };
    if row == "middle" && col == "center" {
        "middle".to_string()
    } else {
        format!("{row}-{col}")
    }
}


fn detect_screen_type(texts: &[String]) -> &'static str {
    if texts.is_empty() {
        return "generic";
    }

    // terminal: shell prompts, Python tracebacks, file references
    let is_terminal = texts.iter().any(|t| {
        let l = t.trim();
        l.starts_with("$ ")
            || l.starts_with("# ")
            || l.starts_with("% ")
            || l.starts_with(">>> ")
            || l.contains("Traceback")
            || l.contains("File \"")
    });
    if is_terminal {
        return "terminal";
    }

    // config: >=2 elements end with ':'
    let colon_count = texts.iter().filter(|t| t.trim().ends_with(':')).count();
    if colon_count >= 2 {
        return "config";
    }

    // conversation: >=40% of elements are 1-3 words
    let short_count = texts
        .iter()
        .filter(|t| {
            let words = t.split_whitespace().count();
            (1..=3).contains(&words)
        })
        .count();
    if short_count * 10 >= texts.len() * 4 {
        return "conversation";
    }

    // error: any element contains 'error' or 'exception' (case-insensitive)
    let has_error = texts.iter().any(|t| {
        let l = t.to_lowercase();
        l.contains("error") || l.contains("exception") || l.contains("elifecycle")
    });
    if has_error {
        return "error";
    }

    "generic"
}

// ── Clipboard -> PNG (macOS only via osascript + sips) ────────────────────────

fn clipboard_to_png() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let out = "/tmp/farscry_clipboard.png";
    let tiff = "/tmp/farscry_clipboard.tiff";

    // Try PNG first; fall back to TIFF (screenshots/osascript default) + sips
    let script = format!(
        r#"set o to "{out}"
set t to "{tiff}"
try
    set d to (the clipboard as «class PNGf»)
    set f to open for access POSIX file o with write permission
    set eof of f to 0
    write d to f
    close access f
    return o
on error
    set d to (the clipboard as TIFF picture)
    set f to open for access POSIX file t with write permission
    set eof of f to 0
    write d to f
    close access f
    do shell script "sips -s format png " & quoted form of t & " --out " & quoted form of o
    return o
end try"#
    );

    let result = Command::new("osascript").arg("-e").arg(&script).output()?;

    if !result.status.success() {
        let err = String::from_utf8_lossy(&result.stderr);
        return Err(format!("clipboard read failed: {}", err.trim()).into());
    }

    Ok(PathBuf::from(out))
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  farscry <image.png>");
        eprintln!("  farscry --from-clipboard");
        eprintln!("  farscry <image.png> | claude \"what is this error?\"");
        std::process::exit(1);
    }

    let image_path: PathBuf = if args[1] == "--from-clipboard" {
        eprintln!("[farscry] reading clipboard...");
        clipboard_to_png()?
    } else {
        PathBuf::from(&args[1])
    };

    if !image_path.exists() {
        eprintln!("[farscry] error: file not found: {}", image_path.display());
        std::process::exit(1);
    }

    // Get image dimensions for position labels (image crate is already a dep)
    let (img_w, img_h) = image::image_dimensions(&image_path)?;

    let models_dir = resolve_models_dir();
    eprintln!("[farscry] models: {}", models_dir.display());
    eprintln!("[farscry] loading OCR model...");
    let ocr = build_ocr(&models_dir)?;

    eprintln!(
        "[farscry] running OCR on {} ({}x{})...",
        image_path.display(),
        img_w,
        img_h
    );
    let img = load_image(&image_path)?;
    let results = ocr.predict(vec![img])?;

    // Extract regions with centroids
    let mut regions: Vec<(f32, f32, String)> = results[0]
        .text_regions
        .iter()
        .filter_map(|r| {
            let text = r.text.as_deref()?.to_string();
            let c = r.bounding_box.center();
            Some((c.x, c.y, text))
        })
        .collect();

    // Sort top-to-bottom, then left-to-right
    regions.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Detect screen type from extracted texts (screen-type router, classifier spike)
    let texts: Vec<String> = regions.iter().map(|(_, _, t)| t.clone()).collect();
    let screen_type = detect_screen_type(&texts);

    // VASP framing header - gives claude context about what it's receiving
    println!("=== farscry visual context ===");
    println!("source: {}", image_path.display());
    println!("screen_type: {screen_type}");
    println!("---");


    for (cx, cy, text) in &regions {
        let label = position_label(*cx, *cy, img_w, img_h);
        println!("[{label}] {text}");
    }

    eprintln!("[farscry] done - {} regions", regions.len());

    Ok(())
}
