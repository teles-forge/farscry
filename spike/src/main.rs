use oar_ocr::core::config::{OrtGraphOptimizationLevel, OrtSessionConfig};
use oar_ocr::domain::TextDetectionConfig;
use oar_ocr::prelude::*;
use oar_ocr::processors::LimitType;
use std::path::Path;
use std::time::{Duration, Instant};

const RUNS: usize = 5;
const MODELS_DIR: &str = "models";
const IMAGE_PATH: &str = "test.png";


struct SpikeConfig {
    label: &'static str,
    det_model: &'static str,
    rec_model: &'static str,
    /// None = oar-ocr default (Level1, available_parallelism threads)
    ort_config: Option<OrtSessionConfig>,
    /// None = oar-ocr default (960px)
    det_limit: Option<u32>,
    /// None = oar-ocr default (6)
    region_batch: Option<usize>,
}

fn build_ocr(cfg: &SpikeConfig) -> Result<OAROCR, Box<dyn std::error::Error>> {
    let models = Path::new(MODELS_DIR);
    let mut builder = OAROCRBuilder::new(
        models.join(cfg.det_model),
        models.join(cfg.rec_model),
        models.join("ppocrv5_en_dict.txt"),
    );

    if let Some(ort_cfg) = &cfg.ort_config {
        builder = builder.ort_session(ort_cfg.clone());
    }

    if let Some(limit) = cfg.det_limit {
        builder = builder.text_detection_config(TextDetectionConfig {
            limit_side_len: Some(limit),
            limit_type: Some(LimitType::Max),
            ..TextDetectionConfig::default()
        });
    }

    if let Some(batch) = cfg.region_batch {
        builder = builder.region_batch_size(batch);
    }

    Ok(builder.build()?)
}

fn run_benchmark(cfg: &SpikeConfig) -> Result<BenchResult, Box<dyn std::error::Error>> {
    let image_path = Path::new(IMAGE_PATH);

    let t_build = Instant::now();
    let ocr = build_ocr(cfg)?;
    let build_time = t_build.elapsed();

    let mut times: Vec<Duration> = Vec::with_capacity(RUNS);
    let mut regions_count = 0;
    let mut detected_texts: Vec<String> = Vec::new();

    for run in 0..RUNS {
        let img = load_image(image_path)?;
        let t = Instant::now();
        let result = ocr.predict(vec![img])?;
        let elapsed = t.elapsed();
        times.push(elapsed);

        if run == 0 {
            let regions = &result[0].text_regions;
            regions_count = regions.len();
            for r in regions {
                if let Some((text, _conf)) = r.text_with_confidence() {
                    detected_texts.push(text.to_string());
                }
            }
        }
    }

    let cold = times[0];
    let warm_times = &times[1..];
    let warm_min = warm_times.iter().min().copied().unwrap_or(cold);
    let warm_avg = Duration::from_nanos(
        (warm_times.iter().map(|d| d.as_nanos()).sum::<u128>() / warm_times.len() as u128) as u64,
    );

    Ok(BenchResult {
        label: cfg.label,
        build_time,
        cold,
        warm_min,
        warm_avg,
        regions: regions_count,
        texts: detected_texts,
    })
}

struct BenchResult {
    label: &'static str,
    build_time: Duration,
    cold: Duration,
    warm_min: Duration,
    warm_avg: Duration,
    regions: usize,
    texts: Vec<String>,
}

