farscry - OCR Backend Architecture

> **Status:** DRAFT - pending @security-engineer, @rust-engineer, @po validation
> **Author:** @architect *review
> **Date:** 2026-05-07
> **Triggered by:** Native CoreML spike confirmed (21ms combined, all goals PASS)

---

Overview

farscry is the Rust reference implementation of the VASP (Visual Agent State Protocol). Its
OCR stage converts raw screenshots into structured text regions (`HocrOutput`) that feed the
downstream element classifier and VASP formatter.

This document covers the OCR backend selection - the single most performance-critical
component in the pipeline - and defines the crate topology, backend strategy, and model
lifecycle for v0.1.0.

---

Performance Baseline (from spikes)

All timings on Apple M-series (macOS), release build, steady-state (run 6 of 6):

| Backend | DET steady | REC steady | Combined (det+10xrec) | <181ms | <100ms | <30ms |
|---|---|---|---|---|---|---|
| CPU ORT (oar-ocr default) | ~150ms | ~15ms | **~300ms** | FAIL | FAIL | FAIL |
| ORT + CoreML EP, **dynamic shapes** | 100ms | 17ms | ~270ms | FAIL | FAIL | FAIL |
| ORT + CoreML EP, **static shapes** | 47ms | 6ms | **53ms** | PASS | PASS | FAIL |
| **Native CoreML (objc2-core-ml)** | **6ms** | **1ms** | **21ms** | **PASS** | **PASS** | **PASS** |

**Key finding - ORT CoreML EP dynamic shapes (E5RT warnings):** ORT passes raw ONNX dynamic
dims to CoreML, which cannot route unbounded-dimension ops to ANE. CoreML silently falls back
to CPU for those layers - explaining the 100ms det with dynamic shapes. E5RT errors in stderr
are warnings, not failures, but they confirm partial CPU fallback. Static shapes suppress this
by giving CoreML a fixed shape contract, allowing full ANE routing (47ms).

**Architecture decision confirmed:** Native CoreML (objc2-core-ml) is the macOS primary path.
It is 2.5x faster than ORT CoreML EP static-shape, and 5x faster in the combined pipeline.
ORT CoreML EP static-shape (53ms) is a viable secondary option but misses the 30ms goal.

**Linux / Windows:** No ANE/CoreML. ORT CPU EP is the baseline. ORT + CUDA EP is possible
but out of scope for v0.1.0.

>  **Correction (2026-05-07, @rust-engineer deep research):** The `ort` crate's
> `download-binaries` feature downloads pyke.io custom builds with `feature set: none`
> (MLAS only - confirmed from `ort-sys` build output). The MKL-DNN benchmark numbers
> cited in PaddleOCR's official docs (28.15ms det / 5.32ms rec) are for their own
> C++ build with `--use_dnnl`. They are **not reproducible** with the `ort` crate
> prebuilt binary. Corrected x86 baseline: ~55-65ms det, ~18-25ms rec per region.

---

Crate Topology

