# Proposal 16e: C++ Language Plugin Parity Completion & Validation

## Problem

The C++ language plugin (`mill-lang-cpp`) claims **83% completion (10/12 traits)** but has **zero test coverage** to validate any claimed functionality. The plugin returns `Some` for 10 traits but NO tests exist to verify these implementations actually work. Additionally, 2 critical traits are missing (ManifestUpdater, LspInstaller).

**Critical validation gap:**
- **10 traits claimed** but **0 tests** to validate them
- No `#[cfg(test)]` module in `lib.rs`
- Cannot trust claimed functionality without verification

**Missing traits:**
1. **ManifestUpdater** - Cannot update CMakeLists.txt or Conan dependencies
2. **LspInstaller** - Cannot auto-install clangd for new users

**Code Evidence** (`languages/mill-lang-cpp/src/lib.rs`):
```rust
// Line 70: Claims imports + workspace
fn capabilities(&self) -> PluginCapabilities {
    PluginCapabilities::none()
        .with_imports()
        .with_workspace()  // ⚠️ Not validated by tests
}

// Lines 76-119: Returns Some for many traits
fn import_parser(&self) -> Option<&dyn ImportParser> {
    Some(&import_support::CppImportSupport)  // ⚠️ No tests
}

fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
    Some(&workspace_support::CppWorkspaceSupport)  // ⚠️ No tests
}

fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
    Some(&refactoring::CppRefactoringProvider)  // ⚠️ No tests
}

// NO #[cfg(test)] module exists!
```

## Solution

Validate all 10 claimed traits with comprehensive test coverage and implement the final 2 missing traits (ManifestUpdater, LspInstaller). This brings C++ to **verified 100% parity** with test-validated functionality for all 12 traits.

**All tasks should be completed in one implementation session** to ensure consistency and avoid partial states.

## Checklists

### Test Coverage & Validation (CRITICAL)
- [ ] Create `#[cfg(test)]` module in `languages/mill-lang-cpp/src/lib.rs`
- [ ] Add `test_cpp_plugin_creation()` to verify plugin instantiation
- [ ] Add `test_cpp_capabilities()` to verify imports and workspace capabilities
- [ ] Add `test_import_parser()` to validate `#include <iostream>` and `#include "header.hpp"` parsing
- [ ] Add `test_import_rename_support()` to validate import renaming
- [ ] Add `test_import_move_support()` to validate import path updates
- [ ] Add `test_import_mutation_support()` to validate import modification
- [ ] Add `test_import_advanced_support()` to validate advanced import operations
- [ ] Add `test_workspace_support()` to validate CMakeLists.txt workspace member management
- [ ] Add `test_refactoring_extract_function()` to validate function extraction
- [ ] Add `test_refactoring_inline_variable()` to validate variable inlining
- [ ] Add `test_refactoring_extract_variable()` to validate variable extraction
- [ ] Add `test_project_factory()` to validate C++ project creation
- [ ] Add `test_module_reference_scanner()` to validate `#include` and namespace reference scanning
- [ ] Add `test_import_analyzer()` to validate include dependency graph building
- [ ] Verify test count reaches 15+ covering all claimed functionality
- [ ] Verify all tests pass: `cargo nextest run -p mill-lang-cpp`
- [ ] Reference Python `lib.rs:390-505` (16 tests) and Java `lib.rs:134-182` (6 tests)

### ManifestUpdater Implementation
- [ ] Create `impl ManifestUpdater for CppPlugin` in `lib.rs`
- [ ] Implement `update_dependency()` for CMakeLists.txt `target_link_libraries()` updates
- [ ] Implement `generate_manifest()` to generate basic CMakeLists.txt
- [ ] Support Conan conanfile.txt dependency updates (optional)
- [ ] Support vcpkg vcpkg.json dependency updates (optional)
- [ ] Handle library linking and package finding (find_package)
- [ ] Add `manifest_updater()` method to `LanguagePlugin` trait impl
- [ ] Add test `test_manifest_updater()` to validate CMakeLists.txt updates
- [ ] Reference Python `lib.rs:174-211` and Rust `lib.rs:358-392`

### LspInstaller Implementation
- [ ] Create `languages/mill-lang-cpp/src/lsp_installer.rs`
- [ ] Implement `CppLspInstaller` struct
- [ ] Implement `is_installed()` to check for clangd in PATH
- [ ] Implement `install()` via package manager (apt install clangd, brew install llvm)
- [ ] Add `lsp_installer` field to `CppPlugin` struct
- [ ] Implement `lsp_installer()` method in `LanguagePlugin` trait
- [ ] Add test `test_lsp_installer()` to verify clangd detection
- [ ] Reference Python `lsp_installer.rs` and TypeScript implementation

### Documentation Updates
- [ ] Update CLAUDE.md parity table to show C++ as 100% (verified)
- [ ] Document CMakeLists.txt/Conan/vcpkg manifest handling
- [ ] Document test coverage proving claimed functionality
- [ ] Add C++ examples to tool documentation
- [ ] Note validation status change (claimed → verified)

## Success Criteria

- [ ] All 12 capability traits implemented AND tested
- [ ] Test count increased from 0 to 15+
- [ ] All tests pass: `cargo nextest run -p mill-lang-cpp --all-features`
- [ ] `cargo check -p mill-lang-cpp` compiles without errors
- [ ] ManifestUpdater supports CMakeLists.txt updates
- [ ] LspInstaller can auto-install clangd
- [ ] CLAUDE.md parity table shows C++ as 100% (verified by tests)
- [ ] No claimed functionality lacks test validation
- [ ] C++ plugin matches Python/Java/Rust parity levels

## Benefits

- **C++ developers** gain **verified** TypeMill support for CMake/Conan/vcpkg projects
- **Test-validated functionality** prevents false claims and bugs
- **CMakeLists.txt manifest management** enables dependency-aware refactoring
- **Refactoring operations** (extract function/variable, inline variable) proven to work
- **Import graph analysis** tracks header dependencies (validated)
- **Auto-installer** reduces setup friction (clangd installation)
- **Build system integration** supports CMake, Makefile, Bazel patterns
- **Quality assurance** with 15+ tests covering all traits
- **Consistency** with other first-class language plugins (Python, Rust, TypeScript)

## References

- Python plugin (100% parity, 16 tests): `languages/mill-lang-python/`
- Java plugin (100% parity, 6 tests): `languages/mill-lang-java/`
- C plugin implementation: `languages/mill-lang-c/`
- CMake documentation: https://cmake.org/documentation/
