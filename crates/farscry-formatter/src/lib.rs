pub mod annotate;

use farscry_core::{
    Confidence, DeltaEntry, ElementType, ScreenType, UiElement, VaspDelta, VaspOutput,
};
use std::fmt::Write;

pub struct VaspFormatter;

impl VaspFormatter {
    pub fn format_vasp(
        output: &VaspOutput,
        source: &str,
        image_width: u32,
        image_height: u32,
    ) -> String {
        Self::format_vasp_with_options(output, source, image_width, image_height, true)
    }

    pub fn format_vasp_with_options(
        output: &VaspOutput,
        source: &str,
        image_width: u32,
        image_height: u32,
        show_affordances: bool,
    ) -> String {
        let mut result = String::new();

        writeln!(result, "=== farscry visual context ===").unwrap();
        writeln!(result, "source: {}", source).unwrap();
        writeln!(
            result,
            "screen_type: {}",
            format_screen_type(&output.screen_type)
        )
        .unwrap();
        writeln!(result, "state_id: {}", output.state_id).unwrap();
        writeln!(
            result,
            "confidence: {}",
            format_confidence(&output.confidence)
        )
        .unwrap();
        writeln!(result, "lang: {}", output.lang).unwrap();
        writeln!(
            result,
            "agent_context: \"{}\"",
            generate_agent_context(output)
        )
        .unwrap();
        writeln!(result, "---").unwrap();

        let mut sorted_elements = output.ui_tree.clone();
        sorted_elements.sort_by(|a, b| {
            a.cy.partial_cmp(&b.cy)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.cx.partial_cmp(&b.cx).unwrap_or(std::cmp::Ordering::Equal))
        });

        for element in &sorted_elements {
            let position_label = compute_position_label(element, image_width, image_height);
            writeln!(
                result,
                "[{:12}]  {:8}  \"{}\"",
                position_label,
                format_element_type(&element.element_type),
                element.text
            )
            .unwrap();

            if let Some(value) = &element.value {
                writeln!(result, "               value=\"{}\"", value).unwrap();
            }
            if let Some(enabled) = element.enabled {
                writeln!(result, "               enabled:{}", enabled).unwrap();
            }
        }

        if show_affordances {
            let affordances = extract_affordances(output);
            if !affordances.is_empty() {
                writeln!(result).unwrap();
                writeln!(result, "affordances:").unwrap();
                for affordance in affordances {
                    writeln!(result, "  {}", affordance).unwrap();
                }
            }
        }

        let token_savings = compute_token_savings(image_width, image_height, &result);
        writeln!(result).unwrap();
        writeln!(
            result,
            "Token savings: ~{} tokens saved vs sending raw image to cloud vision systems",
            token_savings
        )
        .unwrap();

        result
    }

    pub fn format_text_only(output: &VaspOutput) -> String {
        output
            .ui_tree
            .iter()
            .filter(|e| !e.text.is_empty())
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn format_json(output: &VaspOutput, pretty: bool) -> String {
        if pretty {
            serde_json::to_string_pretty(output).unwrap()
        } else {
            serde_json::to_string(output).unwrap()
        }
    }

    pub fn format_diff(delta: &VaspDelta) -> String {
        let mut result = String::new();

        writeln!(result, "=== farscry diff ===").unwrap();
        writeln!(result, "state_id: {}", delta.diff_to).unwrap();
        writeln!(result, "delta_from: {}", delta.diff_from).unwrap();
        writeln!(
            result,
            "context_similarity: {:.3}",
            delta.context_similarity
        )
        .unwrap();
        writeln!(result, "context_changed: {}", delta.context_changed).unwrap();
        writeln!(result, "---").unwrap();

        let _appeared = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Appeared(_)))
            .count();
        let _changed = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Changed { .. }))
            .count();
        let _removed = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Removed(_)))
            .count();
        let unchanged = delta
            .entries
            .iter()
            .filter(|e| matches!(e, DeltaEntry::Unchanged(_)))
            .count();

        for entry in &delta.entries {
            match entry {
                DeltaEntry::Appeared(elem) => {
                    writeln!(
                        result,
                        "appeared:  {:8}  \"{}\"",
                        format_element_type(&elem.element_type),
                        elem.text
                    )
                    .unwrap();
                }
                DeltaEntry::Changed { before, after } => {
                    writeln!(
                        result,
                        "changed:   {:8}  \"{}\" -> \"{}\"",
                        format_element_type(&before.element_type),
                        before.text,
                        after.text
                    )
                    .unwrap();
                }
                DeltaEntry::Removed(elem) => {
                    writeln!(
                        result,
                        "removed:   {:8}  \"{}\"",
                        format_element_type(&elem.element_type),
                        elem.text
                    )
                    .unwrap();
                }
                DeltaEntry::Unchanged(_elem) => {}
            }
        }

        if unchanged > 0 {
            writeln!(result, "unchanged: [{} elements]", unchanged).unwrap();
        }

        if let Some(saved) = delta.tokens_saved {
            writeln!(result).unwrap();
            writeln!(
                result,
                "Token savings: ~{} tokens saved vs re-sending both images",
                saved
            )
            .unwrap();
        }

        result
    }

    pub fn format_batch(outputs: &[(String, VaspOutput, u32, u32)]) -> String {
        let mut result = String::new();

        for (i, (source, output, width, height)) in outputs.iter().enumerate() {
            if i > 0 {
                writeln!(result, "---").unwrap();
            }
            writeln!(result, "file: {}", source).unwrap();
            result.push_str(&Self::format_vasp(output, source, *width, *height));
        }

        result
    }
}

