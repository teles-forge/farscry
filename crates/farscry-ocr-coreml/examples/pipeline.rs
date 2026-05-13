#![cfg(target_os = "macos")]

use farscry_core::traits::OcrEngine;
use farscry_ocr_coreml::CoreMlOcrEngine;
use std::path::PathBuf;

fn main() {
    let models_dir = PathBuf::from("/Users/teles/github/farscry/crates/farscry-ocr-coreml/models");

    println!("Models directory: {}", models_dir.display());

    if !models_dir.exists() {
        panic!("Models directory not found at {}", models_dir.display());
    }

    let det_model = if models_dir.join("farscry-det.mlmodelc").exists() {
        models_dir.join("farscry-det.mlmodelc")
    } else if models_dir.join("pp-ocrv5_mobile_det.mlmodelc").exists() {
        models_dir.join("pp-ocrv5_mobile_det.mlmodelc")
    } else {
        panic!("Detection model not found");
    };

    let rec_model = if models_dir.join("farscry-rec.mlmodelc").exists() {
        models_dir.join("farscry-rec.mlmodelc")
    } else if models_dir.join("en_pp-ocrv5_mobile_rec.mlmodelc").exists() {
        models_dir.join("en_pp-ocrv5_mobile_rec.mlmodelc")
    } else {
        panic!("Recognition model not found");
    };

    println!("Detection model: {}", det_model.display());
    println!("Recognition model: {}", rec_model.display());

    if !det_model.exists() {
        panic!("Detection model not found at {}", det_model.display());
    }

    if !rec_model.exists() {
        panic!("Recognition model not found at {}", rec_model.display());
    }

    let engine = CoreMlOcrEngine::new(det_model, rec_model).unwrap();

    let test_image = PathBuf::from("/Users/teles/github/farscry/spike/test.png");

    if !test_image.exists() {
        panic!("Test image not found at {}", test_image.display());
    };

    println!("Test image: {}", test_image.display());

    let img = image::open(&test_image).unwrap();
    println!("Image size: {}x{}", img.width(), img.height());

    let start = std::time::Instant::now();
    let result = engine.extract(&img).unwrap();
    let elapsed = start.elapsed();

    println!(
        "Total pipeline latency: {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Regions detected: {}", result.regions.len());

    for (i, r) in result.regions.iter().enumerate() {
        println!(
            "  [{}] \"{}\" at ({:.0}, {:.0}) size {:.0}x{:.0}",
            i, r.text, r.cx, r.cy, r.w, r.h
        );
    }
}