```
farscry/                         <- repo root (NO workspace Cargo.toml yet - BLOCKER)
├── Cargo.toml                   <- virtual manifest [workspace] - MUST BE CREATED FIRST
│
├── crates/
│   ├── farscry-core/            <- VASP types, Traits, Error - ZERO external deps
│   │   └── src/
│   │       ├── traits.rs        <- OcrEngine, Preprocessor, Classifier, Formatter, Differ
│   │       ├── types.rs         <- HocrOutput, TypedUiTree, VaspOutput, StateId, VaspDelta
│   │       └── error.rs         <- FarscryError (thiserror)
│   │
│   ├── farscry-ocr/             <- OCR abstraction - re-exports OcrEngine, selects backend
│   │   ├── Cargo.toml           <- features: ["coreml" (default on macOS), "ort"]
│   │   └── src/
│   │       ├── lib.rs           <- pub use backend::OcrBackend; (selected by cfg/feature)
│   │       ├── coreml.rs        <- #[cfg(feature="coreml")] - wraps farscry-ocr-coreml
│   │       └── ort.rs           <- #[cfg(feature="ort")]    - wraps farscry-ocr-ort
│   │
│   ├── farscry-ocr-coreml/      <- CoreML backend (macOS ONLY)
│   │   ├── Cargo.toml           <- objc2-core-ml, objc2-foundation, objc2
│   │   └── src/
│   │       ├── lib.rs           <- pub struct CoreMlOcr; impl OcrEngine for CoreMlOcr
│   │       ├── model.rs         <- model load + first-run compile (MLModel.compileModel)
│   │       ├── inference.rs     <- det + rec inference, MLMultiArray I/O
│   │       └── postprocess.rs   <- raw tensor output -> HocrOutput
│   │
│   ├── farscry-ocr-ort/         <- ORT backend (cross-platform)
│   │   ├── Cargo.toml           <- oar-ocr = { features = ["default"] } or ort directly
│   │   └── src/
│   │       └── lib.rs           <- pub struct OrtOcr; impl OcrEngine for OrtOcr
│   │
│   ├── farscry-preprocessor/    <- image resize, greyscale, padding (image crate)
│   ├── farscry-classifier/      <- EfficientNet-B0 INT8 via ort -> TypedUiTree
│   ├── farscry-diff/            <- semantic delta + StateId fingerprint
│   ├── farscry-formatter/       <- VASP JSON output
│   ├── farscry-mcp/             <- MCP server (farscry serve --mcp)
│   └── farscry/                 <- binary CLI - thin orchestrator, no business logic
│
├── spike/                       <- NOT a workspace member
├── spike-coreml-ep/             <- NOT a workspace member
└── spike-native-coreml/         <- NOT a workspace member
```

---

Backend Selection Strategy

Rule
The `farscry-ocr` crate selects the backend at **compile time** via Cargo features +
`#[cfg]` gates. No runtime detection in v0.1.0.

Feature flags on `farscry-ocr`

```toml
[features]
default = []                     # platform default selected by build.rs or user

coreml = ["dep:farscry-ocr-coreml"]
ort    = ["dep:farscry-ocr-ort"]
```

Platform defaults

The root `farscry/Cargo.toml` (binary) selects the right feature at build time:

```toml
In crates/farscry/Cargo.toml
[target.'cfg(target_os = "macos")'.dependencies]
farscry-ocr = { path = "../farscry-ocr", features = ["coreml"] }

[target.'cfg(not(target_os = "macos"))'.dependencies]
farscry-ocr = { path = "../farscry-ocr", features = ["ort"] }
```

Cross-compilation guard

`farscry-ocr-coreml` has `#![cfg(target_os = "macos")]` at crate root AND in
`Cargo.toml` as a target-specific dependency. This means:
- The crate will not compile on Linux/Windows (hard error, caught at cargo check)
- `cargo check --workspace` on Linux MUST exclude the coreml crate or use feature flags
- CI solution: `cargo check -p farscry-ocr-ort` on Linux, `cargo check --workspace` on macOS

---

Model Lifecycle

Model files
| File | Format | Size | Precision | Architecture | Purpose |
|---|---|---|---|---|---|
| `pp-ocrv5_mobile_det.onnx` | ONNX opset 11 | 4.6 MB | **FP32** | DBNet++ mobile (MobileNetV3-RSE, **no DCN**) | Text detection (det) |
| `en_pp-ocrv5_mobile_rec.onnx` | ONNX opset 11 | 7.5 MB | **FP32** | SVTR-LCNet (CNN+Transformer+CTC, 437-char dict) | Text recognition (rec) |
| `pp-ocrv5_mobile_det.mlpackage` | CoreML MLProgram | ~9 MB | FP32 | Same | Converted det (CoreML input) |
| `en_pp-ocrv5_mobile_rec.mlpackage` | CoreML MLProgram | ~13 MB | FP32 | Same | Converted rec (CoreML input) |
| `pp-ocrv5_mobile_det.mlmodelc` | Compiled CoreML | ~12 MB | FP32 | Same | Runtime-ready (cached) |
| `en_pp-ocrv5_mobile_rec.mlmodelc` | Compiled CoreML | ~16 MB | FP32 | Same | Runtime-ready (cached) |

