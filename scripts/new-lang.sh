#!/usr/bin/env bash
set -euo pipefail

# new-lang.sh - Scaffold a new language plugin for Codebuddy
#
# Usage: ./new-lang.sh <language-name> [options]
# Example: ./new-lang.sh csharp --manifest "*.csproj" --extensions cs,csx
#
# Options:
#   --extensions <ext1,ext2>  Comma-separated file extensions (default: language-name)
#   --manifest <filename>     Manifest filename (required)
#   --source-dir <dir>        Source directory (default: "src")
#   --entry-point <file>      Entry point filename (default: "main.<ext>")
#   --module-sep <sep>        Module separator (default: ".")
#   --dry-run                 Show what would be done without making changes

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CRATES_DIR="$WORKSPACE_ROOT/crates"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <language-name> [options]"
    echo ""
    echo "Options:"
    echo "  --extensions <ext1,ext2>  Comma-separated file extensions (default: language-name)"
    echo "  --manifest <filename>     Manifest filename (required)"
    echo "  --source-dir <dir>        Source directory (default: 'src')"
    echo "  --entry-point <file>      Entry point filename (default: 'main.<ext>')"
    echo "  --module-sep <sep>        Module separator (default: '.')"
    echo "  --dry-run                 Show what would be done without making changes"
    echo ""
    echo "Examples:"
    echo "  $0 csharp --manifest '*.csproj' --extensions cs,csx"
    echo "  $0 kotlin --manifest build.gradle.kts --extensions kt,kts"
    echo "  $0 ruby --manifest Gemfile --extensions rb --source-dir '' --entry-point main.rb"
    exit 1
}

if [ $# -lt 1 ]; then
    usage
fi

LANG_NAME="$1"
shift

# Parse options
DRY_RUN=false
EXTENSIONS=""
MANIFEST=""
SOURCE_DIR="src"
ENTRY_POINT=""
MODULE_SEP="."

while [[ $# -gt 0 ]]; do
    case $1 in
        --extensions)
            EXTENSIONS="$2"
            shift 2
            ;;
        --manifest)
            MANIFEST="$2"
            shift 2
            ;;
        --source-dir)
            SOURCE_DIR="$2"
            shift 2
            ;;
        --entry-point)
            ENTRY_POINT="$2"
            shift 2
            ;;
        --module-sep)
            MODULE_SEP="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        *)
            echo -e "${RED}Error: Unknown option: $1${NC}"
            usage
            ;;
    esac
done

# Validate required options
if [ -z "$MANIFEST" ]; then
    echo -e "${RED}Error: --manifest is required${NC}"
    usage
fi

# Generate language name variants
LANG_LOWER=$(echo "$LANG_NAME" | tr '[:upper:]' '[:lower:]')
LANG_TITLE=$(echo "$LANG_LOWER" | sed 's/.*/\u&/')

# Set defaults based on language name
if [ -z "$EXTENSIONS" ]; then
    EXTENSIONS="$LANG_LOWER"
fi

if [ -z "$ENTRY_POINT" ]; then
    FIRST_EXT=$(echo "$EXTENSIONS" | cut -d',' -f1)
    ENTRY_POINT="main.$FIRST_EXT"
fi

# Convert extensions to array format for Rust
EXTENSIONS_ARRAY_QUOTED=""
IFS=',' read -ra EXT_ARR <<< "$EXTENSIONS"
for ext in "${EXT_ARR[@]}"; do
    if [ -z "$EXTENSIONS_ARRAY_QUOTED" ]; then
        EXTENSIONS_ARRAY_QUOTED="\"$ext\""
    else
        EXTENSIONS_ARRAY_QUOTED="$EXTENSIONS_ARRAY_QUOTED, \"$ext\""
    fi
done

PLUGIN_NAME="cb-lang-${LANG_LOWER}"
PLUGIN_DIR="${CRATES_DIR}/${PLUGIN_NAME}"

echo -e "${BLUE}Creating ${LANG_TITLE} language plugin...${NC}"
echo -e "  Plugin Crate: ${GREEN}$PLUGIN_NAME${NC}"
echo -e "  Extensions: ${GREEN}$EXTENSIONS${NC}"
echo -e "  Manifest: ${GREEN}$MANIFEST${NC}"
echo -e "  Source dir: ${GREEN}$SOURCE_DIR${NC}"
echo -e "  Entry point: ${GREEN}$ENTRY_POINT${NC}"
echo -e "  Module separator: ${GREEN}$MODULE_SEP${NC}"

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}[DRY RUN] - No changes will be made${NC}"
fi

