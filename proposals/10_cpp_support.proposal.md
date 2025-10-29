# C++ Language Support

**Status**: 🚧 In Progress (75% complete) - Core refactoring operations implemented (Phase 1 Complete)
⚠️ **Blocker**: Uses tree-sitter 0.20 which conflicts with workspace tree-sitter 0.25. Requires API migration to tree-sitter 0.23+ for compatibility. Excluded from default build.

**Last Updated**: 2025-10-29 (after implementing core refactoring and identifying tree-sitter version conflict)

## Problem

C++ developers (Rank 2 language, AAA games, embedded systems) cannot use TypeMill. No LSP integration or language plugin exists for C++ projects.

## Solution

Implement full C++ support with `clangd` LSP integration and `mill-lang-cpp` plugin. Use tree-sitter-cpp for AST parsing and support major build systems (CMake, Makefile) and package managers (Conan, vcpkg).

## Current Implementation Status

**Completed:**
- ✅ Basic plugin structure (`crates/mill-lang-cpp`)
- ✅ LSP configuration (clangd)
- ✅ tree-sitter-cpp integration
- ✅ Basic AST parsing (classes, functions, namespaces)
- ✅ Basic `#include` import parsing
- ✅ C++20 `import` statement parsing
- ✅ Basic CMakeLists.txt parsing (project name only)
- ✅ Advanced CMakeLists.txt parsing (add_executable, variables, dependencies)
- ✅ Basic Conan manifest parsing (conanfile.txt)
- ✅ vcpkg manifest parsing (vcpkg.json)
- ✅ ProjectFactory for creating new C++ projects (with CMake)
- ✅ Import rewriting for rename operations
- ✅ Import rewriting for move operations
- ✅ Import mutation support (add/remove imports)
- ✅ WorkspaceSupport trait (stub implementation)
- ✅ **RefactoringProvider trait (FULL implementation - Phase 1 Complete)**
  - ✅ Extract function (simple cases without templates/macros)
  - ✅ Extract variable (using C++ `auto` for type deduction)
  - ✅ Inline variable (with reference finding and replacement)
- ✅ ModuleReferenceScanner trait (stub implementation)
- ✅ ImportAnalyzer trait (stub implementation)
- ✅ All unit tests passing (23 tests, +3 new refactoring tests)
- ✅ Compiles without errors

**Partially Implemented:**
- ⚠️ Import support traits (basic implementation, not all edge cases)
- ⚠️ CMake parsing (basic functionality, missing some advanced features)
- ⚠️ Conan support (basic parsing only)
- ⚠️ Workspace operations (stub only, multi-package not implemented)
- ⚠️ Analysis capabilities (stubs return not-implemented errors)

**Not Yet Implemented:**
- ❌ Complete Conan conanfile.py parsing (Python DSL)
- ❌ Full workspace operations (CMake add_subdirectory parsing)
- ❌ Advanced refactoring features (template extraction, parameter detection, return type inference)
- ❌ Full analysis capabilities (dependency graph, complexity analysis)
- ❌ Full LSP integration testing with clangd

### Technical Approach

- **LSP Server**: `clangd` (part of LLVM)
- **AST Parser**: `tree-sitter-cpp`
- **Build Systems**: CMakeLists.txt, Makefile, Bazel
- **Package Managers**: Conan (`conanfile.txt`, `conanfile.py`), vcpkg (`vcpkg.json`)
- **Import Syntax**: `#include`, `import` (C++20 modules)

## Checklists

### LSP Integration
- [x] Configure file extensions (`.cpp`, `.cc`, `.cxx`, `.h`, `.hpp`)
- [x] Configure LSP: `LspConfig::new("clangd", &["clangd"])`
- [x] Plugin registered in bundle and languages.toml
- [ ] Add `clangd` to default LSP server configurations in `mill setup`
- [ ] Document installation instructions (`apt install clangd` / LLVM downloads)
- [ ] Test initialization and basic navigation (find definition, references)
- [ ] Verify diagnostics and code actions work
- [ ] Test with real C++ projects (multiple build systems)

