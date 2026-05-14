use farscry_core::{
    Affordance, AffordanceAction, Confidence, ElementType, ScreenType, StateId, UiElement,
    VaspOutput,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ClaudeComputerUseResult {
    pub screenshot: Option<ScreenshotInfo>,
    pub elements: Vec<ClaudeElement>,
    pub description: Option<String>,
    pub screen_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScreenshotInfo {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeElement {
    pub role: String,
    pub name: String,
    pub bounding_box: Option<BoundingBox>,
    pub enabled: Option<bool>,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn convert(input: &ClaudeComputerUseResult) -> VaspOutput {
    let ui_tree: Vec<UiElement> = input
        .elements
        .iter()
        .map(|elem| {
            let (cx, cy, w, h) = match &elem.bounding_box {
                Some(bb) => (
                    bb.x + bb.width / 2.0,
                    bb.y + bb.height / 2.0,
                    bb.width,
                    bb.height,
                ),
                None => (0.0, 0.0, 0.0, 0.0),
            };
            UiElement {
                text: elem.name.clone(),
                element_type: map_role(&elem.role),
                cx,
                cy,
                w,
                h,
                enabled: elem.enabled,
                value: elem.value.clone(),
            }
        })
        .collect();

    let affordances = extract_affordances(&ui_tree);

    let screen_type = input
        .screen_type
        .as_deref()
        .map(map_screen_type)
        .unwrap_or(ScreenType::Ui);

    let interactive_count = ui_tree
        .iter()
        .filter(|e| {
            matches!(
                e.element_type,
                ElementType::Button | ElementType::Input | ElementType::Select
            )
        })
        .count();

    let agent_context = input.description.clone().unwrap_or_else(|| {
        format!(
            "Screen captured - {} elements, {} interactive",
            ui_tree.len(),
            interactive_count
        )
    });

    VaspOutput::new(
        StateId::from_bits(0),
        screen_type,
        Confidence::Medium,
        "eng",
        agent_context,
        ui_tree,
        affordances,
    )
}

fn map_role(role: &str) -> ElementType {
    match role.to_lowercase().as_str() {
        "button" => ElementType::Button,
        "textbox" | "text_field" | "input" | "input_field" | "searchbox" => ElementType::Input,
        "combobox" | "listbox" | "select" | "dropdown" => ElementType::Select,
        "label" | "statictext" | "text" | "paragraph" => ElementType::Label,
        "heading" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => ElementType::Heading,
        "alert" | "error" => ElementType::Error,
        _ => ElementType::Unknown,
    }
}

fn map_screen_type(s: &str) -> ScreenType {
    match s.to_lowercase().as_str() {
        "terminal" => ScreenType::Terminal,
        "error" => ScreenType::Error,
        "config" | "settings" => ScreenType::Config,
        "conversation" | "chat" => ScreenType::Conversation,
        "ui" => ScreenType::Ui,
        _ => ScreenType::Unknown,
    }
}

fn extract_affordances(elements: &[UiElement]) -> Vec<Affordance> {
    elements
        .iter()
        .filter_map(|e| match e.element_type {
            ElementType::Button => Some(Affordance {
                action: AffordanceAction::Click,
                label: e.text.clone(),
                cx: e.cx,
                cy: e.cy,
                enabled: e.enabled.unwrap_or(true),
                current_value: None,
            }),
            ElementType::Input => Some(Affordance {
                action: AffordanceAction::Type,
                label: e.text.clone(),
                cx: e.cx,
                cy: e.cy,
                enabled: e.enabled.unwrap_or(true),
                current_value: e.value.clone(),
            }),
            ElementType::Select => Some(Affordance {
                action: AffordanceAction::Select,
                label: e.text.clone(),
                cx: e.cx,
                cy: e.cy,
                enabled: e.enabled.unwrap_or(true),
                current_value: e.value.clone(),
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_basic() {
        let json = r#"{
            "screenshot": { "width": 1920, "height": 1080 },
            "elements": [
                {
                    "role": "heading",
                    "name": "Payment",
                    "bounding_box": { "x": 640, "y": 100, "width": 640, "height": 48 }
                },
                {
                    "role": "textbox",
                    "name": "Card Number",
                    "bounding_box": { "x": 640, "y": 280, "width": 320, "height": 36 },
                    "enabled": true,
                    "value": ""
                },
                {
                    "role": "button",
                    "name": "Pay Now",
                    "bounding_box": { "x": 700, "y": 500, "width": 120, "height": 36 },
                    "enabled": true
                }
            ],
            "description": "Payment confirmation screen",
            "screen_type": "ui"
        }"#;

        let input: ClaudeComputerUseResult = serde_json::from_str(json).unwrap();
        let output = convert(&input);

        assert_eq!(output.ui_tree.len(), 3);
        assert_eq!(output.ui_tree[0].element_type, ElementType::Heading);
        assert_eq!(output.ui_tree[1].element_type, ElementType::Input);
        assert_eq!(output.ui_tree[2].element_type, ElementType::Button);
        assert_eq!(output.affordances.len(), 2);
        assert_eq!(output.screen_type, ScreenType::Ui);
        assert_eq!(output.agent_context, "Payment confirmation screen");
    }

    #[test]
    fn test_bounding_box_center() {
        let json = r#"{
            "elements": [
                {
                    "role": "button",
                    "name": "OK",
                    "bounding_box": { "x": 100.0, "y": 200.0, "width": 80.0, "height": 40.0 }
                }
            ]
        }"#;

        let input: ClaudeComputerUseResult = serde_json::from_str(json).unwrap();
        let output = convert(&input);

        let btn = &output.ui_tree[0];
        assert!((btn.cx - 140.0).abs() < 0.001);
        assert!((btn.cy - 220.0).abs() < 0.001);
    }

    #[test]
    fn test_role_mapping() {
        let cases = vec![
            ("button", ElementType::Button),
            ("textbox", ElementType::Input),
            ("text_field", ElementType::Input),
            ("combobox", ElementType::Select),
            ("heading", ElementType::Heading),
            ("label", ElementType::Label),
            ("alert", ElementType::Error),
            ("unknown_role", ElementType::Unknown),
        ];

        for (role, expected) in cases {
            assert_eq!(map_role(role), expected, "role: {}", role);
        }
    }

    #[test]
    fn test_no_screenshot_defaults() {
        let json = r#"{ "elements": [] }"#;
        let input: ClaudeComputerUseResult = serde_json::from_str(json).unwrap();
        let output = convert(&input);
        assert_eq!(output.screen_type, ScreenType::Ui);
        assert_eq!(output.confidence, Confidence::Medium);
    }
}