> **Notes (2026-05-07):**
> - Both ONNX models are **FP32**. No official INT8 ONNX variants exist - PaddleOCR's
>   INT8 models are Paddle Lite format (`.nb`), incompatible with ONNX Runtime.
>   File sizes (4.6 MB, 7.5 MB) arithmetically confirm FP32 (INT8 equivalents ≈ 1.1 MB / 1.9 MB).
> - **DCN correction:** `pp-ocrv5_mobile_det.onnx` is **confirmed DCN-free** via CoreML MIL
>   IR inspection (zero `deform_conv` ops, opset 11 predate ONNX DeformConv standardization).
>   DCN is exclusive to the server variant (ResNet50+DCNv2, ~84 MB). This corrects an earlier
>   claim in this doc.
> - Total English-only distribution: **~12 MB** (4.6 + 7.5). The "50-100 MB model download"
>   figure in spec v3 §5 refers to the server or full Chinese+Japanese pack.

Distribution strategy
- `.onnx` models are bundled in the binary archive (already planned in release.yml)
- `.mlpackage` files are also bundled (macOS archive only - `aarch64-apple-darwin` + `x86_64-apple-darwin`)
- `.mlmodelc` files are NOT distributed - they are compiled on first run and cached at `~/.farscry/models/`
- Total macOS archive size impact: +~22MB for `.mlpackage` files (acceptable given ORT dylib is already ~22-30MB)

ONNX -> CoreML conversion
Performed ONCE by the maintainer using `spike-native-coreml/convert_models.py` (coremltools 9.0).
The resulting `.mlpackage` files are committed to the release assets.
Users do NOT need Python or coremltools installed.

Conversion command (maintainer only):
```bash
uv run spike-native-coreml/convert_models.py
```

First-run CoreML compilation
On macOS, on first invocation (or if cache is missing):

```
farscry analyze screenshot.png
  -> farscry-ocr-coreml: cache miss at ~/.farscry/models/*.mlmodelc
  -> calls MLModel.compileModel(at: mlpackage_url)   <- ~3s, one-time
  -> writes .mlmodelc to ~/.farscry/models/
  -> loads compiled model -> inference proceeds
```

Subsequent runs: cache hit, model loads in ~185ms (JIT already done by CoreML).

**Implementation note:** `MLModel.compileModelAtURL:error:` is NOT in the current
`objc2-core-ml = "0.3"` generated bindings. Must call via `objc2::msg_send!` macro:

```rust

use objc2::{msg_send, ClassType};
use objc2_core_ml::MLModel;
use objc2_foundation::{NSError, NSURL};

pub unsafe fn compile_model(src: &NSURL, dst: &NSURL) -> Result<NSURL, ...> {
    let compiled_url: Option<Retained<NSURL>> = msg_send![
        MLModel::class(), compileModelAtURL: src, toURL: dst, error: _
    ];

}
```

This is a `@rust-engineer *review` item before implementation.

Model cache path
```
~/.farscry/
├── models/
│   ├── pp-ocrv5_mobile_det.mlmodelc/
│   └── en_pp-ocrv5_mobile_rec.mlmodelc/
└── version             <- written to invalidate cache on model upgrade
```

Cache invalidation: compare bundled model version hash against `~/.farscry/version`.
If mismatch: recompile. This prevents stale compiled models after farscry update.

---

Data Flow

```
DynamicImage (screenshot)
    │
    ▼
farscry-preprocessor
  └── resize to 960x960 (det input), preserve aspect ratio with padding
    │
    ▼
farscry-ocr-coreml::CoreMlOcr::extract()
  ├── det: MLMultiArray [1,3,960,960] -> CoreML -> raw det output tensor
  │   └── postprocess: decode bounding boxes -> Vec<BoundingBox>
  ├── rec: for each BoundingBox (up to ~15 regions):
  │   └── crop + resize to [1,3,48,320] -> CoreML -> raw rec tensor
  │       └── postprocess: CTC decode -> String
  └── returns HocrOutput { regions: Vec<TextRegion { bbox, text, confidence }> }
    │
    ▼
farscry-classifier::classify()
  └── EfficientNet-B0 INT8 -> TypedUiTree
    │
    ▼
farscry-diff::diff() (if previous state available)
  └── VaspDelta
    │
    ▼
farscry-formatter::format()
  └── VaspOutput (stdout - JSON or VASP protocol format)
```

