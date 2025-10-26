# Plugin Refactoring - Consolidate Duplicates and Macro-ize Boilerplate

## Problem

**Data Structure Quadruplication:** Refactoring structs (`CodeRange`, `ExtractableFunction`, `InlineVariableAnalysis`, `ExtractVariableAnalysis`) defined in four locations with byte-for-byte duplication:
- `mill-lang-common/src/refactoring.rs` (canonical with helpers)
- `mill-ast/src/refactoring/mod.rs` (duplicate)
- `mill-lang-python/src/refactoring.rs` (duplicate)
- `mill-lang-typescript/src/refactoring.rs` (duplicate with comment: *"might be better in a shared crate"*)

**Boilerplate Triplication:** Plugin `lib.rs` files are 90% identical across Python/TypeScript/Rust. Current `plugin_scaffold.rs` is string-based generator creating copy-paste templates that diverge over time.

API changes require manual edits to 3+ files per change.

## Solution

1. Establish `mill-lang-common` as single source of truth for refactoring data structures
2. Create `define_language_plugin!` procedural macro to generate boilerplate at compile-time
3. Comprehensive validation to ensure zero regressions

## Checklists

### Phase 1: Consolidate Refactoring Structs

**Update mill-lang-common**
- [ ] Move `ExtractableFunction` from `mill-ast/src/refactoring/mod.rs:61-69` to `mill-lang-common/src/refactoring.rs`
- [ ] Move `InlineVariableAnalysis` from `mill-ast/src/refactoring/mod.rs:72-80` to `mill-lang-common/src/refactoring.rs`
- [ ] Move `ExtractVariableAnalysis` from `mill-ast/src/refactoring/mod.rs:83-92` to `mill-lang-common/src/refactoring.rs`
- [ ] Export in `mill-lang-common/src/lib.rs`: `pub use refactoring::{CodeRange, ExtractableFunction, InlineVariableAnalysis, ExtractVariableAnalysis};`

**Update mill-lang-python**
- [ ] Delete `CodeRange` + 3 analysis structs (lines 22-59 in `src/refactoring.rs`)
- [ ] Add import: `use mill_lang_common::refactoring::{CodeRange, ExtractableFunction, InlineVariableAnalysis, ExtractVariableAnalysis};`

**Update mill-lang-typescript**
- [ ] Delete `CodeRange` + 3 analysis structs (lines 15-52 in `src/refactoring.rs`)
- [ ] Add import: `use mill_lang_common::refactoring::{CodeRange, ExtractableFunction, InlineVariableAnalysis, ExtractVariableAnalysis};`

**Update mill-ast**
- [ ] Delete duplicate structs (lines 40-92 in `src/refactoring/mod.rs`)
- [ ] Add import: `use mill_lang_common::refactoring::{CodeRange, ExtractableFunction, InlineVariableAnalysis, ExtractVariableAnalysis};`

**Test Phase 1**
- [ ] Run `cargo check --workspace` (zero errors)
- [ ] Run `cargo nextest run --workspace` (zero failures)

### Phase 2: Plugin Scaffolding Macro

**Create Macro Crate**
- [ ] Create `crates/mill-lang-macros/` with `proc-macro = true` in `Cargo.toml`
- [ ] Add dependencies: `syn = "2.0"`, `quote = "1.0"`, `proc-macro2 = "1.0"`
- [ ] Add to workspace members in root `Cargo.toml`

**Implement Macro**
- [ ] Define macro signature:
  ```rust
  define_language_plugin! {
      name: "python",
      struct: PythonPlugin,
      extensions: ["py"],
      manifest: "pyproject.toml",
      capabilities: with_imports() | with_workspace() | with_project_factory(),
      lsp: ("pylsp", ["pylsp"])
  }
  ```
- [ ] Parse input using `syn::parse_macro_input!`
- [ ] Generate `Plugin` struct with capability trait fields
- [ ] Generate `METADATA` const
- [ ] Generate `CAPABILITIES` const
- [ ] Generate `Default` trait impl
- [ ] Generate `LanguagePlugin` trait impl with delegating methods
- [ ] Generate `mill_plugin!` registration
- [ ] Add compile-time validation

**Refactor Plugins**
- [ ] Add `mill-lang-macros` dependency to each plugin's `Cargo.toml`
- [ ] Replace Python boilerplate (lines 51-88) with macro call
- [ ] Replace TypeScript boilerplate (lines 27-65) with macro call
- [ ] Replace Rust boilerplate (lines 51-81) with macro call
- [ ] Run `cargo expand -p mill-lang-python` to verify expansion matches original
- [ ] Run `cargo expand -p mill-lang-typescript` to verify expansion matches original
- [ ] Run `cargo expand -p mill-lang-rust` to verify expansion matches original

**Test Phase 2**
- [ ] Run `cargo check --workspace` (zero errors)
- [ ] Run `cargo test -p mill-lang-python` (zero failures)
- [ ] Run `cargo test -p mill-lang-typescript` (zero failures)
- [ ] Run `cargo test -p mill-lang-rust` (zero failures)

### Phase 3: Validation

**Automated Testing**
- [ ] Run `cargo clippy --workspace -- -D warnings` (zero warnings)
- [ ] Run `cargo nextest run --workspace --all-features` (zero failures)
- [ ] Run `cargo nextest run --workspace --features lsp-tests` (zero failures)

**Manual Integration Testing**
- [ ] Test Python: parse source, analyze manifest, all 3 refactorings, LSP with `pylsp`
- [ ] Test TypeScript: parse source, analyze manifest, all 3 refactorings, LSP with `typescript-language-server`
- [ ] Test Rust: parse source, analyze manifest, module locator, reference detector, LSP with `rust-analyzer`

**Cross-Plugin Validation**
- [ ] Verify `CodeRange` usage consistent across all plugins
- [ ] Verify analysis struct serialization/deserialization works
- [ ] Verify plugin metadata correctly registered
- [ ] Verify plugin capabilities correctly exposed

**Documentation**
- [ ] Update `CLAUDE.md` with refactoring outcomes
- [ ] Update `docs/DEVELOPMENT.md` with macro usage guide
- [ ] Document macro API in `mill-lang-macros/src/lib.rs`
- [ ] Create migration guide in `.debug/plugin-refactor-migration/GUIDE.md`

**Cleanup**
- [ ] Run `cargo fmt --all`
- [ ] Verify no `TODO`/`FIXME` comments in refactored code
- [ ] Remove temporary debug code

## Success Criteria

- [ ] `CodeRange` + analysis structs defined in **exactly one location**
- [ ] All plugins import from `mill_lang_common::refactoring`
- [ ] Macro expansion produces identical output to original boilerplate
- [ ] ~150 lines of struct duplication eliminated
- [ ] ~200-250 lines of plugin boilerplate eliminated
- [ ] Zero compilation errors, warnings, or test failures
- [ ] All LSP integrations functional
- [ ] Refactoring operations produce identical output to pre-refactor

## Benefits

- **Single source of truth** for refactoring data models
- **Compile-time enforced consistency** across language plugins
- **Automatic propagation** of plugin API changes
- **Reduced codebase size** (~350-400 lines eliminated)
- **Simplified maintenance** (update once, propagates everywhere)
- **Easier plugin creation** (template instantiation vs copy-paste)
