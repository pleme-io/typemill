# C++ Language Support

**Status**: 🚧 In Progress (40% complete) - Basic plugin structure, import parsing, and ProjectFactory implemented, advanced features pending

**Last Updated**: 2025-10-28 (after merging `feat/cpp-language-support-features` branch)

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
- ✅ Basic CMakeLists.txt parsing (project name only)
- ✅ Advanced CMakeLists.txt parsing (add_executable, variables, dependencies)
- ✅ Basic Conan manifest parsing (conanfile.txt)
- ✅ ProjectFactory for creating new C++ projects (with CMake)
- ✅ Import rewriting for rename operations
- ✅ Import rewriting for move operations
- ✅ Import mutation support (add/remove imports)
- ✅ All 16 unit tests passing
- ✅ Compiles without errors

**Partially Implemented:**
- ⚠️ Import support traits (basic implementation, not all edge cases)
- ⚠️ CMake parsing (basic functionality, missing some advanced features)
- ⚠️ Conan support (basic parsing only, no vcpkg yet)

**Not Yet Implemented:**
- ❌ C++20 module syntax (`import my_module;`)
- ❌ Complete Conan conanfile.py parsing
- ❌ vcpkg package manager integration
- ❌ Workspace operations (multi-package)
- ❌ Refactoring operations (extract function, inline variable)
- ❌ Analysis capabilities (dependency graph, complexity)
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
  - [x] Basic test coverage (`test_parse_imports`)
  - [ ] Parse C++20 `import` statements
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
- [ ] Implement vcpkg support
  - [ ] Parse `vcpkg.json` manifest
  - [ ] Extract dependency list
- [ ] Implement manifest update capabilities

### Advanced Features
- [ ] Implement `WorkspaceSupport` trait
- [ ] Implement `ManifestUpdater` trait
- [ ] Implement `RefactoringProvider` trait (extract function, inline variable)
- [ ] Implement `ModuleReferenceScanner` trait
- [ ] Implement `ImportAnalyzer` trait
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
- [x] All 16 unit tests passing
- [ ] Unit tests for C++20 `import` parsing
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

### Phase 2: Core Features (Current State - 60% Complete)
- [x] Import rewriting actually modifies source (functional)
- [x] Advanced CMakeLists.txt parsing (dependencies, targets, variables)
- [x] All 5 import support traits implemented (basic functionality)
- [x] ProjectFactory for creating new C++ projects
- [x] Conan package manager support (basic parsing)
- [ ] LSP integration works with `clangd`
- [ ] Can navigate C++ codebases (find definition, references)

### Phase 3: Full Parity (Target - 0% Complete)
- [ ] All 15 common capabilities implemented (currently ~8/15)
- [ ] Advanced package manager support (vcpkg, full Conan)
- [ ] Integration tests pass with real C++ projects
- [ ] Handles C++20 modules syntax
- [ ] Feature parity with TypeScript/Rust/Python/Swift
- [ ] Refactoring operations (extract function, inline variable)
- [ ] Workspace operations (multi-package)

## Benefits

- **C++ developers** can use TypeMill for large codebases (game engines, systems software)
- **Build system integration** enables manifest-aware refactoring
- **Package manager support** tracks external dependencies
- **LSP foundation** provides robust navigation via clangd
- **AST fallback** handles cases where LSP is unavailable
- **Market coverage** increases to 80% (from 70%)