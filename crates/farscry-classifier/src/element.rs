use farscry_core::{ElementType, ScreenType, TextRegion, UiElement};

const BUTTON_WORDS: &[&str] = &[
    "Save", "Cancel", "Submit", "Delete", "OK", "Apply", "Next", "Back", "Continue", "Close",
];

pub fn classify_elements(regions: &[TextRegion], screen_type: ScreenType) -> Vec<UiElement> {
    match screen_type {
        ScreenType::Terminal => classify_terminal(regions),
        ScreenType::Config => classify_config(regions),
        ScreenType::Error => classify_error(regions),
        ScreenType::Conversation => classify_conversation(regions),
        ScreenType::Ui | ScreenType::Unknown => classify_ui(regions),
    }
}

fn classify_terminal(regions: &[TextRegion]) -> Vec<UiElement> {
    regions
        .iter()
        .map(|region| UiElement {
            text: region.text.clone(),
            element_type: ElementType::Label,
            cx: region.cx,
            cy: region.cy,
            w: region.w,
            h: region.h,
            enabled: None,
            value: None,
        })
        .collect()
}

fn classify_config(regions: &[TextRegion]) -> Vec<UiElement> {
    regions
        .iter()
        .map(|region| {
            let element_type = classify_config_element(region);
            UiElement {
                text: region.text.clone(),
                element_type,
                cx: region.cx,
                cy: region.cy,
                w: region.w,
                h: region.h,
                enabled: None,
                value: None,
            }
        })
        .collect()
}

fn classify_config_element(region: &TextRegion) -> ElementType {
    if region.text.ends_with(':') {
        return ElementType::Label;
    }

    let aspect_ratio = region.w / region.h.max(1.0);
    if aspect_ratio > 4.0 && region.w > 150.0 {
        return ElementType::Input;
    }

    if BUTTON_WORDS
        .iter()
        .any(|word| region.text.eq_ignore_ascii_case(word))
    {
        return ElementType::Button;
    }

    if region.text.len() < 20
        && region
            .text
            .chars()
            .all(|c| c.is_uppercase() || c.is_whitespace())
    {
        return ElementType::Heading;
    }

    ElementType::Label
}

fn classify_error(regions: &[TextRegion]) -> Vec<UiElement> {
    regions
        .iter()
        .map(|region| {
            let element_type = if region.text.to_lowercase().contains("error") {
                ElementType::Error
            } else {
                ElementType::Label
            };

            UiElement {
                text: region.text.clone(),
                element_type,
                cx: region.cx,
                cy: region.cy,
                w: region.w,
                h: region.h,
                enabled: None,
                value: None,
            }
        })
        .collect()
}

fn classify_conversation(regions: &[TextRegion]) -> Vec<UiElement> {
    regions
        .iter()
        .map(|region| {
            let word_count = region.text.split_whitespace().count();
            let element_type = if (1..=3).contains(&word_count) {
                ElementType::Heading
            } else {
                ElementType::Label
            };

            UiElement {
                text: region.text.clone(),
                element_type,
                cx: region.cx,
                cy: region.cy,
                w: region.w,
                h: region.h,
                enabled: None,
                value: None,
            }
        })
        .collect()
}

fn classify_ui(regions: &[TextRegion]) -> Vec<UiElement> {
    regions
        .iter()
        .map(|region| {
            let element_type = classify_ui_element(region);
            UiElement {
                text: region.text.clone(),
                element_type,
                cx: region.cx,
                cy: region.cy,
                w: region.w,
                h: region.h,
                enabled: None,
                value: None,
            }
        })
        .collect()
}

fn classify_ui_element(region: &TextRegion) -> ElementType {
    if BUTTON_WORDS
        .iter()
        .any(|word| region.text.eq_ignore_ascii_case(word))
    {
        return ElementType::Button;
    }

    if region.text.ends_with(':') {
        return ElementType::Label;
    }

    let aspect_ratio = region.w / region.h.max(1.0);
    if aspect_ratio > 3.0 && region.w > 100.0 {
        return ElementType::Input;
    }

    ElementType::Label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_all_labels() {
        let regions = vec![
            TextRegion {
                text: "$ ls -la".to_string(),
                cx: 0.0,
                cy: 0.0,
                w: 100.0,
                h: 20.0,
            },
            TextRegion {
                text: "drwxr-xr-x".to_string(),
                cx: 0.0,
                cy: 30.0,
                w: 100.0,
                h: 20.0,
            },
        ];

        let elements = classify_terminal(&regions);
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].element_type, ElementType::Label);
        assert_eq!(elements[1].element_type, ElementType::Label);
    }

    #[test]
    fn test_config_colon_label() {
        let region = TextRegion {
            text: "Username:".to_string(),
            cx: 0.0,
            cy: 0.0,
            w: 100.0,
            h: 20.0,
        };

        let element_type = classify_config_element(&region);
        assert_eq!(element_type, ElementType::Label);
    }

    #[test]
    fn test_config_wide_input() {
        let region = TextRegion {
            text: "user@example.com".to_string(),
            cx: 0.0,
            cy: 0.0,
            w: 400.0,
            h: 30.0,
        };

        let element_type = classify_config_element(&region);
        assert_eq!(element_type, ElementType::Input);
    }

    #[test]
    fn test_config_button() {
        let region = TextRegion {
            text: "Save".to_string(),
            cx: 0.0,
            cy: 0.0,
            w: 80.0,
            h: 30.0,
        };

        let element_type = classify_config_element(&region);
        assert_eq!(element_type, ElementType::Button);
    }

    #[test]
    fn test_config_heading() {
        let region = TextRegion {
            text: "SETTINGS".to_string(),
            cx: 0.0,
            cy: 0.0,
            w: 100.0,
            h: 30.0,
        };

        let element_type = classify_config_element(&region);
        assert_eq!(element_type, ElementType::Heading);
    }

    #[test]
    fn test_error_detection() {
        let regions = vec![
            TextRegion {
                text: "TypeError: invalid operation".to_string(),
                cx: 0.0,
                cy: 0.0,
                w: 100.0,
                h: 20.0,
            },
            TextRegion {
                text: "at line 42".to_string(),
                cx: 0.0,
                cy: 30.0,
                w: 100.0,
                h: 20.0,
            },
        ];

        let elements = classify_error(&regions);
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].element_type, ElementType::Error);
        assert_eq!(elements[1].element_type, ElementType::Label);
    }

    #[test]
    fn test_conversation_speaker_heading() {
        let regions = vec![
            TextRegion {
                text: "Alice".to_string(),
                cx: 0.0,
                cy: 0.0,
                w: 50.0,
                h: 20.0,
            },
            TextRegion {
                text: "This is a longer message".to_string(),
                cx: 0.0,
                cy: 30.0,
                w: 200.0,
                h: 20.0,
            },
        ];

        let elements = classify_conversation(&regions);
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].element_type, ElementType::Heading);
        assert_eq!(elements[1].element_type, ElementType::Label);
    }
}
