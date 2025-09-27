use crate::error::{ClientError, ClientResult};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Client configuration for connecting to codeflow-buddy server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// WebSocket server URL (e.g., "ws://localhost:3000")
    pub url: Option<String>,
    /// JWT authentication token
    pub token: Option<String>,
    /// Request timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            url: None,
            token: None,
            timeout_ms: Some(30000), // 30 seconds default
        }
    }
}

impl ClientConfig {
    /// Create a new client configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration builder for fluent configuration
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Load configuration from file, falling back to defaults and environment variables
    pub async fn load() -> ClientResult<Self> {
        ConfigBuilder::new()
            .from_file_if_exists(Self::default_config_path()?)
            .await?
            .with_env_overrides()
            .build()
    }

    /// Create a client config with environment variable overrides
    pub async fn with_env_overrides() -> ClientResult<Self> {
        ConfigBuilder::new()
            .from_file_if_exists(Self::default_config_path()?)
            .await?
            .with_env_overrides()
            .build()
    }

    /// Create configuration from all available sources
    pub async fn from_all_sources() -> ClientResult<Self> {
        Self::load()
    }

    /// Get missing configuration items
    pub fn missing_items(&self) -> Vec<String> {
        let mut missing = Vec::new();

        if self.url.is_none() {
            missing.push("Server URL".to_string());
        }

        missing
    }

    /// Suggest configuration actions
    pub fn suggest_actions(&self) -> Vec<String> {
        let mut actions = Vec::new();

        if self.url.is_none() {
            actions.push("Set server URL with --url or CODEFLOW_BUDDY_URL environment variable".to_string());
        }

        if self.token.is_none() {
            actions.push("Set authentication token with --token or CODEFLOW_BUDDY_TOKEN environment variable".to_string());
        }

        actions
    }

