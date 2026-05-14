use crate::error::FarscryError;
use crate::types::{OcrOutput, ScreenType, StateId, UiElement, VaspDelta, VaspOutput};
use image::DynamicImage;

/// Converts a raw image into the format expected by the OCR engine.
pub trait Preprocessor: Send + Sync + 'static {
    /// Applies preprocessing (resize, normalize, convert) to the image.
    fn process(&self, image: DynamicImage) -> DynamicImage;
}

/// Extracts text regions and their bounding boxes from an image.
pub trait OcrEngine: Send + Sync + 'static {
    /// Runs OCR on the image. Returns an error if the model fails or the image format is unsupported.
    fn extract(&self, image: &DynamicImage) -> Result<OcrOutput, FarscryError>;
}

/// Classifies raw OCR text regions into typed UI elements.
pub trait ElementClassifier: Send + Sync + 'static {
    /// Returns a list of typed UI elements derived from the OCR output.
    fn classify(&self, ocr: &OcrOutput) -> Vec<UiElement>;
}

/// Determines the overall screen type from the classified UI elements.
pub trait ScreenClassifier: Send + Sync + 'static {
    /// Returns the dominant screen type (Terminal, Config, Error, Ui, etc.).
    fn classify(&self, elements: &[UiElement]) -> ScreenType;
}

/// Computes a perceptual hash of an image for change detection.
pub trait StateHasher: Send + Sync + 'static {
    /// Returns a compact state identifier (pHash) for the image.
    fn hash(&self, image: &DynamicImage) -> StateId;
}

/// Converts a classified screen into a VASP output document.
pub trait VaspFormatter: Send + Sync + 'static {
    /// Formats the classified screen into a `VaspOutput` ready for agents.
    fn format(&self, screen: &crate::types::ClassifiedScreen) -> VaspOutput;
}

/// Computes a semantic diff between two VASP snapshots.
pub trait DiffEngine: Send + Sync + 'static {
    /// Returns a `VaspDelta` describing what changed between `before` and `after`.
    /// `before_dims` and `after_dims` are the pixel dimensions of the respective images,
    /// used to compute scroll offsets and positional deltas.
    fn diff(
        &self,
        before: &VaspOutput,
        after: &VaspOutput,
        before_dims: Option<(u32, u32)>,
        after_dims: Option<(u32, u32)>,
    ) -> VaspDelta;
}
