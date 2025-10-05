# Language Plugins Guide

This directory contains language-specific plugins for Codebuddy. Each plugin implements the `LanguageIntelligencePlugin` trait to provide AST parsing, symbol extraction, import analysis, and refactoring support for a specific programming language.

## Plugin Structure

Each language plugin follows this directory structure:

```
crates/languages/cb-lang-{language}/
├── Cargo.toml              # Crate dependencies and metadata
├── README.md               # Plugin-specific documentation
├── resources/              # Optional: embedded tools (e.g., ast_tool.go, ast_tool.js)
│   └── ast_tool.*          # Language-native AST parser subprocess
└── src/
    ├── lib.rs              # Plugin implementation (LanguageIntelligencePlugin trait)
    ├── parser.rs           # Symbol extraction and import parsing
    └── manifest.rs         # Manifest file handling (Cargo.toml, package.json, go.mod, etc.)
```

## Required Trait Implementation

### Core Trait: `LanguageIntelligencePlugin`

Located in `crates/cb-plugin-api/src/lib.rs:319-562`

#### Required Methods

```rust
#[async_trait]
impl LanguageIntelligencePlugin for YourPlugin {
    /// Returns language name (e.g., "Rust", "Go", "TypeScript")
    fn name(&self) -> &'static str;

    /// Returns file extensions (e.g., vec!["rs"], vec!["go"], vec!["ts", "tsx", "js", "jsx"])
    fn file_extensions(&self) -> Vec<&'static str>;

    /// Parses source code into AST and extracts symbols
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource>;

    /// Analyzes manifest file (Cargo.toml, package.json, go.mod, etc.)
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>;
}
```

#### Refactoring Methods (Optional but Recommended)

These methods enable file/directory rename operations and import rewriting:

```rust
    /// Get manifest filename (e.g., "Cargo.toml", "package.json", "go.mod")
    fn manifest_filename(&self) -> &'static str;

    /// Get source directory name (e.g., "src" for Rust/TS, "" for Python)
    fn source_dir(&self) -> &'static str;

    /// Get entry point filename (e.g., "lib.rs", "index.ts", "__init__.py")
    fn entry_point(&self) -> &'static str;

    /// Get module path separator (e.g., "::" for Rust, "." for Python/TS/Go)
    fn module_separator(&self) -> &'static str;

    /// Locate module files given a module path within a package
    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> PluginResult<Vec<PathBuf>>;

    /// Parse imports/use statements from a file
    async fn parse_imports(&self, file_path: &Path) -> PluginResult<Vec<String>>;

    /// Generate manifest file content for a new package
    fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String;

    /// Update import statement from internal to external
    fn rewrite_import(&self, old_import: &str, new_package_name: &str) -> String;

    /// Rewrite import statements in file content for rename operations
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        importing_file: &Path,
        project_root: &Path,
        rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)>;

    /// Find all references to a module within file content
    fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>>;

    /// Update dependency in manifest file
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_path: Option<&str>,
    ) -> PluginResult<String>;
```

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

Add your plugin to `crates/cb-handlers/src/language_plugin_registry.rs:30-48`:

```rust
// Register YourLanguage plugin
#[cfg(feature = "lang-yourlanguage")]
{
    info!(plugin = "yourlanguage", "Registering YourLanguage plugin");
    registry.register(Arc::new(cb_lang_yourlanguage::YourLanguagePlugin::new()));
}
```

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

## Checklist for New Plugin

- [ ] Create `crates/languages/cb-lang-{language}/` directory
- [ ] Implement `LanguageIntelligencePlugin` trait in `src/lib.rs`
- [ ] Implement symbol extraction in `src/parser.rs`
- [ ] Implement manifest parsing in `src/manifest.rs`
- [ ] Add unit tests (minimum 8-12 tests)
- [ ] Add logging using structured key-value format
- [ ] Register plugin in `language_plugin_registry.rs`
- [ ] Add feature flag to workspace `Cargo.toml`
- [ ] Add dependency to `cb-handlers/Cargo.toml`
- [ ] Create plugin-specific `README.md`
- [ ] Test with real projects
- [ ] Document any limitations or fallback behaviors
