# Restore C# Language Plugin

> **Note on Naming**: This proposal references legacy `cb-*` crate names in migration steps. These are the **old names** being migrated **from** (in git tag `pre-language-reduction`) to the current `mill-*` naming convention. Current crate names follow the `mill-*` pattern.

## Problem

C# language plugin (`mill-lang-csharp`) was removed during unified API refactoring and preserved in `pre-language-reduction` git tag. .NET developers currently cannot use TypeMill for C# projects. Plugin needs restoration to current codebase with 100% feature parity.

## Solution

Restore C# plugin from `pre-language-reduction` tag following validated Python migration process. Implement all 15 common capabilities plus ProjectFactory for package creation.

### Technical Approach

- Extract plugin from git tag and rename to `mill-*` convention
- Update dependencies and imports to current API
- Split monolithic `ImportSupport` trait into 5 specialized traits
- Add `mill_plugin!` macro registration
- Implement ProjectFactory capability for C# project creation
- Update protocol type imports to `mill_foundation::protocol::*`

### C#-Specific Details

- **LSP Server**: `omnisharp` or `csharp-ls`
- **Manifest**: `.csproj` (MSBuild XML)
- **Module Separator**: `.`
- **Source Directory**: varies (typically root or `src/`)
- **Parser**: Native C# parser (from pre-reduction) or `tree-sitter-c-sharp`

## Checklists

### Extract and Rename
- [x] Extract `crates/cb-lang-csharp/` from `pre-language-reduction` tag
- [x] Rename to `crates/mill-lang-csharp/`
- [x] Add to workspace members in root `Cargo.toml`

### Update Dependencies
- [x] Update `Cargo.toml` package name to `mill-lang-csharp`
- [x] Replace `cb-plugin-api` → `mill-plugin-api`
- [x] Replace `cb-protocol` → Remove (merged into `mill-foundation`)
- [x] Replace `cb-core` → `mill-foundation`
- [x] Replace `cb-lang-common` → `mill-lang-common`
- [x] Move `tempfile` to `[dev-dependencies]`

### Update Imports
- [x] Replace `use cb_plugin_api` → `use mill_plugin_api`
- [x] Replace `use cb_protocol` → `use mill_foundation::protocol`
- [x] Replace `use cb_core` → `use mill_foundation`
- [x] Replace `cb_lang_csharp::` → `mill_lang_csharp::`
- [x] Update all protocol types to `mill_foundation::protocol::*` namespace

### Split Import Traits
- [x] Add 5 trait imports: `ImportParser`, `ImportRenameSupport`, `ImportMoveSupport`, `ImportMutationSupport`, `ImportAdvancedSupport`
- [x] Split `impl ImportSupport` into 5 separate trait implementations
- [x] Remove duplicate method implementations across traits
- [x] Add `#[derive(Default)]` to `CsharpImportSupport` struct

### Update Plugin Registration
- [x] Add `define_language_plugin!` macro to `lib.rs` (newer macro, replaces mill_plugin!)
- [x] Convert `metadata` to `pub const METADATA: LanguageMetadata`
- [x] Convert capabilities to `pub const CAPABILITIES: PluginCapabilities`
- [x] Update `new()` to return `Box<dyn LanguagePlugin>`
- [x] Add `#[derive(Default)]` to main plugin struct
- [x] Configure LSP: `LspConfig::new("csharp-ls", &[""])` (using csharp-ls)

### Update Trait Implementations
- [x] Update `metadata()` to return `&Self::METADATA`
- [x] Update `capabilities()` to return `Self::CAPABILITIES`
- [x] Replace single `import_support()` with 5 trait methods
- [x] Add `import_parser()` → `Some(&self.import_support)`
- [x] Add `import_rename_support()` → `Some(&self.import_support)`
- [x] Add `import_move_support()` → `Some(&self.import_support)`
- [x] Add `import_mutation_support()` → `Some(&self.import_support)`
- [x] Add `import_advanced_support()` → `Some(&self.import_support)`

### Add ProjectFactory
- [x] Implement `ProjectFactory` trait for CsharpPlugin
- [x] Add `create_package()` method using dotnet CLI templates
- [x] Create `.csproj` manifest template
- [x] Create project directory structure
- [x] Add C# project initialization logic
- [x] Add `project_factory()` method to return `Some(self)`

### Update EditPlanMetadata
- [x] Find all `EditPlanMetadata` initializations
- [x] Add `consolidation: None` field to each initialization

### Testing
- [x] Run `cargo check -p mill-lang-csharp` and fix compilation errors
- [x] Run `cargo test -p mill-lang-csharp` and verify all tests pass (15 tests passing)
- [x] Test AST parsing with C# source files
- [x] Test manifest parsing with `.csproj` files
- [x] Test import rewriting for rename operations
- [x] Test import rewriting for move operations
- [x] Test all 3 refactoring operations (extract function, inline variable, extract variable)
- [x] Test package creation with ProjectFactory

### Documentation
- [x] Update `docs/architecture/overview.md` language support matrix
- [x] Add C# to CLAUDE.md supported languages list
- [~] Document `.csproj` MSBuild XML format (basic parsing implemented)
- [~] Document LSP configuration (using csharp-ls, not omnisharp)

## Success Criteria

- [x] `cargo check -p mill-lang-csharp` compiles without errors
- [x] `cargo check --workspace` compiles without errors
- [~] All 15 common capabilities implemented and tested (core functionality done, some advanced features stubs)
- [x] ProjectFactory capability implemented
- [x] Unit tests pass (15 tests)
- [x] Plugin loads via `define_language_plugin!` macro
- [~] LSP integration configured with `csharp-ls` (not omnisharp)
- [x] Manifest parsing handles `.csproj` MSBuild format
- [~] Import rewriting works for C# using statements (basic implementation)
- [x] Refactoring operations generate valid C# code

## Benefits

- **.NET developers** can use TypeMill for C# projects
- **Package creation** supported via ProjectFactory
- **Feature parity** with Rust, TypeScript, Python (15 capabilities)
- **Validated migration** follows proven Python restoration process
- **Enterprise support** enables C# corporate codebases
