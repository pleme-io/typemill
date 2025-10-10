# Language Support Expansion

> [!NOTE]
> **STATUS: 70% COMPLETE - 3 Languages Remaining**
>
> This proposal tracks completion of top 10 language support. Current status: **7/10 languages fully supported**. See checklist below for remaining work.

## Overview

CodeBuddy supports multiple languages via LSP integration and language plugins. This document tracks expansion to cover the top 10 most popular programming languages, with **7 already complete** and **3 remaining**.

## Current Support Status

### ✅ Completed Languages (7/10)

| Rank | Language           | LSP Support | Language Plugin | Status |
|------|-------------------|-------------|-----------------|--------|
| 1    | **Python**        | ✅          | ✅ `cb-lang-python` | ✅ **COMPLETE** - AST + manifest parsing |
| 3    | **Java**          | ✅          | ✅ `cb-lang-java` | ✅ **COMPLETE** - AST + manifest parsing |
| 4    | **JavaScript/TypeScript** | ✅ | ✅ `cb-lang-typescript` | ✅ **COMPLETE** - SWC parser |
| 5    | **C#**            | ✅          | ✅ `cb-lang-csharp` | ✅ **COMPLETE** - AST + manifest parsing (some refactoring bugs exist) |
| 7    | **Go**            | ✅          | ✅ `cb-lang-go` | ✅ **COMPLETE** - AST + manifest parsing |
| 8    | **Rust**          | ✅          | ✅ `cb-lang-rust` | ✅ **COMPLETE** - AST + manifest + workspace support |
| 9    | **Swift**         | ✅          | ✅ `cb-lang-swift` | ✅ **COMPLETE** - AST + manifest parsing |

**Completion Rate: 70% (7/10 languages)**

### 🚧 Remaining Languages (3/10)

| Rank | Language   | LSP Support | Language Plugin | Blockers |
|------|-----------|-------------|-----------------|----------|
| 2    | **C++**   | ❌          | ❌              | LSP config, tree-sitter parser, build system integration |
| 6    | **C**     | ❌          | ❌              | LSP config, tree-sitter parser |
| 10   | **PHP**   | ❌          | ❌              | LSP config, tree-sitter parser, Composer integration |

**Note:** Kotlin (Rank 9 alongside Swift) was deprioritized in favor of other languages.

## Remaining Work Checklist

### [ ] C++ Support (`crates/cb-lang-cpp`)

**Goal:** Enable full LSP and plugin support for C++ projects

- [ ] **LSP Integration**
  - [ ] Add `clangd` to default LSP server configurations
  - [ ] Document installation instructions (`apt install clangd` / LLVM)
  - [ ] Test initialization and basic navigation (find definition, references)
  - [ ] Verify diagnostics and code actions work

- [ ] **Language Plugin (`cb-lang-cpp`)**
  - [ ] Create crate structure (`crates/cb-lang-cpp/`)
  - [ ] Implement `LanguagePlugin` trait
  - [ ] Integrate `tree-sitter-cpp` for AST parsing
  - [ ] Implement `ImportSupport` trait (C++ includes: `#include`, `import`)
  - [ ] Parse build manifests:
    - [ ] CMakeLists.txt parsing
    - [ ] Makefile parsing (basic)
    - [ ] Bazel BUILD files (optional)
  - [ ] Package manager support:
    - [ ] Conan integration (`conanfile.txt`, `conanfile.py`)
    - [ ] vcpkg integration (`vcpkg.json`)

- [ ] **Testing**
  - [ ] Unit tests for AST parsing
  - [ ] Integration tests with real C++ projects
  - [ ] Manifest parsing tests (CMake, Conan, vcpkg)
  - [ ] LSP integration tests with `clangd`

- [ ] **Documentation**
  - [ ] Update API_REFERENCE.md language support matrix
  - [ ] Add C++ examples to tool documentation
  - [ ] Create C++ plugin development guide

### [ ] C Support (`crates/cb-lang-c`)

**Goal:** Enable full LSP and plugin support for C projects

- [ ] **LSP Integration**
  - [ ] Add `clangd` to LSP configurations (shared with C++)
  - [ ] Configure C-specific file extensions (`.c`, `.h`)
  - [ ] Test with pure C projects (no C++ features)
  - [ ] Verify standard library navigation works

- [ ] **Language Plugin (`cb-lang-c`)**
  - [ ] Create crate structure (`crates/cb-lang-c/`)
  - [ ] Implement `LanguagePlugin` trait
  - [ ] Integrate `tree-sitter-c` for AST parsing
  - [ ] Implement `ImportSupport` trait (C includes: `#include`)
  - [ ] Parse build manifests:
    - [ ] Makefile parsing
    - [ ] CMakeLists.txt parsing (C projects)

- [ ] **Testing**
  - [ ] Unit tests for C AST parsing (no C++ constructs)
  - [ ] Integration tests with C projects (Linux kernel style, embedded)
  - [ ] Manifest parsing tests (Makefile, CMake)
  - [ ] LSP integration tests