### Language Plugin (`crates/mill-lang-cpp`)
- [x] Create crate structure following `mill-lang-*` pattern
- [x] Add to workspace members in root `Cargo.toml`
- [x] Implement `LanguagePlugin` trait
- [x] Set metadata (name: "C++", extensions: `["cpp", "cc", "cxx", "h", "hpp"]`)
- [x] Configure LSP: `LspConfig::new("clangd", &["clangd"])`
- [x] Implement `define_language_plugin!` macro registration
- [x] Register in plugin system for auto-discovery

### AST Parsing
- [x] Integrate `tree-sitter-cpp` dependency
- [x] Implement `parse()` method for symbol extraction
- [x] Parse classes, structs, unions, namespaces
- [x] Parse function definitions
- [x] Basic test coverage (`test_parse_symbols`)
- [ ] Handle C++ templates robustly
- [ ] Handle macros (fallback to clangd LSP)
- [ ] Extract symbol hierarchy (class members, nested namespaces)
- [ ] Parse methods inside classes (currently only top-level)

### Import Support (5 Traits)
- [x] Implement `ImportParser` trait
  - [x] Parse `#include <system>` headers
  - [x] Parse `#include "local"` headers
  - [x] Parse C++20 `import` statements
  - [x] Basic test coverage (`test_parse_imports`, `test_parse_cpp20_imports`)
- [x] Implement `ImportRenameSupport` trait (functional)
- [x] Implement `ImportMoveSupport` trait (functional)
- [x] Implement `ImportMutationSupport` trait (functional)
- [x] Implement `ImportAdvancedSupport` trait (functional)
- [x] Import rewriting logic implemented (path updates for rename/move)
- [x] Tests for import rewriting operations (4 tests passing)

### Manifest Parsing
- [x] Implement `analyze_manifest()` for CMakeLists.txt
  - [x] Parse `project()` declarations (basic)
  - [x] Parse `add_executable()`, `add_library()`
  - [x] Parse `target_link_libraries()` dependencies
  - [x] Parse CMake variables (${VAR} substitution)
  - [x] Advanced test coverage (`test_analyze_cmake_manifest_advanced`)
  - [ ] Extract source file lists
- [x] Implement Conan manifest parsing (basic)
  - [x] Parse `conanfile.txt` dependencies
  - [x] Basic test coverage (`test_analyze_conan_manifest`)
  - [ ] Parse `conanfile.py` Python DSL
- [ ] Implement Makefile parsing (basic)
  - [ ] Parse targets and dependencies
  - [ ] Extract source file lists
- [ ] Implement Bazel BUILD file parsing (optional)
  - [ ] Parse `cc_library`, `cc_binary` rules

### Package Manager Integration
- [x] Implement Conan support (basic)
  - [x] Parse `conanfile.txt` dependencies
  - [ ] Parse `conanfile.py` Python DSL
  - [x] Extract package requirements
- [x] Implement vcpkg support
  - [x] Parse `vcpkg.json` manifest
  - [x] Extract dependency list
- [ ] Implement manifest update capabilities

### Advanced Features
- [x] Implement `WorkspaceSupport` trait (stub implementation)
  - [x] Basic trait methods defined
  - [x] Returns not-implemented for complex operations
  - [ ] Full CMake add_subdirectory parsing
  - [ ] Full workspace member management
- [ ] Implement `ManifestUpdater` trait
- [x] Implement `RefactoringProvider` trait (**COMPLETE - Phase 1**)
  - [x] Extract function (simple cases without templates/macros)
  - [x] Extract variable (using C++ `auto` keyword)
  - [x] Inline variable (with reference finding and replacement)
  - [x] 3 comprehensive tests added and passing
  - [ ] Advanced features (template extraction, parameter detection, return type inference)
- [x] Implement `ModuleReferenceScanner` trait (stub implementation)
  - [x] Basic trait methods defined
  - [x] Returns empty results
  - [ ] Full #include reference scanning
  - [ ] Full C++20 import reference scanning
- [x] Implement `ImportAnalyzer` trait (stub implementation)
  - [x] Basic trait methods defined
  - [x] Returns not-implemented errors
  - [ ] Full import graph building
  - [ ] Full dependency analysis
