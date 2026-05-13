use farscry_core::FarscryError;
use objc2::rc::Retained;
use objc2_core_ml::{MLComputeUnits, MLModel, MLModelConfiguration};
use objc2_foundation::{NSString, NSURL};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn load_model(path: &Path) -> Result<Retained<MLModel>, FarscryError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| FarscryError::OcrFailed("Path is not valid UTF-8".to_string()))?;

    unsafe {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path_str));
        let config = MLModelConfiguration::new();
        config.setComputeUnits(MLComputeUnits::All);
        MLModel::modelWithContentsOfURL_configuration_error(&url, &config).map_err(|e| {
            FarscryError::OcrFailed(format!("MLModel::modelWithContentsOfURL failed: {:?}", e))
        })
    }
}

pub fn ensure_compiled(models_dir: &Path, name: &str) -> Result<PathBuf, FarscryError> {
    let cache_root = cache_dir()?;
    let cached_mc = cache_root.join(format!("{name}.mlmodelc"));
    let pkg_path = models_dir.join(format!("{name}.mlpackage"));

    if cached_mc.exists() && !is_stale(&cached_mc, &pkg_path) {
        return Ok(cached_mc);
    }

    let bundled_mc = models_dir.join(format!("{name}.mlmodelc"));
    if bundled_mc.exists() && !pkg_path.exists() {
        return Ok(bundled_mc);
    }
    if bundled_mc.exists() && !is_stale(&bundled_mc, &pkg_path) {
        return Ok(bundled_mc);
    }

    if !pkg_path.exists() {
        return Err(FarscryError::ModelNotFound { path: pkg_path });
    }

    eprintln!("[farscry] Compiling {name} for Apple Silicon (first run, ~3-5s)…");
    std::fs::create_dir_all(&cache_root)
        .map_err(|e| FarscryError::OcrFailed(format!("Cannot create cache dir: {e}")))?;

    let temp_mc = compile_mlpackage(&pkg_path)?;

    if cached_mc.exists() {
        std::fs::remove_dir_all(&cached_mc)
            .map_err(|e| FarscryError::OcrFailed(format!("Cannot remove stale .mlmodelc: {e}")))?;
    }
    std::fs::rename(&temp_mc, &cached_mc)
        .map_err(|e| FarscryError::OcrFailed(format!("Cannot move compiled model: {e}")))?;

    eprintln!(
        "[farscry] {name} compiled and cached at {}",
        cached_mc.display()
    );
    Ok(cached_mc)
}

fn compile_mlpackage(pkg_path: &Path) -> Result<PathBuf, FarscryError> {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;

    let pkg_str = pkg_path
        .to_str()
        .ok_or_else(|| FarscryError::OcrFailed("mlpackage path is not UTF-8".into()))?;

    let compiled_url: Option<Retained<NSURL>> = unsafe {
        let cls: &AnyClass = objc2::class!(MLModel);
        let pkg_url = NSURL::fileURLWithPath(&NSString::from_str(pkg_str));

        let raw: *mut NSURL = msg_send![cls,
            compileModelAtURL: &*pkg_url,
            error: std::ptr::null_mut::<*mut objc2_foundation::NSError>()
        ];
        if raw.is_null() {
            None
        } else {
            Some(Retained::retain(raw).unwrap())
        }
    };

    let url = compiled_url.ok_or_else(|| {
        FarscryError::OcrFailed(format!(
            "MLModel::compileModelAtURL failed for {:?}",
            pkg_path
        ))
    })?;

    let path_str = url
        .path()
        .ok_or_else(|| FarscryError::OcrFailed("Compiled URL has no path".into()))?
        .to_string();

    Ok(PathBuf::from(path_str))
}

fn cache_dir() -> Result<PathBuf, FarscryError> {
    dirs::home_dir()
        .map(|h| h.join(".farscry").join("models").join("coreml"))
        .ok_or_else(|| FarscryError::OcrFailed("Cannot determine home directory".into()))
}

fn is_stale(compiled: &Path, source: &Path) -> bool {
    if !source.exists() {
        return false;
    }
    let mtime = |p: &Path| -> Option<SystemTime> { std::fs::metadata(p).ok()?.modified().ok() };
    match (mtime(source), mtime(compiled)) {
        (Some(src), Some(cmp)) => src > cmp,
        _ => false,
    }
}
