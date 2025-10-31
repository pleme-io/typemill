# Proposal 16a: C# Language Plugin Parity Completion

## Problem

The C# language plugin (`mill-lang-csharp`) is currently **58% complete (7/12 traits)**, missing critical capabilities that prevent full parity with TypeScript, Rust, and Python plugins. This incomplete implementation blocks C# developers from using TypeMill for workspace management, dependency analysis, and manifest operations on .NET projects.

**Missing capabilities:**
1. **WorkspaceSupport** - Cannot manage .sln file workspace members
2. **ModuleReferenceScanner** - Cannot track `using` statement references during renames
3. **ImportAnalyzer** - Cannot build import dependency graphs
4. **ManifestUpdater** - Cannot update .csproj PackageReference entries
5. **LspInstaller** - Cannot auto-install csharp-ls for new users

**Code Evidence** (`languages/mill-lang-csharp/src/lib.rs`):
```rust
// Line 23: Missing with_workspace capability
capabilities: [with_imports, with_project_factory],  // NO with_workspace!

// Line 54: Only delegates RefactoringProvider
impl_capability_delegations! {
    this => {
        refactoring_provider: RefactoringProvider,  // Missing 3 other traits
    },
}
```

## Solution

Complete the remaining 5 traits by implementing workspace support, analysis capabilities, and LSP installation following the established patterns from Python, Java, and Rust plugins. This brings C# to **100% parity** and enables full .NET project support.

**All tasks should be completed in one implementation session** to ensure consistency and avoid partial states.

## Checklists

### WorkspaceSupport Implementation
- [ ] Create `languages/mill-lang-csharp/src/workspace_support.rs`
- [ ] Implement `CsharpWorkspaceSupport` struct
- [ ] Implement `WorkspaceSupport::add_workspace_member()` to add `<Project Include="..."/>` to .sln
- [ ] Implement `WorkspaceSupport::remove_workspace_member()` to remove projects from .sln
- [ ] Implement `WorkspaceSupport::list_workspace_members()` to parse .sln for project references
- [ ] Update `lib.rs` to include `with_workspace` capability
- [ ] Add `workspace_support` field to plugin struct
- [ ] Reference Python `workspace_support.rs` (lines 1-200) and Java implementation

### ModuleReferenceScanner Implementation
- [ ] Create `impl ModuleReferenceScanner for CsharpPlugin` in `lib.rs`
- [ ] Scan for `using module_name;` statements
- [ ] Scan for qualified paths: `module_name.Class`
- [ ] Scan string literals: `"module_name/*.cs"`
- [ ] Support all three `ScanScope` variants (All, Code, Comments)
- [ ] Reference Python `lib.rs:305-388` and TypeScript `lib.rs:123-132`

### ImportAnalyzer Implementation
- [ ] Add `import_analyzer: ImportAnalyzer` to `impl_capability_delegations!`
- [ ] Implement `build_import_graph()` to analyze using statements
- [ ] Build graph of namespace dependencies
- [ ] Reference Python `lib.rs:288-303` and Rust `lib.rs:336-351`

### ManifestUpdater Implementation
- [ ] Add `manifest_updater: ManifestUpdater` to delegations
- [ ] Implement `update_dependency()` to modify `<PackageReference Include="..."/>` in .csproj
- [ ] Implement `generate_manifest()` to generate .csproj XML
- [ ] Handle version updates for NuGet packages
- [ ] Reference Python `lib.rs:174-211` and Rust `lib.rs:358-392`

### LspInstaller Implementation
- [ ] Create `languages/mill-lang-csharp/src/lsp_installer.rs`
- [ ] Implement `CsharpLspInstaller` struct
- [ ] Implement `is_installed()` to check for csharp-ls in PATH
- [ ] Implement `install()` via `dotnet tool install --global csharp-ls`
- [ ] Add `lsp_installer` field to plugin struct
- [ ] Reference Python `lsp_installer.rs` and TypeScript implementation

### Test Coverage
- [ ] Add `test_workspace_support()` to verify workspace_support field
- [ ] Add `test_module_reference_scanner()` to test using statement detection
- [ ] Add `test_import_analyzer()` to verify import graph building
- [ ] Add `test_manifest_updater()` to test .csproj updates
- [ ] Add `test_lsp_installer()` to verify csharp-ls installation check
- [ ] Increase test count from 4 to 15+ total tests
- [ ] Verify all tests pass: `cargo nextest run -p mill-lang-csharp`
- [ ] Reference Python `lib.rs:390-505` (16 tests) and Java `lib.rs:134-182` (6 tests)

### Documentation Updates
- [ ] Update CLAUDE.md parity table to show C# as 100% (currently shows ✅ incorrectly at 58%)
- [ ] Document .sln workspace file format handling
- [ ] Document .csproj manifest update capabilities
- [ ] Add C# examples to tool documentation

## Success Criteria

- [ ] All 12 capability traits implemented (5 new + 7 existing)
- [ ] Test count increased from 4 to 15+
- [ ] All tests pass: `cargo nextest run -p mill-lang-csharp --all-features`
- [ ] `cargo check -p mill-lang-csharp` compiles without errors
- [ ] CLAUDE.md parity table accurately reflects 100% completion
- [ ] C# plugin matches Python/Java/Rust parity levels

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
