use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum FarscryError {
    #[error("Failed to load image from '{path}': {source}")]
    ImageLoad {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error("Model integrity check failed for '{model}': expected {expected}, got {actual}")]
    ModelIntegrity {
        model: String,
        expected: String,
        actual: String,
    },

    #[error("Model not found at '{path}' - run `farscry --update-models`")]
    ModelNotFound { path: PathBuf },

    #[error("OCR engine failed: {0}")]
    OcrFailed(String),

    #[error("Input validation failed: {message}")]
    InvalidInput { message: String },

    #[error("Language '{0}' is not installed - run `farscry --install-lang {0}`")]
    LanguageNotInstalled(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl FarscryError {
    pub fn exit_code(&self) -> i32 {
        match self {
            FarscryError::LanguageNotInstalled(_) => 3,
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_language_not_installed() {
        let err = FarscryError::LanguageNotInstalled("eng".to_string());
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn test_exit_code_default() {
        let err = FarscryError::OcrFailed("test".to_string());
        assert_eq!(err.exit_code(), 1);
    }
}
