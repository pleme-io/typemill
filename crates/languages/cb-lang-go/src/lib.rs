//! Go Language Plugin for Codebuddy
//!
//! This crate provides complete Go language support, implementing the
//! `LanguageIntelligencePlugin` trait from `cb-plugin-api`.
//!
//! # Features
//!
//! - Full AST-based import parsing using Go's native parser
//! - Fallback regex-based parsing when `go` command is unavailable
//! - Support for all Go import styles (single, grouped, aliased, dot, blank)
//! - go.mod manifest detection
//!
//! # Example
//!
//! ```rust
//! use cb_lang_go::GoPlugin;
//! use cb_plugin_api::LanguageIntelligencePlugin;
//!
//! let plugin = GoPlugin::new();
//! let imports = plugin.analyze_imports(source).await.unwrap();
//! ```

mod parser;

use async_trait::async_trait;
use cb_plugin_api::{
    LanguageIntelligencePlugin, ManifestData, ParsedSource, PluginResult, PluginError,
};
use std::path::Path;

/// Go language plugin implementation.
pub struct GoPlugin;

impl GoPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageIntelligencePlugin for GoPlugin {
    fn name(&self) -> &'static str {
        "Go"
    }

    fn file_extensions(&self) -> Vec<&'static str> {
        vec!["go"]
    }

    async fn parse(&self, _source: &str) -> PluginResult<ParsedSource> {
        // This can be expanded later to extract symbols like functions and structs.
        Ok(ParsedSource {
            data: serde_json::json!({ "status": "Go symbol parsing not yet implemented" }),
            symbols: Vec::new(),
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        Err(PluginError::not_supported("go.mod analysis not yet implemented."))
    }

    fn handles_manifest(&self, filename: &str) -> bool {
        filename == "go.mod"
    }
}
