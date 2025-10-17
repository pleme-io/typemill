# Proposal: Single-Language Build Support

**Status:** Proposal
**Note:** Implement BEFORE Proposal 03 (Language Expansion) - Capability traits make adding new languages easier

## Problem Statement

Currently, all language plugins (Rust, TypeScript, Markdown) are compiled unconditionally on every build, even when developers are only working on features for a single language. This increases build times and forces installation of all language parsers and dependencies.

**Impact:**
- Slower build times (compiling unused languages)
- Larger binary size (includes all language parsers)
- Higher barrier to entry (requires all language toolchains)
- No benefit for developers focused on single-language features

## Current Blockers

### 1. Hard-Wired Dependencies Across Multiple Crates

Multiple core crates hard-wire every language as an unconditional dependency:

```toml
# cb-ast/Cargo.toml:11
cb-lang-rust = { workspace = true }
cb-lang-typescript = { workspace = true }
cb-lang-markdown = { workspace = true }

# crates/cb-services/Cargo.toml:29
cb-lang-rust = { workspace = true }
cb-lang-typescript = { workspace = true }

# crates/cb-plugins/Cargo.toml:17
cb-lang-rust = { workspace = true }
cb-lang-typescript = { workspace = true }

# apps/codebuddy/Cargo.toml:19
cb-lang-rust = { workspace = true }
cb-lang-typescript = { workspace = true }
```

That forces both Rust and TypeScript (plus Markdown) to compile on every build.

### 2. Eager Linking and Direct Code Calls

**cb-services** links languages eagerly:

```rust
// crates/cb-services/src/lib.rs:3
pub extern crate cb_lang_rust;
pub extern crate cb_lang_typescript;
```

Its import graph logic calls directly into each crate:

```rust
// crates/cb-services/src/services/ast_service.rs:169
match extension {
    "rs" => cb_lang_rust::build_import_graph(...),
    "ts" | "tsx" => cb_lang_typescript::build_import_graph(...),
    // ...
}
```

Without TypeScript on the cfg, the file won't compile.

### 3. Direct Downcasts in cb-ast

**cb-ast** contains many direct use statements and downcasts for refactoring and import-updater flows:

```rust
// crates/cb-ast/src/import_updater/edit_builder.rs:135
if let Some(ts_plugin) = plugin.downcast_ref::<cb_lang_typescript::TypeScriptPlugin>() {
    // TypeScript-specific logic
}

// crates/cb-ast/src/refactoring/inline_variable.rs:104
if let Some(rust_plugin) = plugin.downcast_ref::<cb_lang_rust::RustPlugin>() {
    // Rust-specific logic
}

// crates/cb-ast/src/refactoring/extract_function.rs:217
match language {
    Language::Rust => cb_lang_rust::extract_function(...),
    Language::TypeScript => cb_lang_typescript::extract_function(...),
    // ...
}
```

Those branches expect both plugins to exist.

**Why this doesn't scale:**

Every new language requires updating shared code in cb-ast, cb-services, and cb-handlers. Here's the pattern that causes pain:

**Current downcasting pattern:**
```rust
// Every new language requires updating this code in cb-ast
if let Some(rust_plugin) = plugin.downcast_ref::<RustPlugin>() {
    // Rust-specific logic
} else if let Some(ts_plugin) = plugin.downcast_ref::<TypeScriptPlugin>() {
    // TypeScript-specific logic
} else if let Some(go_plugin) = plugin.downcast_ref::<GoPlugin>() {
    // Go-specific logic
} else if let Some(python_plugin) = plugin.downcast_ref::<PythonPlugin>() {
    // Python-specific logic
}
// ... ad infinitum
```

**Better approach with capability traits:**
```rust
// No changes needed when adding new languages!
if let Some(scanner) = plugin.as_capability::<dyn ModuleReferenceScanner>() {
    scanner.scan_references(file_path, content)?
}
```

This is the key architectural unlock—capability traits push language-specific logic behind the trait boundary.

### 4. Tests and Default Constructors

Tests and default constructors pull languages automatically:

