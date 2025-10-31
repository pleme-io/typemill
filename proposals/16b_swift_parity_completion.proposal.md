# Proposal 16b: Swift Language Plugin Parity Completion

## Problem

The Swift language plugin (`mill-lang-swift`) is currently **42% complete (5/12 traits)**, missing critical capabilities including refactoring operations, workspace management, and dependency analysis. This incomplete implementation blocks Swift developers from using TypeMill for iOS/macOS projects and Swift Package Manager workspaces.

**Missing capabilities:**
1. **WorkspaceSupport** - Cannot manage Package.swift workspace members
2. **RefactoringProvider** - No extract function/variable or inline variable operations
3. **ModuleReferenceScanner** - Cannot track `import` statement references during renames
4. **ImportAnalyzer** - Cannot build import dependency graphs
5. **ManifestUpdater** - Cannot update Package.swift dependency entries
6. **LspInstaller** - Cannot auto-install sourcekit-lsp for new users
7. **ProjectFactory** - Exists but minimal test coverage (only 3 tests)

**Code Evidence** (`languages/mill-lang-swift/src/lib.rs`):
```rust
// Line 13: Missing with_workspace capability
capabilities: [with_imports, with_project_factory],  // NO with_workspace!

// Line 108: Only delegates import traits
impl_capability_delegations! {
    import_support => {
        import_parser: ImportParser,
        // Missing: refactoring_provider, module_reference_scanner, import_analyzer, manifest_updater
    },
}
```

## Solution

Complete the remaining 7 traits by implementing refactoring operations, workspace support, analysis capabilities, and LSP installation following the established patterns from Python, Java, and Rust plugins. This brings Swift to **100% parity** and enables full Swift Package Manager support.

**All tasks should be completed in one implementation session** to ensure consistency and avoid partial states.

## Checklists

### WorkspaceSupport Implementation
- [ ] Create `languages/mill-lang-swift/src/workspace_support.rs`
- [ ] Implement `SwiftWorkspaceSupport` struct
- [ ] Implement `WorkspaceSupport::add_workspace_member()` to add `.package(path: "...")` to Package.swift
- [ ] Implement `WorkspaceSupport::remove_workspace_member()` to remove package entries
- [ ] Implement `WorkspaceSupport::list_workspace_members()` to parse Package.swift for local dependencies
- [ ] Update `lib.rs` to include `with_workspace` capability
- [ ] Add `workspace_support` field to plugin struct
- [ ] Add delegation in `impl_capability_delegations!`
- [ ] Reference Java `workspace_support.rs` and Python implementation

### RefactoringProvider Implementation
- [ ] Create `languages/mill-lang-swift/src/refactoring.rs` module
- [ ] Implement `plan_extract_function()` - extract selected lines into new func
- [ ] Implement `plan_inline_variable()` - replace variable references with value
- [ ] Implement `plan_extract_variable()` - extract expression into new variable
- [ ] Add `impl RefactoringProvider for SwiftPlugin` in `lib.rs`
- [ ] Implement `supports_extract_function()` returning true
- [ ] Implement async `plan_extract_function()` method
- [ ] Implement `supports_inline_variable()` returning true
- [ ] Implement async `plan_inline_variable()` method
- [ ] Implement `supports_extract_variable()` returning true
- [ ] Implement async `plan_extract_variable()` method
- [ ] Add `refactoring_provider: RefactoringProvider` to delegations
- [ ] Reference C# `refactoring.rs` (similar syntax) and Java implementation

### ModuleReferenceScanner Implementation
- [ ] Create `impl ModuleReferenceScanner for SwiftPlugin` in `lib.rs`
- [ ] Scan for `import module_name` statements
- [ ] Scan for qualified paths: `module_name.Type`
- [ ] Scan string literals: `"module_name/*.swift"`
- [ ] Support all three `ScanScope` variants (All, Code, Comments)
- [ ] Reference Python `lib.rs:305-388` and TypeScript `lib.rs:123-132`

