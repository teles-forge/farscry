use crate::error::FarscryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StateId([u8; 8]);

impl StateId {
    pub fn from_bits(bits: u64) -> Self {
        Self(bits.to_be_bytes())
    }

    pub fn to_bits(&self) -> u64 {
        u64::from_be_bytes(self.0)
    }

    pub fn hamming(self, other: StateId) -> u8 {
        (self.to_bits() ^ other.to_bits()).count_ones() as u8
    }
}

impl std::fmt::Display for StateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "phash:{}", hex::encode(self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ScreenType {
    Error,
    Config,
    Terminal,
    Conversation,
    Ui,
    Unknown,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Confidence {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ElementType {
    Button,
    Input,
    Select,
    Label,
    Heading,
    Error,
    Badge,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UiElement {
    pub text: String,
    pub element_type: ElementType,
    pub cx: f32,
    pub cy: f32,
    pub w: f32,
    pub h: f32,
    pub enabled: Option<bool>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AffordanceAction {
    Click,
    Type,
    Select,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Affordance {
    pub action: AffordanceAction,
    pub label: String,
    pub cx: f32,
    pub cy: f32,
    pub enabled: bool,
    pub current_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TextRegion {
    pub text: String,
    pub cx: f32,
    pub cy: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone)]
pub struct OcrOutput {
    pub regions: Vec<TextRegion>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VaspOutput {
    pub vasp_version: String,
    pub schema_version: u32,
    pub state_id: StateId,
    pub screen_type: ScreenType,
    pub confidence: Confidence,
    pub lang: String,
    pub agent_context: String,
    pub ui_tree: Vec<UiElement>,
    pub affordances: Vec<Affordance>,
}

impl VaspOutput {
    pub fn new(
        state_id: StateId,
        screen_type: ScreenType,
        confidence: Confidence,
        lang: impl Into<String>,
        agent_context: impl Into<String>,
        ui_tree: Vec<UiElement>,
        affordances: Vec<Affordance>,
    ) -> Self {
        Self {
            vasp_version: "1.0".to_string(),
            schema_version: 1,
            state_id,
            screen_type,
            confidence,
            lang: lang.into(),
            agent_context: agent_context.into(),
            ui_tree,
            affordances,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VaspDelta {
    pub vasp_version: String,
    pub diff_from: StateId,
    pub diff_to: StateId,
    pub context_similarity: f32,
    pub context_changed: bool,
    pub agent_context: String,
    pub entries: Vec<DeltaEntry>,
    pub tokens_saved: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DeltaEntry {
    Appeared(UiElement),
    Removed(UiElement),
    Changed { before: UiElement, after: UiElement },
    Unchanged(UiElement),
}

#[derive(Debug)]
pub struct BatchResult {
    pub path: std::path::PathBuf,
    pub output: Result<VaspOutput, FarscryError>,
}

#[derive(Debug, Clone)]
pub struct ClassifiedScreen {
    pub ui_tree: Vec<UiElement>,
    pub screen_type: ScreenType,
    pub state_id: StateId,
    pub lang: String,
    pub confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vasp_output_serialization() {
        let state_id = StateId::from_bits(0x123456789ABCDEF0);
        let vasp = VaspOutput::new(
            state_id,
            ScreenType::Ui,
            Confidence::High,
            "eng",
            "test context",
            vec![],
            vec![],
        );

        let json = serde_json::to_string(&vasp).unwrap();
        let deserialized: VaspOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(vasp, deserialized);
    }

    #[test]
    fn test_vasp_delta_serialization() {
        let state_from = StateId::from_bits(0x123456789ABCDEF0);
        let state_to = StateId::from_bits(0xFEDCBA9876543210);
        let delta = VaspDelta {
            vasp_version: "1.0".to_string(),
            diff_from: state_from,
            diff_to: state_to,
            context_similarity: 0.5,
            context_changed: false,
            agent_context: "test".to_string(),
            entries: vec![],
            tokens_saved: Some(100),
        };

        let json = serde_json::to_string(&delta).unwrap();
        let deserialized: VaspDelta = serde_json::from_str(&json).unwrap();

        assert_eq!(delta, deserialized);
    }

    #[test]
    fn test_confidence_ord() {
        assert!(Confidence::High > Confidence::Medium);
        assert!(Confidence::Medium > Confidence::Low);
        assert!(Confidence::Low > Confidence::None);
    }

    #[test]
    fn test_element_type_has_select() {
        assert!(matches!(ElementType::Select, ElementType::Select));
    }
}
