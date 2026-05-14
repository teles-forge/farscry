use farscry_core::{UiElement, VaspOutput};
use std::fmt::Write;

use crate::context::{
    extract_affordances, format_confidence, format_element_type, format_screen_type,
    generate_agent_context,
};

pub(crate) fn format_vasp(
    output: &VaspOutput,
    source: &str,
    image_width: u32,
    image_height: u32,
) -> String {
    format_vasp_with_options(output, source, image_width, image_height, true)
}

pub(crate) fn format_vasp_with_options(
    output: &VaspOutput,
    source: &str,
    image_width: u32,
    image_height: u32,
    show_affordances: bool,
) -> String {
    let mut result = render_vasp_header(output, source);

    let mut sorted_elements = output.ui_tree.clone();
    sorted_elements.sort_by(|a, b| {
        a.cy.partial_cmp(&b.cy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cx.partial_cmp(&b.cx).unwrap_or(std::cmp::Ordering::Equal))
    });

    for element in &sorted_elements {
        let position_label = compute_position_label(element, image_width, image_height);
        result.push_str(&render_element_line(element, &position_label));
    }

    if show_affordances {
        render_affordances_block(&mut result, &extract_affordances(output));
    }

    let token_savings = compute_token_savings(image_width, image_height, &result);
    writeln!(result).ok();
    writeln!(
        result,
        "Token savings: ~{} tokens saved vs sending raw image to cloud vision systems",
        token_savings
    )
    .ok();

    result
}

pub(crate) fn format_text_only(output: &VaspOutput) -> String {
    output
        .ui_tree
        .iter()
        .filter(|e| !e.text.is_empty())
        .map(|e| e.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn format_json(output: &VaspOutput, pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(output).unwrap_or_default()
    } else {
        serde_json::to_string(output).unwrap_or_default()
    }
}

pub(crate) fn format_batch(outputs: &[(String, VaspOutput, u32, u32)]) -> String {
    let mut result = String::new();

    for (i, (source, output, width, height)) in outputs.iter().enumerate() {
        if i > 0 {
            writeln!(result, "---").ok();
        }
        writeln!(result, "file: {}", source).ok();
        result.push_str(&format_vasp(output, source, *width, *height));
    }

    result
}

pub(crate) fn compute_position_label(element: &UiElement, width: u32, height: u32) -> String {
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

pub(crate) fn compute_token_savings(image_w: u32, image_h: u32, vasp_text: &str) -> u32 {
    let image_tokens = image_w.div_ceil(512) * image_h.div_ceil(512) * 170 + 85;
    let vasp_tokens = (vasp_text.len() / 4) as u32;
    image_tokens.saturating_sub(vasp_tokens)
}

fn render_vasp_header(output: &VaspOutput, source: &str) -> String {
    let mut s = String::new();
    writeln!(s, "=== farscry visual context ===").ok();
    writeln!(s, "source: {}", source).ok();
    writeln!(
        s,
        "screen_type: {}",
        format_screen_type(&output.screen_type)
    )
    .ok();
    writeln!(s, "state_id: {}", output.state_id).ok();
    writeln!(s, "confidence: {}", format_confidence(&output.confidence)).ok();
    writeln!(s, "lang: {}", output.lang).ok();
    writeln!(s, "agent_context: \"{}\"", generate_agent_context(output)).ok();
    writeln!(s, "---").ok();
    s
}

fn render_affordances_block(result: &mut String, affordances: &[String]) {
    if !affordances.is_empty() {
        writeln!(result).ok();
        writeln!(result, "affordances:").ok();
        for affordance in affordances {
            writeln!(result, "  {}", affordance).ok();
        }
    }
}

fn render_element_line(element: &UiElement, position_label: &str) -> String {
    let mut s = String::new();
    writeln!(
        s,
        "[{:12}]  {:8}  \"{}\"",
        position_label,
        format_element_type(&element.element_type),
        element.text
    )
    .ok();
    if let Some(value) = &element.value {
        writeln!(s, "               value=\"{}\"", value).ok();
    }
    if let Some(enabled) = element.enabled {
        writeln!(s, "               enabled:{}", enabled).ok();
    }
    s
}