    /// Load configuration from a specific file path
    pub async fn load_from_path<P: AsRef<Path>>(path: P) -> ClientResult<Self> {
        let path = path.as_ref();
        debug!("Loading config from {}", path.display());

        let content = fs::read_to_string(path).await
            .map_err(|e| ClientError::ConfigError(format!("Failed to read config file: {}", e)))?;

        let config: Self = serde_json::from_str(&content)
            .map_err(|e| ClientError::ConfigError(format!("Failed to parse config file: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to the default config file
    pub async fn save(&self) -> ClientResult<()> {
        let config_path = Self::default_config_path()?;
        self.save_to_path(&config_path).await
    }

    /// Save configuration to a specific file path
    pub async fn save_to_path<P: AsRef<Path>>(&self, path: P) -> ClientResult<()> {
        let path = path.as_ref();

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| ClientError::ConfigError(format!("Failed to create config directory: {}", e)))?;
        }

        // Serialize config with pretty formatting
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ClientError::ConfigError(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, content).await
            .map_err(|e| ClientError::ConfigError(format!("Failed to write config file: {}", e)))?;

        info!("Configuration saved to {}", path.display());
        Ok(())
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> ClientResult<PathBuf> {
        let home = home_dir()
            .ok_or_else(|| ClientError::ConfigError("Unable to determine home directory".to_string()))?;

        Ok(home.join(".codeflow-buddy").join("config.json"))
    }

    /// Get the configuration directory path
    pub fn config_dir() -> ClientResult<PathBuf> {
        let home = home_dir()
            .ok_or_else(|| ClientError::ConfigError("Unable to determine home directory".to_string()))?;

        Ok(home.join(".codeflow-buddy"))
    }

    /// Validate the configuration
    pub fn validate(&self) -> ClientResult<()> {
        // Validate URL format if provided
        if let Some(ref url) = self.url {
            if let Err(e) = url::Url::parse(url) {
                return Err(ClientError::ConfigError(format!("Invalid URL format: {}", e)));
            }
        }

        // Validate timeout is reasonable
        if let Some(timeout) = self.timeout_ms {
            if timeout == 0 {
                return Err(ClientError::ConfigError("Timeout cannot be zero".to_string()));
            }
            if timeout > 300_000 { // 5 minutes max
                return Err(ClientError::ConfigError("Timeout cannot exceed 5 minutes".to_string()));
            }
        }

        Ok(())
    }

    /// Get the URL, returning an error if not configured
    pub fn get_url(&self) -> ClientResult<&str> {
        self.url.as_deref()
            .ok_or_else(|| ClientError::ConfigError("No server URL configured".to_string()))
    }

    /// Get the timeout in milliseconds
    pub fn get_timeout_ms(&self) -> u64 {
        self.timeout_ms.unwrap_or(30000)
    }

    /// Check if authentication token is available
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// Get the authentication token
    pub fn get_token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Set the URL
    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    /// Set the token
    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    /// Set the timeout
    pub fn set_timeout_ms(&mut self, timeout_ms: u64) {
        self.timeout_ms = Some(timeout_ms);
    }

    /// Clear the token
    pub fn clear_token(&mut self) {
        self.token = None;
    }

    /// Create a config with command line overrides
    pub fn with_overrides(&self, url: Option<String>, token: Option<String>) -> Self {
        let mut config = self.clone();

        if let Some(url) = url {
            config.url = Some(url);
        }

        if let Some(token) = token {
            config.token = Some(token);
        }

        config
    }

    /// Check if the configuration appears to be complete for making requests
    pub fn is_complete(&self) -> bool {
        self.url.is_some()
    }

    /// Get a summary of the configuration for display
    pub fn summary(&self) -> String {
        let url = self.url.as_deref().unwrap_or("<not configured>");
        let token_status = if self.token.is_some() { "configured" } else { "not configured" };
        let timeout = self.get_timeout_ms();

        format!(
            "URL: {}\nToken: {}\nTimeout: {}ms",
            url, token_status, timeout
        )
    }
}

/// Configuration builder for fluent API
pub struct ConfigBuilder {
    config: ClientConfig,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }

    /// Load configuration from file if it exists
    pub async fn from_file_if_exists<P: AsRef<Path>>(mut self, path: P) -> ClientResult<Self> {
        let path = path.as_ref();

        if path.exists() {
            debug!("Loading config from {}", path.display());
            let content = fs::read_to_string(path).await
                .map_err(|e| ClientError::ConfigError(format!("Failed to read config file: {}", e)))?;

            self.config = serde_json::from_str(&content)
                .map_err(|e| ClientError::ConfigError(format!("Failed to parse config file: {}", e)))?;
        } else {
            debug!("No config file found at {}, using defaults", path.display());
        }

        Ok(self)
    }

    /// Load configuration from a specific file
    pub async fn from_file<P: AsRef<Path>>(mut self, path: P) -> ClientResult<Self> {
        let path = path.as_ref();
        debug!("Loading config from {}", path.display());

        let content = fs::read_to_string(path).await
            .map_err(|e| ClientError::ConfigError(format!("Failed to read config file: {}", e)))?;

        self.config = serde_json::from_str(&content)
            .map_err(|e| ClientError::ConfigError(format!("Failed to parse config file: {}", e)))?;

        Ok(self)
    }

    /// Apply environment variable overrides
    pub fn with_env_overrides(mut self) -> Self {
        // Override with environment variables if present
        if let Ok(url) = std::env::var("CODEFLOW_BUDDY_URL") {
            debug!("Using URL from environment variable");
            self.config.url = Some(url);
        }

        if let Ok(token) = std::env::var("CODEFLOW_BUDDY_TOKEN") {
            debug!("Using token from environment variable");
            self.config.token = Some(token);
        }

        if let Ok(timeout) = std::env::var("CODEFLOW_BUDDY_TIMEOUT") {
            match timeout.parse::<u64>() {
                Ok(timeout_ms) => {
                    debug!("Using timeout from environment variable: {}ms", timeout_ms);
                    self.config.timeout_ms = Some(timeout_ms);
                }
                Err(e) => {
                    warn!("Invalid timeout in environment variable: {}", e);
                }
            }
        }

        self
    }

    /// Set the URL
    pub fn with_url(mut self, url: String) -> Self {
        self.config.url = Some(url);
        self
    }

    /// Set the token
    pub fn with_token(mut self, token: String) -> Self {
        self.config.token = Some(token);
        self
    }

    /// Set the timeout in milliseconds
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.config.timeout_ms = Some(timeout_ms);
        self
    }

    /// Apply overrides from another config
    pub fn with_overrides(mut self, url: Option<String>, token: Option<String>) -> Self {
        if let Some(url) = url {
            self.config.url = Some(url);
        }

        if let Some(token) = token {
            self.config.token = Some(token);
        }

        self
    }

    /// Build and validate the final configuration
    pub fn build(self) -> ClientResult<ClientConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serial_test::serial;
    use std::env;

    #[tokio::test]
    async fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut config = ClientConfig::new();
        config.set_url("ws://localhost:3000".to_string());
        config.set_token("test-token".to_string());
        config.set_timeout_ms(60000);

        // Save config
        config.save_to_path(&config_path).await.unwrap();

        // Load config
        let loaded_config = ClientConfig::load_from_path(&config_path).await.unwrap();

        assert_eq!(loaded_config.url, Some("ws://localhost:3000".to_string()));
        assert_eq!(loaded_config.token, Some("test-token".to_string()));
        assert_eq!(loaded_config.timeout_ms, Some(60000));
    }