# Check if plugin already exists
if [ -d "$PLUGIN_DIR" ]; then
    echo -e "${RED}Error: Plugin directory already exists: ${PLUGIN_DIR}${NC}"
    exit 1
fi

# ============================================================================
# Phase 1: Create Plugin Directory Structure
# ============================================================================

echo ""
echo -e "${BLUE}Phase 1: Creating plugin directory structure${NC}"

if [ "$DRY_RUN" = false ]; then
    mkdir -p "$PLUGIN_DIR/src"
    mkdir -p "$PLUGIN_DIR/resources"
fi
echo -e "${GREEN}✓${NC} Created directory: ${PLUGIN_DIR}"

# ============================================================================
# Phase 2: Generate Plugin Files
# ============================================================================

echo ""
echo -e "${BLUE}Phase 2: Generating plugin files${NC}"

# Create Cargo.toml
if [ "$DRY_RUN" = false ]; then
cat > "$PLUGIN_DIR/Cargo.toml" << EOF
[package]
name = "${PLUGIN_NAME}"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "A Codebuddy language plugin for ${LANG_TITLE}"

[dependencies]
# Codebuddy workspace dependencies
cb-plugin-api = { path = "../cb-plugin-api" }
cb-plugin-registry = { path = "../cb-plugin-registry" }
cb-lang-common = { path = "../cb-lang-common" }

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
EOF
fi
echo -e "${GREEN}✓${NC} Generated Cargo.toml"

# Create lib.rs with modern LanguagePlugin implementation
if [ "$DRY_RUN" = false ]; then
cat > "$PLUGIN_DIR/src/lib.rs" << EOF
//! ${LANG_TITLE} language plugin for Codebuddy
//!
//! Provides AST parsing, symbol extraction, and manifest analysis for ${LANG_TITLE}.

mod parser;
mod manifest;

use cb_plugin_api::{
    LanguagePlugin, LanguageMetadata, PluginCapabilities, ManifestData,
    ParsedSource, PluginResult, LspConfig,
};
use cb_plugin_registry::codebuddy_plugin;
use async_trait::async_trait;
use std::path::Path;

// Register the plugin with the Codebuddy system.
// This macro creates a static descriptor that is collected at link-time.
codebuddy_plugin! {
    name: "${LANG_LOWER}",
    extensions: [${EXTENSIONS_ARRAY_QUOTED}],
    manifest: "${MANIFEST}",
    capabilities: ${LANG_TITLE}Plugin::CAPABILITIES,
    factory: ${LANG_TITLE}Plugin::new,
    lsp: None, // TODO: Add LSP config if applicable, e.g., Some(LspConfig::new("gopls", &["gopls"]))
}

/// ${LANG_TITLE} language plugin implementation.
#[derive(Default)]
pub struct ${LANG_TITLE}Plugin;

impl ${LANG_TITLE}Plugin {
    /// Static metadata for the ${LANG_TITLE} language.
    pub const METADATA: LanguageMetadata = LanguageMetadata {
        name: "${LANG_LOWER}",
        extensions: &[${EXTENSIONS_ARRAY_QUOTED}],
        manifest_filename: "${MANIFEST}",
        source_dir: "${SOURCE_DIR}",
        entry_point: "${ENTRY_POINT}",
        module_separator: "${MODULE_SEP}",
    };

    /// The capabilities of this plugin.
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
        imports: false,  // TODO: Set to true when ImportSupport is implemented
        workspace: false, // TODO: Set to true when WorkspaceSupport is implemented
    };

    /// Creates a new, boxed instance of the plugin.
    pub fn new() -> Box<dyn LanguagePlugin> {
        Box::new(Self::default())
    }
}

#[async_trait]
impl LanguagePlugin for ${LANG_TITLE}Plugin {
    fn metadata(&self) -> &LanguageMetadata {
        &Self::METADATA
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        parser::parse_source(source)
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::analyze_manifest(path).await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cb_plugin_api::LanguagePlugin;

    #[test]
    fn test_plugin_creation_and_metadata() {
        let plugin = ${LANG_TITLE}Plugin::new();
        let metadata = plugin.metadata();
        assert_eq!(metadata.name, "${LANG_LOWER}");
        assert!(metadata.extensions.contains(&"${EXTENSIONS}".split(',').next().unwrap()));
    }

    #[test]
    fn test_capabilities() {
        let plugin = ${LANG_TITLE}Plugin::new();
        let caps = plugin.capabilities();
        // Update these assertions as capabilities are implemented
        assert!(!caps.imports);
        assert!(!caps.workspace);
    }
}
EOF
fi
echo -e "${GREEN}✓${NC} Generated src/lib.rs"

# Create parser.rs
if [ "$DRY_RUN" = false ]; then
cat > "$PLUGIN_DIR/src/parser.rs" << EOF
//! ${LANG_TITLE} source code parsing and symbol extraction.
use cb_plugin_api::{ParsedSource, PluginResult, PluginError};

pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    tracing::warn!(
        source_length = source.len(),
        "${LANG_TITLE} parsing not yet implemented - returning empty symbols"
    );

