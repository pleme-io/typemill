# C++ Language Support

## Problem

C++ developers (Rank 2 language, AAA games, embedded systems) cannot use TypeMill. No LSP integration or language plugin exists for C++ projects.

## Solution

Implement full C++ support with `clangd` LSP integration and `mill-lang-cpp` plugin. Use tree-sitter-cpp for AST parsing and support major build systems (CMake, Makefile) and package managers (Conan, vcpkg).

### Technical Approach

- **LSP Server**: `clangd` (part of LLVM)
- **AST Parser**: `tree-sitter-cpp`
- **Build Systems**: CMakeLists.txt, Makefile, Bazel
- **Package Managers**: Conan (`conanfile.txt`, `conanfile.py`), vcpkg (`vcpkg.json`)
- **Import Syntax**: `#include`, `import` (C++20 modules)

## Checklists

### LSP Integration
- [ ] Add `clangd` to default LSP server configurations in `mill setup`
- [ ] Document installation instructions (`apt install clangd` / LLVM downloads)
- [ ] Configure file extensions (`.cpp`, `.cc`, `.cxx`, `.h`, `.hpp`)
- [ ] Test initialization and basic navigation (find definition, references)
- [ ] Verify diagnostics and code actions work
- [ ] Test with real C++ projects (multiple build systems)

### Language Plugin (`crates/mill-lang-cpp`)
- [x] Create crate structure following `mill-lang-*` pattern
- [x] Add to `languages.toml` registry:
  ```toml
  [languages.cpp]
  path = "crates/mill-lang-cpp"
  plugin_struct = "CppPlugin"
  category = "full"
  default = false
  ```
- [x] Run `cargo xtask sync-languages` to generate feature flags
- [x] Implement `LanguagePlugin` trait with `define_language_plugin!` macro
- [x] Set metadata (name: "C++", extensions: `["cpp", "cc", "cxx", "h", "hpp"]`)
- [x] Configure LSP: `LspConfig::new("clangd", &["clangd"])`

### AST Parsing
- [x] Integrate `tree-sitter-cpp` dependency
- [ ] Implement `parse()` method for symbol extraction
- [ ] Parse classes, functions, methods, namespaces
- [ ] Handle C++ templates and macros (fallback to clangd LSP)
- [ ] Extract symbol hierarchy (class members, nested namespaces)

### Import Support (5 Traits)
- [x] Implement `ImportParser` trait
  - [x] Parse `#include <system>` headers
  - [x] Parse `#include "local"` headers
  - [ ] Parse C++20 `import` statements
- [ ] Implement `ImportRenameSupport` trait
- [ ] Implement `ImportMoveSupport` trait
- [ ] Implement `ImportMutationSupport` trait
- [ ] Implement `ImportAdvancedSupport` trait

### Manifest Parsing
- [ ] Implement `analyze_manifest()` for CMakeLists.txt
  - [ ] Parse `project()` declarations
  - [ ] Parse `add_executable()`, `add_library()`
  - [ ] Parse `target_link_libraries()` dependencies
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

### Testing
- [ ] Unit tests for AST parsing (classes, templates, namespaces)
- [x] Unit tests for `#include` parsing (basic test passing)
- [ ] Unit tests for C++20 `import` parsing
- [ ] Integration tests with CMake projects
- [ ] Integration tests with Makefile projects
- [ ] Manifest parsing tests (CMake, Conan, vcpkg)
- [ ] LSP integration tests with `clangd`
- [ ] Test with complex C++ codebases (Boost, LLVM patterns)

### Documentation
- [ ] Update `docs/architecture/overview.md` language support matrix
- [ ] Add C++ examples to `docs/tools/` documentation
- [ ] Document clangd installation and configuration
- [ ] Document build system integration notes
- [ ] Create C++ plugin development guide
- [ ] Note template/macro limitations (rely on clangd for complex cases)

## Success Criteria

- [x] `cargo check -p mill-lang-cpp` compiles without errors
- [x] `cargo check --workspace --features lang-cpp` compiles
- [ ] All unit tests pass for AST and manifest parsing
- [ ] Integration tests pass with real C++ projects
- [x] Plugin loads via `define_language_plugin!` macro
- [ ] LSP integration works with `clangd`
- [ ] Can navigate C++ codebases (find definition, references)
- [ ] Can parse CMake and Conan manifests
- [ ] Import rewriting works for `#include` statements
- [ ] Handles C++20 modules syntax

## Benefits

- **C++ developers** can use TypeMill for large codebases (game engines, systems software)
- **Build system integration** enables manifest-aware refactoring
- **Package manager support** tracks external dependencies
- **LSP foundation** provides robust navigation via clangd
- **AST fallback** handles cases where LSP is unavailable
- **Market coverage** increases to 80% (from 70%)
