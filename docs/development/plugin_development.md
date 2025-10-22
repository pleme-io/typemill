# Language Plugin Development

Fast reference for implementing new language plugins.

## Quick Start

Creating a new language plugin involves creating a standard Cargo crate and registering it with the `mill-plugin-bundle`.

| Step | Action | Notes |
|------|--------|-------|
| 1. Create Crate | `cargo new --lib crates/cb-lang-mynewlang` | Creates the standard plugin structure |
| 2. Implement Logic | Edit `src/lib.rs` to implement `LanguagePlugin` trait | See reference implementations below |
| 3. Register Plugin | Add to `mill-plugin-bundle/Cargo.toml` and `lib.rs` | Makes plugin discoverable via auto-discovery |
| 4. Build Workspace | `cargo build -p cb-lang-mynewlang` | Verify compilation |
| 5. Run Tests | `cargo nextest run -p cb-lang-mynewlang` | Add tests and verify functionality |

## Plugin Structure

All language plugins are independent crates in the `crates/` directory:

```
crates/
└── cb-lang-{language}/
    ├── Cargo.toml              # Dependencies and metadata
    └── src/
        ├── lib.rs              # Main plugin with LanguagePlugin trait
        ├── parser.rs           # Optional: Symbol extraction logic
        ├── manifest.rs         # Optional: Manifest parsing logic
        ├── import_support.rs   # Optional: Import capability traits
        └── workspace_support.rs # Optional: Workspace capability trait
```

## Core Trait: LanguagePlugin

| Method | Purpose | Required |
|--------|---------|----------|
| `metadata()` | Return language metadata | ✅ |
| `parse()` | Extract symbols from source code | ✅ |
| `analyze_manifest()` | Parse manifest files (package.json, Cargo.toml, etc.) | ✅ |
| `capabilities()` | Declare plugin capabilities (imports, workspace) | ✅ |
| `as_any()` | Enable downcasting | ✅ |

## Optional Capability Traits

**NEW: Segregated Import Traits (Interface Segregation Principle)**

Plugins now implement 5 focused traits instead of one monolithic `ImportSupport`:

### ImportParser (parsing only)
| Method | Purpose |
|--------|---------|
| `parse_imports()` | Extract import paths from content |
| `contains_import()` | Check if content imports a specific module |

### ImportRenameSupport (for file/symbol renames)
| Method | Purpose |
|--------|---------|
| `rewrite_imports_for_rename()` | Update imports when symbols/modules rename |

### ImportMoveSupport (for file moves)
| Method | Purpose |
|--------|---------|
| `rewrite_imports_for_move()` | Update imports when files move |

### ImportMutationSupport (add/remove imports)
| Method | Purpose |
|--------|---------|
| `add_import()` | Add a new import statement |
| `remove_import()` | Remove an existing import |
| `remove_named_import()` | Remove a specific symbol from an import |

### ImportAdvancedSupport (advanced operations)
| Method | Purpose |
|--------|---------|
| `update_import_reference()` | Advanced import reference updates |

**Benefits:**
- Simple plugins (like Markdown) only implement `ImportParser` (2 methods)
- Complex plugins (like Rust) implement all traits as needed
- 60% reduction in required code for simple plugins
- Clear separation of concerns

### WorkspaceSupport (if capabilities().workspace = true)

| Method | Purpose |
|--------|---------|
| `is_workspace_manifest()` | Detect workspace manifest files |
| `add_workspace_member()` | Add member to workspace |
| `remove_workspace_member()` | Remove member from workspace |
| `merge_dependencies()` | Merge dependencies between manifests |

## Plugin Registration (Auto-Discovery)

Codebuddy uses compile-time auto-discovery via the `codebuddy_plugin!` macro and `inventory` crate:

### Step 1: Self-Register Your Plugin

In your plugin's `src/lib.rs`:

```rust
use cb_plugin_api::codebuddy_plugin;

codebuddy_plugin! {
    name: "python",
    extensions: ["py", "pyi"],
    manifest: "pyproject.toml",
    capabilities: PythonPlugin::CAPABILITIES,
    factory: PythonPlugin::new,
    lsp: Some(LspConfig::new("pylsp", &[]))
}
```

### Step 2: Add to Plugin Bundle

Edit `../../crates/mill-plugin-bundle/Cargo.toml`:

```toml
[dependencies]
cb-lang-python = { path = "../cb-lang-python" }
```

Edit `../../crates/mill-plugin-bundle/src/lib.rs`:

```rust
use cb_lang_python::PythonPlugin;

fn _force_plugin_linkage() {
    let _: Option<PythonPlugin> = None;
    // ... existing plugins
}
```

**That's it!** The plugin is now automatically discovered and loaded.

## Parser Patterns