fn print_result(r: &BenchResult, baseline_warm_avg: Option<Duration>) {
    let speedup = baseline_warm_avg.map(|b| {
        format!("  ({:.1}x vs baseline)", b.as_secs_f64() / r.warm_avg.as_secs_f64())
    });
    println!("┌─ {} ", r.label);
    println!("│  model load : {:>8.1?}", r.build_time);
    println!("│  cold       : {:>8.1?}", r.cold);
    println!("│  warm avg   : {:>8.1?}{}", r.warm_avg, speedup.as_deref().unwrap_or(""));
    println!("│  warm min   : {:>8.1?}", r.warm_min);
    println!("│  regions    : {}", r.regions);
    let verdict = if r.warm_avg.as_millis() < 80 {
        " PASS (<80ms)"
    } else if r.warm_avg.as_millis() < 150 {
        " PASS (<150ms)"
    } else if r.warm_avg.as_millis() < 300 {
        "  MARGINAL (<300ms)"
    } else {
        "No FAIL (>=300ms)"
    };
    println!("└─ {verdict}");
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  farscry - OCR x86 Optimization Spike                       ║");
    println!("║  Platform: {}                                    ║",
        if cfg!(target_arch = "aarch64") { "aarch64 (results proxy x86)" }
        else if cfg!(target_arch = "x86_64") { "x86_64 (direct measurement)  " }
        else { "unknown                      " });
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Runs per config : {RUNS}  (run 1 = cold, runs 2-{RUNS} = warm)");
    println!("  Image           : {IMAGE_PATH}");
    println!("  Models dir      : {MODELS_DIR}");
    println!();


    let logical_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let physical_threads = if cfg!(target_arch = "x86_64") {
        (logical_threads / 2).max(1)
    } else {
        logical_threads
    };
    println!("  CPU threads     : {logical_threads} logical / {physical_threads} physical (intra_threads)");
    println!();


    let ort_optimized = OrtSessionConfig::new()
        .with_intra_threads(physical_threads)
        .with_inter_threads(1)
        .with_optimization_level(OrtGraphOptimizationLevel::Level2);

    let configs: Vec<SpikeConfig> = vec![

        SpikeConfig {
            label: "BASELINE  FP32 | default ORT | 960px det | batch=6",
            det_model: "pp-ocrv5_mobile_det.onnx",
            rec_model: "en_pp-ocrv5_mobile_rec.onnx",
            ort_config: None,
            det_limit: None,
            region_batch: None,
        },


        SpikeConfig {
            label: "SPIKE E   INT8 | default ORT | 960px det | batch=6",
            det_model: "det_int8.onnx",
            rec_model: "rec_int8.onnx",
            ort_config: None,
            det_limit: None,
            region_batch: None,
        },


        SpikeConfig {
            label: "SPIKE A+B FP32 | thread+L2   | 960px det | batch=6",
            det_model: "pp-ocrv5_mobile_det.onnx",
            rec_model: "en_pp-ocrv5_mobile_rec.onnx",
            ort_config: Some(ort_optimized.clone()),
            det_limit: None,
            region_batch: None,
        },


        SpikeConfig {
            label: "SPIKE A-D FP32 | thread+L2   | 640px det | batch=32",
            det_model: "pp-ocrv5_mobile_det.onnx",
            rec_model: "en_pp-ocrv5_mobile_rec.onnx",
            ort_config: Some(ort_optimized.clone()),
            det_limit: Some(640),
            region_batch: Some(32),
        },


        SpikeConfig {
            label: "SPIKE ALL INT8 | thread+L2   | 640px det | batch=32",
            det_model: "det_int8.onnx",
            rec_model: "rec_int8.onnx",
            ort_config: Some(ort_optimized),
            det_limit: Some(640),
            region_batch: Some(32),
        },
    ];


    let mut results: Vec<BenchResult> = Vec::new();

    for cfg in &configs {
        print!("Running {}... ", cfg.label);
        std::io::Write::flush(&mut std::io::stdout())?;
        match run_benchmark(cfg) {
            Ok(r) => {
                println!("done ({:.0?} warm avg)", r.warm_avg);
                results.push(r);
            }
            Err(e) => {
                println!("ERROR: {e}");

            }
        }
    }


    println!();
    println!("══════════════════════ RESULTS ══════════════════════════════════");
    println!();

    let baseline_warm_avg = results.first().map(|r| r.warm_avg);

    for (i, r) in results.iter().enumerate() {
        let base = if i == 0 { None } else { baseline_warm_avg };
        print_result(r, base);
    }


    println!("══════════════════════ SUMMARY TABLE ════════════════════════════");
    println!("{:<52} {:>9} {:>9} {:>8} {:>6}",
        "Config", "cold", "warm avg", "speedup", "regions");
    println!("{}", "─".repeat(90));
    for (i, r) in results.iter().enumerate() {
        let speedup = if i == 0 {
            "1.00x".to_string()
        } else if let Some(base) = baseline_warm_avg {
            format!("{:.2}x", base.as_secs_f64() / r.warm_avg.as_secs_f64())
        } else {
            "-".to_string()
        };
        println!("{:<52} {:>9.1?} {:>9.1?} {:>8} {:>6}",
            r.label, r.cold, r.warm_avg, speedup, r.regions);
    }


    if results.len() >= 2 {
        println!();
        println!("══════════════════════ SPIKE E ACCURACY CHECK ═══════════════════");
        let baseline = &results[0];
        let int8 = &results[1];
        println!("FP32 regions : {}", baseline.regions);
        println!("INT8 regions : {}", int8.regions);
        let same_count = baseline.regions == int8.regions;
        let text_overlap: usize = int8.texts.iter()
            .filter(|t| baseline.texts.contains(t))
            .count();
        let total = baseline.texts.len().max(1);
        println!("Text overlap : {}/{} exact matches ({:.0}%)",
            text_overlap, total, text_overlap as f64 / total as f64 * 100.0);
        if same_count && text_overlap as f64 / total as f64 > 0.9 {
            println!("INT8 accuracy:  ACCEPTABLE (>=90% text match, same region count)");
        } else {
            println!("INT8 accuracy:   DEGRADED - manual review needed");
            println!("  FP32 texts: {:?}", &baseline.texts[..baseline.texts.len().min(5)]);
            println!("  INT8 texts: {:?}", &int8.texts[..int8.texts.len().min(5)]);
        }
    }


    println!();
    println!("══════════════════════ GO/NO-GO FOR v0.1.0 ═════════════════════");
    println!();

    if let Some(baseline) = baseline_warm_avg {
        println!("Baseline (FP32, default ORT): {:.0?}", baseline);
        println!();

        for r in results.iter().skip(1) {
            let speedup = baseline.as_secs_f64() / r.warm_avg.as_secs_f64();
            let go = if r.warm_avg.as_millis() < 150 {
                " GO - ships in v0.1.0"
            } else if speedup > 1.3 {
                "  PARTIAL - meaningful improvement, not at target"
            } else {
                "No NO-GO"
            };
            println!("  {} -> {:.0?} ({:.1}x)  {go}", r.label, r.warm_avg, speedup);
        }
    }

    println!();
    println!("NOTE: Running on {}. x86 speedups will differ:",
        if cfg!(target_arch = "aarch64") { "Apple Silicon (ARM64)" } else { "x86_64" });
    println!("  - INT8 ARM (SDOT): expected 1.5-2.5x speedup");
    println!("  - INT8 x86 AVX2:   expected 1.4-1.5x speedup");
    println!("  - INT8 x86 VNNI:   expected 2.0-2.5x speedup");
    println!("  - Thread tuning:   ~20-35% on x86 (HT contention avoided)");
    println!("  - 640px det:       ~40-45% det savings on any arch");

    Ok(())
}
