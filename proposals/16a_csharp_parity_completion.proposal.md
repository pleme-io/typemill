# Proposal 16a: C# Language Plugin Parity Completion

**✅ STATUS: IMPLEMENTED AND MERGED**
- **Merged**: 2025-10-31
- **Branch**: `feat/csharp-parity`
- **Tests**: 25/25 passing
- **Merge Commit**: See main branch history

## Problem

The C# language plugin (`mill-lang-csharp`) was **58% complete (7/12 traits)**, missing critical capabilities that prevented full parity with TypeScript, Rust, and Python plugins.

**Missing capabilities (NOW IMPLEMENTED):**
1. ✅ **WorkspaceSupport** - .sln file workspace management
2. ✅ **ModuleReferenceScanner** - Tracks `using` statement references
3. ✅ **ImportAnalyzer** - Builds import dependency graphs
4. ✅ **ManifestUpdater** - Updates .csproj PackageReference entries
5. ✅ **LspInstaller** - Auto-installs csharp-ls

## Solution

Completed all remaining 5 traits by implementing workspace support, analysis capabilities, and LSP installation following established patterns from Python, Java, and Rust plugins. C# now has **verified 100% parity** with test coverage.

## Checklists

### WorkspaceSupport Implementation
- [x] Create `languages/mill-lang-csharp/src/workspace_support.rs`
- [x] Implement `CsharpWorkspaceSupport` struct
- [x] Implement `WorkspaceSupport::add_workspace_member()` to add `<Project Include="..."/>` to .sln
- [x] Implement `WorkspaceSupport::remove_workspace_member()` to remove projects from .sln
- [x] Implement `WorkspaceSupport::list_workspace_members()` to parse .sln for project references
- [x] Update `lib.rs` to include `with_workspace` capability
- [x] Add `workspace_support` field to plugin struct
- [x] Reference Python `workspace_support.rs` (lines 1-200) and Java implementation

### ModuleReferenceScanner Implementation
- [x] Create `impl ModuleReferenceScanner for CsharpPlugin` in `lib.rs`
- [x] Scan for `using module_name;` statements
- [x] Scan for qualified paths: `module_name.Class`
- [x] Scan string literals: `"module_name/*.cs"`
- [x] Support all three `ScanScope` variants (All, Code, Comments)
- [x] Reference Python `lib.rs:305-388` and TypeScript `lib.rs:123-132`

### ImportAnalyzer Implementation
- [x] Add `import_analyzer: ImportAnalyzer` to `impl_capability_delegations!`
- [x] Implement `build_import_graph()` to analyze using statements
- [x] Build graph of namespace dependencies
- [x] Reference Python `lib.rs:288-303` and Rust `lib.rs:336-351`

### ManifestUpdater Implementation
- [x] Add `manifest_updater: ManifestUpdater` to delegations
- [x] Implement `update_dependency()` to modify `<PackageReference Include="..."/>` in .csproj
- [x] Implement `generate_manifest()` to generate .csproj XML
- [x] Handle version updates for NuGet packages
- [x] Reference Python `lib.rs:174-211` and Rust `lib.rs:358-392`

### LspInstaller Implementation
- [x] Create `languages/mill-lang-csharp/src/lsp_installer.rs`
- [x] Implement `CsharpLspInstaller` struct
- [x] Implement `is_installed()` to check for csharp-ls in PATH
- [x] Implement `install()` via `dotnet tool install --global csharp-ls`
- [x] Add `lsp_installer` field to plugin struct
- [x] Reference Python `lsp_installer.rs` and TypeScript implementation

### Test Coverage
- [x] Add `test_workspace_support()` to verify workspace_support field
- [x] Add `test_module_reference_scanner()` to test using statement detection
- [x] Add `test_import_analyzer()` to verify import graph building
- [x] Add `test_manifest_updater()` to test .csproj updates
- [x] Add `test_lsp_installer()` to verify csharp-ls installation check
- [x] Increase test count from 4 to 15+ total tests (achieved 25 tests)
- [x] Verify all tests pass: `cargo nextest run -p mill-lang-csharp`
- [x] Reference Python `lib.rs:390-505` (16 tests) and Java `lib.rs:134-182` (6 tests)

### Documentation Updates
- [x] Update CLAUDE.md parity table to show C# as 100%
- [x] Document .sln workspace file format handling
- [x] Document .csproj manifest update capabilities
- [x] Add C# examples to tool documentation

## Success Criteria

- [x] All 12 capability traits implemented (5 new + 7 existing)
- [x] Test count increased from 4 to 25 (exceeds 15+ requirement)
- [x] All tests pass: `cargo nextest run -p mill-lang-csharp --all-features` (25/25 passing)
- [x] `cargo check -p mill-lang-csharp` compiles without errors
- [x] CLAUDE.md parity table accurately reflects 100% completion
- [x] C# plugin matches Python/Java/Rust parity levels

## Benefits

- **C# developers** gain full TypeMill support for .NET projects and workspaces
- **.sln workspace management** enables multi-project solution refactoring
- **NuGet dependency tracking** via ManifestUpdater enables dependency-aware renames
- **Import graph analysis** enables smarter refactoring decisions
- **Auto-installer** reduces setup friction for new C# users
- **Market coverage** increases from partial to full .NET ecosystem support
- **Consistency** with other first-class language plugins (Python, Rust, TypeScript)

## References

- Python plugin (100% parity): `languages/mill-lang-python/`
- Java plugin (100% parity): `languages/mill-lang-java/`
- Rust workspace support: `languages/mill-lang-rust/src/workspace_support.rs`

## Implementation Details

**Files Created/Modified:**
- `languages/mill-lang-csharp/src/workspace_support.rs` (234 lines)
- `languages/mill-lang-csharp/src/lsp_installer.rs` (51 lines)
- `languages/mill-lang-csharp/src/manifest.rs` (enhanced)
- `languages/mill-lang-csharp/src/lib.rs` (267 lines added)

**Test Results:**
```
Summary [   0.224s] 25 tests run: 25 passed, 0 skipped
```
