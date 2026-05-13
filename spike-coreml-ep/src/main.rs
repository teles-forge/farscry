


use ort::{
    ep::{self, coreml::{ComputeUnits, ModelFormat, SpecializationStrategy}},
    session::Session,
    value::TensorRef,
};
use std::{path::Path, time::Instant};


const CACHE_DIR: &str = "/tmp/farscry-coreml-cache";
const RUNS: usize = 6;

fn build_session(model_path: &Path, static_shapes: bool) -> Result<Session, Box<dyn std::error::Error>> {
    let ep = ep::CoreML::default()
        .with_compute_units(ComputeUnits::All)
        .with_model_format(ModelFormat::MLProgram)
        .with_specialization_strategy(SpecializationStrategy::FastPrediction)
        .with_model_cache_dir(CACHE_DIR)
        .with_static_input_shapes(static_shapes)
        .build();

    let mut session = Session::builder()?
        .with_execution_providers([ep, ep::CPU::default().build()])?
        .commit_from_file(model_path)?;

    Ok(session)
}

fn run_det(session: &mut Session) -> Result<(), Box<dyn std::error::Error>> {
    let data = vec![0.0_f32; 1 * 3 * 960 * 960];
    let _ = session.run(ort::inputs![TensorRef::from_array_view(([1_usize, 3, 960, 960], data.as_slice()))?])?;
    Ok(())
}

fn run_rec(session: &mut Session) -> Result<(), Box<dyn std::error::Error>> {
    let data = vec![0.0_f32; 1 * 3 * 48 * 320];
    let _ = session.run(ort::inputs![TensorRef::from_array_view(([1_usize, 3, 48, 320], data.as_slice()))?])?;
    Ok(())
}

fn bench_session(
    label: &str,
    model_path: &Path,
    run_fn: fn(&mut Session) -> Result<(), Box<dyn std::error::Error>>,
    static_shapes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- {} (static_shapes={}) ---", label, static_shapes);


    let t = Instant::now();
    let mut session = build_session(model_path, static_shapes)?;
    let load_ms = t.elapsed().as_millis();
    println!("  session init  : {}ms", load_ms);


    let mut times = Vec::with_capacity(RUNS);
    for i in 1..=RUNS {
        let t = Instant::now();
        run_fn(&mut session)?;
        let ms = t.elapsed().as_millis();
        times.push(ms);
        let tag = if i == 1 { " <- cold" } else if i == RUNS { " <- steady" } else { "" };
        println!("  run {:02}        : {}ms{}", i, ms, tag);
    }

    let steady = times[RUNS - 1];
    let min = *times.iter().min().unwrap();

    println!("  ── steady (run {}): {}ms  min: {}ms", RUNS, steady, min);
    println!("  ── 181ms goal: {}", if steady < 181 { "PASS OK" } else { "FAIL FAIL" });

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(CACHE_DIR)?;

    let models = Path::new("../spike/models");
    let det = models.join("pp-ocrv5_mobile_det.onnx");
    let rec = models.join("en_pp-ocrv5_mobile_rec.onnx");

    println!("=== farscry - ORT CoreML EP spike ===");
    println!("models : {}", models.display());
    println!("cache  : {}", CACHE_DIR);
    println!("config : MLProgram + FastPrediction + ComputeUnits::All");
    println!();


    bench_session("DET  dynamic shapes", &det, run_det, false)?;
    bench_session("REC  dynamic shapes", &rec, run_rec, false)?;


    bench_session("DET  static shapes ", &det, run_det, true)?;
    bench_session("REC  static shapes ", &rec, run_rec, true)?;


    println!("\n--- COMBINED pipeline (static shapes, steady state) ---");
    let mut det_s = build_session(&det, true)?;
    let mut rec_s = build_session(&rec, true)?;

    let mut combined_times = Vec::with_capacity(RUNS);
    for i in 1..=RUNS {
        let t = Instant::now();
        run_det(&mut det_s)?;
        run_rec(&mut rec_s)?;
        let ms = t.elapsed().as_millis();
        combined_times.push(ms);
        let tag = if i == 1 { " <- cold" } else if i == RUNS { " <- steady" } else { "" };
        println!("  run {:02}        : {}ms{}", i, ms, tag);
    }

    let combined_steady = combined_times[RUNS - 1];
    println!("\n========================================");
    println!("SPIKE SUMMARY (ORT CoreML EP - full options)");
    println!("========================================");
    println!("  combined steady-state : {}ms", combined_steady);
    println!("  target (<181ms)       : {}", if combined_steady < 181 { "PASS OK" } else { "FAIL FAIL" });
    println!("  target (<100ms)       : {}", if combined_steady < 100 { "PASS OK" } else { "FAIL FAIL" });
    println!("========================================");

    Ok(())
}