### Pattern A: Subprocess AST (Python, Node, Go, Java)

| Component | Purpose |
|-----------|---------|
| `SubprocessAstTool` | Spawn language-native parser |
| `run_ast_tool()` | Execute and deserialize JSON output |
| `parse_with_fallback()` | Try AST, fall back to regex |
| `ImportGraphBuilder` | Build structured import graph |

**When to use:** High accuracy needed, language runtime available

### Pattern B: Native Rust (Rust, simple languages)

| Component | Purpose |
|-----------|---------|
| Parser crate (`syn`, `tree-sitter`) | Parse directly in Rust |
| Manual AST traversal | Extract symbols |
| No subprocess | Zero overhead |

**When to use:** Rust parser available, performance critical

## cb-lang-common Utilities

Essential utilities to reduce boilerplate (~460 lines saved):

| Utility | Purpose | Always Use? |
|---------|---------|-------------|
| `SubprocessAstTool` | Spawn external parsers | For subprocess parsers |
| `run_ast_tool()` | Execute subprocess + deserialize | For subprocess parsers |
| `ImportGraphBuilder` | Build ImportGraph | ✅ Always |
| `parse_with_fallback()` | Resilient parsing (AST → regex) | For subprocess parsers |
| `read_manifest()` | Standardized file I/O | ✅ Always |
| `LineExtractor` | Extract/replace source lines | For refactoring |
| `ErrorBuilder` | Rich error context | Recommended |
| `parse_import_alias()` | Parse "foo as bar" syntax | If language has aliases |

## Test Coverage Requirements

| Category | Minimum Tests | What to Test |
|----------|---------------|--------------|
| **Symbol extraction** | 10+ | Functions, classes, structs, enums, edge cases, unicode, syntax errors |
| **Import parsing** | 10+ | All import styles, aliases, multiple imports, empty files |
| **Manifest parsing** | 5+ | Valid manifests, invalid manifests, missing fields |
| **Refactoring** | 5+ | Import rewriting, workspace operations |
| **Total** | **30+** | Comprehensive coverage |

## Reference Implementations

| Use Case | Best Reference | Location |
|----------|---------------|----------|
| Native Rust parsing | Rust plugin | `crates/cb-lang-rust/` |
| Simple document plugin | Markdown plugin | `crates/cb-lang-markdown/` |
| Config file plugin | TOML/YAML plugins | `crates/cb-lang-toml/`, `crates/cb-lang-yaml/` |
| **Auto-Discovery Pattern** | **All current plugins** | **`src/lib.rs` - `codebuddy_plugin!` macro** |
| Simple ImportParser only | Markdown plugin | `cb-lang-markdown/src/import_support.rs` |
| Full import trait suite | Rust plugin | `cb-lang-rust/src/import_support.rs` |
| WorkspaceSupport | Rust plugin | `cb-lang-rust/src/workspace_support.rs` |

## Plugin Comparison

| Plugin | Parser Type | LOC | Import Support | Workspace Support | Tests |
|--------|-------------|-----|----------------|-------------------|-------|
| Rust | Native (`syn`) | ~450 | ✅ | ✅ | 30+ |
| Go | Subprocess | ~520 | ✅ | ✅ | 35+ |
| Python | Subprocess | ~480 | ✅ | ✅ | 32+ |
| TypeScript | Subprocess | ~510 | ✅ | ✅ | 33+ |

## Implementation Checklist

### Core Trait
- [ ] `metadata()` returns correct language info
- [ ] `parse()` extracts all major symbol types
- [ ] `analyze_manifest()` parses manifest correctly
- [ ] `capabilities()` returns accurate flags
- [ ] `as_any()` implemented

### Import Support (if capabilities().imports = true)
**Choose traits based on plugin needs:**
- [ ] **ImportParser** (required): `parse_imports()` handles all import styles, `contains_import()` checks for imports
- [ ] **ImportRenameSupport** (if needed): `rewrite_imports_for_rename()` updates imports for renames
- [ ] **ImportMoveSupport** (if needed): `rewrite_imports_for_move()` updates relative paths for file moves
- [ ] **ImportMutationSupport** (if needed): `add_import()`, `remove_import()`, `remove_named_import()` for import mutations
- [ ] **ImportAdvancedSupport** (if needed): `update_import_reference()` for advanced operations

### Workspace Support (if capabilities().workspace = true)
- [ ] `is_workspace_manifest()` detects workspace files
- [ ] `add_workspace_member()` adds members correctly
- [ ] `remove_workspace_member()` removes members
- [ ] `merge_dependencies()` combines deps

### Additional Capability Traits (Optional)
**Choose based on plugin needs:**

**ManifestUpdater** (for dependency management):
- [ ] `manifest_updater()` discovery method implemented
- [ ] `update_dependency()` updates manifest files (Cargo.toml, package.json)