fn compute_position_label(element: &UiElement, width: u32, height: u32) -> String {
    let w_third = width as f32 / 3.0;
    let h_third = height as f32 / 3.0;

    let horizontal = if element.cx < w_third {
        "left"
    } else if element.cx < 2.0 * w_third {
        "center"
    } else {
        "right"
    };

    let vertical = if element.cy < h_third {
        "top"
    } else if element.cy < 2.0 * h_third {
        "middle"
    } else {
        "bottom"
    };

    format!("[{}-{}]", vertical, horizontal)
}

fn generate_agent_context(output: &VaspOutput) -> String {
    match &output.screen_type {
        ScreenType::Terminal => {
            let error = output
                .ui_tree
                .iter()
                .find(|e| e.element_type == ElementType::Error)
                .map(|e| e.text.clone());

            if let Some(err) = error {
                format!("Build failed - {}", err)
            } else {
                "Script completed successfully".to_string()
            }
        }
        ScreenType::Error => {
            let error = output
                .ui_tree
                .iter()
                .find(|e| e.element_type == ElementType::Error)
                .map(|e| e.text.clone());

            if let Some(err) = error {
                format!("Error - {}", err)
            } else {
                "Error detected".to_string()
            }
        }
        ScreenType::Config => {
            let editable_count = output
                .ui_tree
                .iter()
                .filter(|e| {
                    e.element_type == ElementType::Input || e.element_type == ElementType::Select
                })
                .count();

            let button = output
                .ui_tree
                .iter()
                .find(|e| e.element_type == ElementType::Button)
                .map(|e| e.text.clone());

            if let Some(btn) = button {
                format!(
                    "Config - {} editable fields, {} available",
                    editable_count, btn
                )
            } else {
                format!("Config - {} editable fields", editable_count)
            }
        }
        ScreenType::Conversation => {
            let last_message = output.ui_tree.last();
            if let Some(msg) = last_message {
                format!("Conversation - last: {}", msg.text)
            } else {
                "Conversation captured".to_string()
            }
        }
        ScreenType::Ui | ScreenType::Unknown => {
            let interactive_count = output
                .ui_tree
                .iter()
                .filter(|e| {
                    matches!(
                        e.element_type,
                        ElementType::Button | ElementType::Input | ElementType::Select
                    )
                })
                .count();

            format!(
                "Screen captured - {} elements, {} interactive",
                output.ui_tree.len(),
                interactive_count
            )
        }
    }
}

fn compute_token_savings(image_w: u32, image_h: u32, vasp_text: &str) -> u32 {
    let image_tokens = image_w.div_ceil(512) * image_h.div_ceil(512) * 170 + 85;
    let vasp_tokens = (vasp_text.len() / 4) as u32;
    image_tokens.saturating_sub(vasp_tokens)
}

fn format_screen_type(screen_type: &ScreenType) -> String {
    match screen_type {
        ScreenType::Terminal => "terminal".to_string(),
        ScreenType::Config => "config".to_string(),
        ScreenType::Conversation => "conversation".to_string(),
        ScreenType::Error => "error".to_string(),
        ScreenType::Ui => "ui".to_string(),
        ScreenType::Unknown => "unknown".to_string(),
    }
}

fn format_confidence(confidence: &Confidence) -> String {
    match confidence {
        Confidence::High => "high".to_string(),
        Confidence::Medium => "medium".to_string(),
        Confidence::Low => "low".to_string(),
        Confidence::None => "none".to_string(),
    }
}

fn format_element_type(element_type: &ElementType) -> String {
    match element_type {
        ElementType::Label => "label".to_string(),
        ElementType::Button => "button".to_string(),
        ElementType::Input => "input".to_string(),
        ElementType::Select => "select".to_string(),
        ElementType::Heading => "heading".to_string(),
        ElementType::Error => "error".to_string(),
        ElementType::Badge => "badge".to_string(),
        ElementType::Unknown => "unknown".to_string(),
    }
}

