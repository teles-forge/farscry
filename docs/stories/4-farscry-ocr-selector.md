Story 4 - farscry-ocr (selector crate)

**Status:** Ready
**Blocked by:** Stories 2 and 3
**Estimated hours:** 3h

What to build

A thin selector crate that chooses the right OCR backend at compile time using `cfg` flags. This is the only OCR crate the binary and other crates depend on.

Crate: `crates/farscry-ocr/`

**`src/lib.rs`** - compile-time backend selection:
```rust

pub use farscry_ocr_coreml::CoreMlOcrEngine as PlatformOcrEngine;


pub use farscry_ocr_ort::OrtOcrEngine as PlatformOcrEngine;

pub fn build_ocr_engine(models_dir: &std::path::Path)
    -> Result<PlatformOcrEngine, farscry_core::FarscryError> {
    PlatformOcrEngine::new(models_dir)
}
```

The `coreml` feature is enabled by default on macOS aarch64 builds.

**`Cargo.toml`:**
```toml
[features]
coreml = ["farscry-ocr-coreml"]

[dependencies]
farscry-ocr-ort = { path = "../farscry-ocr-ort" }

[target.'cfg(all(target_os = "macos", target_arch = "aarch64"))'.dependencies]
farscry-ocr-coreml = { path = "../farscry-ocr-coreml", optional = true }
```

Acceptance criteria

- [ ] On macOS aarch64 with `coreml` feature: `PlatformOcrEngine` = `CoreMlOcrEngine`
- [ ] On Linux x86_64: `PlatformOcrEngine` = `OrtOcrEngine`
- [ ] On macOS x86_64: `PlatformOcrEngine` = `OrtOcrEngine`
- [ ] `cargo build -p farscry-ocr` passes on all 4 platforms
- [ ] `build_ocr_engine(models_dir)` is the single public API
- [ ] publish = false in Cargo.toml

Dependencies

Story 2 (farscry-ocr-coreml), Story 3 (farscry-ocr-ort).