- [ ] **Documentation**
  - [ ] Update API_REFERENCE.md language support matrix
  - [ ] Add C examples to tool documentation
  - [ ] Note C vs C++ differences in plugin guides

### [ ] PHP Support (`crates/cb-lang-php`)

**Goal:** Enable full LSP and plugin support for PHP projects

- [ ] **LSP Integration**
  - [ ] Add `intelephense` to LSP configurations (recommended)
  - [ ] Alternative: `phpactor` configuration
  - [ ] Document installation (`npm install -g intelephense`)
  - [ ] Test with Laravel/Symfony projects
  - [ ] Verify namespace navigation and autocompletion

- [ ] **Language Plugin (`cb-lang-php`)**
  - [ ] Create crate structure (`crates/cb-lang-php/`)
  - [ ] Implement `LanguagePlugin` trait
  - [ ] Integrate `tree-sitter-php` for AST parsing
  - [ ] Implement `ImportSupport` trait (PHP: `use`, `require`, `include`)
  - [ ] Parse `composer.json` manifests:
    - [ ] Parse dependencies (`require`, `require-dev`)
    - [ ] Parse autoload configuration (PSR-4, classmap)
  - [ ] Update manifest support (`composer.json` modifications)

- [ ] **Testing**
  - [ ] Unit tests for PHP AST parsing
  - [ ] Integration tests with Composer projects
  - [ ] Manifest parsing tests (`composer.json`)
  - [ ] LSP integration tests with `intelephense`
  - [ ] Test Laravel/Symfony framework navigation

- [ ] **Documentation**
  - [ ] Update API_REFERENCE.md language support matrix
  - [ ] Add PHP examples to tool documentation
  - [ ] Document Composer integration
  - [ ] Note framework-specific considerations (Laravel, Symfony)

## Technical Reference

### Language Plugin Interface

All language plugins implement the `LanguagePlugin` trait with capability-based design. See **[docs/development/languages/README.md](docs/development/languages/README.md)** for complete plugin development guide.

**Core Trait:** 6 required methods + 3 default implementations
- `metadata()` - Language metadata (name, extensions, manifest filename)
- `parse()` - AST parsing and symbol extraction
- `analyze_manifest()` - Manifest file analysis
- `capabilities()` - Feature flags for optional capabilities
- `import_support()` - Optional ImportSupport trait object (6 sync methods)
- `workspace_support()` - Optional WorkspaceSupport trait object (5 sync methods)

### LSP Server Installation

| Language   | LSP Server | Installation Command |
|------------|-----------|---------------------|
| C++        | `clangd`  | `apt install clangd` or download LLVM |
| C          | `clangd`  | Same as C++ |
| PHP        | `intelephense` | `npm install -g intelephense` |

**Configuration:** Add to `.codebuddy/config.json` after installation. Run `codebuddy setup` for auto-detection.

## Success Metrics

### Coverage Goals

| Milestone | Languages | Market Coverage | Status |
|-----------|-----------|-----------------|--------|
| ✅ Current | TypeScript/JS, Go, Rust, Python, Java, C#, Swift | 70% | **COMPLETE** |
| 🎯 Add C++ | + C++ | 80% | 🚧 In Progress |
| 🎯 Add C & PHP | + C, PHP | 90%+ | 📋 Planned |

### Target User Segments

**✅ Currently Supported:**
- Data scientists, ML engineers (Python)
- Enterprise developers (Java, C#)
- Web developers (TypeScript/JS, Go)
- Systems programmers (Rust, Go)
- Mobile developers (Swift for iOS)
- Game developers (C#, Rust)

**🎯 Remaining Segments:**
- C++ systems/game developers (AAA games, embedded systems)
- C embedded developers (IoT, firmware)
- PHP web developers (WordPress, Laravel, legacy web apps)

## Implementation Notes

### Lessons Learned from 7 Completed Plugins

1. **Dual-mode parsing** (AST + regex fallback) provides robustness for complex edge cases
2. **External native parsers** (Java, C#, Swift) offer superior accuracy over tree-sitter for complex grammars
3. **Runtime loading** of parsers avoids build-time environment dependencies
4. **Consistent LanguagePlugin trait** ensures uniform API across all languages
5. **Capability-based design** allows incremental feature additions (ImportSupport, WorkspaceSupport)

### Risks & Mitigation

| Risk | Impact | Mitigation Strategy | Status |
|------|--------|-------------------|--------|
| LSP server quality varies | Medium | Thorough testing per language, document limitations | Ongoing |
| Complex build systems (C++/CMake) | Medium | Focus on LSP features first, build integration secondary | Planned |
| AST parser maintenance burden | Low | Use battle-tested tree-sitter parsers, minimal custom logic | ✅ Validated |
| Parsing C++ templates/macros | High | Use clangd LSP as primary, AST as fallback | Planned |

## Next Steps

1. **Start with C++** - Highest user demand (game developers, systems programmers)
2. **Then C** - Shares `clangd` LSP with C++, simpler grammar
3. **Finally PHP** - Web legacy support, separate LSP server

**Estimated Timeline:** 2-3 weeks per language (LSP + Plugin + Testing + Docs)
