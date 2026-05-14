use anyhow::{Context, Result};
use farscry_core::vasf::{VasfFile, VasfFrame};
use farscry_core::{
    Confidence, DeltaEntry, DiffEngine, ElementType, ScreenType, StateId, UiElement, VaspDelta,
    VaspOutput,
};
use std::path::{Path, PathBuf};

pub fn diff_images(before: PathBuf, after: PathBuf, json: bool) -> Result<()> {
    let before_dims = image::open(&before).ok().map(|i| (i.width(), i.height()));
    let after_dims = image::open(&after).ok().map(|i| (i.width(), i.height()));

    let before_output = crate::pipeline::process_image(&before, 10_000_000)?;
    let after_output = crate::pipeline::process_image(&after, 10_000_000)?;

    let engine = farscry_diff::DiffEngineImpl;
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

pub fn diff_sessions(a: PathBuf, b: PathBuf) -> Result<()> {
    let vasf_a =
        VasfFile::read_from(&a).with_context(|| format!("cannot read {}", a.display()))?;
    let vasf_b =
        VasfFile::read_from(&b).with_context(|| format!("cannot read {}", b.display()))?;

    let a_name = file_stem(&a);
    let b_name = file_stem(&b);

    let diff = compute_session_diff(&vasf_a.frames, &vasf_b.frames);

    println!(
        "Comparing {} states ({}) vs {} states ({})",
        diff.count_a, a_name, diff.count_b, b_name
    );

    print_diff_body(&diff, &vasf_a.frames, &vasf_b.frames, &a_name, &b_name);
    Ok(())
}

pub(crate) struct SessionDiff {
    pub count_a: usize,
    pub count_b: usize,
    pub changed: Vec<(usize, usize, VaspDelta)>,
    pub only_a: Vec<usize>,
    pub only_b: Vec<usize>,
}

pub(crate) fn compute_session_diff(a: &[VasfFrame], b: &[VasfFrame]) -> SessionDiff {
    let pairs = match_states(a, b);
    let matched_a: std::collections::HashSet<usize> = pairs.iter().map(|&(i, _)| i).collect();
    let matched_b: std::collections::HashSet<usize> = pairs.iter().map(|&(_, j)| j).collect();

    let only_a = (0..a.len()).filter(|i| !matched_a.contains(i)).collect();
    let only_b = (0..b.len()).filter(|j| !matched_b.contains(j)).collect();
    let changed = compute_changed_pairs(a, b, &pairs);

    SessionDiff {
        count_a: a.len(),
        count_b: b.len(),
        changed,
        only_a,
        only_b,
    }
}

const HAMMING_THRESHOLD: u8 = 10;

fn match_states(a: &[VasfFrame], b: &[VasfFrame]) -> Vec<(usize, usize)> {
    let mut candidates: Vec<(u8, usize, usize)> = Vec::new();
    for (i, fa) in a.iter().enumerate() {
        for (j, fb) in b.iter().enumerate() {
            let d = hamming_dist(fa.state_id, fb.state_id);
            if d <= HAMMING_THRESHOLD {
                candidates.push((d, i, j));
            }
        }
    }
    candidates.sort_unstable_by_key(|&(d, _, _)| d);

    let mut used_a = vec![false; a.len()];
    let mut used_b = vec![false; b.len()];
    let mut pairs = Vec::new();
    for (_, i, j) in candidates {
        if !used_a[i] && !used_b[j] {
            used_a[i] = true;
            used_b[j] = true;
            pairs.push((i, j));
        }
    }
    pairs
}

fn compute_changed_pairs(
    a: &[VasfFrame],
    b: &[VasfFrame],
    pairs: &[(usize, usize)],
) -> Vec<(usize, usize, VaspDelta)> {
    let engine = farscry_diff::DiffEngineImpl;
    pairs
        .iter()
        .filter(|&&(i, j)| a[i].vasp_data != b[j].vasp_data)
        .filter_map(|&(i, j)| {
            let va = parse_vasp_output(&a[i]);
            let vb = parse_vasp_output(&b[j]);
            let delta = engine.diff(&va, &vb, None, None);
            let has_signal = delta.context_changed
                || delta
                    .entries
                    .iter()
                    .any(|e| !matches!(e, DeltaEntry::Unchanged(_)));
            if has_signal { Some((i, j, delta)) } else { None }
        })
        .collect()
}

fn print_diff_body(
    diff: &SessionDiff,
    a: &[VasfFrame],
    b: &[VasfFrame],
    a_name: &str,
    b_name: &str,
) {
    for (i, j, delta) in &diff.changed {
        println!("\nState {} changed (matches state {} in {}):", i + 1, j + 1, b_name);
        if delta.context_changed {
            println!("  (context changed entirely — different screen)");
        }
        for entry in &delta.entries {
            if let Some(line) = format_entry(entry) {
                println!("  {}", line);
            }
        }
    }
    for &i in &diff.only_a {
        println!("\nState {}: present in {}, missing in {}", i + 1, a_name, b_name);
        println!("  {}", frame_summary(&a[i]));
    }
    for &j in &diff.only_b {
        println!("\nState {}: present in {} only", j + 1, b_name);
        println!("  {}", frame_summary(&b[j]));
    }
}

fn format_entry(entry: &DeltaEntry) -> Option<String> {
    match entry {
        DeltaEntry::Appeared(e) => Some(format!(
            "new:     {} \"{}\" at ({:.0}, {:.0})",
            element_label(e.element_type),
            e.text,
            e.cx,
            e.cy
        )),
        DeltaEntry::Removed(e) => Some(format!(
            "gone:    {} \"{}\"",
            element_label(e.element_type),
            e.text
        )),
        DeltaEntry::Changed { before, after } => Some(format!(
            "changed: {} \"{}\" -> \"{}\"",
            element_label(before.element_type),
            before.text,
            after.text
        )),
        DeltaEntry::Unchanged(_) => None,
    }
}

fn frame_summary(frame: &VasfFrame) -> String {
    let text = std::str::from_utf8(&frame.vasp_data).unwrap_or("");
    let st = vasp_field(text, "screen_type: ");
    let ctx = vasp_field(text, "agent_context: ").trim_matches('"');
    format!("{} | \"{}\"", capitalize(st), ctx)
}

fn element_label(et: ElementType) -> &'static str {
    match et {
        ElementType::Button => "Button",
        ElementType::Input => "Input",
        ElementType::Label => "Label",
        ElementType::Heading => "Heading",
        ElementType::Error => "Error",
        ElementType::Select => "Select",
        ElementType::Badge => "Badge",
        ElementType::Unknown => "Unknown",
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

fn hamming_dist(a: StateId, b: StateId) -> u8 {
    (a.to_bits() ^ b.to_bits()).count_ones() as u8
}

fn file_stem(p: &Path) -> String {
    p.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session")
        .to_string()
}

fn parse_vasp_output(frame: &VasfFrame) -> VaspOutput {
    let text = std::str::from_utf8(&frame.vasp_data).unwrap_or("");
    let screen_type = parse_screen_type(vasp_field(text, "screen_type: "));
    let confidence = parse_confidence(vasp_field(text, "confidence: "));
    let lang = vasp_field(text, "lang: ").to_string();
    let agent_context = vasp_field(text, "agent_context: ")
        .trim_matches('"')
        .to_string();
    let ui_tree = parse_ui_elements(text);
    VaspOutput::new(
        frame.state_id,
        screen_type,
        confidence,
        lang,
        agent_context,
        ui_tree,
        vec![],
    )
}

fn parse_ui_elements(text: &str) -> Vec<UiElement> {
    let mut elements: Vec<UiElement> = Vec::new();
    let mut current: Option<UiElement> = None;
    for line in text.lines() {
        if line.starts_with('[') {
            if let Some(e) = current.take() {
                elements.push(e);
            }
            current = parse_element_line(line);
        } else if let Some(ref mut e) = current {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("enabled:") {
                e.enabled = rest.trim().parse::<bool>().ok();
            } else if let Some(rest) = trimmed.strip_prefix("value=\"") {
                e.value = Some(rest.trim_end_matches('"').to_string());
            }
        }
    }
    if let Some(e) = current {
        elements.push(e);
    }
    elements
}

fn parse_element_line(line: &str) -> Option<UiElement> {
    let close = line.find(']')?;
    let zone = line[1..close].trim();
    let rest = line.get(close + 1..)?.trim_start_matches(' ');
    let type_end = rest.find("  ")?;
    let element_type = parse_element_type(rest[..type_end].trim());
    let after_type = rest.get(type_end..)?.trim_start_matches(' ').trim_start_matches('"');
    let text = after_type.trim_end_matches('"').to_string();
    let (cx, cy) = zone_to_coords(zone);
    Some(UiElement {
        text,
        element_type,
        cx,
        cy,
        w: 100.0,
        h: 30.0,
        enabled: None,
        value: None,
    })
}

fn vasp_field<'a>(text: &'a str, prefix: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(prefix))
        .map(str::trim)
        .unwrap_or("unknown")
}

