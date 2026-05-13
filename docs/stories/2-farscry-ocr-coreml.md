Story 2 - farscry-ocr-coreml

**Status:** Ready - @po Fix 7 applied 2026-05-09
**Blocked by:** Story 1 (farscry-core)
**Estimated hours:** 20h
**Platform:** macOS only (`target_os = "macos"`)

---

What to build

Native CoreML OCR backend. Target: **21ms steady state** on M4 Pro (validated in `spike-native-coreml/`).

Why NOT `oar-ocr` with CoreML feature

The spike proved conclusively:
- `oar-ocr` CPU path: ~300ms
- `oar-ocr` with `features = ["coreml"]` (ORT CoreML bridge): ~298ms - **no improvement**
  - Reason: `oar-ocr-core`'s `OrtExecutionProvider::CoreML` only exposes `ane_only` + `subgraphs`
  - Missing: `ModelFormat::MLProgram`, `SpecializationStrategy::FastPrediction`, `ComputeUnits::All`, `static_input_shapes(true)`, `model_cache_dir`
- Native CoreML via `objc2-core-ml`: **21ms**  - only path that meets the PRD target

**This crate uses `objc2-core-ml` directly. It does NOT use `oar-ocr`.**

---

Architecture

Model format

Production flow:
1. `.mlpackage` files distributed in macOS release archive (pre-converted from ONNX)
2. On first run: compile `.mlpackage` -> `.mlmodelc` via CoreML compilation API (~3s one-time)
3. Cache `.mlmodelc` at `~/.farscry/models/coreml/`
4. Subsequent runs: load from `.mlmodelc` cache -> 21ms steady state

Files distributed in macOS release:
- `farscry-det.mlpackage` - detection model (DBNet++)
- `farscry-rec.mlpackage` - recognition model (SVTR-LCNet, English)

Crate: `crates/farscry-ocr-coreml/`

```
crates/farscry-ocr-coreml/
├── src/
│   ├── lib.rs       - public struct + OcrEngine impl
│   ├── compile.rs   - .mlpackage -> .mlmodelc first-run compilation
│   ├── infer.rs     - CoreML inference via objc2
│   ├── decode.rs    - output tensor -> Vec<TextRegion>
│   └── verify.rs    - SHA256 model integrity
└── Cargo.toml
```

---

`src/lib.rs`

```rust
use farscry_core::{OcrEngine, OcrOutput, FarscryError};
use std::path::{Path, PathBuf};

pub struct CoreMlOcrEngine {
    det_model_path: PathBuf,    // compiled .mlmodelc for detection
    rec_model_path: PathBuf,    // compiled .mlmodelc for recognition
}

impl CoreMlOcrEngine {


    pub fn new(models_dir: &Path) -> Result<Self, FarscryError> {

        verify::verify_models(models_dir)?;

        let det = compile::ensure_compiled(models_dir, "farscry-det")?;
        let rec = compile::ensure_compiled(models_dir, "farscry-rec")?;
        Ok(Self { det_model_path: det, rec_model_path: rec })
    }
}

impl OcrEngine for CoreMlOcrEngine {
    fn extract(&self, image: &image::DynamicImage) -> Result<OcrOutput, FarscryError> {
        infer::run_pipeline(image, &self.det_model_path, &self.rec_model_path)
    }
}
```

---

`src/compile.rs` - `.mlpackage` -> `.mlmodelc`

CoreML compilation requires calling `MLModel.compileModelAtURL:error:` which is NOT in `objc2-core-ml = "0.3"` public bindings. Use `msg_send!` macro:

```rust
use objc2::msg_send;
use objc2::runtime::NSObject;
use objc2_foundation::{NSURL, NSError};


pub fn ensure_compiled(
    models_dir: &Path,
    name: &str,
) -> Result<PathBuf, FarscryError> {
    let cache_path = models_dir.join("coreml").join(format!("{name}.mlmodelc"));


    if cache_path.exists() && !is_stale(&cache_path, models_dir.join(format!("{name}.mlpackage")))? {
        return Ok(cache_path);
    }

    std::fs::create_dir_all(models_dir.join("coreml"))?;

    let pkg_path = models_dir.join(format!("{name}.mlpackage"));
    let pkg_url = path_to_nsurl(&pkg_path)?;


    eprintln!("[farscry] Compiling {name} for Apple Silicon (first run, ~3s)...");

    let compiled_url: *mut NSURL = unsafe {
        let cls = objc2::class!(MLModel);
        msg_send![cls, compileModelAtURL: &pkg_url as *const _, error: std::ptr::null_mut::<*mut NSError>()]
    };
    if compiled_url.is_null() {
        return Err(FarscryError::OcrFailed(format!("CoreML compilation failed for {name}")));
    }


    let compiled_path = nsurl_to_path(compiled_url)?;
    std::fs::rename(&compiled_path, &cache_path)?;

    eprintln!("[farscry] {name} compiled and cached.");
    Ok(cache_path)
}

fn is_stale(cache: &Path, source: PathBuf) -> Result<bool, FarscryError> {

    let cache_mtime = std::fs::metadata(cache)?.modified()?;
    let src_mtime   = std::fs::metadata(source)?.modified()?;
    Ok(src_mtime > cache_mtime)
}
```

> **@security-engineer note:** `msg_send!` bypasses Rust's type system. The `compileModelAtURL:error:` selector is well-documented Apple API (stable since macOS 10.13). Error pointer is null - compilation errors will return a null URL, caught and converted to `FarscryError::OcrFailed`. Risk level: LOW.

---

`src/infer.rs` - CoreML inference

