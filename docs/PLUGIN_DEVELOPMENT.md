# Language Plugin Development

Fast reference for implementing new language plugins.

## Quick Start

| Step | Command | Time |
|------|---------|------|
| 1. Generate structure | `cd crates/languages && ./new-lang.sh kotlin --manifest "build.gradle.kts" --extensions kt,kts` | 5 min |
| 2. Build workspace | `cd ../.. && cargo build --features lang-kotlin` | 1 min |
| 3. Implement logic | Edit `parser.rs`, `manifest.rs` | 1-2 days |
| 4. Run tests | `cargo nextest run -p cb-lang-kotlin` | 1 min |
| 5. Validate | `cd crates/languages && ./check-features.sh` | 30 sec |

## Plugin Structure

```
crates/languages/cb-lang-{language}/
├── Cargo.toml              # Dependencies and metadata
├── README.md               # Plugin documentation
├── resources/              # Optional: embedded AST tools
│   └── ast_tool.*         # Language-native parser subprocess
└── src/
    ├── lib.rs              # Main plugin struct + LanguagePlugin trait
    ├── parser.rs           # Symbol extraction & import parsing
    ├── manifest.rs         # Manifest file parsing
    ├── import_support.rs   # Optional: ImportSupport trait
    └── workspace_support.rs # Optional: WorkspaceSupport trait
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

### ImportSupport (if capabilities().imports = true)

| Method | Purpose |
|--------|---------|
| `parse_imports()` | Extract import statements |
| `rewrite_imports_for_move()` | Update imports when files move |
| `rewrite_imports_for_rename()` | Update imports when modules rename |
| `find_module_references()` | Find all references to a module |

### WorkspaceSupport (if capabilities().workspace = true)

| Method | Purpose |
|--------|---------|
| `is_workspace_manifest()` | Detect workspace manifest files |
| `add_workspace_member()` | Add member to workspace |
| `remove_workspace_member()` | Remove member from workspace |
| `merge_dependencies()` | Merge dependencies between manifests |

## Manual Integration Steps

After running `./new-lang.sh`, manually edit these files:

| File | Action | Example |
|------|--------|---------|
| Root `Cargo.toml` | Add workspace dependency | `cb-lang-java = { path = "crates/languages/cb-lang-java" }` |
| `crates/cb-handlers/Cargo.toml` | Add optional dep + feature | `cb-lang-java = { workspace = true, optional = true }` + `lang-java = ["dep:cb-lang-java"]` |
| `crates/cb-services/src/services/registry_builder.rs` | Register plugin | `#[cfg(feature = "lang-java")] { registry.register(Arc::new(cb_lang_java::JavaPlugin::new())); }` |

**Verify:** `cd crates/languages && ./check-features.sh`

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
| Subprocess AST (compiled) | Go plugin | `crates/languages/cb-lang-go/` |
| Subprocess AST (dynamic) | Python plugin | `crates/languages/cb-lang-python/` |
| Subprocess AST (JS ecosystem) | TypeScript plugin | `crates/languages/cb-lang-typescript/` |
| Native Rust parsing | Rust plugin | `crates/languages/cb-lang-rust/` |
| ImportSupport | All plugins | `src/import_support.rs` in any plugin |
| WorkspaceSupport | Rust, Go, TypeScript | `src/workspace_support.rs` |

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
- [ ] `parse_imports()` handles all import styles
- [ ] `rewrite_imports_for_move()` updates relative paths
- [ ] `rewrite_imports_for_rename()` renames modules
- [ ] `find_module_references()` finds all references

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
- [ ] Added to root `Cargo.toml` workspace dependencies
- [ ] Added to `cb-handlers/Cargo.toml` optional dependencies
- [ ] Feature flag created in `cb-handlers/Cargo.toml`
- [ ] Registered in `registry_builder.rs`
- [ ] `./check-features.sh` passes

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
| Plugin not found during build | Not registered | Check `codebuddy_plugin!` macro, workspace link, registry registration |
| Subprocess AST tool fails | Runtime not installed | Install runtime (python3, node, go), add fallback regex parser |
| ImportGraph has no imports | Not being called | Verify `parse_imports()` called, check regex patterns, use debug logging |
| Tests pass locally, fail in CI | Runtime unavailable in CI | Ensure fallback parser works, use feature flags for integration tests |
| LanguageMetadata constant not found | Build script issue | Run `cargo clean && cargo build`, check `languages.toml` entry |
| Import rewriting changes non-imports | Regex too broad | Use AST-based rewriting or specific regex: `^import\s+{}`  |
| Workspace operations corrupt manifest | String manipulation | Use parser library (`toml_edit`, `serde_json`), validate output |

## Code Examples

See `examples/plugins/` for complete examples:
- Subprocess AST pattern (Go, Python, TypeScript)
- Native Rust parsing (Rust)
- Import support implementation
- Workspace support implementation
- Test suite patterns

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

- [CB_LANG_COMMON.md](cb_lang_common.md) - Shared utility functions
- [README.md](readme.md) - Overview of existing plugins
- [examples/plugins/](../../examples/plugins/) - Complete code examples
- [docs/archive/plugin_development_guide-verbose.md](../archive/plugin_development_guide-verbose.md) - Full guide with explanations