```rust
// crates/cb-plugins/src/system_tools_plugin.rs:47
pub fn default() -> Self {
    Self {
        rust_plugin: Some(cb_lang_rust::RustPlugin::new()),
        ts_plugin: Some(cb_lang_typescript::TypeScriptPlugin::new()),
        // ...
    }
}

// crates/cb-services/src/services/registry_builder.rs:102
pub fn with_default_languages(mut self) -> Self {
    self.languages.push(Box::new(cb_lang_rust::RustPlugin::new()));
    self.languages.push(Box::new(cb_lang_typescript::TypeScriptPlugin::new()));
    self
}
```

So a feature-only solution still requires code rework.

## Proposed Solution

### Phase 1: Feature Plumbing

Mark every language dependency as optional and expose matching features at each layer:

```toml
# cb-ast/Cargo.toml
[dependencies]
cb-lang-rust = { workspace = true, optional = true }
cb-lang-typescript = { workspace = true, optional = true }
cb-lang-markdown = { workspace = true, optional = true }

[features]
default = ["lang-rust", "lang-typescript", "lang-markdown"]
lang-rust = ["dep:cb-lang-rust"]
lang-typescript = ["dep:cb-lang-typescript"]
lang-markdown = ["dep:cb-lang-markdown"]
```

Repeat for:
- `cb-services/Cargo.toml`
- `cb-plugins/Cargo.toml`
- `apps/codebuddy/Cargo.toml`
- `cb-handlers/Cargo.toml`
- `tests/Cargo.toml`

Tie crate features together so enabling `lang-typescript` in the binary flips on the lower-level features automatically.

### Phase 2: Capability Traits

**Problem:** Current downcasting pattern doesn't scale and creates tight coupling.

**Solution:** Replace downcasts with capability traits.

**Start with:** Module reference scanning (simplest capability, used in `import_updater/edit_builder.rs:135`). Once proven, extend to refactoring and import analysis.

#### 2.1: Define Capability Traits in cb-plugin-api

```rust
// crates/cb-plugin-api/src/capabilities.rs

/// Capability for scanning module references in a file
pub trait ModuleReferenceScanner {
    fn scan_references(&self, file_path: &Path, content: &str) -> Result<Vec<ModuleReference>>;
}

/// Capability for refactoring operations
pub trait RefactoringProvider {
    fn supports_inline_variable(&self) -> bool { false }
    fn inline_variable(&self, params: InlineParams) -> Result<WorkspaceEdit>;

    fn supports_extract_function(&self) -> bool { false }
    fn extract_function(&self, params: ExtractParams) -> Result<WorkspaceEdit>;
}

/// Capability for import/dependency analysis
pub trait ImportAnalyzer {
    fn build_import_graph(&self, file_path: &Path) -> Result<ImportGraph>;
    fn find_unused_imports(&self, file_path: &Path) -> Result<Vec<UnusedImport>>;
}
```

#### 2.2: Implement Capabilities in Language Plugins

```rust
// crates/languages/cb-lang-rust/src/lib.rs

impl ModuleReferenceScanner for RustPlugin {
    fn scan_references(&self, file_path: &Path, content: &str) -> Result<Vec<ModuleReference>> {
        // Rust-specific implementation
    }
}

impl RefactoringProvider for RustPlugin {
    fn supports_inline_variable(&self) -> bool { true }
    fn inline_variable(&self, params: InlineParams) -> Result<WorkspaceEdit> {
        // Rust-specific implementation
    }

    fn supports_extract_function(&self) -> bool { true }
    fn extract_function(&self, params: ExtractParams) -> Result<WorkspaceEdit> {
        // Rust-specific implementation
    }
}
```

#### 2.3: Update cb-ast to Use Capability Traits

**Before:**
```rust
// crates/cb-ast/src/import_updater/edit_builder.rs:135
if let Some(ts_plugin) = plugin.downcast_ref::<cb_lang_typescript::TypeScriptPlugin>() {
    let refs = ts_plugin.scan_module_references(file_path, content)?;
    // ...
}
```

**After:**
```rust
// crates/cb-ast/src/import_updater/edit_builder.rs
if let Some(scanner) = plugin.as_capability::<dyn ModuleReferenceScanner>() {
    let refs = scanner.scan_references(file_path, content)?;
    // ...
}
```

#### 2.4: Update cb-services to Use Capability Traits

