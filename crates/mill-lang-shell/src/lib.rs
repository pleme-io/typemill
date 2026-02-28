//! Shell Language Plugin
//!
//! Provides support for shell scripts (.sh, .bash, .zsh).
//! Extracts functions, variables, aliases, and tracks source/dot references.

use async_trait::async_trait;
use mill_plugin_api::mill_plugin;
use mill_plugin_api::{
    LanguageMetadata, LanguagePlugin, ManifestData, ParsedSource, PluginCapabilities,
    PluginResult, SourceLocation, Symbol, SymbolKind,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::json;
use std::path::Path;

// Self-register the plugin with the TypeMill system.
mill_plugin! {
    name: "shell",
    extensions: ["sh", "bash", "zsh"],
    manifest: "",
    capabilities: ShellPlugin::CAPABILITIES,
    factory: ShellPlugin::boxed,
    lsp: None
}

/// Shell language plugin
///
/// Extracts symbols from shell scripts:
/// - Functions: `function foo()`, `foo()`, `foo () {`
/// - Variables: `FOO=bar`, `export FOO=bar`, `local foo=bar`
/// - Aliases: `alias foo=bar`
///
/// Tracks source references:
/// - `source file`, `. file`
pub struct ShellPlugin {
    metadata: LanguageMetadata,
}

static FUNCTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\s*)(?:function\s+)?(\w[\w-]*)\s*\(\s*\)").unwrap()
});
static VARIABLE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\s*)(?:export\s+|local\s+|declare\s+(?:-\w+\s+)*)?(\w+)=").unwrap()
});
static ALIAS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?:\s*)alias\s+([\w-]+)=").unwrap());
static SOURCE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^(?:\s*)(?:source|\.)[ \t]+["']?([^\s"']+)["']?"#).unwrap()
});

impl ShellPlugin {
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none();

    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "shell",
                extensions: &["sh", "bash", "zsh"],
                manifest_filename: "",
                source_dir: ".",
                entry_point: "",
                module_separator: "/",
            },
        }
    }

    pub fn boxed() -> Box<dyn LanguagePlugin> {
        Box::new(Self::new())
    }
}

impl Default for ShellPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for ShellPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let mut symbols = Vec::new();
        let mut sources = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Functions
            if let Some(caps) = FUNCTION_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    symbols.push(Symbol {
                        name: name.as_str().to_string(),
                        kind: SymbolKind::Function,
                        location: SourceLocation {
                            line: line_idx + 1,
                            column: name.start(),
                        },
                        end_location: None,
                        documentation: None,
                    });
                    continue;
                }
            }

            // Aliases
            if let Some(caps) = ALIAS_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    symbols.push(Symbol {
                        name: name.as_str().to_string(),
                        kind: SymbolKind::Variable,
                        location: SourceLocation {
                            line: line_idx + 1,
                            column: name.start(),
                        },
                        end_location: None,
                        documentation: Some("alias".to_string()),
                    });
                    continue;
                }
            }

            // Variables
            if let Some(caps) = VARIABLE_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    symbols.push(Symbol {
                        name: name.as_str().to_string(),
                        kind: SymbolKind::Variable,
                        location: SourceLocation {
                            line: line_idx + 1,
                            column: name.start(),
                        },
                        end_location: None,
                        documentation: None,
                    });
                    continue;
                }
            }

            // Source references
            if let Some(caps) = SOURCE_PATTERN.captures(line) {
                if let Some(path) = caps.get(1) {
                    sources.push(path.as_str().to_string());
                }
            }
        }

        Ok(ParsedSource {
            data: json!({
                "language": "shell",
                "sources": sources,
            }),
            symbols,
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        Err(mill_plugin_api::PluginApiError::not_supported(
            "Shell scripts have no manifest",
        ))
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_plugin_basic() {
        let plugin = ShellPlugin::new();
        assert_eq!(plugin.metadata().name, "shell");
        assert_eq!(plugin.metadata().extensions, &["sh", "bash", "zsh"]);
        assert!(plugin.handles_extension("sh"));
        assert!(plugin.handles_extension("bash"));
        assert!(plugin.handles_extension("zsh"));
        assert!(!plugin.handles_extension("rs"));
    }

    #[tokio::test]
    async fn test_parse_functions() {
        let plugin = ShellPlugin::new();
        let source = r#"#!/bin/bash

function setup() {
    echo "setup"
}

cleanup() {
    echo "cleanup"
}

build_all () {
    echo "build"
}
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"setup"), "Should find setup function");
        assert!(names.contains(&"cleanup"), "Should find cleanup function");
        assert!(names.contains(&"build_all"), "Should find build_all function");
    }

    #[tokio::test]
    async fn test_parse_variables() {
        let plugin = ShellPlugin::new();
        let source = r#"
FOO=bar
export PATH="/usr/bin:$PATH"
local count=0
declare -r CONST=42
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"FOO"));
        assert!(names.contains(&"PATH"));
        assert!(names.contains(&"count"));
        assert!(names.contains(&"CONST"));
    }

    #[tokio::test]
    async fn test_parse_aliases() {
        let plugin = ShellPlugin::new();
        let source = r#"
alias ll='ls -la'
alias gs='git status'
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"ll"));
        assert!(names.contains(&"gs"));
    }

    #[tokio::test]
    async fn test_parse_sources() {
        let plugin = ShellPlugin::new();
        let source = r#"
source ./utils.sh
. /etc/profile
source "$HOME/.bashrc"
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let sources = parsed.data["sources"].as_array().unwrap();
        assert_eq!(sources.len(), 3);
    }

    #[tokio::test]
    async fn test_capabilities() {
        let plugin = ShellPlugin::new();
        let caps = plugin.capabilities();
        assert!(!caps.imports);
        assert!(!caps.workspace);
    }
}
