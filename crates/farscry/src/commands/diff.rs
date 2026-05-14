use anyhow::Result;
use std::path::PathBuf;

pub fn diff_images(before: PathBuf, after: PathBuf, json: bool) -> Result<()> {
    let before_dims = image::open(&before).ok().map(|i| (i.width(), i.height()));
    let after_dims = image::open(&after).ok().map(|i| (i.width(), i.height()));

    let before_output = crate::pipeline::process_image(&before, 10_000_000)?;
    let after_output = crate::pipeline::process_image(&after, 10_000_000)?;

    let engine = farscry_diff::DiffEngineImpl;
    use farscry_core::DiffEngine;
    let delta = engine.diff(&before_output, &after_output, before_dims, after_dims);

    if json {
        let json_output = serde_json::to_string_pretty(&delta)?;
        println!("{}", json_output);
    } else {
        let delta_text = farscry_formatter::VaspFormatter::format_diff(&delta);
        print!("{}", delta_text);
    }

    Ok(())
}
