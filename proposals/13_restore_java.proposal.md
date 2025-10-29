# Restore Java Language Plugin

> **Note on Naming**: This proposal references legacy `cb-*` crate names in migration steps. These are the **old names** being migrated **from** (in git tag `pre-language-reduction`) to the current `mill-*` naming convention. Current crate names follow the `mill-*` pattern.

## Problem

Java language plugin (`mill-lang-java`) was removed during unified API refactoring and preserved in `pre-language-reduction` git tag. Java/JVM developers currently cannot use TypeMill for Java projects. Plugin needs restoration to current codebase with 100% feature parity.

## Solution

Restore Java plugin from `pre-language-reduction` tag following validated Python migration process. Implement all 15 common capabilities plus ProjectFactory for package creation.

### Technical Approach

- Extract plugin from git tag and rename to `mill-*` convention
- Update dependencies and imports to current API
- Split monolithic `ImportSupport` trait into 5 specialized traits
- Add `mill_plugin!` macro registration
- Implement ProjectFactory capability for Java project creation
- Update protocol type imports to `mill_foundation::protocol::*`

### Java-Specific Details

- **LSP Server**: `jdtls` (Eclipse JDT Language Server)
- **Manifest**: `pom.xml` (Maven) or `build.gradle` (Gradle)
- **Module Separator**: `.`
- **Source Directory**: `src/main/java` (Maven convention)
- **Parser**: Native Java parser (from pre-reduction) or `tree-sitter-java`

## Checklists

### Extract and Rename
- [ ] Extract `crates/cb-lang-java/` from `pre-language-reduction` tag
- [ ] Rename to `crates/mill-lang-java/`
- [ ] Add to workspace members in root `Cargo.toml`

### Update Dependencies
- [ ] Update `Cargo.toml` package name to `mill-lang-java`
- [ ] Replace `cb-plugin-api` → `mill-plugin-api`
- [ ] Replace `cb-protocol` → Remove (merged into `mill-foundation`)
- [ ] Replace `cb-core` → `mill-foundation`
- [ ] Replace `cb-lang-common` → `mill-lang-common`
- [ ] Move `tempfile` to `[dev-dependencies]`

### Update Imports
- [ ] Replace `use cb_plugin_api` → `use mill_plugin_api`
- [ ] Replace `use cb_protocol` → `use mill_foundation::protocol`
- [ ] Replace `use cb_core` → `use mill_foundation`
- [ ] Replace `cb_lang_java::` → `mill_lang_java::`
- [ ] Update all protocol types to `mill_foundation::protocol::*` namespace

### Split Import Traits
- [ ] Add 5 trait imports: `ImportParser`, `ImportRenameSupport`, `ImportMoveSupport`, `ImportMutationSupport`, `ImportAdvancedSupport`
- [ ] Split `impl ImportSupport` into 5 separate trait implementations
- [ ] Remove duplicate method implementations across traits
- [ ] Add `#[derive(Default)]` to `JavaImportSupport` struct

### Update Plugin Registration
- [ ] Add `mill_plugin!` macro to `lib.rs`
- [ ] Convert `metadata` to `pub const METADATA: LanguageMetadata`
- [ ] Convert capabilities to `pub const CAPABILITIES: PluginCapabilities`
- [ ] Update `new()` to return `Box<dyn LanguagePlugin>`
- [ ] Add `#[derive(Default)]` to main plugin struct
- [ ] Configure LSP: `LspConfig::new("jdtls", &["jdtls"])`

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
- [ ] Implement `ProjectFactory` trait for JavaPlugin
- [ ] Add `create_package()` method using Maven/Gradle templates
- [ ] Create `pom.xml` manifest template (Maven)
- [ ] Create `build.gradle` manifest template (Gradle)
- [ ] Create Maven standard directory structure (`src/main/java`, `src/test/java`)
- [ ] Add Java project initialization logic
- [ ] Add `project_factory()` method to return `Some(self)`

### Update EditPlanMetadata
- [ ] Find all `EditPlanMetadata` initializations
- [ ] Add `consolidation: None` field to each initialization

### Testing
- [ ] Run `cargo check -p mill-lang-java` and fix compilation errors
- [ ] Run `cargo test -p mill-lang-java` and verify all tests pass
- [ ] Test AST parsing with Java source files
- [ ] Test manifest parsing with `pom.xml` files
- [ ] Test manifest parsing with `build.gradle` files
- [ ] Test import rewriting for rename operations
- [ ] Test import rewriting for move operations
- [ ] Test all 3 refactoring operations (extract function, inline variable, extract variable)
- [ ] Test package creation with ProjectFactory

### Documentation
- [ ] Update `docs/architecture/overview.md` language support matrix
- [ ] Add Java to CLAUDE.md supported languages list
- [ ] Document Maven `pom.xml` format
- [ ] Document Gradle `build.gradle` format
- [ ] Document jdtls LSP configuration

## Success Criteria

- [ ] `cargo check -p mill-lang-java` compiles without errors
- [ ] `cargo check --workspace` compiles without errors
- [ ] All 15 common capabilities implemented and tested
- [ ] ProjectFactory capability implemented
- [ ] Unit tests pass
- [ ] Plugin loads via `mill_plugin!` macro
- [ ] LSP integration works with `jdtls`
- [ ] Manifest parsing handles `pom.xml` and `build.gradle` formats
- [ ] Import rewriting works for Java import statements
- [ ] Refactoring operations generate valid Java code

## Benefits

- **Java developers** can use TypeMill for enterprise projects
- **Package creation** supported via ProjectFactory
- **Feature parity** with Rust, TypeScript, Python (15 capabilities)
- **Validated migration** follows proven Python restoration process
- **Enterprise support** enables large-scale JVM codebases
- **Maven/Gradle support** covers both major Java build systems