**ModuleLocator** (for module file discovery):
- [ ] `module_locator()` discovery method implemented
- [ ] `locate_module_files()` finds files for module paths

**RefactoringProvider** (for AST refactoring):
- [ ] `refactoring_provider()` discovery method implemented
- [ ] `supports_inline_variable()`, `supports_extract_function()`, `supports_extract_variable()` flags
- [ ] `plan_inline_variable()` generates inline variable refactoring plans
- [ ] `plan_extract_function()` generates extract function refactoring plans
- [ ] `plan_extract_variable()` generates extract variable refactoring plans

### Quality
- [ ] Uses `ImportGraphBuilder` for ImportGraph
- [ ] Uses `SubprocessAstTool` for external parsers (if applicable)
- [ ] Uses `read_manifest()` for file I/O
- [ ] Uses `parse_with_fallback()` for resilient parsing
- [ ] Uses `ErrorBuilder` for rich error context
- [ ] Minimum 30 tests total
- [ ] All tests pass: `cargo nextest run -p cb-lang-{language}`
- [ ] Uses structured logging (key-value format)
- [ ] Plugin-specific README.md with examples

### Integration
- [ ] `codebuddy_plugin!` macro implemented in `src/lib.rs`
- [ ] Added to `mill-plugin-bundle/Cargo.toml` dependencies
- [ ] Added to `mill-plugin-bundle/src/lib.rs` `_force_plugin_linkage()`
- [ ] Workspace builds: `cargo build --workspace`

## Running Tests

| Command | Purpose |
|---------|---------|
| `cargo nextest run -p cb-lang-mylanguage` | Single plugin tests |
| `cargo nextest run -p cb-lang-mylanguage --no-capture` | With output |
| `cargo nextest run -p cb-lang-mylanguage test_parse_function` | Specific test |
| `cargo nextest run --workspace --lib` | All language plugin tests |
| `RUST_LOG=debug cargo nextest run -p cb-lang-mylanguage` | With verbose logging |

## Troubleshooting

| Problem | Cause | Solution |
|---------|-------|----------|
| Plugin not found during build | Not registered | Check `codebuddy_plugin!` macro in src/lib.rs, verify bundle link, check `_force_plugin_linkage()` |
| Plugin not auto-discovered | Linker optimization | Ensure plugin is in `_force_plugin_linkage()` in bundle's lib.rs |
| ImportGraph has no imports | Not being called | Verify `parse_imports()` called, check regex patterns, use debug logging |
| LanguageMetadata constant not found | Build error | Run `cargo clean && cargo build -p cb-lang-yourlang` |
| Import rewriting changes non-imports | Regex too broad | Use AST-based rewriting or specific regex: `^import\s+{}`  |
| Workspace operations corrupt manifest | String manipulation | Use parser library (`toml_edit`, `serde_json`), validate output |
| Tests fail with "plugin not found" | Missing from bundle | Add to `mill-plugin-bundle/Cargo.toml` and rebuild |

## Code Examples

See existing plugins for complete reference implementations:
- **Rust**: `crates/cb-lang-rust/` - Full-featured with all capabilities
- **Markdown**: `crates/cb-lang-markdown/` - Simple plugin with basic import support
- **TOML/YAML**: `crates/cb-lang-toml/`, `crates/cb-lang-yaml/` - Config file plugins
- **TypeScript**: `crates/cb-lang-typescript/` - JavaScript ecosystem plugin

## Plugin Dispatch Patterns

> **📌 IMPORTANT: Capability-Based Dispatch is the ONLY Correct Pattern**
>
> All plugin dispatch in shared code MUST use capability traits. Downcasting and
> cfg guards are strictly forbidden. See warning below for details.

### ✅ Capability-Based Dispatch (REQUIRED - Proposals 05 & 07 Complete)

**Status:** Fully implemented and operational. **This is the only approved dispatch pattern.**

Language-specific operations now use trait-based capability queries with zero cfg guards:

```rust
// Example: Manifest updates using ManifestUpdater capability
let manifest_updater = plugin
    .manifest_updater()
    .ok_or_else(|| ApiError::Unsupported(
        format!("Plugin '{}' does not support manifest updates", plugin.metadata().name)
    ))?;

let updated_content = manifest_updater
    .update_dependency(path, old_dep, new_dep, new_path)
    .await?;
```

**Benefits:**
- ✅ Zero downcasting or cfg guards needed
- ✅ Plugins self-advertise capabilities
- ✅ Language-agnostic: shared code doesn't know about specific languages
- ✅ Easy to add new capabilities
- ✅ Clear error messages for unsupported operations

### New Capability Traits

**1. ManifestUpdater** - For manifest file updates

