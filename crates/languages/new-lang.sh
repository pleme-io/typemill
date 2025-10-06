#!/usr/bin/env bash
set -euo pipefail

# new-lang.sh - Scaffold a new language plugin for Codebuddy
#
# Usage: ./new-lang.sh <language-name>
# Example: ./new-lang.sh java

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LANGUAGES_DIR="$SCRIPT_DIR"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <language-name>"
    echo ""
    echo "Examples:"
    echo "  $0 java"
    echo "  $0 kotlin"
    echo "  $0 ruby"
    exit 1
}

if [ $# -ne 1 ]; then
    usage
fi

LANG_NAME="$1"
LANG_LOWER=$(echo "$LANG_NAME" | tr '[:upper:]' '[:lower:]')
LANG_UPPER=$(echo "$LANG_NAME" | tr '[:lower:]' '[:upper:]')
LANG_TITLE=$(echo "$LANG_LOWER" | sed 's/.*/\u&/')

PLUGIN_NAME="cb-lang-${LANG_LOWER}"
PLUGIN_DIR="${LANGUAGES_DIR}/${PLUGIN_NAME}"

echo -e "${BLUE}Creating ${LANG_TITLE} language plugin...${NC}"

# Check if plugin already exists
if [ -d "$PLUGIN_DIR" ]; then
    echo -e "${RED}Error: Plugin directory already exists: ${PLUGIN_DIR}${NC}"
    exit 1
fi

# Create directory structure
echo -e "${GREEN}✓${NC} Creating directory structure..."
mkdir -p "$PLUGIN_DIR/src"
mkdir -p "$PLUGIN_DIR/resources"

# Create Cargo.toml
echo -e "${GREEN}✓${NC} Generating Cargo.toml..."
cat > "$PLUGIN_DIR/Cargo.toml" << EOF
[package]
name = "${PLUGIN_NAME}"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true

[dependencies]
# Codebuddy workspace dependencies
cb-plugin-api = { path = "../../cb-plugin-api" }
cb-protocol = { path = "../../cb-protocol" }
cb-core = { path = "../../cb-core" }

# Async operations
async-trait = { workspace = true }
tokio = { workspace = true }

# Serialization/Deserialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
thiserror = { workspace = true }

# Logging
tracing = { workspace = true }

# Utilities (uncomment as needed)
# regex = "1.10"
# chrono = { version = "0.4", features = ["serde"] }
tempfile = "3.10"
EOF

# Create lib.rs with skeleton implementation
echo -e "${GREEN}✓${NC} Generating src/lib.rs..."
cat > "$PLUGIN_DIR/src/lib.rs" << EOF
//! ${LANG_TITLE} language plugin for Codebuddy
//!
//! Provides AST parsing, symbol extraction, and manifest analysis for ${LANG_TITLE}.

mod parser;
mod manifest;

use cb_plugin_api::{
    LanguageIntelligencePlugin, ManifestData, ParsedSource, PluginResult,
};
use async_trait::async_trait;
use std::path::Path;

/// ${LANG_TITLE} language plugin
pub struct ${LANG_TITLE}Plugin;

impl ${LANG_TITLE}Plugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ${LANG_TITLE}Plugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageIntelligencePlugin for ${LANG_TITLE}Plugin {
    fn name(&self) -> &'static str {
        "${LANG_TITLE}"
    }

    fn file_extensions(&self) -> Vec<&'static str> {
        vec!["${LANG_LOWER}"]
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        parser::parse_source(source)
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::analyze_manifest(path).await
    }

    fn manifest_filename(&self) -> &'static str {
        // Examples: "pom.xml", "build.gradle", "Gemfile", "pyproject.toml"
        "manifest.${LANG_LOWER}"
    }

    fn source_dir(&self) -> &'static str {
        // Examples: "src" for Java/Kotlin, "" for Python/Ruby
        "src"
    }

    fn entry_point(&self) -> &'static str {
        // Examples: "Main.java", "main.kt", "__init__.py"
        "main.${LANG_LOWER}"
    }

    fn module_separator(&self) -> &'static str {
        // Examples: "." for Java/Python, "::" for Rust, "/" for Go
        "."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = ${LANG_TITLE}Plugin::new();
        assert_eq!(plugin.name(), "${LANG_TITLE}");
    }

    #[test]
    fn test_file_extensions() {
        let plugin = ${LANG_TITLE}Plugin::new();
        let extensions = plugin.file_extensions();
        assert!(!extensions.is_empty());
    }
}
EOF

# Create parser.rs
echo -e "${GREEN}✓${NC} Generating src/parser.rs..."
cat > "$PLUGIN_DIR/src/parser.rs" << EOF
//! ${LANG_TITLE} source code parsing and symbol extraction

use cb_plugin_api::{ParsedSource, PluginResult};

