#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use farscry_core::traits::OcrEngine;
    use farscry_ocr_coreml::model::load_model;
    use farscry_ocr_coreml::CoreMlOcrEngine;
    use image::DynamicImage;
    use oar_ocr::core::config::{OrtGraphOptimizationLevel, OrtSessionConfig};
    use oar_ocr::domain::TextDetectionConfig;
    use oar_ocr::prelude::*;
    use std::path::Path;

    let image_path = Path::new("spike/test.png");
    if !image_path.exists() {
        eprintln!("Error: spike/test.png not found");
        std::process::exit(1);
    }

    let img = image::open(&image_path)?;
    let rgb = img.to_rgb8();

    let repo_root = std::env::current_dir()?;
    let models_dir = repo_root.join("spike-native-coreml").join("models");
    let det_model_path = models_dir.join("pp-ocrv5_mobile_det.mlmodelc");

    eprintln!("Loading CoreML detection model...");
    let det_model = load_model(&det_model_path)?;

    use farscry_ocr_coreml::engine::run_detection_inference;
    use farscry_ocr_coreml::postprocess::db_postprocess;
    use farscry_ocr_coreml::preprocess::{normalize_to_tensor, resize_for_detection, LimitType};

    let (resized, scale_info) = resize_for_detection(&rgb, 960, LimitType::Max);
    let det_tensor = normalize_to_tensor(&resized);

    eprintln!("Running CoreML detection...");
    let det_output = run_detection_inference(&det_model, &det_tensor)?;
    let det_output_array4 = det_output.into_dimensionality::<ndarray::Ix4>().unwrap();
    let boxes = db_postprocess(&det_output_array4, vec![scale_info], 0.3, 0.6, 2.0);

    eprintln!("Detected {} regions", boxes[0].len());

    if boxes[0].is_empty() {
        eprintln!("Error: No regions detected");
        std::process::exit(1);
    }

    let first_bbox = &boxes[0][0];
    eprintln!("First bbox: {:?}", first_bbox.points);

    let (min_x, max_x) = first_bbox
        .points
        .iter()
        .fold((f32::MAX, f32::MIN), |(min, max), (x, _)| {
            (min.min(*x), max.max(*x))
        });
    let (min_y, max_y) = first_bbox
        .points
        .iter()
        .fold((f32::MAX, f32::MIN), |(min, max), (_, y)| {
            (min.min(*y), max.max(*y))
        });

    let x = min_x as u32;
    let y = min_y as u32;
    let w = (max_x - min_x) as u32;
    let h = (max_y - min_y) as u32;

    eprintln!("Crop region: x={}, y={}, w={}, h={}", x, y, w, h);
    let cropped = image::imageops::crop(&mut rgb.clone(), x, y, w, h).to_image();
    eprintln!(
        "Cropped image size: {}x{}",
        cropped.width(),
        cropped.height()
    );

    eprintln!("\n=== ORT Recognition ===");
    let ort_models_dir = repo_root.join("spike").join("models");
    let ort_det_model = ort_models_dir.join("pp-ocrv5_mobile_det.onnx");
    let ort_rec_model = ort_models_dir.join("en_pp-ocrv5_mobile_rec.onnx");
    let ort_dict_path = ort_models_dir.join("ppocrv5_en_dict.txt");

    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let physical = if cfg!(target_arch = "x86_64") {
        (logical / 2).max(1)
    } else {
        logical
    };

    let ort_config = OrtSessionConfig::new()
        .with_intra_threads(physical)
        .with_inter_threads(1)
        .with_optimization_level(OrtGraphOptimizationLevel::Level2);

    use oar_ocr::processors::LimitType as OrtLimitType;

    let det_config = TextDetectionConfig {
        limit_side_len: Some(960),
        limit_type: Some(OrtLimitType::Max),
        ..TextDetectionConfig::default()
    };

    let ocr = OAROCRBuilder::new(&ort_det_model, &ort_rec_model, &ort_dict_path)
        .ort_session(ort_config)
        .text_detection_config(det_config)
        .region_batch_size(32)
        .build()?;

    let ort_results = ocr.predict(vec![rgb.clone()])?;
    eprintln!(
        "ORT detected {} regions on full image",
        ort_results[0].text_regions.len()
    );

    if let Some(first_region) = ort_results[0].text_regions.first() {
        eprintln!(
            "ORT first region text: '{}'",
            first_region.text.as_deref().unwrap_or("")
        );
        eprintln!("ORT first region bbox: {:?}", first_region.bounding_box);
    }

    eprintln!("\n=== CoreML Recognition ===");
    let rec_model_path = models_dir.join("en_pp-ocrv5_mobile_rec_b32.mlmodelc");

    let coreml_engine = CoreMlOcrEngine::new(det_model_path, rec_model_path)?;
    let coreml_results = coreml_engine.extract(&DynamicImage::from(rgb.clone()))?;

    eprintln!(
        "CoreML detected {} regions on full image",
        coreml_results.regions.len()
    );

    if let Some(first_region) = coreml_results.regions.first() {
        eprintln!("CoreML first region text: '{}'", first_region.text);
        eprintln!(
            "CoreML first region bbox: ({:.1},{:.1},{:.1},{:.1})",
            first_region.cx, first_region.cy, first_region.w, first_region.h
        );
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Error: This diagnostic tool only runs on macOS");
    std::process::exit(1);
}