---

Daemon Mode (farscry serve --mcp)

```
farscry serve --mcp
  └── loads models ONCE into Arc<Mutex<CoreMlOcr>>
  └── MCP server loop: accepts tool_call messages
      └── each call: lock Arc -> run inference -> release -> return result
```

**Important:** CoreML models stay warm in memory between calls. The ANE JIT compilation
happens once at daemon startup. Subsequent tool calls hit steady-state latency (21ms combined).

The `Arc<Mutex<CoreMlOcr>>` is correct here - CoreML sessions are NOT safe to call
concurrently from multiple threads without explicit handling. `Mutex` is the right primitive.
Do NOT change to `RwLock` (inference is write-side, there is no read-only operation).

---

x86 Latency Optimization Plan (Linux / Windows - ORT CPU EP)

> **Research date:** 2026-05-07 - @rust-engineer *review
> **Scope:** `farscry-ocr-ort` crate on Linux x86_64 / Windows x86_64
> **Corrected baseline:** ~300ms warm (MLAS, FP32, default settings, ~12 text regions)

Corrected Baseline Breakdown

| Stage | Naive estimate | Corrected (MLAS, measured) | Notes |
|---|---|---|---|
| Detection (1080p -> 960px) | 28ms No | **~55-65ms** | MLAS, not MKL-DNN |
| Recognition per region | 5.3ms No | **~18-25ms** | SVTR-LCNet, MLAS |
| 12 regions total | ~91ms No | **~275-360ms** | Matches 300ms validated |

The 28ms/5.3ms numbers in PaddleOCR's docs require `--use_dnnl` in their C++ build.
Not achievable with `ort` `download-binaries`.

Optimization Paths - GO/NO-GO

