use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub api_key: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            api_key: std::env::var("JULES_API_KEY")
                .context("JULES_API_KEY must be set")?,
            log_level: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| default_log_level()),
        })
    }
}

fn default_log_level() -> String {
    "info".to_string()
}