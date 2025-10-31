# Proposal 20: Eliminate Language Leakage from Core Abstractions

## Problem

Core crates contain hardcoded language-specific logic that violates the plugin architecture abstraction. Analysis found **12 critical locations** where handlers and services directly reference language names instead of using plugin capabilities:

**Critical examples**:

```rust
// Extension mapping duplicated across 7 files
pub(crate) fn extension_to_language(extension: &str) -> String {
    match extension {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "py" | "pyi" => "python",
        // ... hardcoded for all languages
    }
}

// Rust-specific logic in core services
let is_rust_crate_rename = old_path.join("Cargo.toml").exists() && ...;
if is_rust_crate_rename {
    // Special Rust crate rename logic
}

// System tools checking for specific language
let has_rust_plugin = registry.all().iter()
    .any(|p| p.metadata().name == "rust");

// Analysis tools with hardcoded patterns
fn get_test_patterns(language: &str) -> Vec<&'static str> {
    match language {
        "rust" => vec![r"#\[test\]", r"fn test_"],
        "typescript" => vec![r"\bit\(", r"\btest\("],
        // ...
    }
}
```

**Impact**: Adding a new language plugin requires modifying core code in 12+ locations. Features like package-level operations are artificially restricted to Rust.

## Solution

Replace all hardcoded language references with plugin capability queries and trait methods. Create new traits for analysis-specific metadata.

**Architecture changes**:

1. **Use existing plugin registry APIs** instead of hardcoded mappings
2. **Add AnalysisMetadata trait** for language-specific patterns
3. **Generalize manifest-based detection** using plugin metadata
4. **Query capabilities** instead of checking plugin names

## Checklists

### Remove Extension Mapping Duplication

- [ ] Replace `extension_to_language()` in `rename_handler/utils.rs` with plugin registry query
- [ ] Replace `extension_to_language()` in `extract_handler.rs` with plugin registry query
- [ ] Replace `extension_to_language()` in `inline_handler.rs` with plugin registry query
- [ ] Replace `extension_to_language()` in `delete_handler.rs` with plugin registry query
- [ ] Replace `extension_to_language()` in `transform_handler.rs` with plugin registry query
- [ ] Replace `extension_to_language()` in `reorder_handler.rs` with plugin registry query
- [ ] Replace `extension_to_language()` in `move/validation.rs` with plugin registry query
- [ ] Use: `plugin_registry.for_extension(ext)?.metadata().name`
- [ ] Verify all handlers work with plugin-provided language names

### Generalize Reference Updater

- [ ] Remove hardcoded `Cargo.toml` check in `reference_updater/mod.rs`
- [ ] Replace `is_rust_crate_rename` with plugin manifest detection
- [ ] Use `plugin.metadata().manifest_filename` for generic detection
- [ ] Replace `extension == "rs"` checks with plugin queries
- [ ] Test package-level rename with multiple languages (Rust, Python, Go)

### Fix System Tools Plugin

- [ ] Remove `p.metadata().name == "rust"` check in `system_tools_plugin.rs:68-77`
- [ ] Replace with capability query: `p.workspace_support().is_some()`
- [ ] Remove runtime Rust checks at lines 333-344
- [ ] Remove runtime Rust checks at lines 783-793
- [ ] Verify `extract_module_to_package` works for all languages with workspace support

### Create AnalysisMetadata Trait

- [ ] Add `AnalysisMetadata` trait to `mill-plugin-api/src/language.rs`
- [ ] Add method: `fn test_patterns(&self) -> Vec<Regex>`
- [ ] Add method: `fn assertion_patterns(&self) -> Vec<Regex>`
- [ ] Add method: `fn doc_comment_style(&self) -> DocCommentStyle`
- [ ] Add method: `fn visibility_keywords(&self) -> Vec<&'static str>`
- [ ] Add method: `fn interface_keywords(&self) -> Vec<&'static str>`
- [ ] Define `DocCommentStyle` enum (TripleSlash, JavaDoc, Hash, etc.)
- [ ] Make trait methods return default empty values

### Implement AnalysisMetadata in Language Plugins

- [ ] Implement `AnalysisMetadata` for `RustPlugin`
- [ ] Implement `AnalysisMetadata` for `TypeScriptPlugin`
- [ ] Implement `AnalysisMetadata` for `PythonPlugin`
- [ ] Implement `AnalysisMetadata` for `GoPlugin`
- [ ] Implement `AnalysisMetadata` for `SwiftPlugin`
- [ ] Implement `AnalysisMetadata` for `CsharpPlugin`
- [ ] Implement `AnalysisMetadata` for `JavaPlugin`
- [ ] Verify patterns are complete for each language

