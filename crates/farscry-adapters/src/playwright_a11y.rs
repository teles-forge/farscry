use farscry_core::{Confidence, ScreenType, StateId, VaspOutput};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PlaywrightA11ySnapshot {
    pub role: Option<String>,
    pub name: Option<String>,
    pub children: Option<Vec<PlaywrightA11ySnapshot>>,
}

pub fn convert(_input: &PlaywrightA11ySnapshot) -> VaspOutput {
    VaspOutput::new(
        StateId::from_bits(0),
        ScreenType::Unknown,
        Confidence::None,
        "eng",
        "playwright-a11y adapter - not yet implemented",
        vec![],
        vec![],
    )
}
