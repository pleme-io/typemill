# Language Plugins Guide

This directory contains language-specific plugins for Codebuddy. Each plugin implements the `LanguagePlugin` trait to provide AST parsing, symbol extraction, import analysis, and refactoring support for a specific programming language.

## 🚀 Common Utilities (cb-lang-common)

**Before implementing a plugin**, familiarize yourself with `cb-lang-common` - a comprehensive utility crate that reduces boilerplate by ~460 lines per plugin:

### Available Utilities

- **Subprocess utilities**: `SubprocessAstTool`, `run_ast_tool` - spawn external parsers
- **Parsing patterns**: `parse_with_fallback`, `try_parsers` - resilient parsing strategies
- **Error handling**: `ErrorBuilder` - rich error context with file/line info
- **Import utilities**: `ImportGraphBuilder`, `parse_import_alias`, `ExternalDependencyDetector`
- **File I/O**: `read_manifest`, `read_source`, `find_source_files`
- **Location tracking**: `LocationBuilder`, `offset_to_position`
- **Versioning**: `detect_dependency_source`, `parse_git_url`
- **Workspace ops**: `TomlWorkspace`, `JsonWorkspace`
- **Testing**: Test fixture generators and mock utilities

See [cb-lang-common/src/lib.rs](cb-lang-common/src/lib.rs) for complete API documentation.

---

## Quick Start: Implementing a New Language Plugin

### 1. Define Your Plugin Struct

```rust
use cb_plugin_api::{LanguagePlugin, LanguageMetadata, LanguageCapabilities};
use cb_lang_common::SubprocessAstTool;  // Use common utilities!

pub struct MyLanguagePlugin {
    metadata: LanguageMetadata,
    import_support: import_support::MyLanguageImportSupport,  // Optional
}
```

### 2. Implement Core Trait (Required)

```rust
#[async_trait]
impl LanguagePlugin for MyLanguagePlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        // TODO: Implement AST parsing
        todo!()
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        // TODO: Parse manifest file
        todo!()
    }

    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            imports: true,  // Set based on your support
            workspace: false,
        }
    }

    fn import_support(&self) -> Option<&dyn cb_plugin_api::ImportSupport> {
        Some(&self.import_support)  // If you have import support
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

### 3. Implement Capability Traits (Optional)

If your language supports imports, create `src/import_support.rs`:

```rust
use cb_plugin_api::{ImportSupport, PluginResult};

pub struct MyLanguageImportSupport;

impl ImportSupport for MyLanguageImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        // Parse import statements
        vec![]
    }

    fn rewrite_imports_for_rename(&self, content: &str, old: &str, new: &str) -> (String, usize) {
        // Rewrite imports
        (content.to_string(), 0)
    }

    // ... implement other methods
}
```

### Practical Examples Using cb-lang-common

**Example 1: Subprocess AST Parsing**
```rust
use cb_lang_common::{SubprocessAstTool, run_ast_tool, parse_with_fallback};

const PYTHON_AST: &str = include_str!("../resources/ast_tool.py");

pub fn parse_symbols(source: &str) -> PluginResult<Vec<Symbol>> {
    // Primary: Use subprocess AST parser
    let primary = || {
        let tool = SubprocessAstTool::new("python3")
            .with_embedded_str(PYTHON_AST)
            .with_temp_filename("ast_tool.py");
        run_ast_tool(tool, source)
    };

    // Fallback: Use regex parser
    let fallback = || parse_symbols_regex(source);

    parse_with_fallback(primary, fallback, "symbol extraction")
}
```

**Example 2: Error Handling with Context**
```rust
use cb_lang_common::ErrorBuilder;

fn parse_manifest(path: &Path) -> PluginResult<Manifest> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ErrorBuilder::manifest("Failed to read")
            .with_path(path)
            .with_context("io_error", e.to_string())
            .build())?;

    toml::from_str(&content)
        .map_err(|e| ErrorBuilder::manifest("Invalid TOML")
            .with_path(path)
            .with_line(e.line_col().map(|(l, _)| l as u32).unwrap_or(0))
            .build())
}
```

**Example 3: ImportGraph Construction**
```rust
use cb_lang_common::ImportGraphBuilder;