| Path | Method | Est. latency (12 regions) | Effort | v0.1.0? | Notes |
|---|---|---|---|---|---|
| **A: Thread tuning** | `with_intra_op_num_threads(physical_cores)`, `inter=1` | ~200ms | 30 min |  YES | Biggest free win |
| **B: Level2 graph opt** | `with_optimization_level(Level2)` | ~185ms | 5 min |  YES | Level3 causes 8x Intel regression (ORT #26992) |
| **C: 640px detection** | `TextDetectionConfig { limit_side_len: 640 }` | ~140ms | 2 hrs |  YES | Screen-type conditional; risk on <12px text |
| **D: Region batching** | `.region_batch_size(16)` | ~130ms | 5 min |  YES | Amortize ORT session overhead |
| **F: English rec model** | Use `en_pp-ocrv5_mobile_rec.onnx` (7.5 MB, not 15.8 MB) | ~110ms | 30 min |  YES | Already implied by `--install-lang` arch |
| **E: INT8 `quantize_static`** | Calibration + `onnxruntime.quantization` static PTQ offline | ~150ms | 2 days |  NEEDS SPIKE | Requires calibration data; NOT tested yet. `quantize_dynamic` is WRONG (see below) |
| **G: DirectML EP** | `features = ["directml"]` + `ep::DirectML` (Windows only) | ~60-120ms | 2 hrs |  NEEDS CI | API confirmed; needs Windows GitHub Actions runner to benchmark |
| **H: oneDNN EP** | Custom ORT build with `--use_dnnl` | ~55-80ms | 1 week | No v0.2.0 | `libonnxruntime_providers_dnnl.so` NOT in official release (confirmed by inspection); must compile |
| **I: OpenVINO EP** | - | ~50-70ms | 3 days | No NO-GO | Intel-only; 200-400 MB user install; dynamic shapes bug |
| **J: rten** | Pure-Rust ONNX runtime, `.rten` format required | ~300ms | 5 days | No v0.2.0 | ~20% slower than ORT on x86; value is smaller binary + musl |
| **K: tract** | Pure-Rust ONNX runtime | ~300ms | 5 days | No NO-GO | Unconfirmed PP-OCR compat; no speed advantage |

> ** INT8 CORRECTION (spike results, 2026-05-08):**
> `quantize_dynamic` is **5x SLOWER** than FP32 for PP-OCRv5 and produces garbled text (14%
> accuracy). Root cause: dynamic quantization stores weights as INT8 but **dequantizes back to
> FP32 at runtime** - zero compute speedup, added overhead. This is the wrong tool for CNNs.
>
> **Correct INT8 path: `quantize_static`** - quantizes both weights and activations, requires
> ~100-500 calibration UI screenshots, produces genuine INT8 compute at inference time.
> `quantize_static` is untested; it is NOT a "zero Rust changes, 1 day" task.
> Estimate revised to 2-3 days (calibration data collection + Python tooling + accuracy validation).

Cumulative Projection (v0.1.0 - A+B+C+D+F combined)

**MEASURED on Apple M4 Pro (ARM64), ORT MLAS, 5-run warm average, 22-region screenshot:**

```
303ms  BASELINE  (FP32, default ORT, 960px det, batch=6)      - measured
263ms  + A+B     (thread tuning + Level2)       1.15x speedup - measured on ARM
226ms  + A+B+C+D (+ 640px det, batch=32)        1.34x speedup - measured on ARM
```

**Accuracy check at 640px:** 23 regions (vs 22 baseline) - no text accuracy loss. 640px confirmed safe for this test image.

**x86 extrapolated projection (M4 Pro ARM -> 8-core x86 i7 AVX2):**

```
300ms  BASELINE  (MLAS, FP32, default settings)
  -> ~210ms  after A+B: thread tuning avoids HT contention     (~30%)
  -> ~160ms  after C: 640px det (area scales quadratically)    (~24%)
  -> ~145ms  after D: region_batch_size(16)                    (~9%)
  -> ~120ms  after F: English model default                    (~17%)
  ───────────────────────────────────────────────────────────────
  ~120ms projected on x86 with configs-only - no model changes
```

**INT8 via `quantize_static` (untested, calibration required):**
- Intel Skylake/Comet Lake (AVX2 only): ~75-100ms (projected)
- Intel Tiger Lake / AMD Zen4 (AVXVNNI): ~50-70ms (projected)
- Status:  NEEDS separate static-INT8 spike with calibration dataset

Implementation Notes for `farscry-ocr-ort`

**Confirmed working API** (verified from oar-ocr-core-0.6.3 source, spike compile-tested):

```rust

use oar_ocr::core::config::{OrtGraphOptimizationLevel, OrtSessionConfig};
use oar_ocr::domain::TextDetectionConfig;
use oar_ocr::prelude::*;
use oar_ocr::processors::LimitType;

let physical_cores = {
    let logical = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    if cfg!(target_arch = "x86_64") { (logical / 2).max(1) } else { logical }
};

let ocr = OAROCRBuilder::new(det_path, rec_path, dict_path)
    .ort_session(OrtSessionConfig::new()
        .with_intra_threads(physical_cores)   // usize, NOT i16
        .with_inter_threads(1)
        .with_optimization_level(OrtGraphOptimizationLevel::Level2))  // NOT Level3
    .text_detection_config(TextDetectionConfig {
        limit_side_len: Some(640),             // 640 confirmed safe on test image
        limit_type: Some(LimitType::Max),
        ..TextDetectionConfig::default()
    })
    .region_batch_size(32)
    .build()?;
```

> **Q-ORT-4 resolved:** oar-ocr v0.6.3 **DOES** fully wire `OrtSessionConfig` through to the
> actual ORT `SessionBuilder` (confirmed in `ort_infer_config.rs::apply_ort_config()`).
> No need to bypass oar-ocr for thread/optimization config. Only CoreML advanced options
> (ModelFormat, SpecializationStrategy, model_cache_dir) require bypassing oar-ocr.

> **Note:** `with_intra_threads` takes `usize` in oar-ocr (wraps to ORT's `i16` internally).

Open Questions for `@rust-engineer *review`

| # | Question | Status | Blocking |
|---|---|---|---|
| Q-ORT-1 | Does `with_intra_threads` on oar-ocr take `i16` or `usize`? |  `usize` - confirmed from source | - closed |
| Q-ORT-2 | Is `ep::OneDNN` in ort API when prebuilt doesn't include it? (API vs runtime mismatch) |  OPEN - `OrtExecutionProvider::OneDNN` exists in config; `libonnxruntime_providers_dnnl.so` not in prebuilt -> silent fallback to CPU EP at runtime | v0.2.0 oneDNN spike |
| Q-ORT-3 | rten: does v0.24.0 support loading `.onnx` directly or requires `.rten` conversion? |  OPEN - conflicting reports; needs `rten-convert --check` spike | v0.2.0 rten evaluation |
| Q-ORT-4 | Is oar-ocr `OrtSessionConfig` wired through to actual `Session::builder()`? |  YES - confirmed from `ort_infer_config.rs::apply_ort_config()` | - closed |
| Q-ORT-5 | Does `quantize_static` on PP-OCRv5 produce correct results without garbling? | No OPEN - `quantize_dynamic` confirmed garbled (14% accuracy); `quantize_static` untested | INT8 v0.1.0 decision |
| Q-ORT-6 | DirectML EP measured latency on Windows with Intel iGPU? | No OPEN - needs Windows runner in CI | SPIKE G completion |

---

Environment Matrix

| Environment | OS | OCR Backend | ORT EP | Expected latency | Notes |
|---|---|---|---|---|---|
| macOS arm64 (M-series) | macOS 13+ | CoreML (native) | N/A | **21ms** | Primary target, ANE engaged |
| macOS x86_64 (Intel) | macOS 13+ | CoreML (native) | N/A | **~60ms** | GPU path, no ANE |
| Linux x86_64 | Ubuntu 22+ | ORT CPU EP | MLAS (CPU) | **~110ms** (optimized) | `download-binaries` MLAS only; oneDNN in v0.2.0 |
| Windows x86_64 | Win 10+ | ORT CPU EP + DirectML | CPU + DirectML iGPU | **~60-110ms** | DirectML in prebuilt; iGPU fallback to CPU |
| Linux x86_64 w/ NVIDIA | Ubuntu 22+ | ORT CUDA EP | CUDA | ~30ms | v0.2.0 target, out of scope now |
| Linux x86_64 (v0.2.0) | Ubuntu 22+ | ORT CPU EP | oneDNN | ~55-80ms | Requires custom ORT build with `--use_dnnl` |

Latency estimates for Linux/Windows assume: English model, 12 text regions, 1080p input,
all v0.1.0 optimizations applied (A+B+C+D+F from x86 plan above).

---

CI/CD Impact

The existing `ci.yml` and `release.yml` were written for ORT-only. They require updates:

ci.yml changes needed
1. Add macOS runner job: `cargo check --workspace` on macOS (needed to validate CoreML crate)
2. Linux: `cargo check --workspace` must exclude `farscry-ocr-coreml` (no Apple SDK)
3. Add step to verify `.mlpackage` files are present in release assets

release.yml changes needed
1. macOS build must bundle `.mlpackage` files alongside the binary
2. Linux/Windows builds must NOT include `.mlpackage` files
3. Separate SHA256 checksums for model files (model integrity verification at first run)

This is `@devops *onboard-project` scope - CI changes must not be made before that step.

---

Rollback Procedure

If native CoreML backend fails in production:

1. **Per-invocation fallback:** `farscry-ocr` can fall back to ORT backend at runtime if
   CoreML model load fails. Gate with `FARSCRY_OCR_BACKEND=ort` env var override.
2. **Binary rollback:** The ORT backend (`farscry-ocr-ort`) is always compiled alongside
   the CoreML backend on macOS. Switching is a feature flag rebuild.
3. **Model rollback:** If a compiled `.mlmodelc` is corrupted, delete `~/.farscry/models/`
   and rerun - triggers recompile from bundled `.mlpackage`.

---

Known Limitations and Technical Debt

| Item | Severity | Notes |
|---|---|---|
| `MLModel.compileModelAtURL:error:` not in objc2-core-ml bindings | HIGH | Must use `msg_send!` - fragile, not type-safe |
| `dataPointer()` is deprecated in CoreML | MEDIUM | Must migrate to `getMutableBytesWithHandler` before v0.2.0 |
| Fixed input shapes (960x960 det, 48x320 rec) | MEDIUM | Images outside these shapes need preprocessing; see farscry-preprocessor |
| `.mlpackage` conversion requires coremltools 9.0 + Python | LOW | Maintainer-only step, documented; users are not affected |
| CoreML backend macOS 13+ only | LOW | macOS 12 users fall back to ORT |
| No CUDA EP support | LOW | v0.2.0 target |
| `spike*` directories are not workspace members | INFO | Remove after workspace is initialized |

---

Open Questions (Blockers Before Any Crate Is Written)

These must be resolved before `@dev` starts. Each is a question for a specific role:

| # | Question | Owner | Blocking what |
|---|---|---|---|
| Q1 | Where is the VASP spec? What version does v0.1.0 target? | @analyst + @pm | farscry-core types, VaspOutput shape |
| Q2 | What does `HocrOutput` look like? (fields, nested types) | @architect + @pm | farscry-core, farscry-ocr interface |
| Q3 | What is the `TypedUiTree` element taxonomy? (Button, Input, Label, ...) | @pm | farscry-classifier |
| Q4 | What is `StateId`? Hash of what? Format string? | @pm | farscry-diff |
| Q5 | What is the MVP scope for v0.1.0? CLI only? MCP? All 7 crates? | @pm | stories, CI scope |
| Q6 | ORT CoreML EP timing? (spike-coreml-ep is built, models are symlinked) | @ml-engineer | backend selection finality |
| Q7 | EfficientNet-B0 training data - RICO dataset? Custom? | @ml-engineer | farscry-classifier design |
| Q8 | Model distribution - GitHub Releases or bundled in archive? | @devops | release.yml, model lifecycle |
| Q9 | `MLModel.compileModelAtURL` - is there a safe objc2 approach vs `msg_send!`? | @rust-engineer | farscry-ocr-coreml/model.rs |
| Q10 | Cross-compilation: how does `cargo check --workspace` work on Linux without Apple SDK? | @devops | ci.yml |

---

Pre-Implementation Checklist (AGENTS.md steps)

Per `AGENTS.md`, the full sequence before `@dev` can write any crate:

- [ ] **Q1-Q5 answered** - foundational questions from above table
- [ ] **spike-coreml-ep timed** - ORT CoreML EP baseline measured (Q6)
- [ ] **@pm *create-doc** - PRD in `docs/projects/farscry-v0.1.0.md`
- [ ] **@devops *onboard-project** - CI/CD updated for 4-platform + macOS CoreML
- [ ] **@sm *draft** - stories in `docs/stories/` for each crate (farscry-core first)
- [ ] **@po *validate-story-draft** - each story validated against VASP spec
- [ ] **@security-engineer *audit** - `objc2`, `objc2-core-ml`, `objc2-foundation` supply chain
- [ ] **@rust-engineer *review** - workspace structure + trait design + `msg_send!` pattern
- [ ] **Workspace Cargo.toml created** - virtual manifest before first crate

---

Architecture Sign-off Required

Before `@dev` proceeds on any crate, the following must sign off on this document:

- [ ] `@po` - VASP spec alignment confirmed
- [ ] `@security-engineer` - objc2 supply chain audit passed
- [ ] `@rust-engineer` - trait design and workspace topology approved
- [ ] `@arromber` - adversarial review (this can wait until pre-release)
