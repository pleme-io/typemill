//! Configuration management for Codeflow Buddy

use cb_types::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Git integration configuration
    #[serde(default)]
    pub git: GitConfig,
    /// Validation configuration
    #[serde(default)]
    pub validation: ValidationConfig,
    /// Plugin selection configuration
    #[serde(default)]
    pub plugin_selection: PluginSelectionConfig,
    /// External language plugin configuration
    #[serde(default)]
    pub language_plugins: LanguagePluginsConfig,
    /// External MCP server configuration (optional)
    #[cfg(feature = "mcp-proxy")]
    pub external_mcp: Option<ExternalMcpConfig>,
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
    /// Custom initialization options to pass to the LSP server (optional)
    /// These are sent in the initialize request's initializationOptions field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialization_options: Option<serde_json::Value>,
}

/// Plugin selection configuration for multi-tiered priority system
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSelectionConfig {
    /// Plugin priority overrides (plugin_name -> priority)
    /// Higher values = higher priority. Default is 50.
    #[serde(default)]
    pub priorities: HashMap<String, u32>,

    /// Whether to error on ambiguous plugin selection
    /// If true, errors when multiple plugins have same priority
    /// If false, picks the first one (deterministic by name)
    #[serde(default = "default_error_on_ambiguity")]
    pub error_on_ambiguity: bool,
}

fn default_error_on_ambiguity() -> bool {
    false
}

/// Configuration for external language plugins
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePluginsConfig {
    /// List of external language plugin configurations
    #[serde(default)]
    pub plugins: Vec<ExternalPluginConfig>,
}

/// Configuration for a single external language plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalPluginConfig {
    /// A unique name for the plugin (e.g., "rust")
    pub name: String,
    /// File extensions this plugin handles
    pub extensions: Vec<String>,
    /// Command to run the plugin executable
    pub command: Vec<String>,
    /// The manifest filename this plugin recognizes (e.g., "Cargo.toml")
    pub manifest_filename: String,
}

/// External MCP server configuration (optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMcpConfig {
    /// List of external MCP servers to proxy
    pub servers: Vec<ExternalMcpServerConfig>,
}

/// Individual external MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMcpServerConfig {
    /// MCP server name (e.g., "context7")
    pub name: String,
    /// Command to spawn the MCP server
    pub command: Vec<String>,
    /// Environment variables (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// Auto-start on codebuddy startup
    pub auto_start: bool,
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

/// Log output format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable format for development
    #[default]
    Pretty,
    /// Structured JSON format for production
    Json,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Output format
    pub format: LogFormat,
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

/// Git integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitConfig {
    /// Auto-detect and use git if available
    pub enabled: bool,
    /// Fail if git expected but unavailable
    pub require: bool,
    /// Which git commands to use for file operations
    pub operations: Vec<String>,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationConfig {
    /// Enable post-operation validation
    pub enabled: bool,
    /// Command to run for validation
    pub command: String,
    /// Action on failure
    pub on_failure: ValidationFailureAction,
}

/// Action to take when validation fails
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ValidationFailureAction {
    /// Just report the error
    #[default]
    Report,
    /// Rollback the operation using git
    Rollback,
    /// Ask the user what to do
    Interactive,
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
                    extensions: vec![
                        "ts".to_string(),
                        "tsx".to_string(),
                        "js".to_string(),
                        "jsx".to_string(),
                    ],
                    command: vec![
                        "typescript-language-server".to_string(),
                        "--stdio".to_string(),
                    ],
                    root_dir: None,
                    restart_interval: Some(10),
                    initialization_options: None,
                },
                LspServerConfig {
                    extensions: vec!["py".to_string()],
                    command: vec!["pylsp".to_string()],
                    root_dir: None,
                    restart_interval: Some(5),
                    initialization_options: None,
                },
                LspServerConfig {
                    extensions: vec!["go".to_string()],
                    command: vec!["gopls".to_string()],
                    root_dir: None,
                    restart_interval: Some(10),
                    initialization_options: None,
                },
                LspServerConfig {
                    extensions: vec!["rs".to_string()],
                    command: vec!["rust-analyzer".to_string()],
                    root_dir: None,
                    restart_interval: Some(15),
                    initialization_options: None,
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
            format: LogFormat::Pretty,
            file: None,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_bytes: 256 * 1024 * 1024, // 256 MB
            ttl_seconds: 3600,                 // 1 hour
            persistent: false,
            cache_dir: None,
        }
    }
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require: false,
            operations: vec!["mv".to_string(), "rm".to_string()],
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command: "cargo check".to_string(),
            on_failure: ValidationFailureAction::Report,
        }
    }
}

