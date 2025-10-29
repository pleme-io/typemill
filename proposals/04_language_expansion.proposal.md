# Language Support Expansion

> [!WARNING]
> **TEMPORARILY PAUSED FOR UNIFIED API REFACTORING**
>
> Language support temporarily reduced to **TypeScript + Rust only** (as of 2025-10-10) to accelerate unified API development.
> Multi-language support (Python, Go, Java, Swift, C#) preserved in git tag `pre-language-reduction` and will be restored after unified API is stable.
>
> See [10_PROPOSAL_LANGUAGE_REDUCTION.md](10_PROPOSAL_LANGUAGE_REDUCTION.md) for strategy and [30_PROPOSAL_UNIFIED_REFACTORING_API.md](30_PROPOSAL_UNIFIED_REFACTORING_API.md) for unified API design.

> [!NOTE]
> **ORIGINAL STATUS: 70% COMPLETE - 3 Languages Remaining**
>
> This proposal tracks completion of top 10 language support. Pre-reduction status: **7/10 languages fully supported**. See checklist below for remaining work.

## Overview

TypeMill supports multiple languages via LSP integration and language plugins. This document tracks expansion to cover the top 10 most popular programming languages, with **7 already complete** and **3 remaining**.

## Current Support Status

### ✅ Completed Languages (7/10)

| Rank | Language           | LSP Support | Language Plugin | Status |
|------|-------------------|-------------|-----------------|--------|
| 1    | **Python**        | ✅          | ✅ `mill-lang-python` | ✅ **COMPLETE** - AST + manifest parsing |
| 3    | **Java**          | ✅          | ⚠️ Legacy `pre-language-reduction` | ✅ **ARCHIVED** - AST + manifest parsing |
| 4    | **JavaScript/TypeScript** | ✅ | ✅ `mill-lang-typescript` | ✅ **COMPLETE** - SWC parser |
| 5    | **C#**            | ✅          | ⚠️ Legacy `pre-language-reduction` | ✅ **ARCHIVED** - AST + manifest parsing (some refactoring bugs exist) |
| 7    | **Go**            | ✅          | ⚠️ Legacy `pre-language-reduction` | ✅ **ARCHIVED** - AST + manifest parsing |
| 8    | **Rust**          | ✅          | ✅ `mill-lang-rust` | ✅ **COMPLETE** - AST + manifest + workspace support |
| 9    | **Swift**         | ✅          | ⚠️ Legacy `pre-language-reduction` | ✅ **ARCHIVED** - AST + manifest parsing |

> **Note on Legacy Plugins**: Java, C#, Go, and Swift plugins are preserved in git tag `pre-language-reduction` and can be restored following the migration guide in `.debug/language-plugin-migration/`. Current active support focuses on TypeScript, Rust, and Python (100% feature parity).

**Completion Rate: 70% (7/10 languages)**

### 🚧 Remaining Languages (3/10)

| Rank | Language   | LSP Support | Language Plugin | Blockers |
|------|-----------|-------------|-----------------|----------|
| 2    | **C++**   | ❌          | ❌              | LSP config, tree-sitter parser, build system integration |
| 6    | **C**     | ❌          | ❌              | LSP config, tree-sitter parser |
| 10   | **PHP**   | ❌          | ❌              | LSP config, tree-sitter parser, Composer integration |

**Note:** Kotlin (Rank 9 alongside Swift) was deprioritized in favor of other languages.

## Individual Language Proposals

**New Implementations:**
- **[10_cpp_support.proposal.md](10_cpp_support.proposal.md)** - C++ language support (foundational skeleton complete)
- **[11_c_support.proposal.md](11_c_support.proposal.md)** - C language support
- **[12_php_support.proposal.md](12_php_support.proposal.md)** - PHP language support

**Language Restorations (from `pre-language-reduction` tag):**
- **[05_restore_swift.proposal.md](05_restore_swift.proposal.md)** - Swift language restoration
- **[06_restore_csharp.proposal.md](06_restore_csharp.proposal.md)** - C# language restoration
- **[09_restore_go.proposal.md](09_restore_go.proposal.md)** - Go language restoration
- **[13_restore_java.proposal.md](13_restore_java.proposal.md)** - Java language restoration

**Status:**
- C++ (10): 🚧 Foundational skeleton complete (~10% done)
- C (11): 📋 Not started
- PHP (12): 📋 Not started
- Swift (05): 📋 Awaiting restoration
- C# (06): 📋 Awaiting restoration
- Go (09): 📋 Awaiting restoration
- Java (13): 📋 Awaiting restoration

## Technical Reference

### Language Plugin Interface

All language plugins implement the `LanguagePlugin` trait with capability-based design. See **[docs/development/languages/README.md](readme.md)** for complete plugin development guide.

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

**Configuration:** Add to `.typemill/config.json` after installation. Run `mill setup` for auto-detection.

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

## Implementation Scope

Each language requires LSP integration, plugin development, testing, and documentation. See individual language proposals for detailed checklists and requirements.