```rust
use objc2_core_ml::{MLModel, MLMultiArray, MLFeatureProvider, MLPredictionOptions};
use objc2_foundation::NSError;


pub fn run_pipeline(
    image: &image::DynamicImage,
    det_path: &Path,
    rec_path: &Path,
) -> Result<farscry_core::OcrOutput, farscry_core::FarscryError> {
    let (w, h) = (image.width(), image.height());

    let preprocessed = preprocess(image);

    let boxes = run_detection(&preprocessed, det_path)?;

    let regions = run_recognition(image, &boxes, rec_path)?;
    Ok(farscry_core::OcrOutput { regions, width: w, height: h })
}

fn run_detection(image: &image::DynamicImage, model_path: &Path) -> Result<Vec<BoundingBox>, farscry_core::FarscryError> {


    todo!("CoreML detection inference")
}

fn run_recognition(
    original: &image::DynamicImage,
    boxes: &[BoundingBox],
    model_path: &Path,
) -> Result<Vec<farscry_core::TextRegion>, farscry_core::FarscryError> {


    todo!("CoreML recognition inference")
}
```

---

`src/verify.rs` - SHA256 model integrity

```rust
use sha2::{Sha256, Digest};
use std::path::Path;


pub fn verify_models(models_dir: &Path) -> Result<(), farscry_core::FarscryError> {
    let manifest_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".farscry")
        .join(".manifest.json");

    let models = [
        (models_dir.join("farscry-det.mlpackage"), "farscry-det"),
        (models_dir.join("farscry-rec.mlpackage"), "farscry-rec"),
    ];


    let mut manifest = load_manifest(&manifest_path);

    for (path, key) in &models {
        let hash = sha256_dir_or_file(path)?;
        match manifest.get(key) {
            Some(expected) if expected != &hash => {
                return Err(farscry_core::FarscryError::ModelIntegrity {
                    model: key.to_string(),
                    expected: expected.clone(),
                    actual: hash,
                });
            }
            None => {

                manifest.insert(key.to_string(), hash);
            }
            Some(_) => {} // hash matches - OK
        }
    }

    save_manifest(&manifest_path, &manifest)?;
    Ok(())
}
```

---

`Cargo.toml`

```toml
[package]
name = "farscry-ocr-coreml"
version = "0.1.0"
edition = "2021"
publish = false

[target.'cfg(target_os = "macos")'.dependencies]
farscry-core  = { path = "../farscry-core" }
image         = "0.25"
objc2         = "0.5"
objc2-core-ml = "0.3"
objc2-foundation = "0.2"
sha2          = "0.10"
serde_json    = "1"
dirs          = "5"

[target.'cfg(not(target_os = "macos"))'.dependencies]
Intentionally empty - crate does not compile on non-macOS
```

---

Model distribution

The macOS release archive (`farscry-aarch64-apple-darwin.tar.gz`) includes:
```
farscry                         - binary
models/farscry-det.mlpackage/   - detection model (pre-converted)
models/farscry-rec.mlpackage/   - recognition model (pre-converted, English)
models/farscry-det.sha256       - expected SHA256
models/farscry-rec.sha256       - expected SHA256
```

On `npm install farscry`, the postinstall script downloads and extracts this archive, placing models at the binary sibling location. On first `farscry` run, `.mlmodelc` compilation happens once (~3s).

**Model conversion (build-time, not user-facing):**
```python
During release preparation:
coremltools.convert(det_onnx, minimum_deployment_target="macOS13") -> farscry-det.mlpackage
coremltools.convert(rec_onnx, minimum_deployment_target="macOS13") -> farscry-rec.mlpackage
```

---

Acceptance criteria

- [ ] `cargo build -p farscry-ocr-coreml` passes on macOS aarch64
- [ ] Does NOT compile on Linux (`cfg(target_os = "macos")` guard verified by CI)
- [ ] Does NOT compile on Windows
- [ ] SHA256 verification runs before any `.mlpackage` is compiled
- [ ] Tamper test: modify one byte of `.mlpackage` -> `FarscryError::ModelIntegrity` returned
- [ ] First-run `.mlmodelc` compilation completes in < 5s on M4 Pro
- [ ] Compilation result cached at `~/.farscry/models/coreml/`
- [ ] Re-run without changes: skips compilation (cache hit), < 500ms to ready
- [ ] `.mlpackage` change -> cache invalidated -> recompiled
- [ ] `.manifest.json` written/updated after first verification
- [ ] Warm steady-state inference on 800x480 screenshot: **< 30ms** (M4 Pro target)
- [ ] Warm steady-state inference on 800x480 screenshot: < 50ms (minimum floor)
- [ ] Uses `objc2-core-ml` - NOT `oar-ocr` - confirmed by `cargo tree -p farscry-ocr-coreml`
- [ ] `publish = false` in Cargo.toml

Dependencies

Story 1 (farscry-core - `OcrEngine` trait, `OcrOutput`, `FarscryError`).

---

@rust-engineer notes

1. The `msg_send!` usage in `compile.rs` is the only unsafe path. Isolate it in a single function and document the safety contract.
2. `MLMultiArray` -> Rust tensor conversion: prefer row-major CHW layout (channels x height x width) as CoreML Vision models expect.
3. If `objc2-core-ml = "0.3"` gains `compileModelAtURL:error:` bindings before implementation, switch from `msg_send!` to the typed binding.
4. Dark mode detection + adaptive resize belongs in `infer.rs::preprocess()` - it is an OCR preprocessing step, not a separate crate.
5. `@security-engineer *audit` required on `objc2`, `objc2-core-ml`, `objc2-foundation` supply chain before merge.
