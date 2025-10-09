use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_api_base_url")]
    pub api_base_url: String,
    pub api_key: String,
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Config {
    pub fn new(api_key: String) -> Self {
        Self {
            api_base_url: default_api_base_url(),
            api_key,
            request_timeout: default_request_timeout(),
            max_retries: default_max_retries(),
        }
    }
}

fn default_api_base_url() -> String {
    "https://jules.googleapis.com/v1alpha".to_string()
}

fn default_request_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}