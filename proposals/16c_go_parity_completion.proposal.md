# Proposal 16c: Go Language Plugin Parity Completion

## Problem

The Go language plugin (`mill-lang-go`) is currently **42% complete (5/12 traits)** and has **misleading capability claims**. The plugin claims to support features it doesn't implement (workspace support, advanced imports) and has existing refactoring code that isn't wired up. This incomplete and inconsistent implementation blocks Go developers from using TypeMill for Go module workspaces.

**Critical issues:**
1. **ImportAdvancedSupport** - Returns `None` despite claiming `imports=true`
2. **WorkspaceSupport** - Claims `workspace=true` in capabilities but NO `workspace_support()` method exists
3. **RefactoringProvider** - `refactoring.rs` EXISTS with code but NOT wired up in `lib.rs`
4. **ModuleReferenceScanner** - Not implemented
5. **ImportAnalyzer** - Not implemented
6. **ManifestUpdater** - Not implemented
7. **LspInstaller** - Not implemented

**Code Evidence** (`languages/mill-lang-go/src/lib.rs`):
```rust
// Line 30: CLAIMS workspace support but doesn't implement!
pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
    workspace: true,  // ❌ FALSE CLAIM
};

// Line 89: Returns None for ImportAdvancedSupport
fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
    None  // ❌ Should return Some(&self.import_support)
}

// NO workspace_support() method despite capability claim
// NO refactoring_provider() method despite refactoring.rs existing!
```

## Solution

Fix false capability claims, wire up existing refactoring code, and complete the remaining 7 traits by implementing workspace support (go.work files), analysis capabilities, and LSP installation. This brings Go to **100% parity** and enables full Go module workspace support.

**All tasks should be completed in one implementation session** to ensure consistency and avoid partial states.

## Checklists

### Fix False Claims (CRITICAL)
- [ ] Fix `import_advanced_support()` to return `Some(&self.import_support)` instead of `None`
- [ ] Wire up existing `refactoring.rs` by adding `impl RefactoringProvider for GoPlugin`
- [ ] Implement `refactoring_provider()` method to return `Some(self)`
- [ ] Fix or remove `CAPABILITIES.workspace=true` claim (remove until WorkspaceSupport implemented)

### WorkspaceSupport Implementation
- [ ] Create `languages/mill-lang-go/src/workspace_support.rs`
- [ ] Implement `GoWorkspaceSupport` struct
- [ ] Implement `WorkspaceSupport::add_workspace_member()` to add `use ./path` to go.work
- [ ] Implement `WorkspaceSupport::remove_workspace_member()` to remove use directives
- [ ] Implement `WorkspaceSupport::list_workspace_members()` to parse go.work file
- [ ] Add `workspace_support` field to `GoPlugin` struct
- [ ] Implement `workspace_support()` method in `LanguagePlugin` trait
- [ ] Set `CAPABILITIES.workspace=true` after implementation complete
- [ ] Reference Java `workspace_support.rs` and Python implementation

### RefactoringProvider Completion
- [ ] Wire up existing `refactoring::plan_inline_variable()` in lib.rs
- [ ] Wire up existing `refactoring::plan_extract_function()` in lib.rs
- [ ] Implement `supports_inline_variable()` returning true
- [ ] Implement async `plan_inline_variable()` method
- [ ] Implement `supports_extract_function()` returning true
- [ ] Implement async `plan_extract_function()` method
- [ ] Implement `plan_extract_variable()` in refactoring.rs (currently missing)
- [ ] Implement `supports_extract_variable()` returning true
- [ ] Implement async `plan_extract_variable()` method
- [ ] Reference C# `refactoring.rs` and Java implementation

### ModuleReferenceScanner Implementation
- [ ] Create `impl ModuleReferenceScanner for GoPlugin` in `lib.rs`
- [ ] Scan for `import "module_name"` statements
- [ ] Scan for qualified paths: `module_name.Function`
- [ ] Scan string literals: `"module_name/*.go"`
- [ ] Support all three `ScanScope` variants (All, Code, Comments)
- [ ] Reference Python `lib.rs:305-388` and TypeScript `lib.rs:123-132`

