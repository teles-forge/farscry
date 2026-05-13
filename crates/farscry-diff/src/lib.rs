use farscry_core::{DeltaEntry, DiffEngine, UiElement, VaspDelta, VaspOutput};
use std::collections::HashSet;

pub struct DiffEngineImpl;

impl DiffEngine for DiffEngineImpl {
    fn diff(
        &self,
        before: &VaspOutput,
        after: &VaspOutput,
        before_dims: Option<(u32, u32)>,
        after_dims: Option<(u32, u32)>,
    ) -> VaspDelta {
        let tokens_saved = compute_tokens_saved(before_dims, after_dims);

        let context_similarity = compute_context_similarity(before, after);
        let context_changed = context_similarity < 0.20;

        if context_changed {
            return VaspDelta {
                vasp_version: "1.0".to_string(),
                diff_from: before.state_id,
                diff_to: after.state_id,
                context_similarity,
                context_changed: true,
                agent_context: after.agent_context.clone(),
                entries: Vec::new(),
                tokens_saved,
            };
        }

        let rough_matches = rough_text_match(&before.ui_tree, &after.ui_tree, 0.70);

        let scroll_offset = compute_scroll_offset(&rough_matches, &before.ui_tree, &after.ui_tree);

        let matches = full_bipartite_match(&before.ui_tree, &after.ui_tree, &scroll_offset);

        let entries = classify_delta_entries(&before.ui_tree, &after.ui_tree, &matches);

        VaspDelta {
            vasp_version: "1.0".to_string(),
            diff_from: before.state_id,
            diff_to: after.state_id,
            context_similarity,
            context_changed: false,
            agent_context: after.agent_context.clone(),
            entries,
            tokens_saved,
        }
    }
}

fn compute_context_similarity(before: &VaspOutput, after: &VaspOutput) -> f32 {
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

fn rough_text_match(
    before: &[UiElement],
    after: &[UiElement],
    threshold: f32,
) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();

    for (i, before_elem) in before.iter().enumerate() {
        for (j, after_elem) in after.iter().enumerate() {
            let text_sim = text_similarity(&before_elem.text, &after_elem.text);
            if text_sim > threshold {
                matches.push((i, j));
                break;
            }
        }
    }

    matches
}

