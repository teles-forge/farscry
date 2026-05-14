use farscry_core::{DeltaEntry, UiElement};
use std::collections::HashSet;

const ELEMENT_IDENTITY_THRESHOLD: f32 = 0.95;
const VASP_BASELINE_TOKENS: usize = 200;

pub(crate) fn classify_delta_entries(
    before: &[UiElement],
    after: &[UiElement],
    matches: &[(usize, usize, f32)],
) -> Vec<DeltaEntry> {
    let mut entries = Vec::new();
    let mut matched_before: HashSet<usize> = HashSet::new();
    let mut matched_after: HashSet<usize> = HashSet::new();

    for (i, j, score) in matches {
        matched_before.insert(*i);
        matched_after.insert(*j);

        if *score > ELEMENT_IDENTITY_THRESHOLD {
            entries.push(DeltaEntry::Unchanged(before[*i].clone()));
        } else {
            entries.push(DeltaEntry::Changed {
                before: before[*i].clone(),
                after: after[*j].clone(),
            });
        }
    }

    for (i, elem) in before.iter().enumerate() {
        if !matched_before.contains(&i) {
            entries.push(DeltaEntry::Removed(elem.clone()));
        }
    }

    for (j, elem) in after.iter().enumerate() {
        if !matched_after.contains(&j) {
            entries.push(DeltaEntry::Appeared(elem.clone()));
        }
    }

    entries
}

pub(crate) fn compute_tokens_saved(
    before_dims: Option<(u32, u32)>,
    after_dims: Option<(u32, u32)>,
) -> Option<usize> {
    if before_dims.is_none() && after_dims.is_none() {
        return None;
    }
    let before_tokens = before_dims
        .map(|(w, h)| ((w as usize) * (h as usize)) / 750)
        .unwrap_or(0);
    let after_tokens = after_dims
        .map(|(w, h)| ((w as usize) * (h as usize)) / 750)
        .unwrap_or(0);
    Some((before_tokens + after_tokens).saturating_sub(VASP_BASELINE_TOKENS))
}