    // TODO: Implement parsing logic here.
    // This could involve:
    // - Using a tree-sitter grammar.
    // - Spawning an external tool via cb_lang_common::SubprocessAstTool.
    // - Using regex for a simpler, less accurate parser.

    Ok(ParsedSource {
        data: serde_json::json!({
            "language": "${LANG_LOWER}",
            "status": "unimplemented",
        }),
        symbols: vec![],
    })
}
EOF
fi
echo -e "${GREEN}✓${NC} Generated src/parser.rs"

# Create manifest.rs
if [ "$DRY_RUN" = false ]; then
cat > "$PLUGIN_DIR/src/manifest.rs" << EOF
//! ${LANG_TITLE} manifest file parsing for ${MANIFEST}.
use cb_plugin_api::{ManifestData, PluginResult, PluginError};
use cb_lang_common::read_manifest;
use std::path::Path;

pub async fn analyze_manifest(path: &Path) -> PluginResult<ManifestData> {
    tracing::warn!(
        manifest_path = %path.display(),
        "${LANG_TITLE} manifest parsing not yet implemented"
    );

    let content = read_manifest(path).await?;

    // TODO: Parse manifest content and extract project metadata.

    Ok(ManifestData {
        name: path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("unknown").to_string(),
        version: "0.0.0".to_string(),
        dependencies: vec![],
        dev_dependencies: vec![],
        raw_data: serde_json::Value::String(content),
    })
}
EOF
fi
echo -e "${GREEN}✓${NC} Generated src/manifest.rs"

# Create README.md
if [ "$DRY_RUN" = false ]; then
cat > "$PLUGIN_DIR/README.md" << EOF
# ${LANG_TITLE} Language Plugin

${LANG_TITLE} language support for Codebuddy via the \`LanguagePlugin\` trait.

This plugin self-registers with the Codebuddy system using the \`codebuddy_plugin!\` macro.

## Configuration

- **Extensions**: ${EXTENSIONS}
- **Manifest**: ${MANIFEST}

## Features

- [ ] AST parsing and symbol extraction
- [ ] Manifest file parsing
- [ ] Import/dependency analysis (\`ImportSupport\` trait)
- [ ] Workspace operations (\`WorkspaceSupport\` trait)

## Implementation Status

🚧 **Under Development**

This plugin has been scaffolded but requires implementation of its core features.

### Next Steps

1.  **Implement \`parser.rs\`**: Add actual AST parsing logic.
2.  **Implement \`manifest.rs\`**: Parse \`${MANIFEST}\` files to extract dependencies and project metadata.
3.  **Add Capabilities**: Implement \`ImportSupport\` or \`WorkspaceSupport\` traits as needed and update the \`CAPABILITIES\` constant in \`lib.rs\`.
4.  **Add Tests**: Write comprehensive unit and integration tests for all implemented features.

See \`docs/development/languages/README.md\` for detailed guidance.
EOF
fi
echo -e "${GREEN}✓${NC} Generated README.md"

# ============================================================================
# Summary and Next Steps
# ============================================================================

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Successfully created ${LANG_TITLE} language plugin!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}Plugin location:${NC}"
echo -e "  ${PLUGIN_DIR}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo ""
echo -e "1. ${BLUE}Add the new crate to the workspace:${NC}"
echo -e "   Add the following line to the \`[workspace.members]\` array in the root \`Cargo.toml\`:"
echo -e "   ${GREEN}\"crates/${PLUGIN_NAME}\"${NC}"
echo ""
echo -e "2. ${BLUE}Build the project to integrate the new plugin:${NC}"
echo -e "   ${GREEN}cargo build${NC}"
echo ""
echo -e "3. ${BLUE}Implement the parsing logic:${NC}"
echo -e "   - Edit ${PLUGIN_DIR}/src/parser.rs"
echo -e "   - Edit ${PLUGIN_DIR}/src/manifest.rs"
echo ""
echo -e "4. ${BLUE}Run tests for your new plugin:${NC}"
echo -e "   ${GREEN}cargo test -p ${PLUGIN_NAME}${NC}"
echo ""
echo -e "${BLUE}For detailed guidance, see:${NC}"
echo -e "  ${GREEN}docs/development/languages/README.md${NC}"
echo ""