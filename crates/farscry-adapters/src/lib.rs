pub mod claude_computer_use;
pub mod openai_vision;
pub mod playwright_a11y;

use farscry_core::VaspOutput;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown adapter: {0}")]
    UnknownAdapter(String),
}

pub fn convert_file(adapter: &str, input: &str) -> Result<VaspOutput, AdapterError> {
    match adapter {
        "claude-computer-use" => {
            let parsed: claude_computer_use::ClaudeComputerUseResult =
                serde_json::from_str(input)?;
            Ok(claude_computer_use::convert(&parsed))
        }
        "playwright-a11y" => {
            let parsed: playwright_a11y::PlaywrightA11ySnapshot = serde_json::from_str(input)?;
            Ok(playwright_a11y::convert(&parsed))
        }
        "openai-vision" => {
            let parsed: openai_vision::OpenAiVisionResponse = serde_json::from_str(input)?;
            Ok(openai_vision::convert(&parsed))
        }
        other => Err(AdapterError::UnknownAdapter(other.to_string())),
    }
}