**Before:**
```rust
// crates/cb-services/src/services/ast_service.rs:169
match extension {
    "rs" => cb_lang_rust::build_import_graph(file_path)?,
    "ts" | "tsx" => cb_lang_typescript::build_import_graph(file_path)?,
    _ => return Err(Error::UnsupportedLanguage),
}
```

**After:**
```rust
// crates/cb-services/src/services/ast_service.rs
let plugin = self.plugin_manager.get_plugin_for_extension(extension)?;
if let Some(analyzer) = plugin.as_capability::<dyn ImportAnalyzer>() {
    analyzer.build_import_graph(file_path)?
} else {
    return Err(Error::CapabilityNotSupported("ImportAnalyzer"));
}
```

#### 2.5: Add Capability Discovery to LanguagePlugin

```rust
// crates/cb-plugin-api/src/traits.rs

pub trait LanguagePlugin: Send + Sync {
    // ... existing methods ...

    /// Check if plugin supports a capability
    fn supports_capability(&self, capability: &str) -> bool {
        false
    }

    /// Get a capability trait object
    fn as_capability<T: ?Sized>(&self) -> Option<&T> {
        None
    }
}
```

Language plugins implement this to expose their capabilities:

```rust
impl LanguagePlugin for RustPlugin {
    fn supports_capability(&self, capability: &str) -> bool {
        matches!(capability,
            "ModuleReferenceScanner" | "RefactoringProvider" | "ImportAnalyzer"
        )
    }

    fn as_capability<T: ?Sized>(&self) -> Option<&T> {
        // Return appropriate trait object based on type
        // This requires some trait casting magic
    }
}
```

### Phase 3: Code Gating

Guard every language-specific use and match arm with `#[cfg(feature = "...")]`:

```rust
// crates/cb-services/src/lib.rs
#[cfg(feature = "lang-rust")]
pub extern crate cb_lang_rust;

#[cfg(feature = "lang-typescript")]
pub extern crate cb_lang_typescript;

// crates/cb-services/src/services/registry_builder.rs:102
pub fn with_default_languages(mut self) -> Self {
    #[cfg(feature = "lang-rust")]
    self.languages.push(Box::new(cb_lang_rust::RustPlugin::new()));

    #[cfg(feature = "lang-typescript")]
    self.languages.push(Box::new(cb_lang_typescript::TypeScriptPlugin::new()));

    #[cfg(feature = "lang-markdown")]
    self.languages.push(Box::new(cb_lang_markdown::MarkdownPlugin::new()));

    self
}
```

### Phase 4: Tests & Tooling

Mark integration tests with `cfg(feature = "...")`:

```rust
// tests/src/test_rust_features.rs
#![cfg(feature = "lang-rust")]

// tests/src/test_typescript_features.rs
#![cfg(feature = "lang-typescript")]
```

Add cargo aliases in `.cargo/config.toml`:

```toml
[alias]
# Single-language builds
check-rust-only = "check --no-default-features --features lang-rust"
test-rust-only = "nextest run --no-default-features --features lang-rust"

check-ts-only = "check --no-default-features --features lang-typescript"
test-ts-only = "nextest run --no-default-features --features lang-typescript"

# Multi-language combinations
check-rust-ts = "check --no-default-features --features lang-rust,lang-typescript"
test-rust-ts = "nextest run --no-default-features --features lang-rust,lang-typescript"
```

Add Makefile targets:

```makefile
# Single-language builds
check-rust-only:
	cargo check-rust-only

test-rust-only:
	cargo test-rust-only

check-ts-only:
	cargo check-ts-only

test-ts-only:
	cargo test-ts-only
```

### Phase 5: Documentation

Update CONTRIBUTING.md with single-language build instructions:

```markdown
### Single-Language Builds

When working on features for a specific language:

```bash
# Rust-only build (excludes TypeScript, Markdown)
make check-rust-only
make test-rust-only

# TypeScript-only build (excludes Rust, Markdown)
make check-ts-only
make test-ts-only
```

**Performance gains:**
- **Compilation:** 30-40% faster (one language vs three)
- **Binary size:** 40-50% smaller
- **Development setup:** Only need one language toolchain
```

## Scaling to 8+ Languages

The capability trait approach prevents duplication as we add more languages. As shown in the blockers section, the current downcasting pattern requires updating shared code in cb-ast/cb-services/cb-handlers for every new language.

