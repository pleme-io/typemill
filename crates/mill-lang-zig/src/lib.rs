//! Zig Language Plugin
//!
//! Provides support for Zig source files (.zig).
//! Extracts functions, types, constants, and tracks `@import` references.

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

use import_support_impl::ZigImportSupport;

// Self-register the plugin with the TypeMill system.
mill_plugin! {
    name: "zig",
    extensions: ["zig"],
    manifest: "build.zig.zon",
    capabilities: ZigPlugin::CAPABILITIES,
    factory: ZigPlugin::boxed,
    lsp: None
}

/// Zig language plugin
///
/// Extracts symbols from Zig source files:
/// - Functions: `pub fn foo()`, `fn foo()`, `export fn foo()`
/// - Types: `const Foo = struct {`, `const Bar = enum {`, `const Baz = union {`
/// - Constants: `const foo = ...`, `pub const foo = ...`
///
/// Tracks imports:
/// - `@import("std")`, `@import("file.zig")`
pub struct ZigPlugin {
    metadata: LanguageMetadata,
    import_support: ZigImportSupport,
}

static FUNCTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\s*)(?:pub\s+|export\s+)?fn\s+(\w+)\s*\(").unwrap()
});
static TYPE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^(?:\s*)(?:pub\s+)?const\s+(\w+)\s*=\s*(?:struct|enum|union|opaque)"#).unwrap()
});
static CONST_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\s*)(?:pub\s+)?const\s+(\w+)\s*(?::\s*\w+)?\s*=\s*").unwrap()
});
static IMPORT_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"@import\("([^"]+)"\)"#).unwrap());

impl ZigPlugin {
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none().with_imports();

    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "zig",
                extensions: &["zig"],
                manifest_filename: "build.zig.zon",
                source_dir: "src",
                entry_point: "main.zig",
                module_separator: "/",
            },
            import_support: ZigImportSupport::new(),
        }
    }

    pub fn boxed() -> Box<dyn LanguagePlugin> {
        Box::new(Self::new())
    }
}

impl Default for ZigPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for ZigPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with("//") {
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
                    // Don't continue — line may also have @import
                }
            }

            // Types (struct, enum, union, opaque)
            if let Some(caps) = TYPE_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    symbols.push(Symbol {
                        name: name.as_str().to_string(),
                        kind: SymbolKind::Class,
                        location: SourceLocation {
                            line: line_idx + 1,
                            column: name.start(),
                        },
                        end_location: None,
                        documentation: None,
                    });
                }
            } else if let Some(caps) = CONST_PATTERN.captures(line) {
                // Constants (only if not a type def)
                if let Some(name) = caps.get(1) {
                    symbols.push(Symbol {
                        name: name.as_str().to_string(),
                        kind: SymbolKind::Constant,
                        location: SourceLocation {
                            line: line_idx + 1,
                            column: name.start(),
                        },
                        end_location: None,
                        documentation: None,
                    });
                }
            }

            // Imports
            for caps in IMPORT_PATTERN.captures_iter(line) {
                if let Some(path) = caps.get(1) {
                    imports.push(path.as_str().to_string());
                }
            }
        }

        Ok(ParsedSource {
            data: json!({
                "language": "zig",
                "imports": imports,
            }),
            symbols,
        })
    }

    async fn analyze_manifest(&self, _path: &Path) -> PluginResult<ManifestData> {
        Err(mill_plugin_api::PluginApiError::not_supported(
            "Zig manifest analysis not yet implemented",
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
    async fn test_zig_plugin_basic() {
        let plugin = ZigPlugin::new();
        assert_eq!(plugin.metadata().name, "zig");
        assert_eq!(plugin.metadata().extensions, &["zig"]);
        assert!(plugin.handles_extension("zig"));
        assert!(!plugin.handles_extension("rs"));
    }

    #[tokio::test]
    async fn test_parse_functions() {
        let plugin = ZigPlugin::new();
        let source = r#"
const std = @import("std");

pub fn main() !void {
    std.debug.print("hello\n", .{});
}

fn helper(x: u32) u32 {
    return x + 1;
}

export fn exported() void {}
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"helper"));
        assert!(names.contains(&"exported"));
    }

    #[tokio::test]
    async fn test_parse_types() {
        let plugin = ZigPlugin::new();
        let source = r#"
const Point = struct {
    x: f64,
    y: f64,
};

pub const Color = enum {
    red,
    green,
    blue,
};

const Tagged = union(enum) {
    int: i32,
    float: f64,
};
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"Point"));
        assert!(names.contains(&"Color"));
        assert!(names.contains(&"Tagged"));
    }

    #[tokio::test]
    async fn test_parse_constants() {
        let plugin = ZigPlugin::new();
        let source = r#"
const MAX_SIZE: usize = 1024;
pub const VERSION = 3;
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let names: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Constant)
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"MAX_SIZE"));
        assert!(names.contains(&"VERSION"));
    }

    #[tokio::test]
    async fn test_parse_imports() {
        let plugin = ZigPlugin::new();
        let source = r#"
const std = @import("std");
const utils = @import("utils.zig");
const config = @import("config/main.zig");
"#;

        let parsed = plugin.parse(source).await.unwrap();
        let imports = parsed.data["imports"].as_array().unwrap();
        assert_eq!(imports.len(), 3);
        assert!(imports.iter().any(|v| v == "std"));
        assert!(imports.iter().any(|v| v == "utils.zig"));
        assert!(imports.iter().any(|v| v == "config/main.zig"));
    }

    #[tokio::test]
    async fn test_capabilities() {
        let plugin = ZigPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.imports);
        assert!(!caps.workspace);
    }

    #[test]
    fn test_rewrite_file_references() {
        let plugin = ZigPlugin::new();
        let content = r#"const utils = @import("utils.zig");
const std = @import("std");
"#;

        let result = plugin.rewrite_file_references(
            content,
            Path::new("utils.zig"),
            Path::new("helpers.zig"),
            Path::new("main.zig"),
            Path::new("."),
            None,
        );

        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);
        assert!(new_content.contains(r#"@import("helpers.zig")"#));
        assert!(new_content.contains(r#"@import("std")"#));
    }
}
