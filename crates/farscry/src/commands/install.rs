use anyhow::Result;

pub fn install_lang(langs: Vec<String>) -> Result<()> {
    let models_dir = crate::pipeline::resolve_models_dir();
    if let Some(lang) = langs.first() {
        eprintln!("[farscry] Installing language model: {lang}");
        eprintln!(
            "[farscry] Place model files manually at: {}",
            models_dir.display()
        );
        return Err(anyhow::anyhow!(
            "language not installed: {lang}. Multi-language support arrives in v0.2."
        ));
    }
    Ok(())
}
