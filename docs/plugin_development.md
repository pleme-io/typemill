# Language Plugin Development

Fast reference for implementing new language plugins.

## Quick Start

Creating a new language plugin involves creating a standard Cargo crate and registering it with the `codebuddy-plugin-bundle`.

| Step | Action | Notes |
|------|--------|-------|
| 1. Create Crate | `cargo new --lib crates/cb-lang-mynewlang` | Creates the standard plugin structure |
| 2. Implement Logic | Edit `src/lib.rs` to implement `LanguagePlugin` trait | See reference implementations below |
| 3. Register Plugin | Add to `codebuddy-plugin-bundle/Cargo.toml` and `lib.rs` | Makes plugin discoverable via auto-discovery |
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

Edit `crates/codebuddy-plugin-bundle/Cargo.toml`:

```toml
[dependencies]
cb-lang-python = { path = "../cb-lang-python" }
```

Edit `crates/codebuddy-plugin-bundle/src/lib.rs`:

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
- [ ] Added to `codebuddy-plugin-bundle/Cargo.toml` dependencies
- [ ] Added to `codebuddy-plugin-bundle/src/lib.rs` `_force_plugin_linkage()`
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
| Tests fail with "plugin not found" | Missing from bundle | Add to `codebuddy-plugin-bundle/Cargo.toml` and rebuild |

## Code Examples

See existing plugins for complete reference implementations:
- **Rust**: `crates/cb-lang-rust/` - Full-featured with all capabilities
- **Markdown**: `crates/cb-lang-markdown/` - Simple plugin with basic import support
- **TOML/YAML**: `crates/cb-lang-toml/`, `crates/cb-lang-yaml/` - Config file plugins
- **TypeScript**: `crates/cb-lang-typescript/` - JavaScript ecosystem plugin

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

- [cb-plugin-api/src/lib.rs](../crates/cb-plugin-api/src/lib.rs) - Core trait definitions
- [codebuddy-plugin-bundle/src/lib.rs](../crates/codebuddy-plugin-bundle/src/lib.rs) - Bundle implementation
- [cb-lang-rust/](../crates/cb-lang-rust/) - Full reference implementation
- [CLAUDE.md](../CLAUDE.md) - Project documentation including plugin overview
