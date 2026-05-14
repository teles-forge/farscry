use farscry_core::{UiElement, VaspOutput};
use std::collections::HashSet;

pub(crate) fn compute_context_similarity(before: &VaspOutput, after: &VaspOutput) -> f32 {
    let before_tokens = extract_tokens(&before.ui_tree);
    let after_tokens = extract_tokens(&after.ui_tree);

    if before_tokens.is_empty() && after_tokens.is_empty() {
        return 1.0;
    }

    let shared: HashSet<_> = before_tokens.intersection(&after_tokens).cloned().collect();
    let max_size = before_tokens.len().max(after_tokens.len()).max(1);

    shared.len() as f32 / max_size as f32
}

fn extract_tokens(elements: &[UiElement]) -> HashSet<String> {
    elements
        .iter()
        .flat_map(|e| e.text.split_whitespace())
        .map(|t| t.to_lowercase())
        .collect()
}