impl AppConfig {
    /// Save configuration to a specified file path
    pub fn save(&self, path: &std::path::Path) -> CoreResult<()> {
        // Ensure the parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Serialize configuration to JSON with pretty formatting
        let json_content = serde_json::to_string_pretty(self)
            .map_err(|e| CoreError::config(format!("Failed to serialize configuration: {}", e)))?;

        // Write to file
        std::fs::write(path, json_content)
            .map_err(|e| CoreError::config(format!("Failed to write configuration file: {}", e)))?;

        Ok(())
    }

    /// Load configuration from environment and config files
    ///
    /// Configuration is loaded in the following priority order (highest to lowest):
    /// 1. Environment variables (CODEBUDDY__*)
    /// 2. Environment-specific profile from codebuddy.toml (based on CODEBUDDY_ENV)
    /// 3. Base configuration from codebuddy.toml
    /// 4. Legacy JSON files (.codebuddy/config.json, etc.) for backward compatibility
    /// 5. Default values
    pub fn load() -> CoreResult<Self> {
        use figment::{
            providers::{Env, Format, Toml},
            Figment,
        };

        // Determine which environment profile to use
        let env_profile = std::env::var("CODEBUDDY_ENV").unwrap_or_else(|_| "default".to_string());

        tracing::debug!(
            profile = %env_profile,
            "Loading configuration with profile"
        );

        // Priority order: Env vars > Profile > Base config > Legacy JSON > Defaults
        // Start with full defaults by serializing the Default implementation
        let default_config = AppConfig::default();
        let default_value =
            serde_json::to_value(&default_config).expect("Failed to serialize default config");

        let figment = Figment::from(figment::providers::Serialized::defaults(default_value));

        // 2. Try to load legacy JSON files for backward compatibility
        let legacy_json_paths = [".codebuddy/config.json", "codebuddy.json"];

        let mut figment_with_legacy = figment;
        for json_path in &legacy_json_paths {
            let path = std::path::Path::new(json_path);
            if path.exists() {
                tracing::debug!(path = %json_path, "Loading legacy JSON config");
                // For JSON files, directly deserialize to preserve camelCase
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(json_config) = serde_json::from_str::<AppConfig>(&content) {
                        // Merge the JSON config
                        if let Ok(json_value) = serde_json::to_value(&json_config) {
                            figment_with_legacy = figment_with_legacy
                                .merge(figment::providers::Serialized::defaults(json_value));
                        }
                        break; // Use first found JSON file
                    }
                }
            }
        }

        // 3. Load codebuddy.toml if it exists (base configuration)
        let toml_paths = ["codebuddy.toml", ".codebuddy/config.toml"];

        let mut toml_found = false;
        for toml_path in &toml_paths {
            let path = std::path::Path::new(toml_path);
            if path.exists() {
                tracing::info!(path = %toml_path, "Loading TOML configuration");
                figment_with_legacy = figment_with_legacy.merge(Toml::file(path));
                toml_found = true;
                break; // Use first found TOML file
            }
        }

        // 4. If TOML was found and environment profile is not "default", merge environment profile
        if toml_found && env_profile != "default" {
            tracing::info!(
                profile = %env_profile,
                "Applying environment-specific profile"
            );
            // Merge environment-specific overrides from [environments.{profile}]
            figment_with_legacy =
                figment_with_legacy.select(format!("environments.{}", env_profile));
        }

        // 5. Apply environment variable overrides
        let figment_final = figment_with_legacy.merge(
            Env::prefixed("CODEBUDDY__")
                .split("__")
                .map(|k| k.as_str().replace("__", ".").to_lowercase().into()),
        );

        // Extract and deserialize configuration
        let app_config: AppConfig = figment_final
            .extract()
            .map_err(|e| CoreError::config(format!("Failed to load configuration: {}", e)))?;

        // Validate configuration
        app_config.validate()?;

        tracing::info!(
            port = app_config.server.port,
            lsp_servers = app_config.lsp.servers.len(),
            "Configuration loaded successfully"
        );

        Ok(app_config)
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
            return Err(CoreError::config(
                "At least one LSP server must be configured",
            ));
        }

        for server in &self.lsp.servers {
            if server.extensions.is_empty() {
                return Err(CoreError::config(
                    "LSP server must handle at least one extension",
                ));
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

        // LogFormat enum ensures valid format values at compile time

        // Validate cache config
        if self.cache.enabled && self.cache.max_size_bytes == 0 {
            return Err(CoreError::config(
                "Cache max size cannot be 0 when cache is enabled",
            ));
        }

        Ok(())
    }
}