pub fn analyze_imports(source: &str, file_path: &Path) -> PluginResult<ImportGraph> {
    let imports = parse_imports(source)?;

    Ok(ImportGraphBuilder::new("mylang")
        .with_source_file(Some(file_path))
        .with_imports(imports)
        .extract_external_dependencies(|path| {
            !path.starts_with("./") && !path.starts_with("../")
        })
        .build())
}
```

### Reference Implementations

- **Full example**: See `crates/languages/cb-lang-rust/src/lib.rs` (imports + workspace)
- **Subprocess parser**: See `crates/languages/cb-lang-go/src/parser.rs` (fallback pattern)
- **ImportGraph usage**: See `crates/languages/cb-lang-typescript/src/parser.rs`
- **Minimal**: See `crates/languages/cb-lang-python/src/lib.rs`

## Automated Plugin Generation (Recommended)

### How It Works: Build-Time Code Generation

The plugin system uses **build-time code generation** for seamless integration:

1. **Single Source of Truth**: `crates/languages/languages.toml` defines all language metadata
2. **Build Scripts**: `build.rs` files in `cb-core`, `cb-plugin-api`, and `cb-services` read this TOML
3. **Generated Code**: Build scripts auto-generate enums, constants, and registration code
4. **Automatic Integration**: No manual file patching needed - everything happens at build time

```toml
# languages.toml - Single source of truth
[languages.Rust]
display_name = "Rust"
extensions = ["rs"]
manifest_filename = "Cargo.toml"
source_dir = "src"
entry_point = "lib.rs"
module_separator = "::"
crate_name = "cb-lang-rust"
feature_name = "lang-rust"
```

**Benefits:**
- ✅ No code duplication across 5+ files
- ✅ Build scripts ensure consistency
- ✅ Adding a language = adding one TOML entry
- ✅ Type-safe generated code
- ✅ Eliminates manual synchronization errors

### Create a New Language Plugin

The `new-lang.sh` script generates a complete plugin structure and registers it in `languages.toml`:

```bash
# Basic usage (manifest filename is required)
./new-lang.sh <language-name> --manifest <filename>

# Example: Create a C# plugin
./new-lang.sh csharp --manifest "*.csproj" --extensions cs,csx

# Example: Create a Kotlin plugin
./new-lang.sh kotlin \
  --manifest "build.gradle.kts" \
  --extensions kt,kts \
  --source-dir "src/main/kotlin" \
  --entry-point "Main.kt"

# Dry run to preview changes without modifying files
./new-lang.sh ruby --manifest Gemfile --dry-run
```

#### Available Options

- `--manifest <filename>` - **Required** - Manifest filename (e.g., `pom.xml`, `Gemfile`, `*.csproj`)
- `--extensions <ext1,ext2>` - File extensions (default: language name)
- `--source-dir <dir>` - Source directory (default: `src`)
- `--entry-point <file>` - Entry point filename (default: `main.<ext>`)
- `--module-sep <sep>` - Module separator (default: `.`)
- `--dry-run` - Preview changes without modifying files

#### What Gets Generated

**Phase 1: Plugin Files**
- ✅ Complete directory structure (`src/`, `resources/`)
- ✅ `Cargo.toml` with workspace dependencies
- ✅ Modern `lib.rs` implementing `LanguagePlugin` trait
- ✅ Template `parser.rs` and `manifest.rs` files
- ✅ Comprehensive `README.md` with TODOs

**Phase 2: Registration**
- ✅ Appends language entry to `languages.toml`

**Phase 3: Build-Time Integration** (happens on next `cargo build`)
- ✅ `ProjectLanguage` enum variant generated in `cb-core`
- ✅ `LanguageMetadata` constant generated in `cb-plugin-api`
- ✅ Plugin registration block generated in `cb-services`
- ✅ Feature gate configuration generated

**No manual file patching required!** Just run `cargo build` after the script completes.

### Validate Configuration

```bash
# Check all plugins are correctly registered
./check-features.sh
```

Verifies:
- ✅ Registration in `registry_builder.rs`
- ✅ Feature flags in root `Cargo.toml`
- ✅ Workspace dependencies configured
- ✅ Integration with `cb-handlers/Cargo.toml`
- ✅ No commented-out or incomplete configs

## Plugin Structure

Each language plugin follows this directory structure:

```
crates/languages/cb-lang-{language}/
├── Cargo.toml              # Crate dependencies and metadata
├── README.md               # Plugin-specific documentation
├── resources/              # Optional: embedded tools (e.g., ast_tool.go, ast_tool.js)
│   └── ast_tool.*          # Language-native AST parser subprocess
└── src/
    ├── lib.rs              # Plugin implementation (LanguagePlugin trait)
    ├── parser.rs           # Symbol extraction and import parsing
    └── manifest.rs         # Manifest file handling (Cargo.toml, package.json, go.mod, etc.)
