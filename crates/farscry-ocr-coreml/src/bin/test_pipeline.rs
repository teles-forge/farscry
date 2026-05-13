#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use farscry_core::traits::OcrEngine;
    use farscry_ocr_coreml::CoreMlOcrEngine;
    use std::path::Path;

    let args: Vec<String> = std::env::args().collect();
    let image_path = if args.len() > 1 {
        Path::new(&args[1])
    } else {
        Path::new("spike/test.png")
    };

    if !image_path.exists() {
        eprintln!("Error: image not found: {}", image_path.display());
        std::process::exit(1);
    }

    let img = image::open(image_path)?;
    eprintln!("Image: {}x{}", img.width(), img.height());

    let repo_root = std::env::current_dir()?;
    let models_dir = repo_root.join("spike-native-coreml").join("models");
    let det_model = models_dir.join("pp-ocrv5_mobile_det.mlmodelc");
    let rec_model = models_dir.join("en_pp-ocrv5_mobile_rec_b32.mlmodelc");

    eprintln!("Detection model: {}", det_model.display());
    eprintln!("Recognition model: {}", rec_model.display());

    let engine = CoreMlOcrEngine::new(det_model.to_path_buf(), rec_model.to_path_buf())?;

    eprintln!("Warming up (3 runs)...");
    for _ in 0..3 {
        let _ = engine.extract(&img)?;
    }

    const BENCH: usize = 10;
    eprintln!("Benchmarking ({BENCH} runs)...");
    let mut times = Vec::with_capacity(BENCH);
    let mut last_result = None;
    for _ in 0..BENCH {
        let t = std::time::Instant::now();
        let r = engine.extract(&img)?;
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        last_result = Some(r);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = times[0];
    let p50 = times[BENCH / 2];
    let avg = times.iter().sum::<f64>() / BENCH as f64;
    eprintln!("\n=== Timing (warm, {BENCH} runs) ===");
    eprintln!("  min={min:.1}ms  p50={p50:.1}ms  avg={avg:.1}ms");
    eprintln!("  Target: <30ms  Floor: <50ms");
    eprintln!(
        "  {}",
        if min < 30.0 {
            "PASS OK (<30ms)"
        } else if min < 50.0 {
            "MARGINAL (30-50ms)"
        } else {
            "FAIL FAIL (>50ms)"
        }
    );

    if let Some(result) = last_result {
        eprintln!("\n=== OCR Results ({} regions) ===", result.regions.len());
        for (i, region) in result.regions.iter().enumerate() {
            if !region.text.is_empty() {
                eprintln!(
                    "  [{i}] '{}' @ ({:.0},{:.0})",
                    region.text, region.cx, region.cy
                );
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Error: CoreML OCR only available on macOS");
    std::process::exit(1);
}
