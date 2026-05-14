use farscry_core::{Confidence, ElementType, ScreenType, UiElement, VaspOutput};

pub(crate) fn generate_agent_context(output: &VaspOutput) -> String {
    match &output.screen_type {
        ScreenType::Terminal => generate_terminal_context(&output.ui_tree),
        ScreenType::Error => generate_error_context(&output.ui_tree),
        ScreenType::Config => generate_config_context(&output.ui_tree),
        ScreenType::Conversation => generate_conversation_context(&output.ui_tree),
        ScreenType::Ui | ScreenType::Unknown => generate_ui_context(&output.ui_tree),
    }
}

fn generate_terminal_context(elements: &[UiElement]) -> String {
    let error = elements
        .iter()
        .find(|e| e.element_type == ElementType::Error)
        .map(|e| e.text.clone());
    if let Some(err) = error {
        format!("Build failed - {}", err)
    } else {
        "Script completed successfully".to_string()
    }
}

fn generate_config_context(elements: &[UiElement]) -> String {
    let editable_count = elements
        .iter()
        .filter(|e| e.element_type == ElementType::Input || e.element_type == ElementType::Select)
        .count();
    let button = elements
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

fn generate_error_context(elements: &[UiElement]) -> String {
    let error = elements
        .iter()
        .find(|e| e.element_type == ElementType::Error)
        .map(|e| e.text.clone());
    if let Some(err) = error {
        format!("Error - {}", err)
    } else {
        "Error detected".to_string()
    }
}

fn generate_conversation_context(elements: &[UiElement]) -> String {
    if let Some(msg) = elements.last() {
        format!("Conversation - last: {}", msg.text)
    } else {
        "Conversation captured".to_string()
    }
}

fn generate_ui_context(elements: &[UiElement]) -> String {
    let interactive_count = elements
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
        elements.len(),
        interactive_count
    )
}

pub(crate) fn format_screen_type(screen_type: &ScreenType) -> String {
    match screen_type {
        ScreenType::Terminal => "terminal".to_string(),
        ScreenType::Config => "config".to_string(),
        ScreenType::Conversation => "conversation".to_string(),
        ScreenType::Error => "error".to_string(),
        ScreenType::Ui => "ui".to_string(),
        ScreenType::Unknown => "unknown".to_string(),
    }
}

pub(crate) fn format_confidence(confidence: &Confidence) -> String {
    match confidence {
        Confidence::High => "high".to_string(),
        Confidence::Medium => "medium".to_string(),
        Confidence::Low => "low".to_string(),
        Confidence::None => "none".to_string(),
    }
}

pub(crate) fn format_element_type(element_type: &ElementType) -> String {
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

pub(crate) fn extract_affordances(output: &VaspOutput) -> Vec<String> {
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