- [x] Implement `ProjectFactory` trait for creating new C++ projects
  - [x] CMake project template
  - [x] Directory structure (include/, src/, tests/)
  - [x] Basic main.cpp and CMakeLists.txt generation
  - [x] Test coverage (`test_project_factory`)

### Testing
- [x] Unit tests for AST parsing (classes, namespaces, functions) - `test_parse_symbols`
- [x] Unit tests for `#include` parsing - `test_parse_imports`
- [x] Unit tests for basic CMakeLists.txt parsing - `test_analyze_cmake_manifest`
- [x] Unit tests for advanced CMakeLists.txt parsing - `test_analyze_cmake_manifest_advanced`
- [x] Unit tests for Conan manifest parsing - `test_analyze_conan_manifest`
- [x] Unit tests for import rewriting (rename) - `test_rewrite_imports_for_rename`
- [x] Unit tests for import rewriting (move) - `test_rewrite_imports_for_move`, `test_rewrite_imports_for_move_to_root`, `test_rewrite_imports_for_move_sibling_dirs`
- [x] Unit tests for import mutation - 5 tests in `import_mutation_tests` module
- [x] Unit tests for ProjectFactory - `test_project_factory`
- [x] Unit tests for C++20 `import` parsing - `test_parse_cpp20_imports`
- [x] Unit tests for refactoring operations - `test_extract_cpp_function`, `test_extract_cpp_variable`, `test_inline_cpp_variable`
- [x] All 23 unit tests passing
- [ ] Integration tests with CMake projects
- [ ] Integration tests with Makefile projects
- [ ] LSP integration tests with `clangd`
- [ ] Test with complex C++ codebases (Boost, LLVM patterns)

### Documentation
- [x] Update `docs/architecture/overview.md` language support matrix (added C++ column)
- [x] Add C++ to CLAUDE.md supported languages list
- [x] Document current implementation status in proposal
- [ ] Add C++ examples to `docs/tools/` documentation
- [ ] Document clangd installation and configuration
- [ ] Document build system integration notes
- [ ] Create C++ plugin development guide
- [ ] Note template/macro limitations (rely on clangd for complex cases)

## Success Criteria

### Phase 1: Foundation (Completed - 100%)
- [x] `cargo check -p mill-lang-cpp` compiles without errors
- [x] `cargo check --workspace` compiles
- [x] Basic AST parsing tests pass
- [x] Basic import parsing tests pass
- [x] Basic manifest parsing tests pass
- [x] Plugin structure follows TypeMill conventions
- [x] `define_language_plugin!` macro registration works

### Phase 2: Core Features (Current State - 85% Complete)
- [x] Import rewriting actually modifies source (functional)
- [x] Advanced CMakeLists.txt parsing (dependencies, targets, variables)
- [x] All 5 import support traits implemented (basic functionality)
- [x] ProjectFactory for creating new C++ projects
- [x] Conan package manager support (basic parsing)
- [x] vcpkg package manager support (vcpkg.json parsing)
- [x] C++20 import statement parsing
- [x] All capability trait stubs implemented (WorkspaceSupport, RefactoringProvider, ModuleReferenceScanner, ImportAnalyzer)
- [ ] LSP integration works with `clangd`
- [ ] Can navigate C++ codebases (find definition, references)

### Phase 3: Full Parity (Target - 40% Complete)
- [x] All capability trait interfaces implemented (8/15 with stubs, 5/15 fully functional)
- [x] Core refactoring operations (extract function, inline variable, extract variable)
- [ ] Advanced refactoring features (template extraction, parameter detection)
- [ ] Full workspace operations (multi-package, CMake add_subdirectory)
- [ ] Full analysis capabilities (dependency graph, complexity analysis)
- [ ] Advanced package manager support (full Conan conanfile.py)
- [ ] Integration tests pass with real C++ projects
- [ ] Handles C++20 modules syntax fully
- [ ] Feature parity with TypeScript/Rust/Python/Swift

## Benefits

- **C++ developers** can use TypeMill for large codebases (game engines, systems software)
- **Build system integration** enables manifest-aware refactoring
- **Package manager support** tracks external dependencies
- **LSP foundation** provides robust navigation via clangd
- **AST fallback** handles cases where LSP is unavailable
- **Market coverage** increases to 80% (from 70%)