### Update Analysis Handlers to Use Trait

- [ ] Replace `get_test_patterns()` in `tests_handler.rs` with trait query
- [ ] Replace hardcoded visibility keywords in `structure.rs` with trait query
- [ ] Replace hardcoded doc comment detection in `documentation.rs` with trait query
- [ ] Use: `plugin.as_any().downcast_ref::<dyn AnalysisMetadata>()`
- [ ] Handle plugins that don't implement AnalysisMetadata (return defaults)
- [ ] Verify analysis tools work for all languages

### Generalize LSP Manager

- [ ] Replace hardcoded `Cargo.toml` check in `detector.rs`
- [ ] Replace hardcoded `package.json` check in `detector.rs`
- [ ] Replace hardcoded `pyproject.toml` check in `detector.rs`
- [ ] Replace hardcoded `go.mod` check in `detector.rs`
- [ ] Use plugin registry: filter plugins where manifest file exists
- [ ] Verify LSP manager detects all installed languages

### Remove LSP Adapter Special Cases

- [ ] Remove `if extension == "rs"` check in `lsp_adapter.rs:123-153`
- [ ] Add generic workspace indexing check if needed
- [ ] Verify LSP operations work uniformly across languages

### Generalize Consolidation Detection

- [ ] Replace `Cargo.toml` check in `directory_rename.rs` with plugin metadata
- [ ] Use `plugin.metadata().manifest_filename` for detection
- [ ] Use `plugin.metadata().source_dir` for target directory check
- [ ] Verify consolidation works for Python, Go, Java (not just Rust)

### Remove CLI TypeScript-Specific Functions

- [ ] Replace `detect_typescript_root()` in `lsp_helpers.rs` with generic version
- [ ] Create `detect_project_root(plugin, start_dir)` function
- [ ] Replace `detect_all_typescript_roots()` with generic version
- [ ] Update TypeScript setup logic in `cli/mod.rs` to use generic detection

### Generalize Package Manager Detection

- [ ] Move `PackageManager` detection from plugin API to plugins
- [ ] Add `package_manager()` method to `LanguagePlugin` trait
- [ ] Implement in each language plugin (Cargo, Npm, Pip, etc.)
- [ ] Update callers to use plugin method

### Abstract Complexity Metrics

- [ ] Add `ComplexityMetadata` trait to plugin API
- [ ] Add method: `fn complexity_keywords(&self) -> Vec<&'static str>`
- [ ] Add method: `fn nesting_penalty(&self) -> f32`
- [ ] Implement in all language plugins
- [ ] Update `complexity/metrics.rs` to use trait
- [ ] Remove hardcoded language match statements

### Verification

- [ ] Search core crates for hardcoded language names: `grep -r '"rust"\|"typescript"\|"python"' crates/mill-handlers/`
- [ ] Verify zero matches (excluding tests/docs)
- [ ] Run full test suite: `cargo nextest run --workspace`
- [ ] Test adding a hypothetical new language plugin requires zero core changes
- [ ] Verify all analysis tools work without language-specific code
- [ ] Verify package operations work for multiple languages

## Success Criteria

- [ ] Zero hardcoded language names in `crates/mill-handlers/`
- [ ] Zero hardcoded language names in `crates/mill-services/`
- [ ] Zero hardcoded language names in `crates/mill-plugin-system/`
- [ ] All language-specific logic isolated to plugin implementations
- [ ] Adding a new language plugin requires zero core code changes
- [ ] Analysis tools work generically via plugin trait methods
- [ ] Reference detection works uniformly for all languages with manifests
- [ ] All 64+ tests continue passing
- [ ] Package-level operations available to all languages with workspace support

## Benefits

**Architectural Cleanliness**:
- True plugin architecture with zero core coupling
- Clear separation of concerns
- Language logic isolated to language plugins

**Maintainability**:
- Eliminates ~200 lines of duplicate code (7× extension mapping)
- Single source of truth for language metadata
- Easier to understand and modify

**Extensibility**:
- Adding new languages requires zero core changes
- New languages automatically get all features (analysis, refactoring, etc.)
- Plugin capabilities determine feature availability

**Feature Parity**:
- Package-level operations work for Python, Go, Java (not just Rust)
- Module extraction available to all languages with workspace support
- Analysis tools work uniformly across all languages

**Code Quality**:
- Removes special-case logic
- Eliminates hardcoded strings
- Uses type-safe trait methods instead of string matching