fn zone_to_coords(zone: &str) -> (f32, f32) {
    match zone {
        "top-left" => (320.0, 180.0),
        "top-center" => (960.0, 180.0),
        "top-right" => (1600.0, 180.0),
        "middle-left" => (320.0, 540.0),
        "middle-center" => (960.0, 540.0),
        "middle-right" => (1600.0, 540.0),
        "bottom-left" => (320.0, 900.0),
        "bottom-center" => (960.0, 900.0),
        "bottom-right" => (1600.0, 900.0),
        _ => (0.0, 0.0),
    }
}

fn parse_element_type(s: &str) -> ElementType {
    match s {
        "button" => ElementType::Button,
        "input" => ElementType::Input,
        "select" => ElementType::Select,
        "label" => ElementType::Label,
        "heading" => ElementType::Heading,
        "error" => ElementType::Error,
        "badge" => ElementType::Badge,
        _ => ElementType::Unknown,
    }
}

fn parse_screen_type(s: &str) -> ScreenType {
    match s {
        "terminal" => ScreenType::Terminal,
        "config" => ScreenType::Config,
        "error" => ScreenType::Error,
        "conversation" => ScreenType::Conversation,
        "ui" => ScreenType::Ui,
        _ => ScreenType::Unknown,
    }
}

fn parse_confidence(s: &str) -> Confidence {
    match s {
        "high" => Confidence::High,
        "medium" => Confidence::Medium,
        "low" => Confidence::Low,
        _ => Confidence::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use farscry_core::vasf::VasfFrame;
    use farscry_core::StateId;

    fn make_frame(id: u64, screen_type: &str, agent_ctx: &str, extra_elem: &str) -> VasfFrame {
        let vasp = format!(
            "=== farscry visual context ===\nsource: screen\nscreen_type: {}\nstate_id: phash:{:016x}\nconfidence: high\nlang: eng\nagent_context: \"{}\"\n---\n[top-left    ]  button    \"Run\"\n{}",
            screen_type, id, agent_ctx, extra_elem
        );
        VasfFrame {
            state_id: StateId::from_bits(id),
            timestamp: 0,
            vasp_data: vasp.into_bytes(),
            delta_data: None,
        }
    }

    #[test]
    fn test_match_states_identical() {
        let a = vec![make_frame(1, "terminal", "ctx", "")];
        let b = vec![make_frame(1, "terminal", "ctx", "")];
        let pairs = match_states(&a, &b);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], (0, 0));
    }

    #[test]
    fn test_match_states_no_match() {
        let a = vec![make_frame(0x0000_0000_0000_0000, "terminal", "ctx", "")];
        let b = vec![make_frame(0xFFFF_FFFF_FFFF_FFFF, "error", "ctx", "")];
        let pairs = match_states(&a, &b);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_compute_session_diff_only_in_a() {
        let a = vec![
            make_frame(1, "terminal", "build ok", ""),
            make_frame(0xFFFF_0000_0000_0000, "config", "settings", ""),
        ];
        let b = vec![make_frame(1, "terminal", "build ok", "")];
        let diff = compute_session_diff(&a, &b);
        assert_eq!(diff.count_a, 2);
        assert_eq!(diff.count_b, 1);
        assert_eq!(diff.only_a.len(), 1);
        assert_eq!(diff.only_a[0], 1);
        assert!(diff.only_b.is_empty());
    }

    #[test]
    fn test_compute_session_diff_only_in_b() {
        let a = vec![make_frame(1, "terminal", "build ok", "")];
        let b = vec![
            make_frame(1, "terminal", "build ok", ""),
            make_frame(0xFFFF_0000_0000_0001, "error", "tx failed", ""),
        ];
        let diff = compute_session_diff(&a, &b);
        assert!(diff.only_a.is_empty());
        assert_eq!(diff.only_b.len(), 1);
        assert_eq!(diff.only_b[0], 1);
    }

    #[test]
    fn test_compute_session_diff_changed() {
        let a = vec![make_frame(1, "terminal", "ctx", "")];
        let b = vec![make_frame(
            1,
            "terminal",
            "ctx",
            "[top-left    ]  label     \"Build failed\"\n",
        )];
        let diff = compute_session_diff(&a, &b);
        assert_eq!(diff.changed.len(), 1);
        assert!(diff.only_a.is_empty());
        assert!(diff.only_b.is_empty());
    }

    #[test]
    fn test_parse_vasp_output_fields() {
        let frame = make_frame(0xABCD_EF01_2345_6789, "config", "my context", "");
        let vasp = parse_vasp_output(&frame);
        assert!(matches!(vasp.screen_type, ScreenType::Config));
        assert_eq!(vasp.agent_context, "my context");
        assert_eq!(vasp.state_id, frame.state_id);
    }

    #[test]
    fn test_parse_element_line_button() {
        let line = "[top-left    ]  button    \"Save\"";
        let elem = parse_element_line(line).expect("should parse");
        assert_eq!(elem.text, "Save");
        assert!(matches!(elem.element_type, ElementType::Button));
        assert!((elem.cx - 320.0).abs() < 1.0);
        assert!((elem.cy - 180.0).abs() < 1.0);
    }

    #[test]
    fn test_parse_ui_elements_with_enabled() {
        let text = "---\n[top-left    ]  button    \"Pay\"\n               enabled:false\n";
        let elements = parse_ui_elements(text);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].enabled, Some(false));
    }

    #[test]
    fn test_hamming_dist_zero() {
        let id = StateId::from_bits(0x1234_5678_9ABC_DEF0);
        assert_eq!(hamming_dist(id, id), 0);
    }

    #[test]
    fn test_hamming_dist_max() {
        let a = StateId::from_bits(0x0000_0000_0000_0000);
        let b = StateId::from_bits(0xFFFF_FFFF_FFFF_FFFF);
        assert_eq!(hamming_dist(a, b), 64);
    }
}
