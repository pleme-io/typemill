# Proposal 16c: Go Language Plugin Parity Completion

**✅ STATUS: IMPLEMENTED AND MERGED**
- **Merged**: 2025-10-31
- **Branch**: `feat/go-plugin-parity`
- **Tests**: 30/30 passing
- **Merge Commit**: See main branch history
- **Critical Fixes**: False capability claims resolved, refactoring.rs wired up

## Problem

The Go language plugin (`mill-lang-go`) was **42% complete (5/12 traits)** with **misleading capability claims**. The plugin claimed features it didn't implement and had existing refactoring code that wasn't wired up.

**Critical issues (NOW FIXED):**
1. ✅ **ImportAdvancedSupport** - Was returning `None`, now returns `Some`
2. ✅ **WorkspaceSupport** - Claimed `workspace=true` but had no implementation, now fully implemented
3. ✅ **RefactoringProvider** - `refactoring.rs` existed but wasn't wired up, now connected
4. ✅ **ModuleReferenceScanner** - Now implemented
5. ✅ **ImportAnalyzer** - Now implemented
6. ✅ **ManifestUpdater** - Now implemented
7. ✅ **LspInstaller** - Now implemented

## Solution

Fixed all false capability claims, wired up existing refactoring code, and completed remaining 7 traits. Go now has **verified 100% parity** with go.work and go.mod support, plus all critical bugs fixed.

## Checklists - ALL COMPLETED ✅

### Fix False Claims (CRITICAL) - COMPLETED
- [x] Fix `import_advanced_support()` to return `Some(&self.import_support)` instead of `None`
- [x] Wire up existing `refactoring.rs` by adding `impl RefactoringProvider for GoPlugin`
- [x] Implement `refactoring_provider()` method to return `Some(self)`
- [x] Set `CAPABILITIES.workspace=true` with actual implementation

### WorkspaceSupport Implementation
- [x] Create `languages/mill-lang-go/src/workspace_support.rs`
- [x] Implement `GoWorkspaceSupport` struct
- [x] Implement `WorkspaceSupport::add_workspace_member()` for go.work
- [x] Implement `WorkspaceSupport::remove_workspace_member()`
- [x] Implement `WorkspaceSupport::list_workspace_members()`
- [x] Add `workspace_support` field to `GoPlugin`
- [x] Implement `workspace_support()` method
- [x] Set `CAPABILITIES.workspace=true` after implementation

### RefactoringProvider Completion
- [x] Wire up existing `refactoring::plan_inline_variable()`
- [x] Wire up existing `refactoring::plan_extract_function()`
- [x] Implement `supports_inline_variable()` returning true
- [x] Implement async `plan_inline_variable()` method
- [x] Implement `supports_extract_function()` returning true
- [x] Implement async `plan_extract_function()` method
- [x] Implement `plan_extract_variable()` in refactoring.rs
- [x] Implement `supports_extract_variable()` returning true
- [x] Implement async `plan_extract_variable()` method

### ModuleReferenceScanner Implementation
- [x] Create `impl ModuleReferenceScanner for GoPlugin`
- [x] Scan for `import "module_name"` statements
- [x] Scan for qualified paths: `module_name.Function`
- [x] Scan string literals
- [x] Support all three `ScanScope` variants

### ImportAnalyzer Implementation
- [x] Create `impl ImportAnalyzer for GoPlugin`
- [x] Implement `build_import_graph()`
- [x] Build dependency graph
- [x] Handle grouped import declarations

### ManifestUpdater Implementation
- [x] Create `impl ManifestUpdater for GoPlugin`
- [x] Implement `update_dependency()` for go.mod
- [x] Implement `generate_manifest()`
- [x] Handle version updates
- [x] Support `replace` directives

### LspInstaller Implementation
- [x] Create `languages/mill-lang-go/src/lsp_installer.rs`
- [x] Implement `GoLspInstaller` struct
- [x] Implement `is_installed()`
- [x] Implement `install()` via `go install golang.org/x/tools/gopls@latest`
- [x] Add `lsp_installer` field
- [x] Implement `lsp_installer()` method

### Test Coverage
- [x] Add `test_import_advanced_support()` - verifies returns Some
- [x] Add `test_workspace_support()`
- [x] Add `test_refactoring_provider()` - verifies wiring
- [x] Add `test_refactoring_extract_function()`
- [x] Add `test_refactoring_inline_variable()`
- [x] Add `test_refactoring_extract_variable()`
- [x] Add `test_module_reference_scanner()`
- [x] Add `test_import_analyzer()`
- [x] Add `test_manifest_updater()`
- [x] Add `test_lsp_installer()`
- [x] 30 tests total passing
- [x] Verify: `cargo nextest run -p mill-lang-go`

### Documentation Updates
- [x] Update CLAUDE.md parity table to show Go as 100%
- [x] Document go.work workspace handling
- [x] Document go.mod manifest capabilities
- [x] Document fixed capability claims

## Success Criteria - ALL MET ✅

- [x] All 12 capability traits implemented
- [x] No false capability claims (all verified)
- [x] Existing refactoring.rs properly wired up
- [x] RefactoringProvider supports 3 operations
- [x] WorkspaceSupport manages go.work
- [x] 30 tests passing (100% pass rate)
- [x] `cargo check -p mill-lang-go` compiles without errors
- [x] CLAUDE.md parity table shows 100%
- [x] Go plugin matches Python/Java/Rust parity

## Implementation Details

**Files Created/Modified:**
- `languages/mill-lang-go/src/workspace_support.rs` (80 lines)
- `languages/mill-lang-go/src/lsp_installer.rs` (51 lines)
- `languages/mill-lang-go/src/lib.rs` (283 lines added)
- `languages/mill-lang-go/src/manifest.rs` (enhanced)
- `languages/mill-lang-go/src/refactoring.rs` (enhanced)

**Test Results:**
```
Summary [   0.150s] 30 tests run: 30 passed, 0 skipped
```

## Benefits

- **Go developers** gain full TypeMill support with verified functionality
- **go.work workspace management** enables multi-module refactoring
- **Refactoring operations** (existing code) now accessible
- **Honest capability reporting** prevents bugs and confusion
- **Dependency tracking** via ManifestUpdater
- **Auto-installer** reduces setup friction
- **Consistency** with other first-class plugins

## References

- Python plugin: `languages/mill-lang-python/`
- Java plugin: `languages/mill-lang-java/`
- Existing Go refactoring: `languages/mill-lang-go/src/refactoring.rs`