fn text_similarity(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    let len_a = a.chars().count();
    let len_b = b.chars().count();
    if len_a == 0 && len_b == 0 {
        return 1.0;
    }
    let max_len = len_a.max(len_b);
    if max_len == 0 {
        return 1.0;
    }
    let dist = levenshtein(a, b);
    1.0 - (dist as f32 / max_len as f32)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

fn compute_scroll_offset(
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

    dy_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    dx_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let dy_median = dy_values[dy_values.len() / 2];
    let dx_median = dx_values[dx_values.len() / 2];

    (dx_median, dy_median)
}

fn full_bipartite_match(
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
            if score > 0.60 {
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

fn position_proximity(before: &UiElement, after: &UiElement, scroll_offset: &(f32, f32)) -> f32 {
    let dx = after.cx - before.cx - scroll_offset.0;
    let dy = after.cy - before.cy - scroll_offset.1;
    let dist_sq = dx * dx + dy * dy;
    const SIGMA: f32 = 80.0;
    (-dist_sq / (2.0 * SIGMA * SIGMA)).exp()
}

fn classify_delta_entries(
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

        if *score > 0.95 {
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

fn compute_tokens_saved(
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
    const VASP_TOKENS: usize = 200;
    Some((before_tokens + after_tokens).saturating_sub(VASP_TOKENS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use farscry_core::{Confidence, ElementType, ScreenType, StateId};

    fn create_test_element(
        text: &str,
        cx: f32,
        cy: f32,
        element_type: ElementType,
        value: Option<String>,
    ) -> UiElement {
        UiElement {
            text: text.to_string(),
            element_type,
            cx,
            cy,
            w: 100.0,
            h: 30.0,
            enabled: None,
            value,
        }
    }

    fn create_test_vasp(elements: Vec<UiElement>) -> VaspOutput {
        VaspOutput::new(
            StateId::from_bits(0x123456789ABCDEF0),
            ScreenType::Ui,
            Confidence::High,
            "eng",
            "test context",
            elements,
            vec![],
        )
    }

    #[test]
    fn test_context_similarity_high() {
        let before = create_test_vasp(vec![
            create_test_element("Username", 50.0, 100.0, ElementType::Label, None),
            create_test_element("Password", 50.0, 150.0, ElementType::Label, None),
        ]);

        let after = create_test_vasp(vec![
            create_test_element("Username", 50.0, 100.0, ElementType::Label, None),
            create_test_element("Password", 50.0, 150.0, ElementType::Label, None),
        ]);

        let sim = compute_context_similarity(&before, &after);
        assert!(sim > 0.8);
    }

    #[test]
    fn test_context_similarity_low() {
        let before = create_test_vasp(vec![create_test_element(
            "Username",
            50.0,
            100.0,
            ElementType::Label,
            None,
        )]);

        let after = create_test_vasp(vec![create_test_element(
            "Completely different text",
            50.0,
            100.0,
            ElementType::Label,
            None,
        )]);

        let sim = compute_context_similarity(&before, &after);
        assert!(sim < 0.5);
    }

    #[test]
    fn test_text_similarity() {
        assert!(text_similarity("Username", "Username") > 0.99);
        assert!(text_similarity("Username", "Password") < 0.5);

        assert!(text_similarity("Hello World", "Hello") > 0.3);
        assert!(text_similarity("Hello World", "Hello") < 0.6);
    }

    #[test]
    fn test_position_proximity() {
        let elem1 = create_test_element("Test", 100.0, 100.0, ElementType::Label, None);
        let elem2 = create_test_element("Test", 100.0, 100.0, ElementType::Label, None);
        let offset = (0.0, 0.0);

        let sim = position_proximity(&elem1, &elem2, &offset);
        assert!(sim > 0.9);
    }

    #[test]
    fn test_position_proximity_scrolled() {
        let elem1 = create_test_element("Test", 100.0, 100.0, ElementType::Label, None);
        let elem2 = create_test_element("Test", 100.0, 340.0, ElementType::Label, None);
        let offset = (0.0, 240.0);

        let sim = position_proximity(&elem1, &elem2, &offset);
        assert!(sim > 0.9);
    }

    #[test]
    fn test_scroll_detection() {
        let before = create_test_vasp(vec![
            create_test_element("Dashboard", 50.0, 100.0, ElementType::Heading, None),
            create_test_element("Users", 50.0, 150.0, ElementType::Label, None),
            create_test_element("Settings", 200.0, 150.0, ElementType::Label, None),
            create_test_element("Logout", 350.0, 150.0, ElementType::Button, None),
        ]);

        let after = create_test_vasp(vec![
            create_test_element("Users", 50.0, 390.0, ElementType::Label, None),
            create_test_element("Settings", 200.0, 390.0, ElementType::Label, None),
            create_test_element("Logout", 350.0, 390.0, ElementType::Button, None),
            create_test_element("Reports", 50.0, 440.0, ElementType::Label, None),
        ]);

        let diff_engine = DiffEngineImpl;
        let delta = diff_engine.diff(&before, &after, None, None);

        assert!(!delta.context_changed);

        let unchanged_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Unchanged(_)))
            .count();
        let removed_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Removed(_)))
            .count();
        let appeared_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Appeared(_)))
            .count();
        let changed_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Changed { .. }))
            .count();

        assert!(unchanged_count > 0);

        assert!(removed_count > 0);

        assert!(appeared_count > 0);

        assert_eq!(
            changed_count, 0,
            "Scroll should not produce false Changed entries"
        );
    }

    #[test]
    fn test_field_filled() {
        let before = create_test_vasp(vec![
            create_test_element("Email", 50.0, 100.0, ElementType::Label, None),
            create_test_element("Enter email", 150.0, 100.0, ElementType::Input, None),
            create_test_element("Password", 50.0, 150.0, ElementType::Label, None),
            create_test_element("•••••••", 150.0, 150.0, ElementType::Input, None),
            create_test_element("Submit", 50.0, 200.0, ElementType::Button, None),
        ]);

        let after = create_test_vasp(vec![
            create_test_element("Email", 50.0, 100.0, ElementType::Label, None),
            create_test_element("Email entered", 150.0, 100.0, ElementType::Input, None),
            create_test_element("Password", 50.0, 150.0, ElementType::Label, None),
            create_test_element("•••••••", 150.0, 150.0, ElementType::Input, None),
            create_test_element("Submit", 50.0, 200.0, ElementType::Button, None),
        ]);

        let diff_engine = DiffEngineImpl;
        let delta = diff_engine.diff(&before, &after, None, None);

        assert!(!delta.context_changed);

        let changed_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Changed { .. }))
            .count();
        let unchanged_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Unchanged(_)))
            .count();

        assert!(
            changed_count >= 1,
            "At least 1 Changed entry expected for the filled input"
        );

        assert!(
            unchanged_count >= 3,
            "At least 3 Unchanged entries expected"
        );
    }

    #[test]
    fn test_error_appeared() {
        let before = create_test_vasp(vec![
            create_test_element("Card Number", 50.0, 100.0, ElementType::Label, None),
            create_test_element("", 150.0, 100.0, ElementType::Input, None),
            create_test_element("Expiry", 50.0, 150.0, ElementType::Label, None),
            create_test_element("", 150.0, 150.0, ElementType::Input, None),
            create_test_element("Pay", 50.0, 200.0, ElementType::Button, None),
        ]);

        let after = create_test_vasp(vec![
            create_test_element("Card Number", 50.0, 100.0, ElementType::Label, None),
            create_test_element("", 150.0, 100.0, ElementType::Input, None),
            create_test_element("Expiry", 50.0, 150.0, ElementType::Label, None),
            create_test_element("", 150.0, 150.0, ElementType::Input, None),
            create_test_element("Pay", 50.0, 200.0, ElementType::Button, None),
            create_test_element(
                "Error: Invalid card number",
                50.0,
                250.0,
                ElementType::Error,
                None,
            ),
        ]);

        let diff_engine = DiffEngineImpl;
        let delta = diff_engine.diff(&before, &after, None, None);

        assert!(!delta.context_changed);

        let appeared_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Appeared(_)))
            .count();
        let unchanged_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Unchanged(_)))
            .count();
        let changed_count = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Changed { .. }))
            .count();

        assert_eq!(appeared_count, 1);

        assert_eq!(unchanged_count, 5);

        assert_eq!(changed_count, 0);
    }

    #[test]
    fn test_context_gate() {
        let before = create_test_vasp(vec![
            create_test_element("Card Number", 50.0, 100.0, ElementType::Label, None),
            create_test_element("Pay", 50.0, 200.0, ElementType::Button, None),
        ]);

        let after = create_test_vasp(vec![
            create_test_element(
                "Welcome to Dashboard",
                50.0,
                100.0,
                ElementType::Heading,
                None,
            ),
            create_test_element(
                "Your balance is $1000",
                50.0,
                150.0,
                ElementType::Label,
                None,
            ),
            create_test_element("View Transactions", 50.0, 200.0, ElementType::Button, None),
        ]);

        let diff_engine = DiffEngineImpl;
        let delta = diff_engine.diff(&before, &after, None, None);

        assert!(delta.context_changed);
        assert!(delta.context_similarity < 0.20);

        assert_eq!(delta.entries.len(), 0);
    }

    #[test]
    fn test_token_savings() {
        assert!(compute_tokens_saved(None, None).is_none());

        let saved = compute_tokens_saved(Some((1920, 1080)), Some((1920, 1080)));
        assert!(saved.is_some());
        let saved = saved.unwrap();
        assert!(
            saved > 5000,
            "1080p pair should save >5000 tokens, got {saved}"
        );

        let small = compute_tokens_saved(Some((640, 480)), None);
        assert!(small.is_some(), "one dim present should still yield Some");

        let tiny = compute_tokens_saved(Some((100, 100)), None);
        assert_eq!(tiny, Some(0), "small image savings saturate at 0");
    }

    #[test]
    fn test_token_savings_in_delta() {
        let diff_engine = DiffEngineImpl;

        let before = create_test_vasp(vec![create_test_element(
            "Save",
            50.0,
            100.0,
            ElementType::Button,
            None,
        )]);
        let after = create_test_vasp(vec![create_test_element(
            "Save",
            50.0,
            100.0,
            ElementType::Button,
            None,
        )]);

        let delta_no_dims = diff_engine.diff(&before, &after, None, None);
        assert!(delta_no_dims.tokens_saved.is_none());

        let delta_with_dims =
            diff_engine.diff(&before, &after, Some((1920, 1080)), Some((1920, 1080)));
        assert!(delta_with_dims.tokens_saved.is_some());
        assert!(delta_with_dims.tokens_saved.unwrap() > 5000);
    }
}