```

## Required Trait Implementation

### Core Trait: `LanguagePlugin`

Located in `crates/cb-plugin-api/src/lib.rs:331-383`

#### Required Methods

```rust
#[async_trait]
impl LanguagePlugin for YourPlugin {
    /// Returns language metadata (name, extensions, manifest filename, etc.)
    fn metadata(&self) -> &LanguageMetadata;

    /// Parses source code into AST and extracts symbols
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource>;

    /// Analyzes manifest file (Cargo.toml, package.json, go.mod, etc.)
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>;

    /// Returns capability flags for optional features
    fn capabilities(&self) -> LanguageCapabilities;

    /// Downcasting support for plugin-specific methods
    fn as_any(&self) -> &dyn std::any::Any;
}
```

#### Plugin-Specific Implementation Methods (Optional)

These methods are **not part of the core trait**. Instead, they are public methods implemented directly on plugin structs. Services access them via downcasting when needed.

**For Import Support** (when `capabilities().imports = true`):

```rust
impl YourPlugin {
    /// Parse imports/use statements from a file
    pub async fn parse_imports(&self, file_path: &Path) -> PluginResult<Vec<String>>;

    /// Rewrite import statements in file content for rename operations
    pub fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        importing_file: &Path,
        project_root: &Path,
        rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)>;

    /// Find all references to a module within file content
    pub fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>>;
}
```

**For Workspace Support** (when `capabilities().workspace = true`):

```rust
impl YourPlugin {
    /// Add a new member to workspace manifest
    pub async fn add_workspace_member(
        &self,
        workspace_content: &str,
        new_member_path: &str,
        workspace_root: &Path,
    ) -> PluginResult<String>;

    /// Check if manifest is a workspace root
    pub async fn is_workspace_manifest(&self, manifest_content: &str) -> PluginResult<bool>;

    /// Remove a member from workspace manifest
    pub async fn remove_workspace_member(
        &self,
        workspace_content: &str,
        member_path: &str,
    ) -> PluginResult<String>;

    /// Merge dependencies from one manifest into another
    pub async fn merge_dependencies(
        &self,
        source_manifest: &str,
        target_manifest: &str,
    ) -> PluginResult<String>;

    /// Update dependency in manifest file
    pub async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_path: Option<&str>,
    ) -> PluginResult<String>;
}
```

**Note:** Metadata fields like `manifest_filename`, `source_dir`, `entry_point`, and `module_separator` are now accessed via `plugin.metadata()` instead of individual trait methods.

## Data Types

### ParsedSource

```rust
pub struct ParsedSource {
    /// Language-specific AST data (JSON Value for flexibility)
    pub data: serde_json::Value,

    /// List of extracted symbols
    pub symbols: Vec<Symbol>,
}
```

### Symbol

```rust
pub struct Symbol {
    /// Symbol name
    pub name: String,

    /// Symbol kind (Function, Class, Struct, Enum, Interface, Variable, Constant, Module, Method, Field)
    pub kind: SymbolKind,

    /// Source location (line, column)
    pub location: SourceLocation,

    /// Optional documentation
    pub documentation: Option<String>,
}
```

### ManifestData

```rust
pub struct ManifestData {
    /// Package name
    pub name: String,

    /// Package version
    pub version: String,

    /// Dependencies
    pub dependencies: Vec<Dependency>,

    /// Dev dependencies
    pub dev_dependencies: Vec<Dependency>,

    /// Raw manifest data (JSON)
    pub raw_data: serde_json::Value,
}
```

### Dependency

```rust
pub struct Dependency {
    /// Dependency name
    pub name: String,

