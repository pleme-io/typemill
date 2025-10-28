# Restore Go Language Plugin

> **Note on Naming**: This proposal references legacy `cb-*` crate names in migration steps. These are the **old names** being migrated **from** (in git tag `pre-language-reduction`) to the current `mill-*` naming convention. Current crate names follow the `mill-*` pattern.

## Problem

Go language plugin (`mill-lang-go`) was removed during unified API refactoring and preserved in `pre-language-reduction` git tag. Go developers currently cannot use TypeMill for Go projects. Plugin needs restoration to current codebase with 100% feature parity.

## Solution

Restore Go plugin from `pre-language-reduction` tag following validated Python migration process. Implement all 15 common capabilities plus ProjectFactory for package creation.

### Technical Approach

- Extract plugin from git tag and rename to `mill-*` convention
- Update dependencies and imports to current API
- Split monolithic `ImportSupport` trait into 5 specialized traits
- Add `mill_plugin!` macro registration
- Implement ProjectFactory capability for Go module creation
- Update protocol type imports to `mill_foundation::protocol::*`

### Go-Specific Details

- **LSP Server**: `gopls`
- **Manifest**: `go.mod` (Go modules)
- **Module Separator**: `/` (package path)
- **Source Directory**: varies (typically root)
- **Parser**: Native Go parser (from pre-reduction) or `tree-sitter-go`

## Checklists

### Extract and Rename
- [ ] Extract `crates/cb-lang-go/` from `pre-language-reduction` tag
- [ ] Rename to `crates/mill-lang-go/`
- [ ] Add to workspace members in root `Cargo.toml`

### Update Dependencies
- [ ] Update `Cargo.toml` package name to `mill-lang-go`
- [ ] Replace `cb-plugin-api` → `mill-plugin-api`
- [ ] Replace `cb-protocol` → Remove (merged into `mill-foundation`)
- [ ] Replace `cb-core` → `mill-foundation`
- [ ] Replace `cb-lang-common` → `mill-lang-common`
- [ ] Move `tempfile` to `[dev-dependencies]`

### Update Imports
- [ ] Replace `use cb_plugin_api` → `use mill_plugin_api`
- [ ] Replace `use cb_protocol` → `use mill_foundation::protocol`
- [ ] Replace `use cb_core` → `use mill_foundation`
- [ ] Replace `cb_lang_go::` → `mill_lang_go::`
- [ ] Update all protocol types to `mill_foundation::protocol::*` namespace

### Split Import Traits
- [ ] Add 5 trait imports: `ImportParser`, `ImportRenameSupport`, `ImportMoveSupport`, `ImportMutationSupport`, `ImportAdvancedSupport`
- [ ] Split `impl ImportSupport` into 5 separate trait implementations
- [ ] Remove duplicate method implementations across traits
- [ ] Add `#[derive(Default)]` to `GoImportSupport` struct

### Update Plugin Registration
- [ ] Add `mill_plugin!` macro to `lib.rs`
- [ ] Convert `metadata` to `pub const METADATA: LanguageMetadata`
- [ ] Convert capabilities to `pub const CAPABILITIES: PluginCapabilities`
- [ ] Update `new()` to return `Box<dyn LanguagePlugin>`
- [ ] Add `#[derive(Default)]` to main plugin struct
- [ ] Configure LSP: `LspConfig::new("gopls", &["gopls"])`

### Update Trait Implementations
- [ ] Update `metadata()` to return `&Self::METADATA`
- [ ] Update `capabilities()` to return `Self::CAPABILITIES`
- [ ] Replace single `import_support()` with 5 trait methods
- [ ] Add `import_parser()` → `Some(&self.import_support)`
- [ ] Add `import_rename_support()` → `Some(&self.import_support)`
- [ ] Add `import_move_support()` → `Some(&self.import_support)`
- [ ] Add `import_mutation_support()` → `Some(&self.import_support)`
- [ ] Add `import_advanced_support()` → `Some(&self.import_support)`

### Add ProjectFactory
- [ ] Implement `ProjectFactory` trait for GoPlugin
- [ ] Add `create_package()` method using Go modules
- [ ] Create `go.mod` manifest template
- [ ] Create project directory structure
- [ ] Add Go module initialization logic
- [ ] Add `project_factory()` method to return `Some(self)`

### Update EditPlanMetadata
- [ ] Find all `EditPlanMetadata` initializations
- [ ] Add `consolidation: None` field to each initialization

### Testing
- [ ] Run `cargo check -p mill-lang-go` and fix compilation errors
- [ ] Run `cargo test -p mill-lang-go` and verify all tests pass
- [ ] Test AST parsing with Go source files
- [ ] Test manifest parsing with `go.mod` files
- [ ] Test import rewriting for rename operations
- [ ] Test import rewriting for move operations
- [ ] Test all 3 refactoring operations (extract function, inline variable, extract variable)
- [ ] Test package creation with ProjectFactory

### Documentation
- [ ] Update `docs/architecture/overview.md` language support matrix
- [ ] Add Go to CLAUDE.md supported languages list
- [ ] Document `go.mod` module format
- [ ] Document gopls LSP configuration

## Success Criteria

- [ ] `cargo check -p mill-lang-go` compiles without errors
- [ ] `cargo check --workspace` compiles without errors
- [ ] All 15 common capabilities implemented and tested
- [ ] ProjectFactory capability implemented
- [ ] Unit tests pass
- [ ] Plugin loads via `mill_plugin!` macro
- [ ] LSP integration works with `gopls`
- [ ] Manifest parsing handles `go.mod` format
- [ ] Import rewriting works for Go import statements
- [ ] Refactoring operations generate valid Go code

## Benefits

- **Go developers** can use TypeMill for Go projects
- **Package creation** supported via ProjectFactory
- **Feature parity** with Rust, TypeScript, Python (15 capabilities)
- **Validated migration** follows proven Python restoration process
- **Cloud-native support** enables modern microservices codebases
