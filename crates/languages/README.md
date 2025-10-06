# Language Plugins Guide

This directory contains language-specific plugins for Codebuddy. Each plugin implements the `LanguagePlugin` trait to provide AST parsing, symbol extraction, import analysis, and refactoring support for a specific programming language.

## Quick Start: Implementing a New Language Plugin

### 1. Define Your Plugin Struct

```rust
use cb_plugin_api::{LanguagePlugin, LanguageMetadata, LanguageCapabilities};

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

### Reference Implementations

- **Full example**: See `crates/languages/cb-lang-rust/src/lib.rs` (imports + workspace)
- **Imports only**: See `crates/languages/cb-lang-typescript/src/lib.rs`
- **Minimal**: See `crates/languages/cb-lang-python/src/lib.rs`

## Automated Plugin Generation (Recommended)

### Create a New Language Plugin with Auto-Patching

The `new-lang.sh` script generates a complete plugin structure **and automatically patches all integration points**:

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

#### What Gets Generated & Patched Automatically

**Phase 1: Plugin Files**
- ✅ Complete directory structure (`src/`, `resources/`)
- ✅ `Cargo.toml` with workspace dependencies
- ✅ Modern `lib.rs` implementing `LanguagePlugin` trait
- ✅ Template `parser.rs` and `manifest.rs` files
- ✅ Comprehensive `README.md` with TODOs

**Phase 2: Automatic Integration**
- ✅ Root `Cargo.toml` - Added to workspace dependencies
- ✅ `crates/cb-handlers/Cargo.toml` - Feature gate and dependency
- ✅ `crates/cb-services/src/services/registry_builder.rs` - Plugin registration
- ✅ `crates/cb-core/src/language.rs` - ProjectLanguage enum and detection
- ✅ `crates/cb-plugin-api/src/metadata.rs` - LanguageMetadata constant

**No manual file editing required!** The script handles all integration points automatically.

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
# 1. Scaffold new plugin with auto-patching
cd crates/languages
./new-lang.sh kotlin --manifest "build.gradle.kts" --extensions kt,kts

# Output shows all auto-patched files:
# ✓ Cargo.toml (workspace dependencies)
# ✓ crates/cb-handlers/Cargo.toml (features & dependencies)
# ✓ crates/cb-services/src/services/registry_builder.rs
# ✓ crates/cb-core/src/language.rs (ProjectLanguage enum)
# ✓ crates/cb-plugin-api/src/metadata.rs (LanguageMetadata constant)

# 2. Verify the integration compiles
cargo check -p cb-lang-kotlin

# 3. Implement parsing logic
# Edit: cb-lang-kotlin/src/parser.rs - Add AST parsing and symbol extraction
# Edit: cb-lang-kotlin/src/manifest.rs - Add manifest parsing

# 4. Optionally add capability trait implementations
# Create: cb-lang-kotlin/src/import_support.rs - ImportSupport trait
# Create: cb-lang-kotlin/src/workspace_support.rs - WorkspaceSupport trait
# Update: cb-lang-kotlin/src/lib.rs - Add support fields and override methods

# 5. Run tests
cargo test -p cb-lang-kotlin

# 6. Validate configuration
./check-features.sh

# 7. Test with real projects
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

⚠️ **Not recommended** - The auto-generator handles all integration automatically.

If you must create a plugin manually:

- [ ] Create `crates/languages/cb-lang-{language}/` directory
- [ ] Implement `LanguagePlugin` trait in `src/lib.rs`
- [ ] Implement symbol extraction in `src/parser.rs`
- [ ] Implement manifest parsing in `src/manifest.rs`
- [ ] Add unit tests (minimum 8-12 tests)
- [ ] Add logging using structured key-value format
- [ ] **Add to `crates/cb-core/src/language.rs`** - ProjectLanguage enum variant
- [ ] **Add to `crates/cb-core/src/language.rs`** - Update `as_str()` and `manifest_filename()`
- [ ] **Add to `crates/cb-core/src/language.rs`** - Add detection logic in `detect_project_language()`
- [ ] **Add to `crates/cb-plugin-api/src/metadata.rs`** - LanguageMetadata constant
- [ ] Register plugin in `crates/cb-services/src/services/registry_builder.rs`
- [ ] Add feature flag to workspace `Cargo.toml`
- [ ] Add workspace dependency to root `Cargo.toml`
- [ ] Add dependency to `cb-handlers/Cargo.toml`
- [ ] Add feature gate to `cb-handlers/Cargo.toml`
- [ ] Create plugin-specific `README.md`
- [ ] Test with real projects
- [ ] Document any limitations or fallback behaviors
- [ ] Run `./check-features.sh` to verify all configuration

**Using the auto-generator eliminates all manual integration steps** (checkboxes in bold).
