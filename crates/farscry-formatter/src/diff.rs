use farscry_core::{DeltaEntry, VaspDelta};
use std::fmt::Write;

use crate::context::format_element_type;

pub(crate) fn format_diff(delta: &VaspDelta) -> String {
    let mut result = String::new();

    writeln!(result, "=== farscry diff ===").ok();
    writeln!(result, "state_id: {}", delta.diff_to).ok();
    writeln!(result, "delta_from: {}", delta.diff_from).ok();
    writeln!(
        result,
        "context_similarity: {:.3}",
        delta.context_similarity
    )
    .ok();
    writeln!(result, "context_changed: {}", delta.context_changed).ok();
    writeln!(result, "---").ok();

    let unchanged = delta
        .entries
        .iter()
        .filter(|e| matches!(e, DeltaEntry::Unchanged(_)))
        .count();

    for entry in &delta.entries {
        result.push_str(&render_delta_entry(entry));
    }

    if unchanged > 0 {
        writeln!(result, "unchanged: [{} elements]", unchanged).ok();
    }

    if let Some(saved) = delta.tokens_saved {
        writeln!(result).ok();
        writeln!(
            result,
            "Token savings: ~{} tokens saved vs re-sending both images",
            saved
        )
        .ok();
    }

    result
}

fn render_delta_entry(entry: &DeltaEntry) -> String {
    let mut s = String::new();
    match entry {
        DeltaEntry::Appeared(elem) => {
            writeln!(
                s,
                "appeared:  {:8}  \"{}\"",
                format_element_type(&elem.element_type),
                elem.text
            )
            .ok();
        }
        DeltaEntry::Changed { before, after } => {
            writeln!(
                s,
                "changed:   {:8}  \"{}\" -> \"{}\"",
                format_element_type(&before.element_type),
                before.text,
                after.text
            )
            .ok();
        }
        DeltaEntry::Removed(elem) => {
            writeln!(
                s,
                "removed:   {:8}  \"{}\"",
                format_element_type(&elem.element_type),
                elem.text
            )
            .ok();
        }
        DeltaEntry::Unchanged(_) => {}
    }
    s
}