```rust
#[async_trait]
pub trait ManifestUpdater: Send + Sync {
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String>;
}

// Implementation in language plugin:
impl cb_plugin_api::ManifestUpdater for RustPlugin {
    async fn update_dependency(...) -> PluginResult<String> {
        // Update Cargo.toml dependencies
    }
}

// Discovery method:
fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> {
    Some(self)
}
```

**2. ModuleLocator** - For module file discovery

```rust
#[async_trait]
pub trait ModuleLocator: Send + Sync {
    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> PluginResult<Vec<PathBuf>>;
}

// Implementation in language plugin:
impl cb_plugin_api::ModuleLocator for RustPlugin {
    async fn locate_module_files(...) -> PluginResult<Vec<PathBuf>> {
        // Locate files for module path like "crate::utils::helpers"
    }
}

// Discovery method:
fn module_locator(&self) -> Option<&dyn ModuleLocator> {
    Some(self)
}
```

**3. RefactoringProvider** - For AST refactoring operations

```rust
#[async_trait]
pub trait RefactoringProvider: Send + Sync {
    fn supports_inline_variable(&self) -> bool;
    fn supports_extract_function(&self) -> bool;
    fn supports_extract_variable(&self) -> bool;

    async fn plan_inline_variable(
        &self,
        source: &str,
        variable_line: u32,
        variable_col: u32,
        file_path: &str,
    ) -> PluginResult<EditPlan>;

    async fn plan_extract_function(
        &self,
        source: &str,
        start_line: u32,
        end_line: u32,
        function_name: &str,
        file_path: &str,
    ) -> PluginResult<EditPlan>;

    async fn plan_extract_variable(
        &self,
        source: &str,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        variable_name: Option<String>,
        file_path: &str,
    ) -> PluginResult<EditPlan>;
}

// Implementation in language plugin:
impl cb_plugin_api::RefactoringProvider for RustPlugin {
    fn supports_inline_variable(&self) -> bool { true }
    fn supports_extract_function(&self) -> bool { true }
    fn supports_extract_variable(&self) -> bool { true }

    async fn plan_inline_variable(...) -> PluginResult<EditPlan> {
        refactoring::plan_inline_variable(source, variable_line, variable_col, file_path)
            .map_err(|e| PluginError::internal(format!("Rust refactoring error: {}", e)))
    }
    // ... other methods
}

// Discovery method:
fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
    Some(self)
}
```

### Migration Pattern: Old → New

> ⚠️ **WARNING: Downcasting is Strictly Forbidden**
>
> **NEVER use `as_any().downcast_ref::<ConcretePlugin>()` in shared code.**
> This pattern:
> - Breaks language-agnostic architecture
> - Reintroduces compile-time coupling
> - Defeats the purpose of the capability system
> - Will be rejected in code review
>
> **Always use capability traits** for language-specific operations. If a capability
> doesn't exist for your use case, create a new one instead of downcasting.

**Old pattern (DEPRECATED and FORBIDDEN - cfg guards + downcasting):**
```rust
match plugin.metadata().name {
    #[cfg(feature = "lang-rust")]
    "rust" => {
        plugin.as_any().downcast_ref::<RustPlugin>()?.method()
    }
    #[cfg(feature = "lang-typescript")]
    "typescript" => {
        plugin.as_any().downcast_ref::<TypeScriptPlugin>()?.method()
    }
    _ => Err(...)
}
```

**New pattern (capability-based):**
```rust
let capability = plugin.capability_trait()
    .ok_or_else(|| ApiError::Unsupported(...))?;
capability.method().await?
```

**Results:** 12 cfg guards removed, 2 downcasts eliminated, -48 net lines while adding MORE functionality.

## Key Principles

| Principle | Rationale |
|-----------|-----------|
| Use cb-lang-common utilities | Reduces boilerplate by ~460 lines |
| Follow existing patterns | Proven architecture, easier review |
| Write comprehensive tests | 30+ tests minimum for robust code |
| Use structured logging | Machine-readable, production-ready |
| Implement fallback parsers | Works in environments without runtime |

## Time Estimates

| Phase | Time |
|-------|------|
| Initial setup | 5 minutes (automated) |
| Core implementation | 1-2 days |
| Testing and polish | 1 day |
| **Total** | **2-3 days** |

## Key References

- [mill-plugin-api/src/lib.rs](../../../crates/mill-plugin-api/src/lib.rs) - Core trait definitions
- [mill-plugin-bundle/src/lib.rs](../../../crates/mill-plugin-bundle/src/lib.rs) - Bundle implementation
- [cb-lang-rust/](../crates/cb-lang-rust/) - Full reference implementation
- [CLAUDE.md](../CLAUDE.md) - Project documentation including plugin overview
