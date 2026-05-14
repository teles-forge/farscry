use farscry_core::FarscryError;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

const DET_WEIGHT_SHA256: &str = "f5469fd464978f48472cb2de09350431efa32c4441acba9505ad32ece01378cd";
const REC_WEIGHT_SHA256: &str = "c18a00ccf919785a33f8f2ebcd22d111e2434e974c6bdb901ed99b7d6dae4a2d";

fn weight_bin_path(mlpackage_dir: &Path) -> std::path::PathBuf {
    mlpackage_dir
        .join("Data")
        .join("com.apple.CoreML")
        .join("weights")
        .join("weight.bin")
}

pub fn verify_models(models_dir: &Path) -> Result<(), FarscryError> {
    let models: &[(&str, &str)] = &[
        ("farscry-det.mlpackage", DET_WEIGHT_SHA256),
        ("farscry-rec.mlpackage", REC_WEIGHT_SHA256),
    ];

    let mut manifest: HashMap<String, String> = HashMap::new();

    for &(pkg_name, expected) in models {
        let pkg_path = models_dir.join(pkg_name);

        if !pkg_path.exists() {
            continue;
        }

        let weight_path = weight_bin_path(&pkg_path);
        if !weight_path.exists() {
            return Err(FarscryError::ModelNotFound { path: weight_path });
        }

        let actual = sha256_file(&weight_path)?;
        if actual != expected {
            return Err(FarscryError::ModelIntegrity {
                model: pkg_name.to_string(),
                expected: expected.to_string(),
                actual,
            });
        }

        manifest.insert(pkg_name.to_string(), actual);
    }

    let Some(home) = dirs::home_dir() else {
        return Ok(());
    };
    let manifest_path = home.join(".farscry").join(".manifest.json");
    let mut existing = load_manifest(&manifest_path);
    existing.extend(manifest);
    save_manifest(&manifest_path, &existing)?;

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
    let serialized = serde_json::to_string_pretty(manifest)
        .map_err(|e| FarscryError::OcrFailed(format!("Failed to serialize manifest: {e}")))?;
    std::fs::write(path, serialized)
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

        assert!(result.is_ok());
    }

    #[test]
    fn test_weight_bin_path() {
        let pkg = std::path::Path::new("/models/farscry-det.mlpackage");
        let expected = pkg
            .join("Data")
            .join("com.apple.CoreML")
            .join("weights")
            .join("weight.bin");
        assert_eq!(weight_bin_path(pkg), expected);
    }
}
