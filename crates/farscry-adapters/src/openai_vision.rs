use farscry_core::{Confidence, ScreenType, StateId, VaspOutput};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OpenAiVisionResponse {
    pub choices: Option<Vec<OpenAiChoice>>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiChoice {
    pub message: Option<OpenAiMessage>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiMessage {
    pub content: Option<String>,
}

pub fn convert(_input: &OpenAiVisionResponse) -> VaspOutput {
    VaspOutput::new(
        StateId::from_bits(0),
        ScreenType::Unknown,
        Confidence::None,
        "eng",
        "openai-vision adapter - not yet implemented",
        vec![],
        vec![],
    )
}
