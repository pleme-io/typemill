//! Configuration management for Codeflow Buddy

mod json_helper;

use crate::error::{CoreError, CoreResult};
use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use json_helper::to_camel_case_keys;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// LSP configuration
    pub lsp: LspConfig,
    /// FUSE configuration (optional)
    pub fuse: Option<FuseConfig>,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Cache configuration
    pub cache: CacheConfig,
}

/// Server-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to bind to
    pub port: u16,
    /// Maximum number of concurrent clients
    pub max_clients: Option<usize>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Enable TLS
    pub tls: Option<TlsConfig>,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert_path: PathBuf,
    /// Path to private key file
    pub key_path: PathBuf,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    /// JWT secret for signing tokens
    pub jwt_secret: String,
    /// JWT expiry in seconds
    pub jwt_expiry_seconds: u64,
    /// JWT issuer
    pub jwt_issuer: String,
    /// JWT audience
    pub jwt_audience: String,
}

/// LSP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspConfig {
    /// List of LSP server configurations
    pub servers: Vec<LspServerConfig>,
    /// Default timeout for LSP requests in milliseconds
    pub default_timeout_ms: u64,
    /// Enable LSP server preloading
    pub enable_preload: bool,
}

/// Individual LSP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspServerConfig {
    /// File extensions this server handles
    pub extensions: Vec<String>,
    /// Command to run the LSP server
    pub command: Vec<String>,
    /// Working directory (optional)
    pub root_dir: Option<PathBuf>,
    /// Auto-restart interval in minutes (optional)
    pub restart_interval: Option<u64>,
}

/// FUSE filesystem configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuseConfig {
    /// Mount point for the FUSE filesystem
    pub mount_point: PathBuf,
    /// Enable read-only mode
    pub read_only: bool,
    /// Cache timeout in seconds
    pub cache_timeout_seconds: u64,
    /// Maximum file size to cache in bytes
    pub max_file_size_bytes: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Output format (json, pretty)
    pub format: String,
    /// Enable file logging
    pub file: Option<FileLoggingConfig>,
}

/// File logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLoggingConfig {
    /// Path to log file
    pub path: PathBuf,
    /// Maximum log file size in bytes
    pub max_size_bytes: u64,
    /// Number of log files to retain
    pub max_files: usize,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,
    /// Cache size limit in bytes
    pub max_size_bytes: u64,
    /// Cache entry TTL in seconds
    pub ttl_seconds: u64,
    /// Enable persistent cache
    pub persistent: bool,
    /// Cache directory (for persistent cache)
    pub cache_dir: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            lsp: LspConfig::default(),
            fuse: None,
            logging: LoggingConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3040,
            max_clients: Some(10),
            timeout_ms: 30000,
            tls: None,
            auth: None,
        }
    }
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            servers: vec![
                LspServerConfig {
                    extensions: vec!["ts".to_string(), "tsx".to_string(), "js".to_string(), "jsx".to_string()],
                    command: vec!["typescript-language-server".to_string(), "--stdio".to_string()],
                    root_dir: None,
                    restart_interval: Some(10),
                },
                LspServerConfig {
                    extensions: vec!["py".to_string()],
                    command: vec!["pylsp".to_string()],
                    root_dir: None,
                    restart_interval: Some(5),
                },
                LspServerConfig {
                    extensions: vec!["go".to_string()],
                    command: vec!["gopls".to_string()],
                    root_dir: None,
                    restart_interval: Some(10),
                },
            ],
            default_timeout_ms: 5000,
            enable_preload: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file: None,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_bytes: 256 * 1024 * 1024, // 256 MB
            ttl_seconds: 3600, // 1 hour
            persistent: false,
            cache_dir: None,
        }
    }
}

