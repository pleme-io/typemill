//! Plugin scaffolding and code generation
//!
//! Provides programmatic generation of plugin boilerplate code.
//! This is the Rust equivalent of the `new-lang.sh` shell script.

/// Configuration for generating a new language plugin
#[derive(Debug, Clone)]
pub struct PluginScaffold {
    /// Language name (e.g., "CSharp", "Ruby")
    pub language: String,

    /// File extensions (e.g., ["cs", "csx"])
    pub extensions: Vec<String>,

    /// Manifest filename (e.g., "*.csproj", "Gemfile")
    pub manifest_file: String,

    /// Source directory (e.g., "src", "lib")
    pub source_dir: String,

    /// Entry point file (e.g., "main.cs", "index.ts")
    pub entry_point: String,

    /// Module separator (e.g., ".", "::")
    pub module_separator: String,

    /// Whether the plugin has import support
    pub has_import_support: bool,

    /// Whether the plugin has workspace support
    pub has_workspace_support: bool,
}

impl PluginScaffold {
    /// Create a new scaffold configuration
    pub fn new(language: &str) -> Self {
        let lang_lower = language.to_lowercase();
        let first_ext = lang_lower.as_str();

        Self {
            language: language.to_string(),
            extensions: vec![first_ext.to_string()],
            manifest_file: "manifest.toml".to_string(),
            source_dir: "src".to_string(),
            entry_point: format!("main.{}", first_ext),
            module_separator: ".".to_string(),
            has_import_support: false,
            has_workspace_support: false,
        }
    }

    /// Set file extensions
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Set manifest filename
    pub fn with_manifest(mut self, manifest: &str) -> Self {
        self.manifest_file = manifest.to_string();
        self
    }

    /// Set source directory
    pub fn with_source_dir(mut self, dir: &str) -> Self {
        self.source_dir = dir.to_string();
        self
    }

    /// Set entry point
    pub fn with_entry_point(mut self, entry: &str) -> Self {
        self.entry_point = entry.to_string();
        self
    }

    /// Set module separator
    pub fn with_module_separator(mut self, sep: &str) -> Self {
        self.module_separator = sep.to_string();
        self
    }

    /// Enable import support
    pub fn with_import_support(mut self, enabled: bool) -> Self {
        self.has_import_support = enabled;
        self
    }

    /// Enable workspace support
    pub fn with_workspace_support(mut self, enabled: bool) -> Self {
        self.has_workspace_support = enabled;
        self
    }

    /// Generate lib.rs content
    pub fn generate_lib_rs(&self) -> String {
        let lang = &self.language;
        let lang_upper = lang.to_uppercase();
        let imports_enabled = if self.has_import_support {
            "true"
        } else {
            "false"
        };
        let workspace_enabled = if self.has_workspace_support {
            "true"
        } else {
            "false"
        };

        format!(
            r#"//! {lang} language plugin for TypeMill
//!
//! Provides AST parsing, symbol extraction, and manifest analysis for {lang}.

mod parser;
mod manifest;
pub mod import_support;
pub mod workspace_support;

use mill_plugin_api::{{ LanguagePlugin , LanguageMetadata , LanguageCapabilities , ManifestData , ParsedSource , PluginResult , }};
use async_trait::async_trait;
use std::path::Path;

/// {lang} language plugin implementation
pub struct {lang}Plugin {{
    metadata: LanguageMetadata,
}}

impl {lang}Plugin {{
    /// Create a new {lang} plugin instance
    pub fn new() -> Self {{
        Self {{
            metadata: LanguageMetadata::{lang_upper},
        }}
    }}
}}

impl Default for {lang}Plugin {{
    fn default() -> Self {{
        Self::new()
    }}
}}

#[async_trait]
impl LanguagePlugin for {lang}Plugin {{
    fn metadata(&self) -> &LanguageMetadata {{
        &self.metadata
    }}

    fn capabilities(&self) -> LanguageCapabilities {{
        LanguageCapabilities {{
            imports: {imports_enabled},
            workspace: {workspace_enabled},
        }}
    }}

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {{
        parser::parse_source(source)
    }}

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {{
        manifest::analyze_manifest(path).await
    }}

    fn as_any(&self) -> &dyn std::any::Any {{
        self
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_plugin_creation() {{
        let plugin = {lang}Plugin::new();
        assert_eq!(plugin.metadata().name, "{lang}");
    }}
}}
"#
        )
    }

    /// Generate parser.rs content
    pub fn generate_parser_rs(&self) -> String {
        let lang = &self.language;

        format!(
            r#"//! {lang} source code parsing and symbol extraction

use mill_plugin_api::{{ ParsedSource , PluginResult }};

/// Parse {lang} source code and extract symbols
///
/// TODO: Implement actual parsing logic
pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {{
    tracing::warn!(
        source_length = source.len(),
        "{lang} parsing not yet implemented - returning empty symbols"
    );

    Ok(ParsedSource {{
        data: serde_json::json!({{
            "language": "{lang}",
            "source_length": source.len(),
        }}),
        symbols: vec![],
    }})
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_parse_source() {{
        let result = parse_source("// test source");
        assert!(result.is_ok());
    }}
}}
"#
        )
    }

    /// Generate manifest.rs content
    pub fn generate_manifest_rs(&self) -> String {
        let lang = &self.language;
        let manifest_file = &self.manifest_file;

        format!(
            r#"//! {lang} manifest file parsing
//!
//! Handles {manifest_file} files for {lang} projects.

use mill_plugin_api::{{ ManifestData , PluginError , PluginResult }};
use std::path::Path;

/// Analyze {lang} manifest file
///
/// TODO: Implement actual manifest parsing logic
pub async fn analyze_manifest(path: &Path) -> PluginResult<ManifestData> {{
    tracing::warn!(
        manifest_path = %path.display(),
        "{lang} manifest parsing not yet implemented"
    );

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| PluginError::manifest(format!("Failed to read manifest: {{}}", e)))?;

    Ok(ManifestData {{
        name: path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        version: "0.0.0".to_string(),
        dependencies: vec![],
        dev_dependencies: vec![],
        raw_data: serde_json::json!({{
            "content_length": content.len(),
        }}),
    }})
}}

#[cfg(test)]
mod tests {{
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_analyze_manifest() {{
        use std::io::Write;
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "test").unwrap();

        let result = analyze_manifest(temp_file.path()).await;
        assert!(result.is_ok());
    }}
}}
"#
        )
    }