    #[test]
    fn test_config_validation() {
        let mut config = ClientConfig::new();

        // Valid config
        assert!(config.validate().is_ok());

        // Invalid URL
        config.set_url("invalid-url".to_string());
        assert!(config.validate().is_err());

        // Valid URL
        config.set_url("ws://localhost:3000".to_string());
        assert!(config.validate().is_ok());

        // Invalid timeout
        config.set_timeout_ms(0);
        assert!(config.validate().is_err());

        // Timeout too large
        config.set_timeout_ms(400_000);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_overrides() {
        let mut base_config = ClientConfig::new();
        base_config.set_url("ws://localhost:3000".to_string());
        base_config.set_token("base-token".to_string());

        let overridden = base_config.with_overrides(
            Some("ws://example.com:4000".to_string()),
            Some("override-token".to_string())
        );

        assert_eq!(overridden.url, Some("ws://example.com:4000".to_string()));
        assert_eq!(overridden.token, Some("override-token".to_string()));

        // Original should be unchanged
        assert_eq!(base_config.url, Some("ws://localhost:3000".to_string()));
        assert_eq!(base_config.token, Some("base-token".to_string()));
    }

    // New comprehensive tests for ConfigBuilder

    #[tokio::test]
    async fn test_config_builder_basic() {
        let config = ConfigBuilder::new()
            .with_url("ws://test:8080".to_string())
            .with_token("test-token".to_string())
            .with_timeout_ms(45000)
            .build()
            .unwrap();

        assert_eq!(config.url, Some("ws://test:8080".to_string()));
        assert_eq!(config.token, Some("test-token".to_string()));
        assert_eq!(config.timeout_ms, Some(45000));
    }

    #[tokio::test]
    async fn test_config_builder_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("builder_test.json");

        // Create a config file
        let file_config = ClientConfig {
            url: Some("ws://file:3000".to_string()),
            token: Some("file-token".to_string()),
            timeout_ms: Some(30000),
        };
        file_config.save_to_path(&config_path).await.unwrap();

        // Load using builder
        let config = ConfigBuilder::new()
            .from_file(config_path)
            .await
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(config.url, Some("ws://file:3000".to_string()));
        assert_eq!(config.token, Some("file-token".to_string()));
        assert_eq!(config.timeout_ms, Some(30000));
    }

    #[tokio::test]
    async fn test_config_builder_file_if_exists_missing() {
        let temp_dir = TempDir::new().unwrap();
        let missing_path = temp_dir.path().join("missing.json");

        // Should not fail when file doesn't exist
        let config = ConfigBuilder::new()
            .from_file_if_exists(missing_path)
            .await
            .unwrap()
            .with_url("ws://default:3000".to_string())
            .build()
            .unwrap();

        assert_eq!(config.url, Some("ws://default:3000".to_string()));
        assert_eq!(config.token, None);
    }

    #[serial]
    #[tokio::test]
    async fn test_config_builder_env_overrides() {
        // Clean up any existing env vars
        let original_url = env::var("CODEFLOW_BUDDY_URL").ok();
        let original_token = env::var("CODEFLOW_BUDDY_TOKEN").ok();
        let original_timeout = env::var("CODEFLOW_BUDDY_TIMEOUT").ok();

        // Set test environment variables
        env::set_var("CODEFLOW_BUDDY_URL", "ws://env:4000");
        env::set_var("CODEFLOW_BUDDY_TOKEN", "env-token");
        env::set_var("CODEFLOW_BUDDY_TIMEOUT", "25000");

        let config = ConfigBuilder::new()
            .with_url("ws://builder:3000".to_string()) // Should be overridden
            .with_env_overrides()
            .build()
            .unwrap();

        assert_eq!(config.url, Some("ws://env:4000".to_string()));
        assert_eq!(config.token, Some("env-token".to_string()));
        assert_eq!(config.timeout_ms, Some(25000));

        // Clean up
        env::remove_var("CODEFLOW_BUDDY_URL");
        env::remove_var("CODEFLOW_BUDDY_TOKEN");
        env::remove_var("CODEFLOW_BUDDY_TIMEOUT");

        // Restore original values if they existed
        if let Some(url) = original_url {
            env::set_var("CODEFLOW_BUDDY_URL", url);
        }
        if let Some(token) = original_token {
            env::set_var("CODEFLOW_BUDDY_TOKEN", token);
        }
        if let Some(timeout) = original_timeout {
            env::set_var("CODEFLOW_BUDDY_TIMEOUT", timeout);
        }
    }

