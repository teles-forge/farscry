


#![cfg(target_os = "macos")]

use objc2::{rc::Retained, runtime::{AnyObject, ProtocolObject}, AnyThread};
use objc2_core_ml::{
    MLComputeUnits, MLDictionaryFeatureProvider, MLFeatureProvider, MLFeatureValue,
    MLModel, MLModelConfiguration, MLMultiArray, MLMultiArrayDataType,
    MLPredictionOptions,
};
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString, NSURL};
use std::{path::Path, time::Instant};

const RUNS: usize = 6;


fn load_model(path: &Path) -> Retained<MLModel> {
    let path_str = path.to_str().expect("non-UTF8 path");
    unsafe {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path_str));
        let config = MLModelConfiguration::new();
        config.setComputeUnits(MLComputeUnits::All);
        MLModel::modelWithContentsOfURL_configuration_error(&url, &config)
            .expect("MLModel::modelWithContentsOfURL failed")
    }
}


fn zeros_multi_array(shape: &[usize]) -> Retained<MLMultiArray> {
    unsafe {
        let ns_shape: Vec<Retained<NSNumber>> = shape
            .iter()
            .map(|&d| NSNumber::numberWithInteger(d as isize))
            .collect();
        let ns_shape_refs: Vec<&NSNumber> = ns_shape.iter().map(|n| n.as_ref()).collect();
        let shape_array = NSArray::from_slice(&ns_shape_refs);

        let arr = MLMultiArray::initWithShape_dataType_error(
            MLMultiArray::alloc(),
            &shape_array,
            MLMultiArrayDataType::Float32,
        )
        .expect("MLMultiArray init failed");


        let count = arr.count() as usize;
        let ptr = arr.dataPointer().as_ptr() as *mut f32;
        std::slice::from_raw_parts_mut(ptr, count).fill(0.0_f32);

        arr
    }
}


fn make_input(arr: &MLMultiArray) -> Retained<MLDictionaryFeatureProvider> {
    unsafe {
        let key = NSString::from_str("x");
        let val = MLFeatureValue::featureValueWithMultiArray(arr);


        let val_any: &AnyObject = &*(val.as_ref() as *const MLFeatureValue as *const AnyObject);
        let key_ref: &NSString = &key;
        let dict: Retained<NSDictionary<NSString, AnyObject>> =
            NSDictionary::from_slices::<NSString>(&[key_ref], &[val_any]);

        MLDictionaryFeatureProvider::initWithDictionary_error(
            MLDictionaryFeatureProvider::alloc(),
            &dict,
        )
        .expect("MLDictionaryFeatureProvider init failed")
    }
}

fn run_inference(model: &MLModel, input: &MLDictionaryFeatureProvider) {
    unsafe {
        let opts = MLPredictionOptions::new();

        let input_proto: &ProtocolObject<dyn MLFeatureProvider> =
            ProtocolObject::from_ref(input);
        let _out = model
            .predictionFromFeatures_options_error(input_proto, &opts)
            .expect("CoreML prediction failed");
    }
}

fn bench(label: &str, model: &MLModel, shape: &[usize]) {
    let arr = zeros_multi_array(shape);
    let input = make_input(&arr);

    println!("\n--- {} {:?} ---", label, shape);
    let mut times = Vec::with_capacity(RUNS);
    for i in 1..=RUNS {
        let t = Instant::now();
        run_inference(model, &input);
        let ms = t.elapsed().as_millis();
        times.push(ms);
        let tag = if i == 1 { " <- cold" } else if i == RUNS { " <- steady" } else { "" };
        println!("  run {:02}: {}ms{}", i, ms, tag);
    }

    let steady = times[RUNS - 1];
    let min_t = *times.iter().min().unwrap();
    println!("  ── steady: {}ms  min: {}ms", steady, min_t);
    println!("  ── 181ms : {}", if steady < 181 { "PASS OK" } else { "FAIL FAIL" });
    println!("  ── 100ms : {}", if steady < 100 { "PASS OK" } else { "FAIL FAIL" });
    println!("  ── 30ms  : {}", if steady < 30  { "PASS OK" } else { "FAIL FAIL" });
}

fn main() {
    let models_dir = Path::new("models");
    let det_pkg = models_dir.join("pp-ocrv5_mobile_det.mlmodelc");
    let rec_pkg = models_dir.join("en_pp-ocrv5_mobile_rec.mlmodelc");

    println!("=== farscry - native CoreML spike (Rust / objc2-core-ml) ===");
    println!("models: {}", models_dir.display());
    println!("config: ComputeUnits::All (ANE + GPU + CPU scheduler)");
    println!();

    if !det_pkg.exists() || !rec_pkg.exists() {
        eprintln!("ERROR: .mlpackage files not found.");
        eprintln!("Run first: uv run ../spike-native-coreml/convert_models.py");
        std::process::exit(1);
    }

    println!("[load] detection model...");
    let t = Instant::now();
    let det_model = load_model(&det_pkg);
    println!("  -> {}ms", t.elapsed().as_millis());

    println!("[load] recognition model...");
    let t = Instant::now();
    let rec_model = load_model(&rec_pkg);
    println!("  -> {}ms", t.elapsed().as_millis());


    bench("DET", &det_model, &[1, 3, 960, 960]);
    bench("REC", &rec_model, &[1, 3, 48, 320]);


    println!("\n--- COMBINED pipeline (det + 10x rec) ---");
    let det_arr   = zeros_multi_array(&[1, 3, 960, 960]);
    let rec_arr   = zeros_multi_array(&[1, 3, 48, 320]);
    let det_input = make_input(&det_arr);
    let rec_input = make_input(&rec_arr);

    let mut combined_times = Vec::with_capacity(RUNS);
    for i in 1..=RUNS {
        let t = Instant::now();
        run_inference(&det_model, &det_input);
        for _ in 0..10 {
            run_inference(&rec_model, &rec_input);
        }
        let ms = t.elapsed().as_millis();
        combined_times.push(ms);
        let tag = if i == 1 { " <- cold" } else if i == RUNS { " <- steady" } else { "" };
        println!("  run {:02}: {}ms{}", i, ms, tag);
    }

    let combined_steady = combined_times[RUNS - 1];
    let combined_min = *combined_times.iter().min().unwrap();

    println!("\n========================================");
    println!("SPIKE SUMMARY (native CoreML - Rust)");
    println!("========================================");
    println!("  combined steady-state : {}ms", combined_steady);
    println!("  combined min          : {}ms", combined_min);
    println!("  ── 181ms goal : {}", if combined_steady < 181 { "PASS OK" } else { "FAIL FAIL" });
    println!("  ── 100ms goal : {}", if combined_steady < 100 { "PASS OK" } else { "FAIL FAIL" });
    println!("  ── 30ms goal  : {}", if combined_steady < 30  { "PASS OK" } else { "FAIL FAIL" });
    println!("========================================");

    let verdict = if combined_steady < 30 {
        "GO - ANE fully engaged, 15-30ms target confirmed. Implement farscry-ocr-coreml crate."
    } else if combined_steady < 100 {
        "GO - well under 181ms. Proceed with native CoreML backend."
    } else if combined_steady < 181 {
        "MARGINAL - hits 181ms. Check if ORT CoreML EP is already sufficient."
    } else {
        "NO-GO - native CoreML does not meet 181ms. Likely dynamic shape fallback or unsupported ops."
    };
    println!("VERDICT: {}", verdict);
}
