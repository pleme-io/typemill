# Proposal 16d: C Language Plugin Parity Completion


**Status**: ✅ IMPLEMENTED AND MERGED
**Branch**: feat/c-language-plugin-parity
**Tests**: 23/23 passing (100%)
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
- [x] Implement `plan_extract_function()` in `refactoring.rs` (basic line-range extraction)
- [x] Implement `plan_inline_variable()` for simple variable substitution
- [x] Implement `plan_extract_variable()` for expression extraction
- [x] Update `supports_extract_function()` to return true
- [x] Update `supports_inline_variable()` to return true
- [x] Update `supports_extract_variable()` to return true
- [x] Wire up async methods to call refactoring module
- [x] Note limitations: No complex scope analysis (C has manual memory management)
- [x] Reference C++ refactoring.rs and Java implementation

### ProjectFactory Implementation
- [x] Create `languages/mill-lang-c/src/project_factory.rs`
- [x] Implement `CProjectFactory` struct
- [x] Implement `create_package()` to generate directory structure (src/, include/)
- [x] Generate basic Makefile template with CC, CFLAGS, and target rules
- [x] Generate main.c hello world template
- [x] Add `project_factory` field to `CPlugin` struct
- [x] Implement `project_factory()` method
- [x] Update capabilities to include `with_project_factory()`
- [x] Reference Java and Python project factories

### WorkspaceSupport Implementation
- [x] Create `languages/mill-lang-c/src/workspace_support.rs`
- [x] Implement `CWorkspaceSupport` struct
- [x] Implement `add_workspace_member()` to add subdirectory to root Makefile
- [x] Implement `remove_workspace_member()` to remove subdirectory entries
- [x] Implement `list_workspace_members()` to parse Makefile SUBDIRS variable
- [x] Add `workspace_support` field to `CPlugin` struct
- [x] Implement `workspace_support()` method
- [x] Update capabilities to include `with_workspace()`
- [x] Reference Java `workspace_support.rs` and Python implementation

### ModuleReferenceScanner Implementation
- [x] Create `impl ModuleReferenceScanner for CPlugin` in `lib.rs`
- [x] Scan for `#include "module.h"` statements (local headers)
- [x] Scan for `#include <module.h>` statements (system headers)
- [x] Scan string literals: `"module/*.h"`
- [x] Support all three `ScanScope` variants (All, Code, Comments)
- [x] Reference Python `lib.rs:305-388` and TypeScript `lib.rs:123-132`

### ImportAnalyzer Implementation
- [x] Create `impl ImportAnalyzer for CPlugin` in `lib.rs`
- [x] Implement `build_import_graph()` to parse #include directives
- [x] Build dependency graph of header inclusion relationships
- [x] Handle both `"local.h"` and `<system.h>` include styles
- [x] Reference Python `lib.rs:288-303` and Rust `lib.rs:336-351`

### ManifestUpdater Implementation
- [x] Create `impl ManifestUpdater for CPlugin` in `lib.rs`
- [x] Implement `update_dependency()` for Makefile LIBS variable updates
- [x] Implement `generate_manifest()` to generate basic Makefile
- [x] Support CMakeLists.txt target_link_libraries() updates (optional)
- [x] Handle library flags (-lfoo) and include paths (-I/path)
- [x] Reference Python `lib.rs:174-211` and Rust `lib.rs:358-392`

### LspInstaller Implementation
- [x] Create `languages/mill-lang-c/src/lsp_installer.rs`
- [x] Implement `CLspInstaller` struct
- [x] Implement `is_installed()` to check for clangd in PATH
- [x] Implement `install()` via package manager (apt install clangd, brew install llvm)
- [x] Add `lsp_installer` field to `CPlugin` struct
- [x] Implement `lsp_installer()` method
- [x] Reference Python `lsp_installer.rs` and TypeScript implementation

### Test Coverage
- [x] Add `test_import_parser()` to verify #include parsing
- [x] Add `test_project_factory()` to verify C project creation
- [x] Add `test_workspace_support()` to verify workspace_support field
- [x] Add `test_refactoring_extract_function()` with sample C code
- [x] Add `test_refactoring_inline_variable()` with variable declaration
- [x] Add `test_refactoring_extract_variable()` with expression
- [x] Add `test_module_reference_scanner()` to test #include detection
- [x] Add `test_import_analyzer()` to verify include graph building
- [x] Add `test_manifest_updater()` to test Makefile updates
- [x] Add `test_lsp_installer()` to verify clangd detection
- [x] Increase test count from 0 to 12+ total tests
- [x] Verify all tests pass: `cargo nextest run -p mill-lang-c`
- [x] Reference Python `lib.rs:390-505` (16 tests) and Java `lib.rs:134-182` (6 tests)

### Documentation Updates
- [x] Update CLAUDE.md parity table to show C as 100% (experimental)
- [x] Document Makefile/CMake workspace file format handling
- [x] Document experimental status and limitations
- [x] Add C examples to tool documentation
- [x] Note: No standard package manager, manual dependency management

## Success Criteria

- [x] All 12 capability traits implemented
- [x] RefactoringProvider supports 3 operations (no longer stubs)
- [x] ProjectFactory can create C projects with Makefile
- [x] WorkspaceSupport manages multi-directory C projects
- [x] Test count increased from 0 to 12+
- [x] All tests pass: `cargo nextest run -p mill-lang-c --all-features`
- [x] `cargo check -p mill-lang-c` compiles without errors
- [x] CLAUDE.md parity table shows C as 100% (experimental)
- [x] Documented as experimental with pragmatic feature set

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