impl AppConfig {
    /// Load configuration from environment and config files
    pub fn load() -> CoreResult<Self> {
        // Start with default configuration
        let mut app_config = AppConfig::default();

        // Load from configuration file if it exists
        let config_paths = [
            ".codebuddy/config.json",
            ".codebuddy/config.toml",
            "codebuddy.json", // Legacy support
            "codebuddy.toml", // Legacy support
        ];

        // Try direct JSON loading first (preserves camelCase)
        for config_path in &config_paths {
            if config_path.ends_with(".json") {
                let path = std::path::Path::new(config_path);
                if path.exists() {
                    let content = std::fs::read_to_string(path)?;
                    match serde_json::from_str::<AppConfig>(&content) {
                        Ok(loaded) => {
                            app_config = loaded;

                            // Apply environment overrides
                            Self::apply_env_overrides(&mut app_config);

                            // Validate configuration
                            app_config.validate()?;

                            return Ok(app_config);
                        }
                        Err(_) => {
                            // Continue to try other files or methods
                        }
                    }
                }
            }
        }

        // Fallback to config crate for TOML
        let mut config_builder = Config::builder();
        let mut file_found = false;

        for config_path in &config_paths {
            if config_path.ends_with(".toml") {
                let path = std::path::Path::new(config_path);
                if path.exists() {
                    config_builder = config_builder.add_source(File::from(path).format(FileFormat::Toml));
                    file_found = true;
                    break;
                }
            }
        }

        // If a config file was found, merge it with defaults
        if file_found {
            // Override with environment variables
            config_builder = config_builder.add_source(
                Environment::with_prefix("CODEFLOW_BUDDY")
                    .separator("__")
                    .try_parsing(true),
            );

            let config = config_builder.build()?;

            // Merge file/env config into our default config
            // The config crate lowercases all keys, so we need to convert them back to camelCase
            if let Ok(server_value) = config.get::<serde_json::Value>("server") {
                let camel_value = to_camel_case_keys(server_value);
                if let Ok(server_config) = serde_json::from_value::<ServerConfig>(camel_value) {
                    app_config.server = server_config;
                }
            }
            if let Ok(lsp_value) = config.get::<serde_json::Value>("lsp") {
                let camel_value = to_camel_case_keys(lsp_value);
                if let Ok(lsp_config) = serde_json::from_value::<LspConfig>(camel_value) {
                    app_config.lsp = lsp_config;
                }
            }
            if let Ok(fuse_value) = config.get::<serde_json::Value>("fuse") {
                let camel_value = to_camel_case_keys(fuse_value);
                if let Ok(fuse_config) = serde_json::from_value::<FuseConfig>(camel_value) {
                    app_config.fuse = Some(fuse_config);
                }
            }
            if let Ok(logging_value) = config.get::<serde_json::Value>("logging") {
                let camel_value = to_camel_case_keys(logging_value);
                if let Ok(logging_config) = serde_json::from_value::<LoggingConfig>(camel_value) {
                    app_config.logging = logging_config;
                }
            }
            if let Ok(cache_value) = config.get::<serde_json::Value>("cache") {
                let camel_value = to_camel_case_keys(cache_value);
                if let Ok(cache_config) = serde_json::from_value::<CacheConfig>(camel_value) {
                    app_config.cache = cache_config;
                }
            }
        } else {
            // No file found, just use defaults with environment overrides
            Self::apply_env_overrides(&mut app_config);
        }

        // Validate configuration
        app_config.validate()?;

        Ok(app_config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(config: &mut Self) {
        use std::env;

        // Check for common environment overrides
        if let Ok(port) = env::var("CODEFLOW_BUDDY__SERVER__PORT") {
            if let Ok(port_value) = port.parse::<u16>() {
                config.server.port = port_value;
            }
        }

        if let Ok(host) = env::var("CODEFLOW_BUDDY__SERVER__HOST") {
            config.server.host = host;
        }

        if let Ok(level) = env::var("CODEFLOW_BUDDY__LOGGING__LEVEL") {
            config.logging.level = level;
        }

        if let Ok(enabled) = env::var("CODEFLOW_BUDDY__CACHE__ENABLED") {
            if let Ok(enabled_value) = enabled.parse::<bool>() {
                config.cache.enabled = enabled_value;
            }
        }
    }

    /// Validate the configuration
    fn validate(&self) -> CoreResult<()> {
        // Validate server config
        if self.server.port == 0 {
            return Err(CoreError::config("Server port cannot be 0"));
        }

        if self.server.timeout_ms == 0 {
            return Err(CoreError::config("Server timeout cannot be 0"));
        }

        // Validate LSP config
        if self.lsp.servers.is_empty() {
            return Err(CoreError::config("At least one LSP server must be configured"));
        }

        for server in &self.lsp.servers {
            if server.extensions.is_empty() {
                return Err(CoreError::config("LSP server must handle at least one extension"));
            }
            if server.command.is_empty() {
                return Err(CoreError::config("LSP server command cannot be empty"));
            }
        }

        // Validate logging config
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(CoreError::config(format!(
                "Invalid log level '{}', must be one of: {}",
                self.logging.level,
                valid_levels.join(", ")
            )));
        }

        let valid_formats = ["json", "pretty"];
        if !valid_formats.contains(&self.logging.format.as_str()) {
            return Err(CoreError::config(format!(
                "Invalid log format '{}', must be one of: {}",
                self.logging.format,
                valid_formats.join(", ")
            )));
        }

        // Validate cache config
        if self.cache.enabled && self.cache.max_size_bytes == 0 {
            return Err(CoreError::config("Cache max size cannot be 0 when cache is enabled"));
        }

        Ok(())
    }
}