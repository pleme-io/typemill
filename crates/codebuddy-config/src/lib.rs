//! Configuration management for Codeflow Buddy
//!
//! This crate provides configuration types and loading logic for both
//! application settings and refactoring presets.

pub mod config;
pub mod refactor_config;

// Re-export commonly used types at the crate root for convenience
pub use config::{
    AppConfig, AuthConfig, CacheConfig, ExternalMcpConfig, ExternalMcpServerConfig,
    ExternalPluginConfig, FileLoggingConfig, FuseConfig, GitConfig, LanguagePluginsConfig,
    LogFormat, LoggingConfig, LspConfig, LspServerConfig, PluginSelectionConfig, ServerConfig,
    TlsConfig, ValidationConfig, ValidationFailureAction,
};
pub use refactor_config::{RefactorConfig, RefactorDefaults, RefactorPreset};
