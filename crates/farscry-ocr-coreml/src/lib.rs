#[cfg(target_os = "macos")]
pub mod dict;
#[cfg(target_os = "macos")]
pub mod engine;
#[cfg(target_os = "macos")]
pub mod model;
#[cfg(target_os = "macos")]
pub mod postprocess;
#[cfg(target_os = "macos")]
pub mod preprocess;
#[cfg(target_os = "macos")]
pub mod verify;

#[cfg(target_os = "macos")]
pub use engine::CoreMlOcrEngine;

#[cfg(not(target_os = "macos"))]
compile_error!("farscry-ocr-coreml is macOS-only and does not compile on other platforms");
