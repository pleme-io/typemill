# Language Plugin Development Guide

**Version**: 1.0.0-rc2
**Last Updated**: 2025-10-07

A comprehensive guide for implementing new language plugins for Codebuddy's MCP server.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Prerequisites](#prerequisites)
3. [Plugin Architecture Overview](#plugin-architecture-overview)
4. [Step-by-Step Implementation](#step-by-step-implementation)
5. [Using cb-lang-common Utilities](#using-cb-lang-common-utilities)
6. [Common Patterns](#common-patterns)
7. [Testing Guide](#testing-guide)
8. [Checklist Before Submitting](#checklist-before-submitting)
9. [Reference Implementations](#reference-implementations)
10. [Troubleshooting](#troubleshooting)

---

## Quick Start

**Fastest path to creating a new language plugin:**

```bash
# 1. Generate plugin structure (replaces manual setup)
cd crates/languages
./new-lang.sh kotlin --manifest "build.gradle.kts" --extensions kt,kts

# 2. Build workspace to auto-generate integration code
cd ../..
cargo build --features lang-kotlin

# 3. Implement your language-specific logic
# Edit: cb-lang-kotlin/src/parser.rs
# Edit: cb-lang-kotlin/src/manifest.rs

# 4. Run tests
cargo nextest run -p cb-lang-kotlin

# 5. Validate configuration
cd crates/languages
./check-features.sh
```

**Time estimate**:
- Initial setup: **5 minutes** (automated)
- Core implementation: **1-2 days** (depending on language complexity)
- Testing and polish: **1 day**

---

## Prerequisites

### Required Knowledge

- **Rust programming** - Intermediate level (traits, async, error handling)
- **Your target language** - Deep understanding of syntax and semantics
- **AST parsing basics** - Understanding of abstract syntax trees (helpful but not required)

### Required Tools

- Rust toolchain (1.70+)
- Your target language runtime (for AST parsing, optional if using pure Rust)
- Git

### Recommended Reading

Before implementing a plugin, read:

1. **[CB_LANG_COMMON.md](CB_LANG_COMMON.md)** - Shared utility functions reference
2. **[README.md](README.md)** - Overview of existing plugins
3. One reference implementation:
   - **Go plugin** - Best for compiled languages with subprocess AST
   - **Python plugin** - Best for dynamic languages
   - **Rust plugin** - Best for native Rust parsing (no subprocess)

---

## Plugin Architecture Overview

### Minimum Required Files

Every language plugin has this structure:

```
crates/languages/cb-lang-{language}/
├── Cargo.toml              # Dependencies and metadata
├── README.md               # Plugin documentation
├── resources/              # Optional: embedded AST tools
│   └── ast_tool.*         # Language-native parser subprocess
└── src/
    ├── lib.rs              # Main plugin struct + LanguagePlugin trait
    ├── parser.rs           # Symbol extraction & import parsing
    ├── manifest.rs         # Manifest file parsing (Cargo.toml, package.json, etc.)
    ├── import_support.rs   # Optional: ImportSupport trait implementation
    └── workspace_support.rs # Optional: WorkspaceSupport trait implementation
```

### Core Trait: `LanguagePlugin`

Located at: `/workspace/crates/cb-plugin-api/src/lib.rs:331-383`

Every plugin **must** implement these 5 methods:

```rust
#[async_trait]
impl LanguagePlugin for YourPlugin {
    // 1. Return language metadata
    fn metadata(&self) -> &LanguageMetadata;

    // 2. Parse source code and extract symbols
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource>;

    // 3. Parse manifest files (package.json, go.mod, Cargo.toml, etc.)
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>;

    // 4. Declare which capabilities your plugin supports
    fn capabilities(&self) -> LanguageCapabilities;

    // 5. Enable downcasting (for accessing plugin-specific methods)
    fn as_any(&self) -> &dyn std::any::Any;
}
```

### Optional Capability Traits

If `capabilities().imports = true`, implement:

```rust
impl ImportSupport for YourPlugin {
    fn parse_imports(&self, content: &str) -> Vec<String>;
    fn rewrite_imports_for_move(&self, content: &str, old: &Path, new: &Path) -> (String, usize);
    fn rewrite_imports_for_rename(&self, content: &str, old: &str, new: &str) -> (String, usize);
    fn find_module_references(&self, content: &str, module: &str) -> Vec<ModuleReference>;
}
```

If `capabilities().workspace = true`, implement:

```rust
impl WorkspaceSupport for YourPlugin {
    fn is_workspace_manifest(&self, content: &str) -> bool;
    fn add_workspace_member(&self, content: &str, member_path: &str) -> Result<String>;
    fn remove_workspace_member(&self, content: &str, member_path: &str) -> Result<String>;
    fn merge_dependencies(&self, source: &str, target: &str) -> Result<String>;
}
```

---

## Step-by-Step Implementation

### Step 1: Scaffolding and Integration

#### 1A. Generate Plugin Structure (Automated)

Run the scaffolding script from `crates/languages`:

```bash
cd crates/languages

# Example: Create a Java plugin
./new-lang.sh java

# Example: Create a Kotlin plugin with options
./new-lang.sh kotlin \
  --manifest "build.gradle.kts" \
  --extensions kt,kts
```

**What gets generated:**

✅ Complete directory structure (`crates/languages/cb-lang-{language}/`)
✅ `Cargo.toml` with dependencies
✅ Stub `src/lib.rs`, `src/parser.rs`, `src/manifest.rs`
✅ Placeholder `README.md`

**Output example:**
```
Created crates/cb-lang-java/
  ✅ Cargo.toml
  ✅ src/lib.rs
  ✅ src/parser.rs
  ✅ src/manifest.rs
  ✅ README.md

Next steps: Manual integration required (see below)
```

#### 1B. Manual Integration (Required)

After scaffolding, you **must** manually integrate the plugin into the workspace. Edit these files:

**1. Root `Cargo.toml`** - Add workspace dependency:

```toml
[workspace.dependencies]
# ... existing plugins ...
cb-lang-go = { path = "crates/languages/cb-lang-go" }
cb-lang-java = { path = "crates/cb-lang-java" } # ← Add this
cb-lang-rust = { path = "crates/cb-lang-rust" }
```

**2. `crates/cb-handlers/Cargo.toml`** - Add optional dependency and feature:

```toml
[dependencies]
# ... existing plugins ...
cb-lang-go = { workspace = true, optional = true }
cb-lang-java = { workspace = true, optional = true } # ← Add this
cb-lang-rust = { workspace = true, optional = true }

[features]
# ... existing features ...
lang-go = ["dep:cb-lang-go"]
lang-java = ["dep:cb-lang-java"] # ← Add this
lang-rust = ["dep:cb-lang-rust"]
```

**3. `crates/cb-services/src/services/registry_builder.rs`** - Register plugin:

```rust
// In build_language_plugin_registry() function:

// Register Go plugin
#[cfg(feature = "lang-go")]
{
    registry.register(Arc::new(cb_lang_go::GoPlugin::new()));
    plugin_count += 1;
}

// Register Java plugin
#[cfg(feature = "lang-java")] // ← Add this block
{
    registry.register(Arc::new(cb_lang_java::JavaPlugin::new()));
    plugin_count += 1;
}
```

#### 1C. Verify Integration

Run the verification script to ensure all files are correctly configured:

```bash
cd crates/languages
./check-features.sh
```

**Expected output:**
```
✅ cb-lang-java found in workspace dependencies
✅ cb-lang-java found in cb-handlers dependencies
✅ lang-java feature found in cb-handlers
✅ JavaPlugin registered in registry_builder.rs
All checks passed!
```

If checks fail, review the manual integration steps above.

---

### Step 2: Implement Core `LanguagePlugin` Trait

**File**: `cb-lang-{language}/src/lib.rs`

**Example implementation** (`cb-lang-go/src/lib.rs:79-156`):

```rust
use async_trait::async_trait;
use cb_plugin_api::{
    LanguagePlugin, LanguageMetadata, LanguageCapabilities,
    ManifestData, ParsedSource, PluginResult,
};

pub struct GoPlugin {
    metadata: LanguageMetadata,
    import_support: import_support::GoImportSupport,
    workspace_support: workspace_support::GoWorkspaceSupport,
}

impl GoPlugin {
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::GO,  // Auto-generated constant
            import_support: import_support::GoImportSupport,
            workspace_support: workspace_support::GoWorkspaceSupport::new(),
        }
    }
}

#[async_trait]
impl LanguagePlugin for GoPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            imports: true,      // This plugin supports import analysis
            workspace: true,    // This plugin supports workspace operations
        }
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        // Delegate to parser module
        let symbols = parser::extract_symbols(source)?;

        Ok(ParsedSource {
            data: serde_json::json!({
                "language": "go",
                "symbols_count": symbols.len()
            }),
            symbols,
        })
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        // Delegate to manifest module
        manifest::load_go_mod(path).await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn import_support(&self) -> Option<&dyn ImportSupport> {
        Some(&self.import_support)
    }

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        Some(&self.workspace_support)
    }
}
```

**Key points:**

- **Metadata**: Use auto-generated constant (e.g., `LanguageMetadata::GO`)
- **Capabilities**: Return `true` only for features you actually implement
- **Delegation**: Keep `lib.rs` simple, delegate logic to `parser` and `manifest` modules
- **Trait objects**: Return capability trait implementations via getter methods

---

### Step 3: Implement Parser Module

**File**: `cb-lang-{language}/src/parser.rs`

This module has two responsibilities:

1. **Symbol extraction** - Extract functions, classes, structs, etc.
2. **Import parsing** - Build an `ImportGraph` from source code

#### Pattern A: Subprocess-based AST Parsing (Python, Node, Go, Java)

**Example** (`cb-lang-go/src/parser.rs:9-24`):

```rust
use cb_lang_common::{
    SubprocessAstTool, run_ast_tool, parse_with_fallback, ImportGraphBuilder
};
use cb_plugin_api::{PluginResult, Symbol, SymbolKind};
use cb_protocol::{ImportGraph, ImportInfo};

/// Analyze imports using AST parser with regex fallback
pub fn analyze_imports(source: &str, file_path: Option<&Path>) -> PluginResult<ImportGraph> {
    let imports = parse_with_fallback(
        || parse_go_imports_ast(source),      // Primary: accurate AST
        || parse_go_imports_regex(source),    // Fallback: regex
        "Go import parsing"
    )?;

    Ok(ImportGraphBuilder::new("go")
        .with_source_file(file_path)
        .with_imports(imports)
        .extract_external_dependencies(is_external_dependency)
        .build())
}

/// Primary parser: spawn Go subprocess with embedded AST tool
fn parse_go_imports_ast(source: &str) -> Result<Vec<ImportInfo>, PluginError> {
    const AST_TOOL_GO: &str = include_str!("../resources/ast_tool.go");

    let tool = SubprocessAstTool::new("go")
        .with_embedded_str(AST_TOOL_GO)
        .with_temp_filename("ast_tool.go")
        .with_args(vec!["run".to_string(), "{script}".to_string(), "analyze-imports".to_string()]);

    run_ast_tool(tool, source)
}

/// Fallback parser: regex-based (when Go runtime unavailable)
fn parse_go_imports_regex(source: &str) -> PluginResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    // ... regex parsing logic ...
    Ok(imports)
}
```

**Benefits:**
- **High accuracy**: Native language parser understands all edge cases
- **Graceful degradation**: Works in environments without language runtime
- **No boilerplate**: `cb-lang-common` handles subprocess lifecycle

#### Pattern B: Native Rust Parsing (Rust, simple languages)

**Example** (`cb-lang-rust/src/parser.rs`):

```rust
use syn::{File, Item, ItemUse};
use cb_plugin_api::{PluginResult, Symbol, SymbolKind};

pub fn extract_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    let ast: File = syn::parse_file(source)
        .map_err(|e| PluginError::parse(format!("Failed to parse Rust: {}", e)))?;

    let mut symbols = Vec::new();

    for item in ast.items {
        match item {
            Item::Fn(func) => {
                symbols.push(Symbol {
                    name: func.sig.ident.to_string(),
                    kind: SymbolKind::Function,
                    location: /* ... */,
                    documentation: extract_doc_comment(&func.attrs),
                });
            }
            Item::Struct(s) => { /* ... */ }
            Item::Enum(e) => { /* ... */ }
            _ => {}
        }
    }

    Ok(symbols)
}
```

**Use when:**
- Pure Rust parser crates exist (e.g., `syn` for Rust, `tree-sitter` for others)
- You want zero subprocess overhead
- Target language is simple enough for regex

---

### Step 4: Implement Manifest Module

**File**: `cb-lang-{language}/src/manifest.rs`

Parse your language's manifest file (e.g., `package.json`, `Cargo.toml`, `go.mod`).

**Example** (`cb-lang-typescript/src/manifest.rs`):

```rust
use cb_plugin_api::{ManifestData, Dependency, DependencySource, PluginResult};
use cb_lang_common::read_manifest;  // Standardized file I/O
use serde_json::Value;

pub async fn load_package_json(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;  // Use cb-lang-common!

    let json: Value = serde_json::from_str(&content)
        .map_err(|e| PluginError::manifest(format!("Invalid JSON: {}", e)))?;

    let name = json["name"]
        .as_str()
        .ok_or_else(|| PluginError::manifest("Missing 'name' field"))?
        .to_string();

    let version = json["version"]
        .as_str()
        .unwrap_or("0.0.0")
        .to_string();

    let dependencies = extract_dependencies(&json, "dependencies");
    let dev_dependencies = extract_dependencies(&json, "devDependencies");

    Ok(ManifestData {
        name,
        version,
        dependencies,
        dev_dependencies,
        raw_data: json,
    })
}

fn extract_dependencies(json: &Value, key: &str) -> Vec<Dependency> {
    json.get(key)
        .and_then(|v| v.as_object())
        .map(|deps| {
            deps.iter()
                .map(|(name, version)| Dependency {
                    name: name.clone(),
                    source: DependencySource::Version(
                        version.as_str().unwrap_or("*").to_string()
                    ),
                })
                .collect()
        })
        .unwrap_or_default()
}
```

**Key patterns:**

- **Always use `cb_lang_common::read_manifest()`** instead of raw `fs::read_to_string()`
- Extract name, version, dependencies, dev_dependencies
- Store raw parsed data in `raw_data` field for advanced operations
- Use `PluginError::manifest()` for errors

---

### Step 5: Implement ImportSupport Trait (Optional)

**File**: `cb-lang-{language}/src/import_support.rs`

**Example** (`cb-lang-python/src/import_support.rs`):

```rust
use cb_plugin_api::{ImportSupport, ModuleReference, ReferenceKind};
use std::path::Path;

pub struct PythonImportSupport;

impl ImportSupport for PythonImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Match: import foo
            if line.starts_with("import ") {
                if let Some(module) = line.strip_prefix("import ") {
                    let module = module.split_whitespace().next().unwrap_or("");
                    imports.push(module.to_string());
                }
            }

            // Match: from foo import bar
            if line.starts_with("from ") {
                if let Some(rest) = line.strip_prefix("from ") {
                    if let Some(module) = rest.split_whitespace().next() {
                        imports.push(module.to_string());
                    }
                }
            }
        }

        imports
    }

    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        // Convert paths to Python module names
        let old_module = path_to_module(old_path);
        let new_module = path_to_module(new_path);

        self.rewrite_imports_for_rename(content, &old_module, &new_module)
    }

    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        let mut result = String::new();
        let mut count = 0;

        for line in content.lines() {
            if line.contains(&format!("import {}", old_name)) {
                result.push_str(&line.replace(old_name, new_name));
                count += 1;
            } else if line.contains(&format!("from {}", old_name)) {
                result.push_str(&line.replace(old_name, new_name));
                count += 1;
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }

        (result, count)
    }

    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
    ) -> Vec<ModuleReference> {
        let mut references = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if line.contains(&format!("import {}", module_to_find))
                || line.contains(&format!("from {}", module_to_find)) {
                references.push(ModuleReference {
                    line: line_num + 1,
                    column: 0,
                    length: line.len(),
                    text: line.to_string(),
                    kind: ReferenceKind::Declaration,
                });
            }
        }

        references
    }
}
```

---

### Step 6: Implement WorkspaceSupport Trait (Optional)

**File**: `cb-lang-{language}/src/workspace_support.rs`

**Example** (`cb-lang-rust/src/workspace_support.rs`):

```rust
use cb_plugin_api::WorkspaceSupport;
use toml_edit::{Document, Item, Array};

pub struct RustWorkspaceSupport;

impl WorkspaceSupport for RustWorkspaceSupport {
    fn is_workspace_manifest(&self, content: &str) -> bool {
        content.contains("[workspace]")
    }

    fn add_workspace_member(
        &self,
        content: &str,
        member_path: &str,
    ) -> Result<String, PluginError> {
        let mut doc = content.parse::<Document>()
            .map_err(|e| PluginError::manifest(format!("Invalid TOML: {}", e)))?;

        // Get or create workspace.members array
        let workspace = doc["workspace"]
            .or_insert(Item::Table(Default::default()));

        let members = workspace["members"]
            .or_insert(Item::Value(toml_edit::Value::Array(Array::new())));

        if let Some(arr) = members.as_array_mut() {
            arr.push(member_path);
        }

        Ok(doc.to_string())
    }

    fn remove_workspace_member(
        &self,
        content: &str,
        member_path: &str,
    ) -> Result<String, PluginError> {
        let mut doc = content.parse::<Document>()
            .map_err(|e| PluginError::manifest(format!("Invalid TOML: {}", e)))?;

        if let Some(arr) = doc["workspace"]["members"].as_array_mut() {
            arr.retain(|item| item.as_str() != Some(member_path));
        }

        Ok(doc.to_string())
    }

    fn merge_dependencies(
        &self,
        source_content: &str,
        target_content: &str,
    ) -> Result<String, PluginError> {
        // Parse both manifests
        let source_doc: Document = source_content.parse()?;
        let mut target_doc: Document = target_content.parse()?;

        // Copy dependencies from source to target
        if let Some(source_deps) = source_doc["dependencies"].as_table() {
            let target_deps = target_doc["dependencies"]
                .or_insert(Item::Table(Default::default()))
                .as_table_mut()
                .unwrap();

            for (name, value) in source_deps {
                target_deps.insert(name, value.clone());
            }
        }

        Ok(target_doc.to_string())
    }
}
```

---

### Step 7: Add Tests

**Minimum 30+ tests recommended** covering:

1. **Parser tests** - Symbol extraction edge cases
2. **Import tests** - All import styles your language supports
3. **Manifest tests** - Valid and invalid manifests
4. **Refactoring tests** - Import rewriting, workspace operations

**Example** (`cb-lang-python/src/lib.rs:173-286`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_python_plugin_basic() {
        let plugin = PythonPlugin::new();
        assert_eq!(plugin.metadata().name, "Python");
        assert_eq!(plugin.metadata().extensions, &["py"]);
    }

    #[tokio::test]
    async fn test_python_plugin_parse() {
        let plugin = PythonPlugin::new();

        let source = r#"
import os
from pathlib import Path

CONSTANT = 42

def hello():
    print('Hello, world!')

class MyClass:
    pass
"#;

        let result = plugin.parse(source).await;
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert!(!parsed.symbols.is_empty());

        // Verify specific symbols
        let has_function = parsed.symbols.iter()
            .any(|s| s.name == "hello" && s.kind == SymbolKind::Function);
        let has_class = parsed.symbols.iter()
            .any(|s| s.name == "MyClass" && s.kind == SymbolKind::Class);

        assert!(has_function);
        assert!(has_class);
    }

    #[test]
    fn test_python_capabilities() {
        let plugin = PythonPlugin::new();
        let caps = plugin.capabilities();

        assert!(caps.imports, "Python should support imports");
        assert!(caps.workspace, "Python should support workspace");
    }
}
```

**Run tests:**

```bash
# Single plugin tests
cargo nextest run -p cb-lang-python

# With output
cargo nextest run -p cb-lang-python --no-capture

# All language plugin tests
cargo nextest run --workspace --lib
```

---

## Using cb-lang-common Utilities

The `cb-lang-common` crate provides **16 utility modules** to reduce boilerplate. See **[CB_LANG_COMMON.md](CB_LANG_COMMON.md)** for complete reference.

### Essential Utilities (Always Use These)

#### 1. SubprocessAstTool - Spawn External Parsers

**When to use**: Python, Node, Go, Java parsers

**Example** (`cb-lang-typescript/src/parser.rs:53-62`):

```rust
use cb_lang_common::{SubprocessAstTool, run_ast_tool};

const AST_TOOL_JS: &str = include_str!("../resources/ast_tool.js");

fn parse_imports_ast(source: &str) -> Result<Vec<ImportInfo>, PluginError> {
    let tool = SubprocessAstTool::new("node")
        .with_embedded_str(AST_TOOL_JS)
        .with_temp_filename("ast_tool.js")
        .with_arg("analyze-imports");

    run_ast_tool(tool, source)  // Returns deserialized JSON
}
```

**Benefits:**
- Automatic temp file creation and cleanup
- Stdin/stdout handling
- Process lifecycle management
- JSON deserialization

#### 2. ImportGraphBuilder - Build ImportGraph

**When to use**: ALWAYS when building ImportGraph

**Example** (`cb-lang-python/src/parser.rs:35-48`):

```rust
use cb_lang_common::ImportGraphBuilder;

pub fn analyze_imports(source: &str, file_path: Option<&Path>) -> PluginResult<ImportGraph> {
    let imports = parse_python_imports(source)?;

    Ok(ImportGraphBuilder::new("python")
        .with_source_file(file_path)
        .with_imports(imports)
        .extract_external_dependencies(|path| !path.starts_with('.'))
        .with_parser_version("0.1.0-plugin")
        .build())
}
```

**Benefits:**
- Automatic timestamp generation
- Consistent metadata format
- External dependency detection
- Builder pattern (fluent API)

#### 3. parse_with_fallback - Resilient Parsing

**When to use**: Subprocess AST with regex fallback

**Example** (`cb-lang-go/src/parser.rs:11-16`):

```rust
use cb_lang_common::parse_with_fallback;

let imports = parse_with_fallback(
    || parse_go_imports_ast(source),      // Try AST first
    || parse_go_imports_regex(source),    // Fallback to regex
    "Go import parsing"
)?;
```

**Benefits:**
- Automatic error logging
- Structured logging output
- Zero boilerplate

#### 4. read_manifest - Standardized File I/O

**When to use**: ALWAYS when reading manifest files

**Example** (`cb-lang-rust/src/manifest.rs`):

```rust
use cb_lang_common::read_manifest;

pub async fn load_cargo_toml(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;  // Standardized error handling

    let parsed: CargoToml = toml::from_str(&content)
        .map_err(|e| PluginError::manifest(format!("Invalid TOML: {}", e)))?;

    // ... rest of parsing
}
```

**Benefits:**
- Consistent error messages
- Structured logging
- `PluginError::manifest()` errors

#### 5. LineExtractor - Extract Source Lines

**When to use**: Refactoring operations, indentation detection

**Example**:

```rust
use cb_lang_common::LineExtractor;

// Get indentation of a line
let indent = LineExtractor::get_indentation_str(source, 42);

// Extract specific line
let line = LineExtractor::extract_line(source, 10)?;

// Replace a range
let new_source = LineExtractor::replace_range(
    source,
    CodeRange::from_lines(10, 15),
    "new code here"
);
```

### High ROI Utilities (Use When Applicable)

#### ErrorBuilder - Rich Error Context

**Example** (`cb-lang-common/src/error_helpers.rs:22-149`):

```rust
use cb_lang_common::ErrorBuilder;

let error = ErrorBuilder::parse("Invalid syntax")
    .with_path(&file_path)
    .with_line(42)
    .with_column(10)
    .with_source_snippet(line.trim())
    .build();
```

#### parse_import_alias - Parse "foo as bar"

**Example**:

```rust
use cb_lang_common::parse_import_alias;

let (name, alias) = parse_import_alias("foo as bar");
assert_eq!(name, "foo");
assert_eq!(alias, Some("bar".to_string()));
```

---

## Common Patterns

### Pattern 1: Subprocess-based AST Parsing

**Full example from TypeScript plugin:**

```rust
// File: cb-lang-typescript/src/parser.rs

use cb_lang_common::{SubprocessAstTool, run_ast_tool, parse_with_fallback, ImportGraphBuilder};
use serde::Deserialize;

// Define AST tool output structure
#[derive(Deserialize)]
struct TsImportInfo {
    module_path: String,
    import_type: String,
    named_imports: Vec<TsNamedImport>,
    default_import: Option<String>,
    type_only: bool,
}

#[derive(Deserialize)]
struct TsNamedImport {
    name: String,
    alias: Option<String>,
}

pub fn analyze_imports(source: &str, file_path: Option<&Path>) -> PluginResult<ImportGraph> {
    let imports = parse_with_fallback(
        || parse_imports_ast(source),
        || parse_imports_regex(source),
        "TypeScript import parsing"
    )?;

    Ok(ImportGraphBuilder::new("typescript")
        .with_source_file(file_path)
        .with_imports(imports)
        .extract_external_dependencies(|path| {
            !path.starts_with("./") && !path.starts_with("../")
        })
        .build())
}

fn parse_imports_ast(source: &str) -> PluginResult<Vec<ImportInfo>> {
    const AST_TOOL_JS: &str = include_str!("../resources/ast_tool.js");

    let tool = SubprocessAstTool::new("node")
        .with_embedded_str(AST_TOOL_JS)
        .with_temp_filename("ast_tool.js")
        .with_arg("analyze-imports");

    let ts_imports: Vec<TsImportInfo> = run_ast_tool(tool, source)?;

    // Convert to protocol ImportInfo
    Ok(ts_imports.into_iter().map(convert_import).collect())
}

fn parse_imports_regex(source: &str) -> PluginResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    let import_re = regex::Regex::new(r#"^import\s+.*?from\s+['"]([^'"]+)['"]"#)?;

    for (line_num, line) in source.lines().enumerate() {
        if let Some(caps) = import_re.captures(line) {
            let module_path = caps[1].to_string();
            imports.push(ImportInfo {
                module_path,
                import_type: ImportType::EsModule,
                // ... rest of fields
            });
        }
    }

    Ok(imports)
}
```

---

### Pattern 2: Native Rust Parsing

**Full example from Rust plugin:**

```rust
// File: cb-lang-rust/src/parser.rs

use syn::{File, Item, ItemFn, ItemStruct, ItemEnum, Attribute};
use cb_plugin_api::{Symbol, SymbolKind, PluginResult, PluginError};
use cb_protocol::SourceLocation;

pub fn extract_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    // Parse with syn
    let ast: File = syn::parse_file(source)
        .map_err(|e| PluginError::parse(format!("Failed to parse Rust: {}", e)))?;

    let mut symbols = Vec::new();

    for item in ast.items {
        match item {
            Item::Fn(func) => symbols.push(extract_function(&func)),
            Item::Struct(s) => symbols.push(extract_struct(&s)),
            Item::Enum(e) => symbols.push(extract_enum(&e)),
            _ => {}
        }
    }

    Ok(symbols)
}

fn extract_function(func: &ItemFn) -> Symbol {
    Symbol {
        name: func.sig.ident.to_string(),
        kind: SymbolKind::Function,
        location: extract_location(&func.sig.ident),
        documentation: extract_doc_comments(&func.attrs),
    }
}

fn extract_doc_comments(attrs: &[Attribute]) -> Option<String> {
    let doc_lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                attr.meta.require_name_value()
                    .ok()
                    .and_then(|nv| {
                        if let syn::Expr::Lit(lit) = &nv.value {
                            if let syn::Lit::Str(s) = &lit.lit {
                                return Some(s.value());
                            }
                        }
                        None
                    })
            } else {
                None
            }
        })
        .collect();

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}
```

---

### Pattern 3: Import Rewriting

**Full example from Go plugin:**

```rust
// File: cb-lang-go/src/import_support.rs

use cb_plugin_api::ImportSupport;
use regex::Regex;

pub struct GoImportSupport;

impl ImportSupport for GoImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_module: &str,
        new_module: &str,
    ) -> (String, usize) {
        let mut result = String::new();
        let mut count = 0;

        // Regex to match: import "old/module/path"
        let import_re = Regex::new(&format!(r#"import\s+"{}""#, regex::escape(old_module)))
            .unwrap();

        for line in content.lines() {
            if import_re.is_match(line) {
                let new_line = line.replace(
                    &format!(r#""{}""#, old_module),
                    &format!(r#""{}""#, new_module)
                );
                result.push_str(&new_line);
                count += 1;
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }

        (result, count)
    }
}
```

---

## Testing Guide

### Test Structure

Every plugin should have:

1. **Unit tests** in `src/parser.rs`, `src/manifest.rs` - Test individual functions
2. **Integration tests** in `src/lib.rs` - Test full plugin lifecycle
3. **Edge case tests** - Empty input, malformed code, Unicode, etc.

### Test Coverage Goals

- **Minimum 30 tests** total
- **Symbol extraction**: 10+ tests (different symbol types, edge cases)
- **Import parsing**: 10+ tests (all import styles, aliases, errors)
- **Manifest parsing**: 5+ tests (valid, invalid, missing fields)
- **Refactoring**: 5+ tests (import rewriting, workspace ops)

### Example Test Suite

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // === Basic Plugin Tests ===

    #[tokio::test]
    async fn test_metadata() {
        let plugin = MyPlugin::new();
        assert_eq!(plugin.metadata().name, "MyLanguage");
        assert_eq!(plugin.metadata().extensions, &["ml"]);
    }

    #[test]
    fn test_capabilities() {
        let plugin = MyPlugin::new();
        let caps = plugin.capabilities();
        assert!(caps.imports);
        assert!(caps.workspace);
    }

    // === Symbol Extraction Tests ===

    #[tokio::test]
    async fn test_parse_function() {
        let plugin = MyPlugin::new();
        let source = "function hello() { return 'world'; }";

        let result = plugin.parse(source).await.unwrap();

        assert_eq!(result.symbols.len(), 1);
        assert_eq!(result.symbols[0].name, "hello");
        assert_eq!(result.symbols[0].kind, SymbolKind::Function);
    }

    #[tokio::test]
    async fn test_parse_class() {
        let source = "class MyClass { method() {} }";
        let result = plugin.parse(source).await.unwrap();

        let class_sym = result.symbols.iter()
            .find(|s| s.kind == SymbolKind::Class)
            .unwrap();
        assert_eq!(class_sym.name, "MyClass");
    }

    #[tokio::test]
    async fn test_parse_empty_file() {
        let result = plugin.parse("").await.unwrap();
        assert!(result.symbols.is_empty());
    }

    #[tokio::test]
    async fn test_parse_unicode() {
        let source = "function 日本語() { /* Unicode test */ }";
        let result = plugin.parse(source).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_parse_syntax_error() {
        let source = "function incomplete {";
        let result = plugin.parse(source).await;
        assert!(result.is_err());
    }

    // === Import Tests ===

    #[test]
    fn test_import_parsing_basic() {
        let imports = parse_imports("import foo").unwrap();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module_path, "foo");
    }

    #[test]
    fn test_import_parsing_with_alias() {
        let imports = parse_imports("import foo as bar").unwrap();
        assert_eq!(imports[0].module_path, "foo");
        assert_eq!(imports[0].default_import, Some("bar".to_string()));
    }

    #[test]
    fn test_import_parsing_multiple() {
        let source = "import foo\nimport bar\nimport baz";
        let imports = parse_imports(source).unwrap();
        assert_eq!(imports.len(), 3);
    }

    // === Manifest Tests ===

    #[tokio::test]
    async fn test_manifest_valid() {
        let manifest = r#"
            name = "test-package"
            version = "1.0.0"
        "#;

        let temp_file = write_temp_manifest(manifest);
        let result = plugin.analyze_manifest(&temp_file).await.unwrap();

        assert_eq!(result.name, "test-package");
        assert_eq!(result.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_manifest_missing_name() {
        let manifest = r#"version = "1.0.0""#;
        let temp_file = write_temp_manifest(manifest);

        let result = plugin.analyze_manifest(&temp_file).await;
        assert!(result.is_err());
    }

    // === Refactoring Tests ===

    #[test]
    fn test_rewrite_imports() {
        let source = "import old_module\nimport other";
        let (result, count) = rewrite_imports_for_rename(
            source, "old_module", "new_module"
        );

        assert_eq!(count, 1);
        assert!(result.contains("import new_module"));
        assert!(result.contains("import other"));
    }
}

// Helper functions
fn write_temp_manifest(content: &str) -> PathBuf {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("manifest.toml");
    std::fs::write(&path, content).unwrap();
    path
}
```

### Running Tests

```bash
# Run all plugin tests
cargo nextest run -p cb-lang-mylanguage

# Run with output
cargo nextest run -p cb-lang-mylanguage --no-capture

# Run specific test
cargo nextest run -p cb-lang-mylanguage test_parse_function

# Run all language plugin tests
cargo nextest run --workspace --lib

# Run with verbose logging
RUST_LOG=debug cargo nextest run -p cb-lang-mylanguage
```

---

## Checklist Before Submitting

Use this checklist to ensure your plugin is complete:

### Implementation Checklist

- [ ] **Core trait implemented**
  - [ ] `metadata()` returns correct language info
  - [ ] `parse()` extracts all major symbol types
  - [ ] `analyze_manifest()` parses manifest correctly
  - [ ] `capabilities()` returns accurate flags
  - [ ] `as_any()` implemented

- [ ] **Import support** (if `capabilities().imports = true`)
  - [ ] `parse_imports()` handles all import styles
  - [ ] `rewrite_imports_for_move()` updates relative paths
  - [ ] `rewrite_imports_for_rename()` renames modules
  - [ ] `find_module_references()` finds all references

- [ ] **Workspace support** (if `capabilities().workspace = true`)
  - [ ] `is_workspace_manifest()` detects workspace files
  - [ ] `add_workspace_member()` adds members correctly
  - [ ] `remove_workspace_member()` removes members
  - [ ] `merge_dependencies()` combines deps

### Quality Checklist

- [ ] **Uses cb-lang-common utilities**
  - [ ] `ImportGraphBuilder` for building ImportGraph
  - [ ] `SubprocessAstTool` for external parsers (if applicable)
  - [ ] `read_manifest()` for file I/O
  - [ ] `parse_with_fallback()` for resilient parsing
  - [ ] `ErrorBuilder` for rich error context

- [ ] **Testing**
  - [ ] Minimum 30 tests total
  - [ ] Symbol extraction tests (10+)
  - [ ] Import parsing tests (10+)
  - [ ] Manifest parsing tests (5+)
  - [ ] Refactoring tests (5+)
  - [ ] All tests pass: `cargo nextest run -p cb-lang-{language}`

- [ ] **Logging**
  - [ ] Uses structured logging (key-value format)
  - [ ] No string interpolation in logs
  - [ ] Follows [LOGGING_GUIDELINES.md](/workspace/docs/development/LOGGING_GUIDELINES.md)

- [ ] **Documentation**
  - [ ] Plugin-specific `README.md` with examples
  - [ ] Doc comments on public functions
  - [ ] Examples in doc comments

### Integration Checklist

- [ ] **Registration**
  - [ ] Entry added to `languages.toml`
  - [ ] `cargo build` succeeds
  - [ ] Plugin appears in `./check-features.sh` output

- [ ] **Dependencies**
  - [ ] All dependencies use workspace versions
  - [ ] No unnecessary dependencies
  - [ ] Feature flags configured correctly

---

## Reference Implementations

### Best Examples by Use Case

| Use Case | Best Reference | Location |
|----------|---------------|----------|
| **Subprocess AST (compiled language)** | Go plugin | `/workspace/crates/languages/cb-lang-go/` |
| **Subprocess AST (dynamic language)** | Python plugin | `/workspace/crates/cb-lang-python/` |
| **Subprocess AST (JavaScript ecosystem)** | TypeScript plugin | `/workspace/crates/cb-lang-typescript/` |
| **Native Rust parsing** | Rust plugin | `/workspace/crates/cb-lang-rust/` |
| **ImportSupport implementation** | All plugins | `src/import_support.rs` in any plugin |
| **WorkspaceSupport implementation** | Rust, Go, TypeScript | `src/workspace_support.rs` |
| **Manifest parsing (TOML)** | Rust plugin | `cb-lang-rust/src/manifest.rs` |
| **Manifest parsing (JSON)** | TypeScript plugin | `cb-lang-typescript/src/manifest.rs` |
| **Manifest parsing (custom format)** | Go plugin | `cb-lang-go/src/manifest.rs` (go.mod) |

### Plugin Comparison

| Plugin | Parser Type | Lines of Code | Import Support | Workspace Support | Tests |
|--------|-------------|---------------|----------------|-------------------|-------|
| **Rust** | Native (`syn`) | ~450 | ✅ | ✅ | 30+ |
| **Go** | Subprocess | ~520 | ✅ | ✅ | 35+ |
| **Python** | Subprocess | ~480 | ✅ | ✅ | 32+ |
| **TypeScript** | Subprocess | ~510 | ✅ | ✅ | 33+ |

### Key Files to Study

1. **Start here**: `/workspace/crates/languages/cb-lang-go/src/lib.rs` (cleanest structure)
2. **AST subprocess pattern**: `/workspace/crates/cb-lang-typescript/src/parser.rs`
3. **Import rewriting**: `/workspace/crates/cb-lang-rust/src/import_support.rs`
4. **Workspace operations**: `/workspace/crates/cb-lang-rust/src/workspace_support.rs`
5. **Manifest parsing**: `/workspace/crates/cb-lang-typescript/src/manifest.rs`

---

## Troubleshooting

### Common Issues

#### Issue 1: "Plugin not found during build"

**Symptom**: `cargo build` fails with "no such crate `cb-lang-mylanguage`"

**Solution**:
1. Check `languages.toml` has your entry
2. Run `cargo build` again (build scripts run before compilation)
3. Verify feature flag in root `Cargo.toml`:
   ```toml
   [features]
   lang-mylanguage = ["cb-lang-mylanguage"]
   ```

#### Issue 2: "Subprocess AST tool fails"

**Symptom**: Tests fail with "Failed to spawn python3 subprocess"

**Solution**:
- Ensure runtime (python3, node, go) is installed and in PATH
- Test manually: `python3 resources/ast_tool.py < test.py`
- Add fallback regex parser for environments without runtime
- Check temp file permissions (Linux: `/tmp/`)

#### Issue 3: "ImportGraph has no imports"

**Symptom**: `analyze_imports()` returns empty graph

**Solution**:
- Verify `parse_imports()` is actually being called
- Check regex patterns match your import syntax
- Use `extract_external_dependencies()` with correct detector function
- Add debug logging: `debug!(imports_count = imports.len(), "Parsed imports");`

#### Issue 4: "Tests pass locally but fail in CI"

**Symptom**: Tests work on your machine but fail in GitHub Actions

**Solution**:
- Check if subprocess runtime is available in CI (python3, node, etc.)
- Ensure fallback parser works without runtime
- Use `#[cfg(feature = "integration-tests")]` for tests requiring external tools
- Test with minimal environment: `docker run -it rust:latest bash`

#### Issue 5: "LanguageMetadata constant not found"

**Symptom**: `LanguageMetadata::MYLANGUAGE` doesn't exist

**Solution**:
1. Verify `languages.toml` entry is correct
2. Run `cargo clean` then `cargo build`
3. Check build script output: `cargo build -vv`
4. Build scripts generate this constant at compile time

#### Issue 6: "Import rewriting doesn't preserve non-use content"

**Symptom**: Rewriting changes documentation or comments containing module name

**Solution**:
- Use AST-based rewriting (like Rust plugin with `syn`)
- If using regex, be very specific:
  ```rust
  // ❌ BAD: Matches anywhere in file
  content.replace(old_module, new_module)

  // ✅ GOOD: Only matches import statements
  let import_re = Regex::new(&format!(r#"^import\s+{}""#, old_module))?;
  ```

#### Issue 7: "Workspace operations corrupt manifest"

**Symptom**: `add_workspace_member()` creates invalid TOML/JSON

**Solution**:
- Use proper parser library (`toml_edit`, `serde_json`)
- Don't use string replacement for structured formats
- Validate output before returning:
  ```rust
  let result = doc.to_string();
  // Validate it parses back correctly
  let _test: Document = result.parse()?;
  Ok(result)
  ```

### Getting Help

1. **Check existing plugins** for similar patterns
2. **Read cb-lang-common docs**: [CB_LANG_COMMON.md](CB_LANG_COMMON.md)
3. **Review logging guidelines**: `/workspace/docs/development/LOGGING_GUIDELINES.md`
4. **Ask in discussions** with specific error messages and code snippets

---

## Summary

You now have everything you need to implement a language plugin:

1. **Generate structure**: `./new-lang.sh` (5 minutes)
2. **Implement core trait**: `lib.rs`, `parser.rs`, `manifest.rs` (1-2 days)
3. **Add capability traits**: `import_support.rs`, `workspace_support.rs` (1 day)
4. **Write tests**: Minimum 30 tests (1 day)
5. **Validate**: `./check-features.sh` and manual testing

**Key principles:**

- **Use cb-lang-common utilities** to reduce boilerplate by ~460 lines
- **Follow existing patterns** from Go, Python, TypeScript, or Rust plugins
- **Write comprehensive tests** (30+ tests minimum)
- **Use structured logging** (key-value format, not string interpolation)
- **Implement fallback parsers** for environments without language runtime

**Next steps:**

1. Choose your reference implementation (Go for subprocess, Rust for native)
2. Generate your plugin structure
3. Start with `parse()` method (symbol extraction)
4. Add import support
5. Write tests as you go
6. Submit PR with all checklist items complete

Good luck! 🚀