### ImportAnalyzer Implementation
- [ ] Create `impl ImportAnalyzer for GoPlugin` in `lib.rs`
- [ ] Implement `build_import_graph()` to parse import statements
- [ ] Build dependency graph of import relationships
- [ ] Handle both single and grouped import declarations
- [ ] Reference Python `lib.rs:288-303` and Rust `lib.rs:336-351`

### ManifestUpdater Implementation
- [ ] Create `impl ManifestUpdater for GoPlugin` in `lib.rs`
- [ ] Implement `update_dependency()` to modify `require` directives in go.mod
- [ ] Implement `generate_manifest()` to generate go.mod file
- [ ] Handle version updates for Go modules
- [ ] Support `replace` directives for local development
- [ ] Reference Python `lib.rs:174-211` and Rust `lib.rs:358-392`

### LspInstaller Implementation
- [ ] Create `languages/mill-lang-go/src/lsp_installer.rs`
- [ ] Implement `GoLspInstaller` struct
- [ ] Implement `is_installed()` to check for gopls in PATH
- [ ] Implement `install()` via `go install golang.org/x/tools/gopls@latest`
- [ ] Add `lsp_installer` field to plugin struct
- [ ] Implement `lsp_installer()` method in `LanguagePlugin` trait
- [ ] Reference Python `lsp_installer.rs` and TypeScript implementation

### Test Coverage
- [ ] Add `test_import_advanced_support()` to verify it returns Some
- [ ] Add `test_workspace_support()` to verify workspace_support field
- [ ] Add `test_refactoring_provider()` to verify refactoring_provider wiring
- [ ] Add `test_refactoring_extract_function()` with sample Go code
- [ ] Add `test_refactoring_inline_variable()` with var declaration example
- [ ] Add `test_refactoring_extract_variable()` with expression extraction
- [ ] Add `test_module_reference_scanner()` to test import detection
- [ ] Add `test_import_analyzer()` to verify import graph building
- [ ] Add `test_manifest_updater()` to test go.mod updates
- [ ] Add `test_lsp_installer()` to verify gopls detection
- [ ] Increase test count from 3 to 15+ total tests
- [ ] Verify all tests pass: `cargo nextest run -p mill-lang-go`
- [ ] Reference Python `lib.rs:390-505` (16 tests) and Java `lib.rs:134-182` (6 tests)

### Documentation Updates
- [ ] Update CLAUDE.md parity table to show Go as 100% (currently shows ✅ incorrectly at 42%)
- [ ] Document go.work workspace file format handling
- [ ] Document go.mod manifest update capabilities
- [ ] Add Go examples to tool documentation
- [ ] Document fixed capability claims

## Success Criteria

- [ ] All 12 capability traits implemented (7 new + 5 existing)
- [ ] No false capability claims (ImportAdvancedSupport returns Some, workspace claim accurate)
- [ ] Existing refactoring.rs code properly wired up
- [ ] RefactoringProvider supports 3 operations (extract function/variable, inline variable)
- [ ] WorkspaceSupport manages go.work workspace files
- [ ] Test count increased from 3 to 15+
- [ ] All tests pass: `cargo nextest run -p mill-lang-go --all-features`
- [ ] `cargo check -p mill-lang-go` compiles without errors
- [ ] CLAUDE.md parity table accurately reflects 100% completion
- [ ] Go plugin matches Python/Java/Rust parity levels

## Benefits

- **Go developers** gain full TypeMill support for Go module projects and workspaces
- **go.work workspace management** enables multi-module refactoring
- **Refactoring operations** (already partially implemented) become accessible
- **Honest capability reporting** prevents confusion and bugs
- **Dependency tracking** via ManifestUpdater enables dependency-aware renames
- **Import graph analysis** enables smarter refactoring decisions
- **Auto-installer** reduces setup friction for new Go users
- **Market coverage** increases from partial to full Go ecosystem support
- **Consistency** with other first-class language plugins (Python, Rust, TypeScript)

## References

- Python plugin (100% parity): `languages/mill-lang-python/`
- Java plugin (100% parity): `languages/mill-lang-java/`
- Existing Go refactoring code: `languages/mill-lang-go/src/refactoring.rs`
