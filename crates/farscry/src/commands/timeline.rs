use anyhow::{Context, Result};
use farscry_core::vasf::VasfFile;
use std::path::PathBuf;

pub fn timeline(input: PathBuf) -> Result<()> {
    let vasf =
        VasfFile::read_from(&input).with_context(|| format!("cannot read {}", input.display()))?;
    for (i, frame) in vasf.frames.iter().enumerate() {
        let text = std::str::from_utf8(&frame.vasp_data)
            .with_context(|| format!("frame {} vasp_data is not valid UTF-8", i + 1))?;
        let screen_type = vasp_field(text, "screen_type: ");
        let agent_ctx = vasp_field(text, "agent_context: ");
        println!(
            "frame {:03}  {}  {:12}  {}",
            i + 1,
            frame.state_id,
            screen_type,
            agent_ctx
        );
    }
    Ok(())
}

fn vasp_field<'a>(text: &'a str, prefix: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(prefix))
        .map(|v| v.trim().trim_matches('"'))
        .unwrap_or("unknown")
}