/// Parse ${LANG_TITLE} source code and extract symbols
pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    // A placeholder implementation that returns no symbols.
    // This should be replaced with actual parsing logic.
    tracing::warn!("${LANG_TITLE} parsing not yet implemented");

    Ok(ParsedSource {
        data: serde_json::json!({}),
        symbols: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_source() {
        let result = parse_source("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_simple_source() {
        let source = r#"
            // A simple ${LANG_TITLE} source file
        "#;
        let result = parse_source(source);
        assert!(result.is_ok());
    }
}
EOF

# Create manifest.rs
echo -e "${GREEN}✓${NC} Generating src/manifest.rs..."
cat > "$PLUGIN_DIR/src/manifest.rs" << EOF
//! ${LANG_TITLE} manifest file parsing
//!
//! Handles manifest files for ${LANG_TITLE} projects.

use cb_plugin_api::{ManifestData, PluginError, PluginResult};
use std::path::Path;

/// Analyze ${LANG_TITLE} manifest file
pub async fn analyze_manifest(path: &Path) -> PluginResult<ManifestData> {
    // A placeholder implementation for manifest parsing.
    // This should be replaced with actual logic for parsing manifest files
    // like pom.xml, build.gradle, Gemfile, etc.
    tracing::warn!(
        manifest_path = %path.display(),
        "${LANG_TITLE} manifest parsing not yet implemented"
    );

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| PluginError::manifest(format!("Failed to read manifest: {}", e)))?;

    Ok(ManifestData {
        name: "unknown".to_string(),
        version: "0.0.0".to_string(),
        dependencies: vec![],
        dev_dependencies: vec![],
        raw_data: serde_json::json!({ "content": content }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_analyze_empty_manifest() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "# Empty manifest").unwrap();

        let result = analyze_manifest(temp_file.path()).await;
        assert!(result.is_ok());
    }
}
EOF

# Create README.md
echo -e "${GREEN}✓${NC} Generating README.md..."
cat > "$PLUGIN_DIR/README.md" << EOF
# ${LANG_TITLE} Language Plugin

${LANG_TITLE} language support for Codebuddy via the \`LanguageIntelligencePlugin\` trait.

## Features

- [ ] AST parsing and symbol extraction
- [ ] Import/dependency analysis
- [ ] Manifest file parsing
- [ ] Refactoring support

## Implementation Status

🚧 **Under Development**

This plugin has been scaffolded but requires implementation of its core features.

## Testing

\`\`\`bash
# Run plugin tests
cargo test -p ${PLUGIN_NAME}

# Run with output
cargo test -p ${PLUGIN_NAME} -- --nocapture
\`\`\`

## Registration

The plugin must be registered in \`crates/cb-services/src/services/registry_builder.rs\`:

\`\`\`rust
// Register ${LANG_TITLE} plugin
#[cfg(feature = "lang-${LANG_LOWER}")]
{
    registry.register(Arc::new(${PLUGIN_NAME}::${LANG_TITLE}Plugin::new()));
    plugin_count += 1;
}
\`\`\`

## References

- [Language Plugin Guide](../README.md)
- [API Documentation](../../cb-plugin-api/src/lib.rs)
- Reference implementations: \`cb-lang-rust\`, \`cb-lang-go\`, \`cb-lang-typescript\`
EOF

echo ""
echo -e "${GREEN}✓${NC} Successfully created ${LANG_TITLE} language plugin at:"
echo -e "  ${BLUE}${PLUGIN_DIR}${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo ""
echo -e "1. ${BLUE}Add to workspace dependencies${NC} in ${WORKSPACE_ROOT}/Cargo.toml:"
echo -e "   ${GREEN}[workspace.dependencies]${NC}"
echo -e "   ${GREEN}${PLUGIN_NAME} = { path = \"crates/languages/${PLUGIN_NAME}\" }${NC}"
echo ""
echo -e "2. ${BLUE}Add features to cb-handlers${NC} in ${WORKSPACE_ROOT}/crates/cb-handlers/Cargo.toml:"
echo -e "   ${GREEN}[dependencies]${NC}"
echo -e "   ${GREEN}${PLUGIN_NAME} = { workspace = true, optional = true }${NC}"
echo -e ""
echo -e "   ${GREEN}[features]${NC}"
echo -e "   ${GREEN}lang-${LANG_LOWER} = [\"dep:${PLUGIN_NAME}\"]${NC}"
echo ""
echo -e "3. ${BLUE}Register plugin${NC} in ${WORKSPACE_ROOT}/crates/cb-services/src/services/registry_builder.rs"
echo ""
echo -e "4. ${BLUE}Implement parsing logic${NC} in:"
echo -e "   - ${PLUGIN_DIR}/src/parser.rs"
echo -e "   - ${PLUGIN_DIR}/src/manifest.rs"
echo ""
echo -e "5. ${BLUE}Add tests${NC} and run:"
echo -e "   ${GREEN}cargo test -p ${PLUGIN_NAME}${NC}"
echo ""
echo -e "6. ${BLUE}Verify configuration${NC}:"
echo -e "   ${GREEN}./crates/languages/check-features.sh${NC}"
echo ""
echo -e "${BLUE}For examples, see:${NC}"
echo -e "  - Pure Rust parser: ${GREEN}crates/languages/cb-lang-rust${NC}"
echo -e "  - Dual-mode parser: ${GREEN}crates/languages/cb-lang-go${NC}"
echo ""