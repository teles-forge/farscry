use crate::error::FarscryError;
use crate::traits::{
    ElementClassifier, OcrEngine, Preprocessor, ScreenClassifier, StateHasher, VaspFormatter,
};
use crate::types::{BatchResult, ClassifiedScreen, Confidence, VaspOutput};
use image::DynamicImage;
use std::sync::Arc;

pub struct Pipeline {
    preprocessor: Arc<dyn Preprocessor>,
    ocr: Arc<dyn OcrEngine>,
    element_classifier: Arc<dyn ElementClassifier>,
    screen_classifier: Arc<dyn ScreenClassifier>,
    state_hasher: Arc<dyn StateHasher>,
    formatter: Arc<dyn VaspFormatter>,
}

impl Pipeline {
    pub fn new(
        preprocessor: Arc<dyn Preprocessor>,
        ocr: Arc<dyn OcrEngine>,
        element_classifier: Arc<dyn ElementClassifier>,
        screen_classifier: Arc<dyn ScreenClassifier>,
        state_hasher: Arc<dyn StateHasher>,
        formatter: Arc<dyn VaspFormatter>,
    ) -> Self {
        Self {
            preprocessor,
            ocr,
            element_classifier,
            screen_classifier,
            state_hasher,
            formatter,
        }
    }

    pub fn process(&self, image: DynamicImage) -> Result<VaspOutput, FarscryError> {
        let preprocessed = self.preprocessor.process(image);
        let state_id = self.state_hasher.hash(&preprocessed);
        let ocr_output = self.ocr.extract(&preprocessed)?;
        let elements = self.element_classifier.classify(&ocr_output);
        let screen_type = self.screen_classifier.classify(&elements);
        let screen = ClassifiedScreen {
            ui_tree: elements,
            screen_type,
            state_id,
            lang: "eng".into(),
            confidence: Confidence::High,
        };
        Ok(self.formatter.format(&screen))
    }

    pub fn process_batch(&self, paths: Vec<std::path::PathBuf>) -> Vec<BatchResult> {
        use rayon::prelude::*;
        paths
            .par_iter()
            .map(|path| {
                let result = image::open(path)
                    .map_err(|e| FarscryError::ImageLoad {
                        path: path.clone(),
                        source: e,
                    })
                    .and_then(|img| self.process(img));
                BatchResult {
                    path: path.clone(),
                    output: result,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::phash_image;
    use crate::types::{ScreenType, StateId, UiElement};

    struct MockPreprocessor;
    impl Preprocessor for MockPreprocessor {
        fn process(&self, image: DynamicImage) -> DynamicImage {
            image
        }
    }

    struct MockOcrEngine;
    impl OcrEngine for MockOcrEngine {
        fn extract(&self, _image: &DynamicImage) -> Result<crate::types::OcrOutput, FarscryError> {
            Ok(crate::types::OcrOutput {
                regions: vec![],
                width: 100,
                height: 100,
            })
        }
    }

    struct MockElementClassifier;
    impl ElementClassifier for MockElementClassifier {
        fn classify(&self, _ocr: &crate::types::OcrOutput) -> Vec<UiElement> {
            vec![]
        }
    }

    struct MockScreenClassifier;
    impl ScreenClassifier for MockScreenClassifier {
        fn classify(&self, _elements: &[UiElement]) -> ScreenType {
            ScreenType::Unknown
        }
    }

    struct MockStateHasher;
    impl StateHasher for MockStateHasher {
        fn hash(&self, image: &DynamicImage) -> StateId {
            phash_image(image)
        }
    }

    struct MockFormatter;
    impl VaspFormatter for MockFormatter {
        fn format(&self, screen: &ClassifiedScreen) -> VaspOutput {
            VaspOutput::new(
                screen.state_id,
                screen.screen_type,
                screen.confidence,
                &screen.lang,
                "mock context",
                screen.ui_tree.clone(),
                vec![],
            )
        }
    }

    #[test]
    fn test_pipeline_process() {
        let pipeline = Pipeline::new(
            Arc::new(MockPreprocessor),
            Arc::new(MockOcrEngine),
            Arc::new(MockElementClassifier),
            Arc::new(MockScreenClassifier),
            Arc::new(MockStateHasher),
            Arc::new(MockFormatter),
        );

        let img = image::RgbImage::new(100, 100);
        let result = pipeline.process(DynamicImage::ImageRgb8(img));

        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_process_batch() {
        let pipeline = Pipeline::new(
            Arc::new(MockPreprocessor),
            Arc::new(MockOcrEngine),
            Arc::new(MockElementClassifier),
            Arc::new(MockScreenClassifier),
            Arc::new(MockStateHasher),
            Arc::new(MockFormatter),
        );

        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("test_pipeline_batch.png");
        let img = image::RgbImage::new(100, 100);
        img.save(&test_path).unwrap();

        let results = pipeline.process_batch(vec![test_path.clone()]);
        assert_eq!(results.len(), 1);
        assert!(results[0].output.is_ok());

        std::fs::remove_file(test_path).ok();
    }
}