**With capability traits:** Adding a new language just means implementing the trait in its crate. No changes needed in shared code.

### Additional Scaling Benefits

1. **Centralized Helpers**: Move shared refactoring infrastructure to `cb-lang-common` so new languages reuse code instead of duplicating it.

2. **Capability Discovery**: Plugin manager can query capabilities at runtime:
   ```rust
   let available_languages = plugin_manager
       .plugins()
       .filter(|p| p.supports_capability("RefactoringProvider"))
       .collect();
   ```

3. **Graceful Degradation**: If a language doesn't support a capability, we can provide helpful error messages or fallback behavior instead of compile errors.

## Implementation Dependencies

| Phase | Task | Dependencies |
|-------|------|--------------|
| 1 | Feature plumbing (manifests) | None |
| 2 | Capability traits design + implementation | Phase 1 |
| 3 | Code gating with #[cfg] | Phase 2 |
| 4 | Test infrastructure + CI | Phase 3 |
| 5 | Documentation | Phase 4 |

## Success Metrics

- [ ] `cargo check --no-default-features --features lang-rust` compiles successfully
- [ ] `cargo check --no-default-features --features lang-typescript` compiles successfully
- [ ] `cargo nextest run --no-default-features --features lang-rust` passes with Rust tests only
- [ ] `cargo nextest run --no-default-features --features lang-typescript` passes with TypeScript tests only
- [ ] Rust-only build is 30-40% faster than full build
- [ ] Binary size reduced by 40-50% for single-language builds
- [ ] No downcasts to concrete language plugin types in shared code
- [ ] CI jobs test single-language builds to prevent regressions
- [ ] Documentation covers single-language workflows

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Capability traits too complex | High | Prototype with one capability first, iterate on design |
| Feature flags become unmaintainable | Medium | Use feature unification, document patterns clearly |
| Test coverage gaps | Medium | Add CI matrix testing all feature combinations |
| Breaking changes for plugins | Low | Keep old API during transition, deprecate gradually |

## Suggested Next Steps

1. **Prototype capability traits** in a branch:
   - Define `ModuleReferenceScanner` trait in `cb-plugin-api`
   - Implement for `RustPlugin` and `TypeScriptPlugin`
   - Refactor one usage site in `cb-ast` to use trait instead of downcast
   - Validate approach works before full rollout

2. **Make cb-services compile with `--no-default-features --features lang-rust`**:
   - Add optional deps and features to `cb-services/Cargo.toml`
   - Gate language imports with `#[cfg(feature = "...")]`
   - This exposes precise trait gaps that need capability traits

3. **Design capability trait hierarchy**:
   - List all current downcast sites and their requirements
   - Group into logical capability traits
   - Design trait signatures with input from language plugin maintainers

4. **Roll manifest changes across workspace**:
   - Update all `Cargo.toml` files with optional language deps
   - Add feature flags at each layer
   - Test feature unification works correctly

5. **Add CI jobs for single-language builds**:
   - Add matrix builds for `lang-rust`, `lang-typescript`, and full
   - Ensure regressions stay visible
   - Add to pre-merge checks

## References

- **Refactoring Feature Flags**: Similar pattern already implemented in `cb-handlers/Cargo.toml` (commit 8b7da653)
- **Rust Feature Unification**: [Cargo Book - Features](https://doc.rust-lang.org/cargo/reference/features.html)
- **Trait Objects**: [Rust Book - Trait Objects](https://doc.rust-lang.org/book/ch17-02-trait-objects.html)
- **Design Pattern**: Similar to [LSP's capabilities model](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities)

## Conclusion

Single-language builds are **currently blocked** by hard-wired dependencies and downcasting patterns across multiple crates. The solution requires:

1. Feature plumbing (straightforward but broad)
2. **Capability traits** (biggest lift, but enables scaling to 8+ languages)
3. Code gating with `#[cfg]`
4. Test infrastructure updates

**Key benefit:** Not just faster builds today, but a **scalable architecture** that eliminates duplication as we add more languages.

The capability trait approach is the critical piece—it transforms "every new language touches N files" into "every new language implements M traits." This pays dividends as we scale from 3 languages to 8+.
