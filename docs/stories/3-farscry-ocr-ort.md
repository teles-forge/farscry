Story 3 - farscry-ocr-ort

**Status:** Ready
**Blocked by:** Story 1 (farscry-core)
**Estimated hours:** 6h

What to build

Cross-platform OCR backend using ONNX Runtime with CPU optimizations A+B+C+D+F. Target: ~120ms on x86 (projected from ARM64 measurements).

Crate: `crates/farscry-ocr-ort/`

Compiles on all platforms (Linux x86_64, Windows x86_64, macOS Intel, macOS Apple Silicon fallback).

**`src/lib.rs`** - implements `OcrEngine` trait:
```rust
pub struct OrtOcrEngine {
    ocr: oar_ocr::OAROCR,
}

impl OrtOcrEngine {
    pub fn new(models_dir: &Path) -> Result<Self, FarscryError>;
}

impl OcrEngine for OrtOcrEngine {
    fn extract(&self, image: &DynamicImage) -> Result<OcrOutput, FarscryError>;
}
```

**ORT config (optimizations A+B+C+D validated in spike):**
```rust
let logical = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
let physical = if cfg!(target_arch = "x86_64") { (logical / 2).max(1) } else { logical };

OAROCRBuilder::new(det, rec, dict)
    .ort_session(
        OrtSessionConfig::new()
            .with_intra_threads(physical)      // A: physical cores only
            .with_inter_threads(1)
            .with_optimization_level(OrtGraphOptimizationLevel::Level2)  // B: L2 opt
    )
    .text_detection_config(TextDetectionConfig {
        limit_side_len: Some(640),             // C: 640px det limit
        limit_type: Some(LimitType::Max),
        ..Default::default()
    })
    .region_batch_size(32)                     // D: batch 32
    .build()
```

**Optimization F - English model default:**
Default models are the English-only rec model (~7.5MB) + det model (~4.6MB). Multi-language models downloaded on demand via `farscry --install-lang <code>`.

Same SHA256 verification pattern as Story 2 (verify_model + .manifest.json).

**`Cargo.toml`:**
```toml
[dependencies]
oar-ocr = { version = "0.6", features = ["default"] }
```
farscry-core as workspace dep.

Acceptance criteria

- [ ] `cargo test -p farscry-ocr-ort` passes on Linux x86_64
- [ ] `cargo test -p farscry-ocr-ort` passes on macOS x86_64
- [ ] `cargo test -p farscry-ocr-ort` passes on Windows x86_64
- [ ] SHA256 verification before every ONNX Runtime load
- [ ] English model is the default; `--install-lang` downloads others
- [ ] Warm inference on macOS aarch64 ARM: < 300ms (dev build reference)
- [ ] publish = false in Cargo.toml

Dependencies

Story 1 (farscry-core).
