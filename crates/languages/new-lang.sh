#!/usr/bin/env bash
set -euo pipefail

# new-lang.sh - Scaffold a new language plugin for Codebuddy
#
# Usage: ./new-lang.sh <language-name> [options]
# Example: ./new-lang.sh csharp
#
# Options:
#   --extensions <ext1,ext2>  Comma-separated file extensions (default: language-name)
#   --manifest <filename>     Manifest filename (required)
#   --source-dir <dir>        Source directory (default: "src")
#   --entry-point <file>      Entry point filename (default: "main.<ext>")
#   --module-sep <sep>        Module separator (default: ".")
#   --dry-run                 Show what would be done without making changes

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
LANG_UPPER=$(echo "$LANG_NAME" | tr '[:lower:]' '[:upper:]')
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
EXTENSIONS_ARRAY=""
IFS=',' read -ra EXT_ARR <<< "$EXTENSIONS"
for ext in "${EXT_ARR[@]}"; do
    if [ -z "$EXTENSIONS_ARRAY" ]; then
        EXTENSIONS_ARRAY="\"$ext\""
    else
        EXTENSIONS_ARRAY="$EXTENSIONS_ARRAY, \"$ext\""
    fi
done

PLUGIN_NAME="cb-lang-${LANG_LOWER}"
PLUGIN_DIR="${LANGUAGES_DIR}/${PLUGIN_NAME}"

echo -e "${BLUE}Creating ${LANG_TITLE} language plugin...${NC}"
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
# toml = "0.9"
# toml_edit = "0.23"
tempfile = "3.10"
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
    LanguagePlugin, LanguageMetadata, LanguageCapabilities, ManifestData,
    ParsedSource, PluginResult,
};
use async_trait::async_trait;
use std::path::Path;

/// ${LANG_TITLE} language plugin implementation
pub struct ${LANG_TITLE}Plugin {
    metadata: LanguageMetadata,
}

impl ${LANG_TITLE}Plugin {
    /// Create a new ${LANG_TITLE} plugin instance
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::${LANG_UPPER},
        }
    }
}

