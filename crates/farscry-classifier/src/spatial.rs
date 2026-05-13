use farscry_core::{Affordance, AffordanceAction, ElementType, UiElement};

pub fn extract_affordances(elements: &[UiElement]) -> Vec<Affordance> {
    let mut affordances = Vec::new();

    for element in elements {
        match element.element_type {
            ElementType::Button => {
                affordances.push(Affordance {
                    action: AffordanceAction::Click,
                    label: element.text.clone(),
                    cx: element.cx,
                    cy: element.cy,
                    enabled: element.enabled.unwrap_or(true),
                    current_value: None,
                });
            }
            ElementType::Input => {
                let label = nearest_label(element, elements);
                affordances.push(Affordance {
                    action: AffordanceAction::Type,
                    label,
                    cx: element.cx,
                    cy: element.cy,
                    enabled: element.enabled.unwrap_or(true),
                    current_value: element.value.clone(),
                });
            }
            ElementType::Select => {
                affordances.push(Affordance {
                    action: AffordanceAction::Select,
                    label: element.text.clone(),
                    cx: element.cx,
                    cy: element.cy,
                    enabled: element.enabled.unwrap_or(true),
                    current_value: element.value.clone(),
                });
            }
            _ => {}
        }
    }

    affordances
}

fn nearest_label(input: &UiElement, all: &[UiElement]) -> String {
    all.iter()
        .filter(|e| e.element_type == ElementType::Label)
        .filter(|e| e.cx < input.cx + 10.0 || e.cy < input.cy + 10.0)
        .filter(|e| {
            let dx = e.cx - input.cx;
            let dy = e.cy - input.cy;
            let distance = (dx * dx + dy * dy).sqrt();
            distance < 200.0
        })
        .min_by_key(|e| {
            let dx = e.cx - input.cx;
            let dy = e.cy - input.cy;
            (dx * dx + dy * dy) as u32
        })
        .map(|e| e.text.clone())
        .unwrap_or_else(|| input.text.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_button_affordance() {
        let elements = vec![UiElement {
            text: "Save".to_string(),
            element_type: ElementType::Button,
            cx: 100.0,
            cy: 200.0,
            w: 80.0,
            h: 30.0,
            enabled: Some(true),
            value: None,
        }];

        let affordances = extract_affordances(&elements);
        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].action, AffordanceAction::Click);
        assert_eq!(affordances[0].label, "Save");
        assert_eq!(affordances[0].cx, 100.0);
        assert_eq!(affordances[0].cy, 200.0);
        assert!(affordances[0].enabled);
    }

    #[test]
    fn test_extract_input_affordance_with_label() {
        let elements = vec![
            UiElement {
                text: "Username:".to_string(),
                element_type: ElementType::Label,
                cx: 50.0,
                cy: 100.0,
                w: 80.0,
                h: 20.0,
                enabled: None,
                value: None,
            },
            UiElement {
                text: "user@example.com".to_string(),
                element_type: ElementType::Input,
                cx: 150.0,
                cy: 100.0,
                w: 200.0,
                h: 30.0,
                enabled: Some(true),
                value: None,
            },
        ];

        let affordances = extract_affordances(&elements);
        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].action, AffordanceAction::Type);
        assert_eq!(affordances[0].label, "Username:");
    }

    #[test]
    fn test_extract_input_affordance_fallback() {
        let elements = vec![UiElement {
            text: "user@example.com".to_string(),
            element_type: ElementType::Input,
            cx: 150.0,
            cy: 100.0,
            w: 200.0,
            h: 30.0,
            enabled: Some(true),
            value: None,
        }];

        let affordances = extract_affordances(&elements);
        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].action, AffordanceAction::Type);
        assert_eq!(affordances[0].label, "user@example.com");
    }

    #[test]
    fn test_nearest_label_left() {
        let elements = vec![
            UiElement {
                text: "Password:".to_string(),
                element_type: ElementType::Label,
                cx: 50.0,
                cy: 100.0,
                w: 80.0,
                h: 20.0,
                enabled: None,
                value: None,
            },
            UiElement {
                text: "secret".to_string(),
                element_type: ElementType::Input,
                cx: 150.0,
                cy: 100.0,
                w: 150.0,
                h: 30.0,
                enabled: None,
                value: None,
            },
        ];

        let label = nearest_label(&elements[1], &elements);
        assert_eq!(label, "Password:");
    }

    #[test]
    fn test_nearest_label_above() {
        let elements = vec![
            UiElement {
                text: "Email:".to_string(),
                element_type: ElementType::Label,
                cx: 150.0,
                cy: 50.0,
                w: 60.0,
                h: 20.0,
                enabled: None,
                value: None,
            },
            UiElement {
                text: "test@example.com".to_string(),
                element_type: ElementType::Input,
                cx: 150.0,
                cy: 100.0,
                w: 200.0,
                h: 30.0,
                enabled: None,
                value: None,
            },
        ];

        let label = nearest_label(&elements[1], &elements);
        assert_eq!(label, "Email:");
    }

    #[test]
    fn test_nearest_label_distance_threshold() {
        let elements = vec![
            UiElement {
                text: "Far Label:".to_string(),
                element_type: ElementType::Label,
                cx: 50.0,
                cy: 50.0,
                w: 80.0,
                h: 20.0,
                enabled: None,
                value: None,
            },
            UiElement {
                text: "input".to_string(),
                element_type: ElementType::Input,
                cx: 500.0,
                cy: 500.0,
                w: 150.0,
                h: 30.0,
                enabled: None,
                value: None,
            },
        ];

        let label = nearest_label(&elements[1], &elements);

        assert_eq!(label, "input");
    }

    #[test]
    fn test_extract_select_affordance() {
        let elements = vec![UiElement {
            text: "Option A".to_string(),
            element_type: ElementType::Select,
            cx: 100.0,
            cy: 100.0,
            w: 150.0,
            h: 30.0,
            enabled: Some(true),
            value: Some("A".to_string()),
        }];

        let affordances = extract_affordances(&elements);
        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].action, AffordanceAction::Select);
        assert_eq!(affordances[0].label, "Option A");
        assert_eq!(affordances[0].current_value, Some("A".to_string()));
    }

    #[test]
    fn test_label_no_affordance() {
        let elements = vec![UiElement {
            text: "Some label".to_string(),
            element_type: ElementType::Label,
            cx: 100.0,
            cy: 100.0,
            w: 100.0,
            h: 20.0,
            enabled: None,
            value: None,
        }];

        let affordances = extract_affordances(&elements);
        assert_eq!(affordances.len(), 0);
    }
}