fn extract_affordances(output: &VaspOutput) -> Vec<String> {
    let mut affordances = Vec::new();

    for element in &output.ui_tree {
        match element.element_type {
            ElementType::Button => {
                let enabled = element.enabled.unwrap_or(true);
                affordances.push(format!(
                    "click -> \"{}\" at ({:.0},{:.0})  enabled:{}",
                    element.text, element.cx, element.cy, enabled
                ));
            }
            ElementType::Input => {
                let current = element
                    .value
                    .as_ref()
                    .map(|v| format!("current:\"{}\"", v))
                    .unwrap_or_default();
                affordances.push(format!(
                    "type  -> \"{}\" at ({:.0},{:.0})  {}",
                    element.text, element.cx, element.cy, current
                ));
            }
            ElementType::Select => {
                let current = element
                    .value
                    .as_ref()
                    .map(|v| format!("current:\"{}\"", v))
                    .unwrap_or_default();
                affordances.push(format!(
                    "select -> \"{}\" at ({:.0},{:.0})  {}",
                    element.text, element.cx, element.cy, current
                ));
            }
            _ => {}
        }
    }

    affordances
}

#[cfg(test)]
mod tests {
    use super::*;
    use farscry_core::StateId;

    fn create_test_element(text: &str, cx: f32, cy: f32, element_type: ElementType) -> UiElement {
        UiElement {
            text: text.to_string(),
            element_type,
            cx,
            cy,
            w: 100.0,
            h: 30.0,
            enabled: None,
            value: None,
        }
    }

    fn create_test_vasp(elements: Vec<UiElement>) -> VaspOutput {
        VaspOutput::new(
            StateId::from_bits(0x123456789ABCDEF0),
            ScreenType::Config,
            Confidence::High,
            "eng",
            "test context",
            elements,
            vec![],
        )
    }

    #[test]
    fn test_position_label_top_left() {
        let element = create_test_element("Test", 100.0, 100.0, ElementType::Label);
        let label = compute_position_label(&element, 1920, 1080);
        assert_eq!(label, "[top-left]");
    }

    #[test]
    fn test_position_label_middle_center() {
        let element = create_test_element("Test", 960.0, 540.0, ElementType::Label);
        let label = compute_position_label(&element, 1920, 1080);
        assert_eq!(label, "[middle-center]");
    }

    #[test]
    fn test_position_label_bottom_right() {
        let element = create_test_element("Test", 1800.0, 1000.0, ElementType::Label);
        let label = compute_position_label(&element, 1920, 1080);
        assert_eq!(label, "[bottom-right]");
    }

    #[test]
    fn test_agent_context_config() {
        let output = create_test_vasp(vec![
            create_test_element("Max Value:", 200.0, 120.0, ElementType::Label),
            create_test_element("1500", 400.0, 120.0, ElementType::Input),
            create_test_element("Save", 600.0, 200.0, ElementType::Button),
        ]);

        let context = generate_agent_context(&output);
        assert!(context.contains("Config"));
        assert!(context.contains("editable fields"));
    }

    #[test]
    fn test_agent_context_terminal() {
        let mut output = create_test_vasp(vec![create_test_element(
            "Build failed",
            100.0,
            100.0,
            ElementType::Error,
        )]);
        output.screen_type = ScreenType::Terminal;

        let context = generate_agent_context(&output);
        assert!(context.contains("Build failed"));
    }

    #[test]
    fn test_token_savings() {
        let vasp_text = "test output";
        let savings = compute_token_savings(1920, 1080, vasp_text);

        assert!(savings > 2000);
    }

    #[test]
    fn test_vasp_format_output() {
        let output = create_test_vasp(vec![
            create_test_element("Payment Settings", 960.0, 100.0, ElementType::Heading),
            create_test_element("Max Value:", 200.0, 300.0, ElementType::Label),
            create_test_element("1500", 400.0, 300.0, ElementType::Input),
            create_test_element("Save Changes", 600.0, 400.0, ElementType::Button),
        ]);

        let formatted = VaspFormatter::format_vasp(&output, "screenshot.png", 1920, 1080);

        assert!(formatted.contains("=== farscry visual context ==="));
        assert!(formatted.contains("source: screenshot.png"));
        assert!(formatted.contains("screen_type: config"));
        assert!(formatted.contains("state_id: phash:"));
        assert!(formatted.contains("confidence: high"));
        assert!(formatted.contains("lang: eng"));
        assert!(formatted.contains("agent_context:"));
        assert!(formatted.contains("---"));
        assert!(formatted.contains("Token savings:"));
    }

