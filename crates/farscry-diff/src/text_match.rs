use farscry_core::UiElement;

pub(crate) fn rough_text_match(
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

pub(crate) fn text_similarity(a: &str, b: &str) -> f32 {
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
