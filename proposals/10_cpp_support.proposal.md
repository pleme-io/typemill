# C++ Language Support

**Status**: 🚧 In Progress (25% complete) - Basic plugin structure and import parsing implemented, advanced features pending

**Last Updated**: 2025-10-28 (after merging `feat/cpp-language-support` branch)

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
- ✅ Compiles without errors

**Partially Implemented (Stubs):**
- ⚠️ Import support traits (methods return source unchanged)
- ⚠️ `ImportRenameSupport` - stub only
- ⚠️ `ImportMoveSupport` - stub only
- ⚠️ `ImportMutationSupport` - stub only
- ⚠️ `ImportAdvancedSupport` - stub only

**Not Yet Implemented:**
- ❌ C++20 module syntax (`import my_module;`)
- ❌ Advanced CMakeLists.txt parsing (dependencies, targets)
- ❌ Package manager integration (Conan, vcpkg)
- ❌ ProjectFactory for creating new C++ projects
- ❌ Workspace operations (multi-package)
- ❌ Refactoring operations
- ❌ Analysis capabilities
- ❌ Full LSP integration testing

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
- [x] Implement `mill_plugin!` macro registration
- [ ] Register in plugin system for auto-discovery

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
- [x] Add `ImportRenameSupport` trait (stub implementation)
- [x] Add `ImportMoveSupport` trait (stub implementation)
- [x] Add `ImportMutationSupport` trait (stub implementation)
- [x] Add `ImportAdvancedSupport` trait (stub implementation)
- [ ] **TODO**: Implement actual import rewriting logic (currently returns source unchanged)
- [ ] Add tests for import rewriting operations

### Manifest Parsing
- [x] Implement `analyze_manifest()` for CMakeLists.txt
  - [x] Parse `project()` declarations (basic)
  - [x] Basic test coverage (`test_analyze_cmake_manifest`)
  - [ ] Parse `add_executable()`, `add_library()`
  - [ ] Parse `target_link_libraries()` dependencies
  - [ ] Extract source file lists
  - [ ] Parse CMake variables
- [ ] Implement Makefile parsing (basic)
  - [ ] Parse targets and dependencies
  - [ ] Extract source file lists
- [ ] Implement Bazel BUILD file parsing (optional)
  - [ ] Parse `cc_library`, `cc_binary` rules

### Package Manager Integration
- [ ] Implement Conan support
  - [ ] Parse `conanfile.txt` dependencies
  - [ ] Parse `conanfile.py` Python DSL
  - [ ] Extract package requirements
- [ ] Implement vcpkg support
  - [ ] Parse `vcpkg.json` manifest
  - [ ] Extract dependency list
- [ ] Implement manifest update capabilities

### Advanced Features (Not Yet Implemented)
- [ ] Implement `WorkspaceSupport` trait
- [ ] Implement `ManifestUpdater` trait
- [ ] Implement `RefactoringProvider` trait (extract function, inline variable)
- [ ] Implement `ModuleReferenceScanner` trait
- [ ] Implement `ImportAnalyzer` trait
- [ ] Implement `ProjectFactory` trait for creating new C++ projects
  - [ ] CMake project template
  - [ ] Directory structure (include/, src/, tests/)
  - [ ] Basic main.cpp and CMakeLists.txt generation

### Testing
- [x] Unit tests for AST parsing (classes, namespaces, functions) - `test_parse_symbols`
- [x] Unit tests for `#include` parsing - `test_parse_imports`
- [x] Unit tests for CMakeLists.txt parsing - `test_analyze_cmake_manifest`
- [ ] Unit tests for C++20 `import` parsing
- [ ] Unit tests for import rewriting operations
- [ ] Integration tests with CMake projects
- [ ] Integration tests with Makefile projects
- [ ] Manifest parsing tests (Conan, vcpkg)
- [ ] LSP integration tests with `clangd`
- [ ] Test with complex C++ codebases (Boost, LLVM patterns)

### Documentation
- [x] Update `docs/architecture/overview.md` language support matrix (added C++ column)
- [ ] Add C++ examples to `docs/tools/` documentation
- [ ] Document clangd installation and configuration
- [ ] Document build system integration notes
- [ ] Create C++ plugin development guide
- [ ] Note template/macro limitations (rely on clangd for complex cases)
- [ ] Document stub implementations and what needs completion

## Success Criteria

### Phase 1: Foundation (Current State - 25% Complete)
- [x] `cargo check -p mill-lang-cpp` compiles without errors
- [x] `cargo check --workspace` compiles
- [x] Basic AST parsing tests pass
- [x] Basic import parsing tests pass
- [x] Basic manifest parsing tests pass
- [x] Plugin structure follows TypeMill conventions
- [x] `mill_plugin!` macro registration works

### Phase 2: Core Features (Next Steps - 0% Complete)
- [ ] Import rewriting actually modifies source (not stubs)
- [ ] Advanced CMakeLists.txt parsing (dependencies, targets)
- [ ] All 5 import support traits fully implemented
- [ ] LSP integration works with `clangd`
- [ ] Can navigate C++ codebases (find definition, references)

### Phase 3: Full Parity (Target - 0% Complete)
- [ ] All 15 common capabilities implemented
- [ ] ProjectFactory for creating new C++ projects
- [ ] Package manager support (Conan or vcpkg)
- [ ] Integration tests pass with real C++ projects
- [ ] Handles C++20 modules syntax
- [ ] Feature parity with TypeScript/Rust/Python/Swift

## Benefits

- **C++ developers** can use TypeMill for large codebases (game engines, systems software)
- **Build system integration** enables manifest-aware refactoring
- **Package manager support** tracks external dependencies
- **LSP foundation** provides robust navigation via clangd
- **AST fallback** handles cases where LSP is unavailable
- **Market coverage** increases to 80% (from 70%)
