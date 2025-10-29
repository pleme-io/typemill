# C Language Support

## Problem

C developers (Rank 6 language, embedded systems, IoT, firmware) cannot use TypeMill. No LSP integration or language plugin exists for pure C projects.

## Solution

Implement full C support with `clangd` LSP integration (shared with C++) and `mill-lang-c` plugin. Use tree-sitter-c for AST parsing and support Makefile and CMake build systems.

### Technical Approach

- **LSP Server**: `clangd` (shared with C++, already configured)
- **AST Parser**: `tree-sitter-c`
- **Build Systems**: Makefile, CMakeLists.txt
- **Import Syntax**: `#include` (no C++ features)

## Checklists

### LSP Integration
- [ ] Add C file extensions (`.c`, `.h`) to `clangd` configuration
- [ ] Configure C-specific file type detection
- [ ] Test with pure C projects (no C++ constructs)
- [ ] Verify standard library navigation works (stdio.h, stdlib.h)
- [ ] Test with embedded C projects (ARM, AVR patterns)
- [ ] Verify diagnostics show C-specific errors (not C++ warnings)

### Language Plugin (`crates/mill-lang-c`)
- [ ] Create crate structure following `mill-lang-*` pattern
- [ ] Add to `languages.toml` registry:
  ```toml
  [languages.c]
  path = "crates/mill-lang-c"
  plugin_struct = "CPlugin"
  category = "full"
  default = false
  ```
- [ ] Run `cargo xtask sync-languages` to generate feature flags
- [ ] Implement `LanguagePlugin` trait with `define_language_plugin!` macro
- [ ] Set metadata (name: "C", extensions: `["c", "h"]`)
- [ ] Configure LSP: `LspConfig::new("clangd", &["clangd"])`

### AST Parsing
- [ ] Integrate `tree-sitter-c` dependency
- [ ] Implement `parse()` method for symbol extraction
- [ ] Parse functions, structs, enums, typedefs
- [ ] Handle C preprocessor macros
- [ ] Extract symbol hierarchy (struct members, nested declarations)
- [ ] Validate NO C++ constructs in parsed output

### Import Support (5 Traits)
- [ ] Implement `ImportParser` trait
  - [ ] Parse `#include <system>` headers
  - [ ] Parse `#include "local"` headers
  - [ ] NO C++ `import` statements
- [ ] Implement `ImportRenameSupport` trait
- [ ] Implement `ImportMoveSupport` trait
- [ ] Implement `ImportMutationSupport` trait
- [ ] Implement `ImportAdvancedSupport` trait

### Manifest Parsing
- [ ] Implement `analyze_manifest()` for Makefile
  - [ ] Parse targets and dependencies
  - [ ] Extract source file lists
  - [ ] Parse compiler flags (CFLAGS)
- [ ] Implement CMakeLists.txt parsing (C projects)
  - [ ] Parse `project()` with C language specification
  - [ ] Parse `add_executable()`, `add_library()`
  - [ ] Parse `target_link_libraries()` dependencies
  - [ ] Distinguish C vs C++ CMake projects

### Testing
- [ ] Unit tests for AST parsing (functions, structs, no C++)
- [ ] Unit tests for `#include` parsing
- [ ] Integration tests with Makefile projects
- [ ] Integration tests with CMake C projects
- [ ] Manifest parsing tests (Makefile, CMake)
- [ ] LSP integration tests with `clangd`
- [ ] Test with Linux kernel style code
- [ ] Test with embedded C projects (bare metal patterns)
- [ ] Verify NO C++ features leak into C parser

### Documentation
- [ ] Update `docs/architecture/overview.md` language support matrix
- [ ] Add C examples to `docs/tools/` documentation
- [ ] Document C vs C++ differences in plugin behavior
- [ ] Note shared `clangd` configuration with C++
- [ ] Create C plugin development guide
- [ ] Document embedded C considerations

## Success Criteria

- [ ] `cargo check -p mill-lang-c` compiles without errors
- [ ] `cargo check --workspace --features lang-c` compiles
- [ ] All unit tests pass for AST and manifest parsing
- [ ] Integration tests pass with pure C projects
- [ ] Plugin loads via `define_language_plugin!` macro
- [ ] LSP integration works with `clangd`
- [ ] Can navigate C codebases (find definition, references)
- [ ] Can parse Makefile and CMake manifests
- [ ] Import rewriting works for `#include` statements
- [ ] Correctly rejects C++ constructs in pure C files

## Benefits

- **C developers** can use TypeMill for embedded and systems projects
- **Embedded systems** developers get code intelligence for IoT/firmware
- **Build system integration** enables Makefile-aware refactoring
- **Shared LSP** leverages existing clangd infrastructure
- **Simpler grammar** than C++ reduces parser complexity
- **Market coverage** moves toward 90%