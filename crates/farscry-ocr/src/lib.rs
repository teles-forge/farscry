use farscry_core::FarscryError;
use std::path::Path;

#[cfg(all(target_os = "macos", target_arch = "aarch64", feature = "coreml"))]
pub use farscry_ocr_coreml::CoreMlOcrEngine as PlatformOcrEngine;

#[cfg(not(all(target_os = "macos", target_arch = "aarch64", feature = "coreml")))]
pub use farscry_ocr_ort::OrtOcrEngine as PlatformOcrEngine;

pub use farscry_core::traits::OcrEngine;

pub fn build_ocr_engine(models_dir: &Path) -> Result<PlatformOcrEngine, FarscryError> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64", feature = "coreml"))]
    return farscry_ocr_coreml::CoreMlOcrEngine::from_models_dir(models_dir);

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64", feature = "coreml")))]
    return farscry_ocr_ort::OrtOcrEngine::from_models_dir(models_dir);
}

pub fn active_backend() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64", feature = "coreml"))]
    return "CoreML (native ANE/GPU, batch=32)";

    #[cfg(not(all(target_os = "macos", target_arch = "aarch64", feature = "coreml")))]
    return "ORT (CPU, cross-platform)";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_backend_is_non_empty() {
        assert!(!active_backend().is_empty());
    }
}