### ImportAnalyzer Implementation
- [ ] Add `import_analyzer: ImportAnalyzer` to `impl_capability_delegations!`
- [ ] Create `impl ImportAnalyzer for SwiftPlugin`
- [ ] Implement `build_import_graph()` to parse import statements
- [ ] Build dependency graph of import relationships
- [ ] Reference Python `lib.rs:288-303` and Rust `lib.rs:336-351`

### ManifestUpdater Implementation
- [ ] Add `manifest_updater: ManifestUpdater` to delegations
- [ ] Create `impl ManifestUpdater for SwiftPlugin`
- [ ] Implement `update_dependency()` to modify `.package(name: "...")` in Package.swift
- [ ] Implement `generate_manifest()` to generate Package.swift file
- [ ] Handle version updates for Swift packages
- [ ] Reference Python `lib.rs:174-211` and Rust `lib.rs:358-392`

### LspInstaller Implementation
- [ ] Create `languages/mill-lang-swift/src/lsp_installer.rs`
- [ ] Implement `SwiftLspInstaller` struct
- [ ] Implement `is_installed()` to check for sourcekit-lsp in PATH
- [ ] Implement `install()` for macOS (included with Xcode)
- [ ] Implement `install()` for Linux (via apt-get/package manager)
- [ ] Add `lsp_installer` field to plugin struct
- [ ] Reference Python `lsp_installer.rs` and TypeScript implementation

### Test Coverage
- [ ] Add `test_workspace_support()` to verify workspace_support field
- [ ] Add `test_refactoring_extract_function()` with sample Swift code
- [ ] Add `test_refactoring_inline_variable()` with let binding example
- [ ] Add `test_refactoring_extract_variable()` with expression extraction
- [ ] Add `test_module_reference_scanner()` to test import detection
- [ ] Add `test_import_analyzer()` to verify import graph building
- [ ] Add `test_manifest_updater()` to test Package.swift updates
- [ ] Add `test_lsp_installer()` to verify sourcekit-lsp detection
- [ ] Increase test count from 3 to 15+ total tests
- [ ] Verify all tests pass: `cargo nextest run -p mill-lang-swift`
- [ ] Reference Python `lib.rs:390-505` (16 tests) and Java `lib.rs:134-182` (6 tests)

### Documentation Updates
- [ ] Update CLAUDE.md parity table to show Swift as 100% (currently shows ✅ incorrectly at 42%)
- [ ] Document Package.swift workspace file format handling
- [ ] Document Swift Package Manager manifest update capabilities
- [ ] Add Swift examples to tool documentation

## Success Criteria

- [ ] All 12 capability traits implemented (7 new + 5 existing)
- [ ] RefactoringProvider supports 3 operations (extract function/variable, inline variable)
- [ ] WorkspaceSupport manages Package.swift workspace members
- [ ] Test count increased from 3 to 15+
- [ ] All tests pass: `cargo nextest run -p mill-lang-swift --all-features`
- [ ] `cargo check -p mill-lang-swift` compiles without errors
- [ ] CLAUDE.md parity table accurately reflects 100% completion
- [ ] Swift plugin matches Python/Java/Rust parity levels

## Benefits

- **Swift developers** gain full TypeMill support for iOS/macOS projects and SPM workspaces
- **Package.swift workspace management** enables multi-package refactoring
- **Refactoring operations** enable extract function/variable and inline variable transformations
- **Dependency tracking** via ManifestUpdater enables dependency-aware renames
- **Import graph analysis** enables smarter refactoring decisions
- **Auto-installer** reduces setup friction for new Swift users
- **Market coverage** increases from partial to full Apple ecosystem support
- **Consistency** with other first-class language plugins (Python, Rust, TypeScript)

## References

- Python plugin (100% parity): `languages/mill-lang-python/`
- Java plugin (100% parity): `languages/mill-lang-java/`
- C# refactoring: `languages/mill-lang-csharp/src/refactoring.rs`
