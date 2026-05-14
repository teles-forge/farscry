use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SessionsConfig {
    pub record: Option<bool>,
    pub output_dir: Option<String>,
    pub hamming_threshold: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FarscryConfig {
    pub agent: Option<AgentConfig>,
    pub sessions: Option<SessionsConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub preferred: String,
    pub default_prompt: String,
}

pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".farscry")
        .join("config.toml")
}

pub fn read_farscry_config() -> FarscryConfig {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn write_farscry_config(agent: &str, default_prompt: &str) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cfg = FarscryConfig {
        agent: Some(AgentConfig {
            preferred: agent.to_string(),
            default_prompt: default_prompt.to_string(),
        }),
        sessions: None,
    };
    let content = toml::to_string_pretty(&cfg)?;
    std::fs::write(&path, content)?;
    Ok(())
}
