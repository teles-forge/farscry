use crate::text_match::text_similarity;
use farscry_core::UiElement;
use std::collections::HashSet;

const MATCH_SCORE_THRESHOLD: f32 = 0.60;
const POSITION_SIGMA_PX: f32 = 80.0;

pub(crate) fn compute_scroll_offset(
    matches: &[(usize, usize)],
    before: &[UiElement],
    after: &[UiElement],
) -> (f32, f32) {
    if matches.is_empty() {
        return (0.0, 0.0);
    }

    let mut dy_values: Vec<f32> = Vec::new();
    let mut dx_values: Vec<f32> = Vec::new();

    for (before_idx, after_idx) in matches {
        let before_elem = &before[*before_idx];
        let after_elem = &after[*after_idx];
        dy_values.push(after_elem.cy - before_elem.cy);
        dx_values.push(after_elem.cx - before_elem.cx);
    }

    dy_values.sort_by(|a, b| a.total_cmp(b));
    dx_values.sort_by(|a, b| a.total_cmp(b));

    let dy_median = dy_values[dy_values.len() / 2];
    let dx_median = dx_values[dx_values.len() / 2];

    (dx_median, dy_median)
}

pub(crate) fn full_bipartite_match(
    before: &[UiElement],
    after: &[UiElement],
    scroll_offset: &(f32, f32),
) -> Vec<(usize, usize, f32)> {
    let mut matches = Vec::new();
    let mut used_after: HashSet<usize> = HashSet::new();

    for (i, before_elem) in before.iter().enumerate() {
        let mut best_match: Option<(usize, f32)> = None;

        for (j, after_elem) in after.iter().enumerate() {
            if used_after.contains(&j) {
                continue;
            }

            let score = compute_match_score(before_elem, after_elem, scroll_offset);
            if score > MATCH_SCORE_THRESHOLD {
                match best_match {
                    Some((_, best_score)) if score > best_score => {
                        best_match = Some((j, score));
                    }
                    None => {
                        best_match = Some((j, score));
                    }
                    _ => {}
                }
            }
        }

        if let Some((j, score)) = best_match {
            matches.push((i, j, score));
            used_after.insert(j);
        }
    }

    matches
}

fn compute_match_score(before: &UiElement, after: &UiElement, scroll_offset: &(f32, f32)) -> f32 {
    let text_sim = text_similarity(&before.text, &after.text);
    let position_sim = position_proximity(before, after, scroll_offset);

    let type_match = if before.element_type == after.element_type {
        1.0
    } else {
        0.5
    };

    0.4 * text_sim + 0.4 * position_sim + 0.2 * type_match
}

pub(crate) fn position_proximity(
    before: &UiElement,
    after: &UiElement,
    scroll_offset: &(f32, f32),
) -> f32 {
    let dx = after.cx - before.cx - scroll_offset.0;
    let dy = after.cy - before.cy - scroll_offset.1;
    let dist_sq = dx * dx + dy * dy;
    (-dist_sq / (2.0 * POSITION_SIGMA_PX * POSITION_SIGMA_PX)).exp()
}
