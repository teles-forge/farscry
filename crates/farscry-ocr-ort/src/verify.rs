use farscry_core::FarscryError;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

const DET_SHA256: &str = "1eb7b4f7ab657ebd1c66d5f79bca7497f29768a2e3c15e52daecbba1a8e4a039";
const REC_SHA256: &str = "8307465d3c9ef2ba4055c3bd0be55aafe11f518630212b7598b70ccb376028ac";

pub fn verify_models(models_dir: &Path) -> Result<(), FarscryError> {
    let models: &[(&str, &str)] = &[
        ("pp-ocrv5_mobile_det.onnx", DET_SHA256),
        ("en_pp-ocrv5_mobile_rec.onnx", REC_SHA256),
    ];

    for &(filename, expected) in models {
        let path = models_dir.join(filename);

        if !path.exists() {
            return Err(FarscryError::ModelNotFound { path });
        }

        let actual = sha256_file(&path)?;
        if actual != expected {
            return Err(FarscryError::ModelIntegrity {
                model: filename.to_string(),
                expected: expected.to_string(),
                actual,
            });
        }
    }

    let manifest_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".farscry")
        .join(".manifest.json");
    let mut manifest = load_manifest(&manifest_path);
    for &(filename, expected) in models {
        manifest.insert(filename.to_string(), expected.to_string());
    }
    save_manifest(&manifest_path, &manifest)?;

    Ok(())
}

fn load_manifest(path: &Path) -> HashMap<String, String> {
    if path.exists() {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_default()).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_manifest(path: &Path, manifest: &HashMap<String, String>) -> Result<(), FarscryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(manifest).unwrap())
        .map_err(|e| FarscryError::OcrFailed(format!("Failed to write manifest: {e}")))
}

fn sha256_file(path: &Path) -> Result<String, FarscryError> {
    let contents = std::fs::read(path)
        .map_err(|e| FarscryError::OcrFailed(format!("Failed to read file: {e}")))?;
    let hash = Sha256::digest(&contents);
    Ok(format!("{:x}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_sha256.txt");
        std::fs::write(&test_file, b"test content").unwrap();

        let hash = sha256_file(&test_file).unwrap();
        assert_eq!(hash.len(), 64);

        std::fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_verify_models_missing_files() {
        let temp_dir = std::env::temp_dir();
        let result = verify_models(&temp_dir);

        assert!(result.is_err());
    }
}
