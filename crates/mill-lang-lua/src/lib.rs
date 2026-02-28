//! Lua Language Plugin
//!
//! Provides support for Lua source files (.lua).
//! Extracts functions, variables, and tracks `require` references.

use async_trait::async_trait;
use mill_plugin_api::mill_plugin;
use mill_plugin_api::{
    import_support::{ImportMoveSupport, ImportRenameSupport},
    LanguageMetadata, LanguagePlugin, ManifestData, ParsedSource, PluginCapabilities,
    PluginResult, SourceLocation, Symbol, SymbolKind,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::json;
use std::path::Path;

mod import_support_impl;

use import_support_impl::LuaImportSupport;

// Self-register the plugin with the TypeMill system.
mill_plugin! {
    name: "lua",
    extensions: ["lua"],
    manifest: "",
    capabilities: LuaPlugin::CAPABILITIES,
    factory: LuaPlugin::boxed,
    lsp: None
}

/// Lua language plugin
///
/// Extracts symbols from Lua source files:
/// - Functions: `function foo()`, `local function foo()`, `M.foo = function()`
/// - Variables: `local foo = ...`, `M.foo = ...`
///
/// Tracks requires:
/// - `require("module")`, `require "module"`
pub struct LuaPlugin {
    metadata: LanguageMetadata,
    import_support: LuaImportSupport,
}

static FUNCTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\s*)(?:local\s+)?function\s+([\w.:]+)\s*\(").unwrap()
});
static TABLE_FUNCTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\s*)([\w.]+)\s*=\s*function\s*\(").unwrap()
});
static LOCAL_VAR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\s*)local\s+(\w+)\s*=\s*").unwrap()
});
static REQUIRE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"require\s*[\("]\s*["']([^"']+)["']"#).unwrap()
});

impl LuaPlugin {
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none().with_imports();

    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "lua",
                extensions: &["lua"],
                manifest_filename: "",
                source_dir: ".",
                entry_point: "init.lua",
                module_separator: ".",
            },
            import_support: LuaImportSupport::new(),
        }
    }

    pub fn boxed() -> Box<dyn LanguagePlugin> {
        Box::new(Self::new())
    }
}

impl Default for LuaPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for LuaPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let mut symbols = Vec::new();
        let mut requires = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }

            // Named functions: function foo() or local function foo()
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
                    // Don't continue — line may also have require
                }
            }
            // Table function assignments: M.foo = function()
            else if let Some(caps) = TABLE_FUNCTION_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    symbols.push(Symbol {
                        name: name.as_str().to_string(),
                        kind: SymbolKind::Function,
                        location: SourceLocation {
                            line: line_idx + 1,
                            column: name.start(),
                        },
                        end_location: None,
                        documentation: Some("table function".to_string()),
                    });
                }
            }
            // Local variables (not functions)
            else if let Some(caps) = LOCAL_VAR_PATTERN.captures(line) {
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
                }
            }

            // Requires
            for caps in REQUIRE_PATTERN.captures_iter(line) {
                if let Some(module) = caps.get(1) {
                    requires.push(module.as_str().to_string());
                }
            }
        }

        Ok(ParsedSource {
            data: json!({
                "language": "lua",
                "requires": requires,
            }),
            symbols,
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        Err(mill_plugin_api::PluginApiError::not_supported(
            "Lua manifest analysis not yet implemented",
        ))
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    fn import_rename_support(&self) -> Option<&dyn ImportRenameSupport> {
        Some(&self.import_support)
    }

    fn import_move_support(&self) -> Option<&dyn ImportMoveSupport> {
        Some(&self.import_support)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn rewrite_file_references(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        _current_file: &Path,
        project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        let old_rel = old_path.strip_prefix(project_root).unwrap_or(old_path);
        let new_rel = new_path.strip_prefix(project_root).unwrap_or(new_path);

        let (result, count) = self
            .import_support
            .rewrite_imports_for_move(content, old_rel, new_rel);

        if count > 0 {
            Some((result, count))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lua_plugin_basic() {
        let plugin = LuaPlugin::new();
        assert_eq!(plugin.metadata().name, "lua");
        assert_eq!(plugin.metadata().extensions, &["lua"]);
        assert!(plugin.handles_extension("lua"));
        assert!(!plugin.handles_extension("rs"));
    }

    #[tokio::test]
    async fn test_parse_functions() {
        let plugin = LuaPlugin::new();
        let source = r#"
function greet(name)
    print("Hello, " .. name)
end

local function helper()
    return 42
end

function M:method(x)
    self.x = x
end
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"greet"));
        assert!(names.contains(&"helper"));
        assert!(names.contains(&"M:method"));
    }

    #[tokio::test]
    async fn test_parse_table_functions() {
        let plugin = LuaPlugin::new();
        let source = r#"
local M = {}

M.init = function(config)
    M.config = config
end

M.process = function(data)
    return data
end
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"M.init"));
        assert!(names.contains(&"M.process"));
    }

    #[tokio::test]
    async fn test_parse_variables() {
        let plugin = LuaPlugin::new();
        let source = r#"
local config = {}
local count = 0
local name = "test"
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Variable)
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"config"));
        assert!(names.contains(&"count"));
        assert!(names.contains(&"name"));
    }

    #[tokio::test]
    async fn test_parse_requires() {
        let plugin = LuaPlugin::new();
        let source = r#"
local json = require("cjson")
local utils = require("lib.utils")
local lfs = require 'lfs'
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let requires = parsed.data["requires"].as_array().unwrap();
        assert!(requires.iter().any(|v| v == "cjson"));
        assert!(requires.iter().any(|v| v == "lib.utils"));
    }

    #[tokio::test]
    async fn test_capabilities() {
        let plugin = LuaPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.imports);
        assert!(!caps.workspace);
    }

    #[test]
    fn test_rewrite_file_references() {
        let plugin = LuaPlugin::new();
        let content = r#"local utils = require("lib.utils")
local json = require("cjson")
"#;

        let result = plugin.rewrite_file_references(
            content,
            Path::new("lib/utils.lua"),
            Path::new("lib/helpers.lua"),
            Path::new("main.lua"),
            Path::new("."),
            None,
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);
        assert!(new_content.contains(r#"require("lib.helpers")"#));
        assert!(new_content.contains(r#"require("cjson")"#));
    }
}