    /// Generate Cargo.toml content
    pub fn generate_cargo_toml(&self) -> String {
        let lang_lower = self.language.to_lowercase();
        let crate_name = format!("mill-lang-{}", lang_lower);

        format!(
            r#"[package]
name = "{crate_name}"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true

[dependencies]
# TypeMill workspace dependencies
mill-plugin-api = {{ path = "../../mill-plugin-api" }}
mill-plugin-api = {{ path = "../../mill-plugin-api" }}
mill-foundation = {{ path = "../../mill-foundation" }}
mill-lang-common = {{ path = "../mill-lang-common" }}

# Async operations
async-trait = {{ workspace = true }}
tokio = {{ workspace = true }}

# Serialization/Deserialization
serde = {{ workspace = true }}
serde_json = {{ workspace = true }}

# Error handling
thiserror = {{ workspace = true }}

# Logging
tracing = {{ workspace = true }}

# Utilities
tempfile = "3.10"
"#
        )
    }

    /// Generate README.md content
    pub fn generate_readme(&self) -> String {
        let lang = &self.language;
        let lang_lower = self.language.to_lowercase();
        let exts = self.extensions.join(", ");

        format!(
            r#"# {lang} Language Plugin

{lang} language support for TypeMill.

## Configuration

- **Extensions**: {exts}
- **Manifest**: {}
- **Source Directory**: {}
- **Entry Point**: {}
- **Module Separator**: {}

## Features

- [ ] AST parsing and symbol extraction
- [{}] Import support (ImportSupport trait)
- [{}] Workspace support (WorkspaceSupport trait)
- [ ] Manifest file parsing

## Testing

```bash
cargo test -p mill-lang-{lang_lower}
```

## Implementation Status

🚧 **Under Development** - Core features need implementation.
"#,
            self.manifest_file,
            self.source_dir,
            self.entry_point,
            self.module_separator,
            if self.has_import_support { "x" } else { " " },
            if self.has_workspace_support { "x" } else { " " },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_scaffold_creation() {
        let scaffold = PluginScaffold::new("Ruby")
            .with_extensions(vec!["rb".to_string()])
            .with_manifest("Gemfile")
            .with_module_separator("::");

        assert_eq!(scaffold.language, "Ruby");
        assert_eq!(scaffold.extensions, vec!["rb".to_string()]);
        assert_eq!(scaffold.manifest_file, "Gemfile");
        assert_eq!(scaffold.module_separator, "::");
    }

    #[test]
    fn test_generate_lib_rs() {
        let scaffold = PluginScaffold::new("CSharp");
        let lib_rs = scaffold.generate_lib_rs();

        assert!(lib_rs.contains("CSharp language plugin"));
        assert!(lib_rs.contains("pub struct CSharpPlugin"));
        assert!(lib_rs.contains("impl LanguagePlugin for CSharpPlugin"));
    }

    #[test]
    fn test_generate_parser_rs() {
        let scaffold = PluginScaffold::new("Kotlin");
        let parser_rs = scaffold.generate_parser_rs();

        assert!(parser_rs.contains("Kotlin source code parsing"));
        assert!(parser_rs.contains("pub fn parse_source"));
    }

    #[test]
    fn test_generate_manifest_rs() {
        let scaffold = PluginScaffold::new("Swift").with_manifest("Package.swift");
        let manifest_rs = scaffold.generate_manifest_rs();

        assert!(manifest_rs.contains("Package.swift"));
        assert!(manifest_rs.contains("pub async fn analyze_manifest"));
    }

    #[test]
    fn test_generate_cargo_toml() {
        let scaffold = PluginScaffold::new("Elixir");
        let cargo_toml = scaffold.generate_cargo_toml();

        assert!(cargo_toml.contains("name = \"mill-lang-elixir\""));
        assert!(cargo_toml.contains("mill-lang-common"));
    }
}