impl Default for ${LANG_TITLE}Plugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for ${LANG_TITLE}Plugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            imports: false,  // TODO: Set to true when import support is implemented
            workspace: false, // TODO: Set to true when workspace support is implemented
        }
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

    // Optional: Override import_support() when ready
    // fn import_support(&self) -> Option<&dyn ImportSupport> {
    //     Some(&self.import_support)
    // }

    // Optional: Override workspace_support() when ready
    // fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
    //     Some(&self.workspace_support)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = ${LANG_TITLE}Plugin::new();
        assert_eq!(plugin.metadata().name, "${LANG_TITLE}");
    }

    #[test]
    fn test_file_extensions() {
        let plugin = ${LANG_TITLE}Plugin::new();
        let extensions = plugin.metadata().extensions;
        assert!(!extensions.is_empty());
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
//! ${LANG_TITLE} source code parsing and symbol extraction

use cb_plugin_api::{ParsedSource, PluginResult};

/// Parse ${LANG_TITLE} source code and extract symbols
///
/// TODO: Implement actual parsing logic using appropriate parser library
pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    tracing::warn!(
        source_length = source.len(),
        "${LANG_TITLE} parsing not yet implemented - returning empty symbols"
    );

    Ok(ParsedSource {
        data: serde_json::json!({
            "language": "${LANG_TITLE}",
            "source_length": source.len(),
        }),
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
        let parsed = result.unwrap();
        assert_eq!(parsed.symbols.len(), 0);
    }

    #[test]
    fn test_parse_simple_source() {
        let source = r#"
            // A simple ${LANG_TITLE} source file
            function hello() {
                console.log("Hello, World!");
            }
        "#;
        let result = parse_source(source);
        assert!(result.is_ok());
    }
}
EOF
fi
echo -e "${GREEN}✓${NC} Generated src/parser.rs"

# Create manifest.rs
if [ "$DRY_RUN" = false ]; then
cat > "$PLUGIN_DIR/src/manifest.rs" << EOF
//! ${LANG_TITLE} manifest file parsing
//!
//! Handles ${MANIFEST} files for ${LANG_TITLE} projects.

use cb_plugin_api::{ManifestData, PluginError, PluginResult};
use std::path::Path;

/// Analyze ${LANG_TITLE} manifest file
///
/// TODO: Implement actual manifest parsing logic
pub async fn analyze_manifest(path: &Path) -> PluginResult<ManifestData> {
    tracing::warn!(
        manifest_path = %path.display(),
        "${LANG_TITLE} manifest parsing not yet implemented"
    );

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| PluginError::manifest(format!("Failed to read manifest: {}", e)))?;

    // TODO: Parse manifest content and extract:
    // - Project name
    // - Version
    // - Dependencies
    // - Dev dependencies

    Ok(ManifestData {
        name: path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        version: "0.0.0".to_string(),
        dependencies: vec![],
        dev_dependencies: vec![],
        raw_data: serde_json::json!({
            "content_length": content.len(),
            "path": path.display().to_string(),
        }),
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

    #[tokio::test]
    async fn test_analyze_nonexistent_manifest() {
        let result = analyze_manifest(Path::new("/nonexistent/manifest")).await;
        assert!(result.is_err());
    }
}
EOF
fi
echo -e "${GREEN}✓${NC} Generated src/manifest.rs"

# Create README.md
if [ "$DRY_RUN" = false ]; then
cat > "$PLUGIN_DIR/README.md" << EOF
# ${LANG_TITLE} Language Plugin

${LANG_TITLE} language support for Codebuddy via the \`LanguagePlugin\` trait.

## Configuration

- **Extensions**: ${EXTENSIONS}
- **Manifest**: ${MANIFEST}
- **Source Directory**: ${SOURCE_DIR}
- **Entry Point**: ${ENTRY_POINT}
- **Module Separator**: ${MODULE_SEP}

## Features

- [ ] AST parsing and symbol extraction
- [ ] Import/dependency analysis (ImportSupport trait)
- [ ] Workspace operations (WorkspaceSupport trait)
- [ ] Manifest file parsing

## Implementation Status

🚧 **Under Development**

This plugin has been scaffolded but requires implementation of its core features.

### Next Steps

1. **Implement parser.rs**: Add actual AST parsing logic
   - Choose appropriate parser library
   - Extract symbols (functions, classes, etc.)
   - Handle syntax errors gracefully

2. **Implement manifest.rs**: Parse ${MANIFEST} files
   - Extract project metadata
   - Parse dependencies
   - Handle different manifest formats

3. **Add Import Support** (optional): Implement \`ImportSupport\` trait
   - Parse import statements
   - Rewrite imports during refactoring
   - Add missing imports

4. **Add Workspace Support** (optional): Implement \`WorkspaceSupport\` trait
   - Add workspace members
   - Remove workspace members
   - Update workspace manifests

## Testing

\`\`\`bash
# Run plugin tests
cargo test -p ${PLUGIN_NAME}

# Run with output
cargo test -p ${PLUGIN_NAME} -- --nocapture

# Test specific module
cargo test -p ${PLUGIN_NAME} parser::tests
\`\`\`

## Integration

This plugin has been automatically registered in:
- Root \`Cargo.toml\` workspace dependencies
- \`crates/cb-handlers/Cargo.toml\` with feature gate \`lang-${LANG_LOWER}\`
- \`crates/cb-services/src/services/registry_builder.rs\`
- \`crates/cb-core/src/language.rs\` (ProjectLanguage enum)
- \`crates/cb-plugin-api/src/metadata.rs\` (LanguageMetadata constant)

## References

- [Language Plugin Guide](../README.md)
- [API Documentation](../../cb-plugin-api/src/lib.rs)
- Reference implementations:
  - \`cb-lang-rust\` - Full implementation with import and workspace support
  - \`cb-lang-go\` - Dual-mode parser (tree-sitter + regex fallback)
  - \`cb-lang-typescript\` - Tree-sitter based parser
  - \`cb-lang-python\` - Python-specific patterns
  - \`cb-lang-java\` - Java integration example
EOF
fi
echo -e "${GREEN}✓${NC} Generated README.md"

# ============================================================================
# Phase 3: Auto-patch Workspace Files
# ============================================================================

echo ""
echo -e "${BLUE}Phase 3: Patching workspace configuration files${NC}"

# 3.1: Patch root Cargo.toml workspace dependencies
echo -e "${BLUE}  → Patching root Cargo.toml...${NC}"
ROOT_CARGO="${WORKSPACE_ROOT}/Cargo.toml"

if [ "$DRY_RUN" = false ]; then
    # Check if dependency already exists
    if grep -q "^${PLUGIN_NAME} = " "$ROOT_CARGO"; then
        echo -e "${YELLOW}    ⚠ Dependency already exists in root Cargo.toml${NC}"
    else
        # Find the last language plugin line number and add after it
        if grep -q "^cb-lang-" "$ROOT_CARGO"; then
            # Use awk to insert after the last cb-lang- entry
            awk -v line="${PLUGIN_NAME} = { path = \"crates/languages/${PLUGIN_NAME}\" }" '
                /^cb-lang-/ { last_line = NR }
                { lines[NR] = $0 }
                END {
                    for (i = 1; i <= NR; i++) {
                        print lines[i]
                        if (i == last_line) print line
                    }
                }
            ' "$ROOT_CARGO" > "$ROOT_CARGO.tmp" && mv "$ROOT_CARGO.tmp" "$ROOT_CARGO"
            echo -e "${GREEN}    ✓ Added to workspace dependencies${NC}"
        else
            echo -e "${RED}    ✗ Could not find language plugin section${NC}"
        fi
    fi
else
    echo -e "${YELLOW}    [DRY RUN] Would add: ${PLUGIN_NAME} = { path = \"crates/languages/${PLUGIN_NAME}\" }${NC}"
fi

# 3.2: Patch crates/cb-handlers/Cargo.toml
echo -e "${BLUE}  → Patching cb-handlers/Cargo.toml...${NC}"
HANDLERS_CARGO="${WORKSPACE_ROOT}/crates/cb-handlers/Cargo.toml"

if [ "$DRY_RUN" = false ]; then
    # Add to dependencies section
    if grep -q "^${PLUGIN_NAME} = " "$HANDLERS_CARGO"; then
        echo -e "${YELLOW}    ⚠ Dependency already exists in cb-handlers/Cargo.toml${NC}"
    else
        # Insert after the last cb-lang- dependency using awk
        awk -v line="${PLUGIN_NAME} = { path = \"../languages/${PLUGIN_NAME}\", optional = true }" '
            /^cb-lang-.*optional = true/ { last_dep = NR }
            { lines[NR] = $0 }
            END {
                for (i = 1; i <= NR; i++) {
                    print lines[i]
                    if (i == last_dep) print line
                }
            }
        ' "$HANDLERS_CARGO" > "$HANDLERS_CARGO.tmp" && mv "$HANDLERS_CARGO.tmp" "$HANDLERS_CARGO"
        echo -e "${GREEN}    ✓ Added optional dependency${NC}"
    fi

    # Add to features section
    if grep -q "^lang-${LANG_LOWER} = " "$HANDLERS_CARGO"; then
        echo -e "${YELLOW}    ⚠ Feature already exists in cb-handlers/Cargo.toml${NC}"
    else
        # Insert after the last lang- feature using awk
        awk -v line="lang-${LANG_LOWER} = [\"dep:${PLUGIN_NAME}\"]" '
            /^lang-.*= \[/ { last_feat = NR }
            { lines[NR] = $0 }
            END {
                for (i = 1; i <= NR; i++) {
                    print lines[i]
                    if (i == last_feat) print line
                }
            }
        ' "$HANDLERS_CARGO" > "$HANDLERS_CARGO.tmp" && mv "$HANDLERS_CARGO.tmp" "$HANDLERS_CARGO"
        echo -e "${GREEN}    ✓ Added feature gate${NC}"
    fi

    # Add to default features
    if grep "^default = " "$HANDLERS_CARGO" | grep -q "lang-${LANG_LOWER}"; then
        echo -e "${YELLOW}    ⚠ Already in default features${NC}"
    else
        # Add to default features array using awk (portable across macOS/Linux)
        awk -v lang="lang-${LANG_LOWER}" '
            /^default = \[/ {
                sub(/\]$/, ", \"" lang "\"]")
            }
            { print }
        ' "$HANDLERS_CARGO" > "$HANDLERS_CARGO.tmp" && mv "$HANDLERS_CARGO.tmp" "$HANDLERS_CARGO"
        echo -e "${GREEN}    ✓ Added to default features${NC}"
    fi
else
    echo -e "${YELLOW}    [DRY RUN] Would add dependency and feature gate${NC}"
fi

# 3.3: Patch registry_builder.rs
echo -e "${BLUE}  → Patching registry_builder.rs...${NC}"
REGISTRY_FILE="${WORKSPACE_ROOT}/crates/cb-services/src/services/registry_builder.rs"

if [ "$DRY_RUN" = false ]; then
    # Check if already registered
    if grep -q "cb_lang_${LANG_LOWER}" "$REGISTRY_FILE"; then
        echo -e "${YELLOW}    ⚠ Plugin already registered${NC}"
    else
        # Create registration block in a temp file
        cat > /tmp/plugin_registration.txt << REGEOF

    // Register ${LANG_TITLE} plugin
    #[cfg(feature = "lang-${LANG_LOWER}")]
    {
        registry.register(Arc::new(cb_lang_${LANG_LOWER}::${LANG_TITLE}Plugin::new()));
        plugin_count += 1;
    }
REGEOF
        # Insert before the final plugin_count line using awk
        awk '
            /let _ = plugin_count;/ {
                while ((getline line < "/tmp/plugin_registration.txt") > 0) {
                    print line
                }
                close("/tmp/plugin_registration.txt")
            }
            { print }
        ' "$REGISTRY_FILE" > "$REGISTRY_FILE.tmp" && mv "$REGISTRY_FILE.tmp" "$REGISTRY_FILE"
        rm -f /tmp/plugin_registration.txt
        echo -e "${GREEN}    ✓ Registered plugin${NC}"
    fi
else
    echo -e "${YELLOW}    [DRY RUN] Would register plugin${NC}"
fi

# 3.4: Patch Language enum in cb-core
echo -e "${BLUE}  → Patching Language enum in cb-core...${NC}"
LANGUAGE_FILE="${WORKSPACE_ROOT}/crates/cb-core/src/language.rs"

if [ "$DRY_RUN" = false ]; then
    # Add enum variant
    if grep -q "^\s*${LANG_TITLE}," "$LANGUAGE_FILE"; then
        echo -e "${YELLOW}    ⚠ Enum variant already exists${NC}"
    else
        # Insert before Unknown variant using awk
        awk -v title="${LANG_TITLE}" -v manifest="${MANIFEST}" '
            /\/\/\/ Unknown or mixed-language project/ {
                print "    /// " title " projects (" manifest ")"
                print "    " title ","
            }
            { print }
        ' "$LANGUAGE_FILE" > "$LANGUAGE_FILE.tmp" && mv "$LANGUAGE_FILE.tmp" "$LANGUAGE_FILE"
        echo -e "${GREEN}    ✓ Added enum variant${NC}"
    fi

    # Add to as_str() match
    if grep -q "ProjectLanguage::${LANG_TITLE}" "$LANGUAGE_FILE"; then
        echo -e "${YELLOW}    ⚠ as_str() already updated${NC}"
    else
        awk -v title="${LANG_TITLE}" -v lower="${LANG_LOWER}" '
            /ProjectLanguage::Unknown => "unknown"/ {
                print "            ProjectLanguage::" title " => \"" lower "\","
            }
            { print }
        ' "$LANGUAGE_FILE" > "$LANGUAGE_FILE.tmp" && mv "$LANGUAGE_FILE.tmp" "$LANGUAGE_FILE"
        echo -e "${GREEN}    ✓ Updated as_str()${NC}"
    fi

    # Add to manifest_filename() match
    if grep -q "ProjectLanguage::${LANG_TITLE}.*=>.*\"${MANIFEST}\"" "$LANGUAGE_FILE"; then
        echo -e "${YELLOW}    ⚠ manifest_filename() already updated${NC}"
    else
        awk -v title="${LANG_TITLE}" -v manifest="${MANIFEST}" '
            /ProjectLanguage::Unknown => ""/ {
                print "            ProjectLanguage::" title " => \"" manifest "\","
            }
            { print }
        ' "$LANGUAGE_FILE" > "$LANGUAGE_FILE.tmp" && mv "$LANGUAGE_FILE.tmp" "$LANGUAGE_FILE"
        echo -e "${GREEN}    ✓ Updated manifest_filename()${NC}"
    fi

    # Add to detect_project_language()
    if grep -q "Check for ${LANG_TITLE}" "$LANGUAGE_FILE"; then
        echo -e "${YELLOW}    ⚠ detect_project_language() already updated${NC}"
    else
        # Create detection block in temp file
        cat > /tmp/detect_block.txt << DETECTEOF

    // Check for ${LANG_TITLE}
    if project_path.join("${MANIFEST}").exists() {
        debug!("Detected ${LANG_TITLE} project (found ${MANIFEST})");
        return ProjectLanguage::${LANG_TITLE};
    }
DETECTEOF
        # Insert before "Could not detect" line
        awk '
            /debug!\("Could not detect project language"\)/ {
                while ((getline line < "/tmp/detect_block.txt") > 0) {
                    print line
                }
                close("/tmp/detect_block.txt")
            }
            { print }
        ' "$LANGUAGE_FILE" > "$LANGUAGE_FILE.tmp" && mv "$LANGUAGE_FILE.tmp" "$LANGUAGE_FILE"
        rm -f /tmp/detect_block.txt
        echo -e "${GREEN}    ✓ Updated detect_project_language()${NC}"
    fi
else
    echo -e "${YELLOW}    [DRY RUN] Would update Language enum and related functions${NC}"
fi

# 3.5: Patch LanguageMetadata constants
echo -e "${BLUE}  → Patching LanguageMetadata in cb-plugin-api...${NC}"
METADATA_FILE="${WORKSPACE_ROOT}/crates/cb-plugin-api/src/metadata.rs"

if [ "$DRY_RUN" = false ]; then
    # Check if constant already exists
    if grep -q "pub const ${LANG_UPPER}: Self" "$METADATA_FILE"; then
        echo -e "${YELLOW}    ⚠ Metadata constant already exists${NC}"
    else
        # Create the constant block in a temp file
        cat > /tmp/metadata_const.txt << METAEOF

    /// ${LANG_TITLE} language metadata constant
    pub const ${LANG_UPPER}: Self = Self {
        name: "${LANG_TITLE}",
        extensions: &[${EXTENSIONS_ARRAY}],
        manifest_filename: "${MANIFEST}",
        source_dir: "${SOURCE_DIR}",
        entry_point: "${ENTRY_POINT}",
        module_separator: "${MODULE_SEP}",
        language: ProjectLanguage::${LANG_TITLE},
    };
METAEOF
        # Insert before the last closing brace of impl LanguageMetadata
        # Find the line with the last "}" that closes the impl block
        awk '
            /^}$/ {
                if (in_impl) {
                    while ((getline line < "/tmp/metadata_const.txt") > 0) {
                        print line
                    }
                    close("/tmp/metadata_const.txt")
                    in_impl = 0
                }
                print
                next
            }
            /^impl LanguageMetadata/ { in_impl = 1 }
            { print }
        ' "$METADATA_FILE" > "$METADATA_FILE.tmp" && mv "$METADATA_FILE.tmp" "$METADATA_FILE"
        rm -f /tmp/metadata_const.txt
        echo -e "${GREEN}    ✓ Added metadata constant${NC}"
    fi
else
    echo -e "${YELLOW}    [DRY RUN] Would add LanguageMetadata::${LANG_UPPER}${NC}"
fi

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
echo -e "${BLUE}Auto-patched files:${NC}"
echo -e "  ${GREEN}✓${NC} Cargo.toml (workspace dependencies)"
echo -e "  ${GREEN}✓${NC} crates/cb-handlers/Cargo.toml (features & dependencies)"
echo -e "  ${GREEN}✓${NC} crates/cb-services/src/services/registry_builder.rs"
echo -e "  ${GREEN}✓${NC} crates/cb-core/src/language.rs (ProjectLanguage enum)"
echo -e "  ${GREEN}✓${NC} crates/cb-plugin-api/src/metadata.rs (LanguageMetadata constant)"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo ""
echo -e "1. ${BLUE}Verify the changes:${NC}"
echo -e "   ${GREEN}cargo build --features lang-${LANG_LOWER}${NC}"
echo ""
echo -e "2. ${BLUE}Implement parsing logic:${NC}"
echo -e "   - Edit ${PLUGIN_DIR}/src/parser.rs"
echo -e "   - Edit ${PLUGIN_DIR}/src/manifest.rs"
echo ""
echo -e "3. ${BLUE}Run tests:${NC}"
echo -e "   ${GREEN}cargo test -p ${PLUGIN_NAME}${NC}"
echo ""
echo -e "4. ${BLUE}Optional - Add capability traits:${NC}"
echo -e "   - Implement ImportSupport for import analysis"
echo -e "   - Implement WorkspaceSupport for workspace operations"
echo ""
echo -e "5. ${BLUE}Verify feature gate configuration:${NC}"
echo -e "   ${GREEN}./crates/languages/check-features.sh${NC}"
echo ""
echo -e "${BLUE}For implementation examples, see:${NC}"
echo -e "  - Full-featured: ${GREEN}crates/languages/cb-lang-rust${NC}"
echo -e "  - Dual parser:   ${GREEN}crates/languages/cb-lang-go${NC}"
echo -e "  - Tree-sitter:   ${GREEN}crates/languages/cb-lang-typescript${NC}"
echo ""