    /// Source (Version, Path, or Git)
    pub source: DependencySource,
}

pub enum DependencySource {
    Version(String),          // e.g., "1.0.0", "^1.2.3"
    Path(String),             // e.g., "../my-crate"
    Git { url: String, rev: Option<String> },
}
```

## Cargo.toml Dependencies

Minimal dependencies for a language plugin:

```toml
[dependencies]
# Codebuddy workspace
cb-plugin-api = { path = "../../cb-plugin-api" }
cb-protocol = { path = "../../cb-protocol" }
cb-core = { path = "../../cb-core" }

# Async
async-trait = { workspace = true }
tokio = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
thiserror = { workspace = true }

# Logging
tracing = { workspace = true }

# Language-specific parsing (examples)
# regex = "1.10"           # Fallback parsing
# tempfile = "3.10"        # For subprocess tools
```

## Plugin Registration

### Centralized Registry Builder

**IMPORTANT**: Plugins are registered in a **single location** via the centralized registry builder at `crates/cb-services/src/services/registry_builder.rs`. This is the ONLY place where plugins should be instantiated for production use.

Add your plugin to `crates/cb-services/src/services/registry_builder.rs:50-90`:

```rust
// Register YourLanguage plugin
#[cfg(feature = "lang-yourlanguage")]
{
    info!(plugin = "yourlanguage", "Registering YourLanguage plugin");
    registry.register(Arc::new(cb_lang_yourlanguage::YourLanguagePlugin::new()));
    plugin_count += 1;
}
```

**Why centralized registration?**
- ✅ Single source of truth for all plugins
- ✅ Easy to add/remove languages (one location)
- ✅ Testable via dependency injection
- ✅ No code duplication across services
- ✅ Services receive registry via constructor injection

**Services receive the registry via dependency injection:**

```rust
use cb_services::build_language_plugin_registry;

// In service initialization:
let plugin_registry = build_language_plugin_registry();
let file_service = FileService::new(
    project_root,
    ast_cache,
    lock_manager,
    operation_queue,
    config,
    plugin_registry.clone(),
);
```

**DO NOT create registries directly in services** - this defeats the purpose of centralized management.

Add feature flag to workspace `Cargo.toml`:

```toml
[features]
default = ["lang-rust", "lang-go", "lang-typescript", "lang-yourlanguage"]
lang-yourlanguage = ["cb-lang-yourlanguage"]

[workspace.dependencies]
cb-lang-yourlanguage = { path = "crates/languages/cb-lang-yourlanguage" }
```

Add dependency to `crates/cb-handlers/Cargo.toml`:

```toml
[dependencies]
cb-lang-yourlanguage = { workspace = true, optional = true }

[features]
lang-yourlanguage = ["dep:cb-lang-yourlanguage"]
```

## Implementation Approaches

### Dual-Mode Pattern (Recommended)

Use a native parser subprocess for accuracy with regex fallback for environments without the language runtime:

**AST Mode (Primary)**
- Embed `resources/ast_tool.*` (Go, JavaScript, Python script)
- Spawn subprocess to parse using native language parser
- Return accurate symbols and imports

**Regex Mode (Fallback)**
- Use regex patterns for basic import detection
- Return empty or minimal symbols when native parser unavailable

**Examples:** `cb-lang-go`, `cb-lang-typescript`

### Pure Rust Parser

Use Rust parser crates for the language (e.g., `syn` for Rust):

**Advantages**
- No subprocess overhead
- Works in all environments
- Fast and memory-safe

**Examples:** `cb-lang-rust`

## Logging Standards

Follow structured logging (see `docs/development/LOGGING_GUIDELINES.md`):

```rust
// ✅ Correct - structured key-value format
debug!(module = %module_name, symbols_count = symbols.len(), "Parsed source");
warn!(dependency = %dep_name, "Dependency not found in manifest");
error!(error = %e, path = %path.display(), "Failed to parse manifest");

// ❌ Incorrect - string interpolation
error!("Failed to parse {} at {}", path, e);
```

## Testing

Each plugin should include:

1. **Unit tests** in `src/parser.rs`, `src/manifest.rs`
2. **Integration tests** for full plugin lifecycle

```bash
# Run plugin tests
cargo test -p cb-lang-{language}