    #[test]
    fn test_json_format_output() {
        let output = create_test_vasp(vec![create_test_element(
            "Test",
            100.0,
            100.0,
            ElementType::Label,
        )]);

        let json = VaspFormatter::format_json(&output, true);

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed["vasp_version"], "1.0");
    }

    #[test]
    fn test_json_roundtrip() {
        let output = create_test_vasp(vec![create_test_element(
            "Test",
            100.0,
            100.0,
            ElementType::Label,
        )]);

        let json = VaspFormatter::format_json(&output, false);
        let parsed: VaspOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.state_id, output.state_id);
        assert_eq!(parsed.ui_tree.len(), output.ui_tree.len());
    }

    #[test]
    fn test_batch_format() {
        let output1 = create_test_vasp(vec![create_test_element(
            "Screen 1",
            100.0,
            100.0,
            ElementType::Label,
        )]);

        let output2 = create_test_vasp(vec![create_test_element(
            "Screen 2",
            100.0,
            100.0,
            ElementType::Label,
        )]);

        let batch = VaspFormatter::format_batch(&[
            ("img1.png".to_string(), output1, 1920, 1080),
            ("img2.png".to_string(), output2, 1920, 1080),
        ]);

        assert!(batch.contains("file: img1.png"));
        assert!(batch.contains("file: img2.png"));
        assert!(batch.contains("---"));
    }

    #[test]
    fn test_diff_format() {
        let delta = VaspDelta {
            vasp_version: "1.0".to_string(),
            diff_from: StateId::from_bits(0x123456789ABCDEF0),
            diff_to: StateId::from_bits(0xFEDCBA9876543210),
            context_similarity: 0.923,
            context_changed: false,
            agent_context: "Test context".to_string(),
            entries: vec![
                DeltaEntry::Appeared(create_test_element(
                    "New Element",
                    100.0,
                    100.0,
                    ElementType::Label,
                )),
                DeltaEntry::Changed {
                    before: create_test_element("Old", 200.0, 200.0, ElementType::Button),
                    after: create_test_element("New", 200.0, 200.0, ElementType::Button),
                },
                DeltaEntry::Removed(create_test_element(
                    "Removed",
                    300.0,
                    300.0,
                    ElementType::Label,
                )),
                DeltaEntry::Unchanged(create_test_element(
                    "Unchanged",
                    400.0,
                    400.0,
                    ElementType::Label,
                )),
            ],
            tokens_saved: Some(3028),
        };

        let formatted = VaspFormatter::format_diff(&delta);

        assert!(formatted.contains("=== farscry diff ==="));
        assert!(formatted.contains("state_id: phash:"));
        assert!(formatted.contains("delta_from: phash:"));
        assert!(formatted.contains("context_similarity: 0.923"));
        assert!(formatted.contains("appeared:"));
        assert!(formatted.contains("changed:"));
        assert!(formatted.contains("removed:"));
        assert!(formatted.contains("unchanged:"));
        assert!(formatted.contains("Token savings:"));
    }

    #[test]
    fn test_format_vasp_affordances_included_by_default() {
        let mut output = create_test_vasp(vec![create_test_element(
            "Save",
            600.0,
            200.0,
            ElementType::Button,
        )]);
        output.screen_type = ScreenType::Ui;

        let formatted = VaspFormatter::format_vasp(&output, "test.png", 1920, 1080);

        assert!(
            formatted.contains("affordances:"),
            "affordances section must appear by default"
        );
        assert!(
            formatted.contains("click ->"),
            "click affordance must appear"
        );
    }

    #[test]
    fn test_format_vasp_affordances_omitted() {
        let mut output = create_test_vasp(vec![create_test_element(
            "Save",
            600.0,
            200.0,
            ElementType::Button,
        )]);
        output.screen_type = ScreenType::Ui;

        let formatted =
            VaspFormatter::format_vasp_with_options(&output, "test.png", 1920, 1080, false);

        assert!(
            !formatted.contains("affordances:"),
            "affordances must be absent when show_affordances=false"
        );

        assert!(formatted.contains("Save"), "element text must still appear");
    }

    #[test]
    fn test_format_text_only() {
        let output = create_test_vasp(vec![
            create_test_element("Error: something failed", 100.0, 100.0, ElementType::Error),
            create_test_element("Retry", 200.0, 200.0, ElementType::Button),
        ]);

        let text = VaspFormatter::format_text_only(&output);
        assert!(text.contains("Error: something failed"));
        assert!(text.contains("Retry"));

        assert!(!text.contains("=== farscry"));
        assert!(!text.contains("state_id:"));
        assert!(!text.contains("affordances:"));
    }

    #[test]
    fn test_format_text_only_filters_empty() {
        let output = create_test_vasp(vec![
            create_test_element("", 100.0, 100.0, ElementType::Label),
            create_test_element("Not empty", 200.0, 200.0, ElementType::Label),
        ]);

        let text = VaspFormatter::format_text_only(&output);

        assert_eq!(text, "Not empty");
    }
}
