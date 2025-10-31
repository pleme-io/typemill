# Proposal 16d: C Language Plugin Parity Completion

## Problem

The C language plugin (`mill-lang-c`) is currently **42% complete (5/12 traits)** with stub implementations and **zero test coverage**. While C is an experimental language plugin due to inherent complexity (no standard package manager, no native module system), completing the remaining traits enables basic TypeMill functionality for C projects using Makefile/CMake build systems.

**Missing/incomplete capabilities:**
1. **RefactoringProvider** - EXISTS but all `supports_*()` return false (stub-only)
2. **WorkspaceSupport** - Not implemented (multi-directory C projects)
3. **ProjectFactory** - Not implemented (cannot create new C projects)
4. **ModuleReferenceScanner** - Not implemented (cannot track #include references)
5. **ImportAnalyzer** - Not implemented (cannot build include dependency graph)
6. **ManifestUpdater** - Not implemented (cannot update Makefile/CMakeLists.txt)
7. **LspInstaller** - Not implemented (cannot auto-install clangd)

**Code Evidence** (`languages/mill-lang-c/src/lib.rs`):
```rust
// Line 62: Only claims imports
fn capabilities(&self) -> PluginCapabilities {
    PluginCapabilities::none().with_imports()  // Missing other capabilities
}

// Line 104-159: RefactoringProvider is all stubs
fn supports_inline_variable(&self) -> bool {
    false  // ❌ Not yet supported
}
fn supports_extract_function(&self) -> bool {
    false  // ❌ Not yet supported
}
```

**Test Evidence** (`languages/mill-lang-c/src/tests.rs`):
- Module exists but has **zero test functions** (empty file)

## Solution

Complete the remaining 6-7 traits with **pragmatic implementations** that acknowledge C's limitations while providing useful functionality. Focus on Makefile/CMake support and basic refactoring operations. This brings C to **100% trait coverage** (experimental feature set) suitable for embedded systems and legacy C codebases.

**All tasks should be completed in one implementation session** to ensure consistency and avoid partial states.

## Checklists

### RefactoringProvider Implementation
- [ ] Implement `plan_extract_function()` in `refactoring.rs` (basic line-range extraction)
- [ ] Implement `plan_inline_variable()` for simple variable substitution
- [ ] Implement `plan_extract_variable()` for expression extraction
- [ ] Update `supports_extract_function()` to return true
- [ ] Update `supports_inline_variable()` to return true
- [ ] Update `supports_extract_variable()` to return true
- [ ] Wire up async methods to call refactoring module
- [ ] Note limitations: No complex scope analysis (C has manual memory management)
- [ ] Reference C++ refactoring.rs and Java implementation

### ProjectFactory Implementation
- [ ] Create `languages/mill-lang-c/src/project_factory.rs`
- [ ] Implement `CProjectFactory` struct
- [ ] Implement `create_package()` to generate directory structure (src/, include/)
- [ ] Generate basic Makefile template with CC, CFLAGS, and target rules
- [ ] Generate main.c hello world template
- [ ] Add `project_factory` field to `CPlugin` struct
- [ ] Implement `project_factory()` method
- [ ] Update capabilities to include `with_project_factory()`
- [ ] Reference Java and Python project factories

### WorkspaceSupport Implementation
- [ ] Create `languages/mill-lang-c/src/workspace_support.rs`
- [ ] Implement `CWorkspaceSupport` struct
- [ ] Implement `add_workspace_member()` to add subdirectory to root Makefile
- [ ] Implement `remove_workspace_member()` to remove subdirectory entries
- [ ] Implement `list_workspace_members()` to parse Makefile SUBDIRS variable
- [ ] Add `workspace_support` field to `CPlugin` struct
- [ ] Implement `workspace_support()` method
- [ ] Update capabilities to include `with_workspace()`
- [ ] Reference Java `workspace_support.rs` and Python implementation

### ModuleReferenceScanner Implementation
- [ ] Create `impl ModuleReferenceScanner for CPlugin` in `lib.rs`
- [ ] Scan for `#include "module.h"` statements (local headers)
- [ ] Scan for `#include <module.h>` statements (system headers)
- [ ] Scan string literals: `"module/*.h"`
- [ ] Support all three `ScanScope` variants (All, Code, Comments)
- [ ] Reference Python `lib.rs:305-388` and TypeScript `lib.rs:123-132`

### ImportAnalyzer Implementation
- [ ] Create `impl ImportAnalyzer for CPlugin` in `lib.rs`
- [ ] Implement `build_import_graph()` to parse #include directives
- [ ] Build dependency graph of header inclusion relationships
- [ ] Handle both `"local.h"` and `<system.h>` include styles
- [ ] Reference Python `lib.rs:288-303` and Rust `lib.rs:336-351`

### ManifestUpdater Implementation
- [ ] Create `impl ManifestUpdater for CPlugin` in `lib.rs`
- [ ] Implement `update_dependency()` for Makefile LIBS variable updates
- [ ] Implement `generate_manifest()` to generate basic Makefile
- [ ] Support CMakeLists.txt target_link_libraries() updates (optional)
- [ ] Handle library flags (-lfoo) and include paths (-I/path)
- [ ] Reference Python `lib.rs:174-211` and Rust `lib.rs:358-392`

### LspInstaller Implementation
- [ ] Create `languages/mill-lang-c/src/lsp_installer.rs`
- [ ] Implement `CLspInstaller` struct
- [ ] Implement `is_installed()` to check for clangd in PATH
- [ ] Implement `install()` via package manager (apt install clangd, brew install llvm)
- [ ] Add `lsp_installer` field to `CPlugin` struct
- [ ] Implement `lsp_installer()` method
- [ ] Reference Python `lsp_installer.rs` and TypeScript implementation

### Test Coverage
- [ ] Add `test_import_parser()` to verify #include parsing
- [ ] Add `test_project_factory()` to verify C project creation
- [ ] Add `test_workspace_support()` to verify workspace_support field
- [ ] Add `test_refactoring_extract_function()` with sample C code
- [ ] Add `test_refactoring_inline_variable()` with variable declaration
- [ ] Add `test_refactoring_extract_variable()` with expression
- [ ] Add `test_module_reference_scanner()` to test #include detection
- [ ] Add `test_import_analyzer()` to verify include graph building
- [ ] Add `test_manifest_updater()` to test Makefile updates
- [ ] Add `test_lsp_installer()` to verify clangd detection
- [ ] Increase test count from 0 to 12+ total tests
- [ ] Verify all tests pass: `cargo nextest run -p mill-lang-c`
- [ ] Reference Python `lib.rs:390-505` (16 tests) and Java `lib.rs:134-182` (6 tests)

### Documentation Updates
- [ ] Update CLAUDE.md parity table to show C as 100% (experimental)
- [ ] Document Makefile/CMake workspace file format handling
- [ ] Document experimental status and limitations
- [ ] Add C examples to tool documentation
- [ ] Note: No standard package manager, manual dependency management

## Success Criteria

- [ ] All 12 capability traits implemented
- [ ] RefactoringProvider supports 3 operations (no longer stubs)
- [ ] ProjectFactory can create C projects with Makefile
- [ ] WorkspaceSupport manages multi-directory C projects
- [ ] Test count increased from 0 to 12+
- [ ] All tests pass: `cargo nextest run -p mill-lang-c --all-features`
- [ ] `cargo check -p mill-lang-c` compiles without errors
- [ ] CLAUDE.md parity table shows C as 100% (experimental)
- [ ] Documented as experimental with pragmatic feature set

## Benefits

- **C developers** gain TypeMill support for embedded systems and legacy codebases
- **Makefile/CMake support** enables manifest-aware operations
- **Basic refactoring** enables extract function/variable and inline variable
- **Multi-directory projects** supported via WorkspaceSupport
- **Import graph analysis** tracks header dependencies
- **Auto-installer** reduces setup friction (clangd installation)
- **Experimental status** sets appropriate expectations (no package manager, limited module system)
- **Consistency** with plugin architecture (all 12 traits implemented)

## References

- Python plugin (100% parity): `languages/mill-lang-python/`
- C++ plugin implementation: `languages/mill-lang-cpp/`
- Makefile format documentation: GNU Make manual
