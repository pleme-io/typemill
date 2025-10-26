# Restore Swift Language Plugin

> **Note on Naming**: This proposal references legacy `cb-*` crate names in migration steps. These are the **old names** being migrated **from** (in git tag `pre-language-reduction`) to the current `mill-*` naming convention. Current crate names follow the `mill-*` pattern.

## Problem

Swift language plugin (`mill-lang-swift`) was removed during unified API refactoring and preserved in `pre-language-reduction` git tag. iOS/macOS developers currently cannot use TypeMill for Swift projects. Plugin needs restoration to current codebase with 100% feature parity.

## Solution

Restore Swift plugin from `pre-language-reduction` tag following validated Python migration process. Implement all 15 common capabilities plus ProjectFactory for package creation.

### Technical Approach

1. Extract plugin from git tag and rename to `mill-*` convention
2. Update dependencies and imports to current API
3. Split monolithic `ImportSupport` trait into 5 specialized traits
4. Add `mill_plugin!` macro registration
5. Implement ProjectFactory capability for Swift package creation
6. Update protocol type imports to `mill_foundation::protocol::*`

### Swift-Specific Details

- **LSP Server**: `sourcekit-lsp`
- **Manifest**: `Package.swift` (Swift DSL, executable code)
- **Module Separator**: `.`
- **Source Directory**: `Sources`
- **Parser**: Native Swift parser (from pre-reduction) or `tree-sitter-swift`

## Checklists

### Phase 1: Extract and Rename
- [ ] Extract `crates/cb-lang-swift/` from `pre-language-reduction` tag
- [ ] Rename to `crates/mill-lang-swift/`
- [ ] Add to workspace members in root `Cargo.toml`

### Phase 2: Update Dependencies
- [ ] Update `Cargo.toml` package name to `mill-lang-swift`
- [ ] Replace `cb-plugin-api` → `mill-plugin-api`
- [ ] Replace `cb-protocol` → Remove (merged into `mill-foundation`)
- [ ] Replace `cb-core` → `mill-foundation`
- [ ] Replace `cb-lang-common` → `mill-lang-common`
- [ ] Move `tempfile` to `[dev-dependencies]`

### Phase 3: Update Imports
- [ ] Replace `use cb_plugin_api` → `use mill_plugin_api`
- [ ] Replace `use cb_protocol` → `use mill_foundation::protocol`
- [ ] Replace `use cb_core` → `use mill_foundation`
- [ ] Replace `cb_lang_swift::` → `mill_lang_swift::`
- [ ] Update all protocol types to `mill_foundation::protocol::*` namespace

### Phase 4: Split Import Traits
- [ ] Add 5 trait imports: `ImportParser`, `ImportRenameSupport`, `ImportMoveSupport`, `ImportMutationSupport`, `ImportAdvancedSupport`
- [ ] Split `impl ImportSupport` into 5 separate trait implementations
- [ ] Remove duplicate method implementations across traits
- [ ] Add `#[derive(Default)]` to `SwiftImportSupport` struct

### Phase 5: Update Plugin Registration
- [ ] Add `mill_plugin!` macro to `lib.rs`
- [ ] Convert `metadata` to `pub const METADATA: LanguageMetadata`
- [ ] Convert capabilities to `pub const CAPABILITIES: PluginCapabilities`
- [ ] Update `new()` to return `Box<dyn LanguagePlugin>`
- [ ] Add `#[derive(Default)]` to main plugin struct
- [ ] Configure LSP: `LspConfig::new("sourcekit-lsp", &["sourcekit-lsp"])`

### Phase 6: Update Trait Implementations
- [ ] Update `metadata()` to return `&Self::METADATA`
- [ ] Update `capabilities()` to return `Self::CAPABILITIES`
- [ ] Replace single `import_support()` with 5 trait methods
- [ ] Add `import_parser()` → `Some(&self.import_support)`
- [ ] Add `import_rename_support()` → `Some(&self.import_support)`
- [ ] Add `import_move_support()` → `Some(&self.import_support)`
- [ ] Add `import_mutation_support()` → `Some(&self.import_support)`
- [ ] Add `import_advanced_support()` → `Some(&self.import_support)`

### Phase 7: Add ProjectFactory
- [ ] Implement `ProjectFactory` trait for SwiftPlugin
- [ ] Add `create_package()` method using Swift Package Manager templates
- [ ] Create `Package.swift` manifest template
- [ ] Create `Sources/` and `Tests/` directory structure
- [ ] Add Swift package initialization logic
- [ ] Add `project_factory()` method to return `Some(self)`

### Phase 8: Update EditPlanMetadata
- [ ] Find all `EditPlanMetadata` initializations
- [ ] Add `consolidation: None` field to each initialization

### Phase 9: Testing
- [ ] Run `cargo check -p mill-lang-swift` and fix compilation errors
- [ ] Run `cargo test -p mill-lang-swift` and verify all tests pass
- [ ] Test AST parsing with Swift source files
- [ ] Test manifest parsing with `Package.swift` files
- [ ] Test import rewriting for rename operations
- [ ] Test import rewriting for move operations
- [ ] Test all 3 refactoring operations (extract function, inline variable, extract variable)
- [ ] Test package creation with ProjectFactory

### Phase 10: Documentation
- [ ] Update `docs/architecture/overview.md` language support matrix
- [ ] Add Swift to CLAUDE.md supported languages list
- [ ] Note macOS-only testing limitation
- [ ] Document `Package.swift` executable manifest quirk

## Success Criteria

- [ ] `cargo check -p mill-lang-swift` compiles without errors
- [ ] `cargo check --workspace` compiles without errors
- [ ] All 15 common capabilities implemented and tested
- [ ] ProjectFactory capability implemented
- [ ] Unit tests pass
- [ ] Plugin loads via `mill_plugin!` macro
- [ ] LSP integration works with `sourcekit-lsp` on macOS
- [ ] Manifest parsing handles `Package.swift` Swift DSL
- [ ] Import rewriting works for Swift import syntax
- [ ] Refactoring operations generate valid Swift code

## Benefits

- **iOS/macOS developers** can use TypeMill for Swift projects
- **Package creation** supported via ProjectFactory
- **Feature parity** with Rust, TypeScript, Python (15 capabilities)
- **Validated migration** follows proven Python restoration process
- **Apple ecosystem support** completes mobile development coverage