# Run with output
cargo test -p cb-lang-{language} -- --nocapture
```

## Reference Implementations

- **Rust Plugin** (`cb-lang-rust`): Pure Rust parser using `syn` crate
- **Go Plugin** (`cb-lang-go`): Dual-mode with embedded `ast_tool.go` subprocess
- **TypeScript Plugin** (`cb-lang-typescript`): Dual-mode with embedded `ast_tool.js` subprocess

Read these plugins' README.md files for language-specific implementation details.

## Development Workflow

### Automated Approach (Recommended)

```bash
# 1. Scaffold new plugin and register in languages.toml
cd crates/languages
./new-lang.sh kotlin --manifest "build.gradle.kts" --extensions kt,kts

# Output shows:
# ✓ Created plugin directory: crates/languages/cb-lang-kotlin
# ✓ Generated lib.rs, parser.rs, manifest.rs, README.md
# ✓ Registered Kotlin in languages.toml

# 2. Build workspace to generate integration code
cd ../..
cargo build --features lang-kotlin

# Build scripts auto-generate:
# - ProjectLanguage::Kotlin enum variant (cb-core)
# - LanguageMetadata::KOTLIN constant (cb-plugin-api)
# - Plugin registration block (cb-services)

# 3. Verify the plugin compiles
cargo check -p cb-lang-kotlin

# 4. Implement parsing logic
# Edit: cb-lang-kotlin/src/parser.rs - Add AST parsing and symbol extraction
# Edit: cb-lang-kotlin/src/manifest.rs - Add manifest parsing

# 5. Optionally add capability trait implementations
# Create: cb-lang-kotlin/src/import_support.rs - ImportSupport trait
# Create: cb-lang-kotlin/src/workspace_support.rs - WorkspaceSupport trait
# Update: cb-lang-kotlin/src/lib.rs - Add support fields and override methods

# 6. Run tests
cargo test -p cb-lang-kotlin

# 7. Validate configuration
cd crates/languages
./check-features.sh

# 8. Test with real projects
cd ../..
cargo build --features lang-kotlin
```

### What to Implement (After Auto-Generation)

The generator creates stub implementations that return empty data. You need to implement:

**1. Parser (`src/parser.rs`)**
- Choose parser approach (Pure Rust crate, subprocess, or regex)
- Extract symbols (functions, classes, structs, etc.)
- Return `ParsedSource` with AST data and symbols

**2. Manifest (`src/manifest.rs`)**
- Parse language-specific manifest format
- Extract project name, version, and dependencies
- Return `ManifestData` structure

**3. Capabilities (Optional)**
- Implement `ImportSupport` trait for import analysis
- Implement `WorkspaceSupport` trait for workspace operations
- Update `capabilities()` to return `true` for implemented features

**4. Tests**
- Add unit tests for parser edge cases
- Add manifest parsing tests
- Test capability implementations

### Manual Checklist (If Not Using Auto-Generator)

⚠️ **Not recommended** - The auto-generator handles plugin creation and registration automatically.

If you must create a plugin manually:

**Plugin Implementation:**
- [ ] Create `crates/languages/cb-lang-{language}/` directory
- [ ] Implement `LanguagePlugin` trait in `src/lib.rs`
- [ ] Implement symbol extraction in `src/parser.rs`
- [ ] Implement manifest parsing in `src/manifest.rs`
- [ ] Add unit tests (minimum 8-12 tests)
- [ ] Add logging using structured key-value format
- [ ] Create plugin-specific `README.md`

**Registration (Build-Time Generation):**
- [ ] **Add entry to `crates/languages/languages.toml`** - Single TOML entry with all metadata
- [ ] Run `cargo build` to auto-generate integration code:
  - `ProjectLanguage` enum variant (cb-core)
  - `LanguageMetadata` constant (cb-plugin-api)
  - Plugin registration block (cb-services)

**Testing:**
- [ ] Test with real projects
- [ ] Document any limitations or fallback behaviors
- [ ] Run `./check-features.sh` to verify all configuration

**Note:** The auto-generator and build-time code generation system handle all integration automatically. Simply add one entry to `languages.toml` and run `cargo build`.