    #[serial]
    #[tokio::test]
    async fn test_config_precedence_cli_over_env() {
        // Set up environment variables
        env::set_var("CODEFLOW_BUDDY_URL", "ws://env:5000");
        env::set_var("CODEFLOW_BUDDY_TOKEN", "env-token");

        let config = ConfigBuilder::new()
            .with_env_overrides()
            .with_url("ws://cli:6000".to_string()) // CLI should override env
            .build()
            .unwrap();

        assert_eq!(config.url, Some("ws://cli:6000".to_string())); // CLI wins
        assert_eq!(config.token, Some("env-token".to_string())); // Env used for token

        // Clean up
        env::remove_var("CODEFLOW_BUDDY_URL");
        env::remove_var("CODEFLOW_BUDDY_TOKEN");
    }

    #[serial]
    #[tokio::test]
    async fn test_config_precedence_env_over_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("precedence_test.json");

        // Create config file
        let file_config = ClientConfig {
            url: Some("ws://file:7000".to_string()),
            token: Some("file-token".to_string()),
            timeout_ms: Some(60000),
        };
        file_config.save_to_path(&config_path).await.unwrap();

        // Set environment variable (should override file)
        env::set_var("CODEFLOW_BUDDY_URL", "ws://env:8000");

        let config = ConfigBuilder::new()
            .from_file_if_exists(config_path)
            .await
            .unwrap()
            .with_env_overrides()
            .build()
            .unwrap();

        assert_eq!(config.url, Some("ws://env:8000".to_string())); // Env overrides file
        assert_eq!(config.token, Some("file-token".to_string())); // File used for token
        assert_eq!(config.timeout_ms, Some(60000)); // File used for timeout

        // Clean up
        env::remove_var("CODEFLOW_BUDDY_URL");
    }

    #[serial]
    #[tokio::test]
    async fn test_invalid_env_timeout_handled_gracefully() {
        env::set_var("CODEFLOW_BUDDY_TIMEOUT", "invalid-number");

        let config = ConfigBuilder::new()
            .with_timeout_ms(15000) // Default value
            .with_env_overrides() // Should ignore invalid env var
            .build()
            .unwrap();

        assert_eq!(config.timeout_ms, Some(15000)); // Should keep original value

        // Clean up
        env::remove_var("CODEFLOW_BUDDY_TIMEOUT");
    }

    #[test]
    fn test_config_builder_validation_errors() {
        // Test that builder validates on build
        let result = ConfigBuilder::new()
            .with_url("invalid-url".to_string())
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid URL format"));
    }

    #[test]
    fn test_config_builder_chaining() {
        // Test that all methods return self for chaining
        let config = ConfigBuilder::new()
            .with_url("ws://chain:9000".to_string())
            .with_token("chain-token".to_string())
            .with_timeout_ms(20000)
            .with_overrides(
                Some("ws://override:9001".to_string()),
                None
            )
            .build()
            .unwrap();

        assert_eq!(config.url, Some("ws://override:9001".to_string()));
        assert_eq!(config.token, Some("chain-token".to_string()));
        assert_eq!(config.timeout_ms, Some(20000));
    }

    #[test]
    fn test_config_missing_items_and_suggestions() {
        let config = ClientConfig::new();

        let missing = config.missing_items();
        assert_eq!(missing, vec!["Server URL"]);

        let suggestions = config.suggest_actions();
        assert_eq!(suggestions.len(), 2); // URL and token suggestions
        assert!(suggestions[0].contains("Server URL"));
        assert!(suggestions[1].contains("authentication token"));
    }

    #[test]
    fn test_config_complete_check() {
        let mut config = ClientConfig::new();
        assert!(!config.is_complete());

        config.set_url("ws://complete:3000".to_string());
        assert!(config.is_complete());
    }

    #[tokio::test]
    async fn test_config_methods_delegated_to_builder() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("delegate_test.json");

        // Create a config file
        let file_config = ClientConfig {
            url: Some("ws://delegate:3000".to_string()),
            token: None,
            timeout_ms: Some(30000),
        };
        file_config.save_to_path(&config_path).await.unwrap();

        // Test that load() uses the builder internally
        let config = ClientConfig::load_from_path(&config_path).await.unwrap();
        assert_eq!(config.url, Some("ws://delegate:3000".to_string()));
    }
}