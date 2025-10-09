# Language Support Expansion

> [!WARNING]
> **STATUS: OUTDATED - For Historical Reference Only**
>
> The language support status described herein is no longer accurate. Please refer to `API_REFERENCE.md` for the current support matrix.

## Overview

CodeBuddy supports multiple languages via LSP integration and language plugins. This document outlines expansion to cover the top 10 most popular programming languages.

## Current vs Target Support Matrix

### Top 10 Languages (2025 Rankings)

| Rank | Language           | Current LSP | Language Plugin | Notes |
|------|-------------------|-------------|-----------------|-------|
| 1    | **Python**        | ✅          | ✅              | Plugin complete (AST + manifest parsing) |
| 2    | **C++**           | ❌          | ❌              | High-performance systems, game engines |
| 3    | **Java**          | ❌          | ✅              | Plugin complete (AST + manifest parsing) |
| 4    | **JavaScript/TypeScript** | ✅ | ✅              | Fully supported |
| 5    | **C#**            | ❌          | ✅              | Plugin complete (AST + manifest parsing) |
| 6    | **C**             | ❌          | ❌              | Systems programming, embedded devices |
| 7    | **Go**            | ✅          | ✅              | Fully supported |
| 8    | **Rust**          | ✅          | ✅              | Fully supported |
| 9    | **Swift/Kotlin**  | ❌          | ✅/❌           | Swift plugin complete, Kotlin pending |
| 10   | **PHP**           | ❌          | ❌              | Web development, dynamic pages |

### Summary Statistics

- **Fully Supported**: 7/10 (TypeScript/JS, Go, Rust, Python, Java, C#, Swift)
- **Not Supported**: 3/10 (C++, C, Kotlin, PHP)
- **Coverage**: 70% complete (7/10 languages)

## Remaining Work

### C++ Support (`crates/cb-lang-cpp`)
- LSP: `clangd`
- AST parsing via `tree-sitter-cpp`
- Build systems: CMake, Makefile, Bazel
- Package managers: Conan, vcpkg

### C Support (`crates/languages/cb-lang-c`)
- LSP: `clangd` (shared with C++)
- AST parsing via `tree-sitter-c`
- Build systems: Make, CMake

### Kotlin Support (`crates/languages/cb-lang-kotlin`)
- LSP: `kotlin-language-server`
- AST parsing via `tree-sitter-kotlin`
- Build: Gradle

### PHP Support (`crates/languages/cb-lang-php`)
- LSP: `intelephense` or `phpactor`
- AST parsing via `tree-sitter-php`
- Package manager: Composer

## Technical Requirements

### Language Plugin Interface

Each language plugin must implement:

```rust
pub trait LanguageIntelligencePlugin: Send + Sync {
    fn parse_source(&self, source: &str, file_path: &Path) -> Result<ParsedSource>;
    fn extract_imports(&self, source: &ParsedSource) -> Result<Vec<ImportInfo>>;
    fn parse_manifest(&self, content: &str, manifest_type: ManifestType) -> Result<ManifestData>;
    fn update_manifest(&self, manifest: &mut ManifestData, changes: ManifestChanges) -> Result<String>;
}
```

### LSP Server Configuration

Each language requires LSP server configuration in `.codebuddy/config.json`:

| Language   | LSP Server Command | Installation |
|------------|-------------------|--------------|
| Python     | `pylsp`           | `pip install python-lsp-server` |
| Java       | `jdtls`           | Download from Eclipse |
| C++        | `clangd`          | `apt install clangd` or LLVM |
| C#         | `omnisharp` or `csharp-ls` | `dotnet tool install -g csharp-ls` |
| C          | `clangd`          | Same as C++ |
| Swift      | `sourcekit-lsp`   | Included with Swift toolchain |
| Kotlin     | `kotlin-language-server` | Download from GitHub |
| PHP        | `intelephense`    | `npm install -g intelephense` |

## Market Coverage

| Languages | Market Coverage | Cumulative Coverage |
|-----------|-----------------|---------------------|
| **Current** TypeScript/JS, Go, Rust, Python, Java, C#, Swift | **70%** | **70%** |
| Add C++ | +10% | 80% |
| Add C, Kotlin, PHP | +20% | 100% |

## Target User Segments

- **Currently Supported**:
  - Data scientists, ML engineers (Python)
  - Enterprise developers (Java, C#)
  - Web developers (TypeScript/JS, Go)
  - Systems programmers (Rust, Go)
  - Mobile developers (Swift for iOS)
  - Game developers (C#, Rust)
- **Remaining**: C++ systems/game developers, C embedded developers, Kotlin/Android developers, PHP web developers

## Risks & Mitigation

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| LSP server quality varies | High | Thorough testing per language, document limitations |
| Complex build systems (C++, Java) | Medium | Focus on LSP features first, build integration secondary |
| AST parser maintenance burden | Medium | Use battle-tested tree-sitter parsers, minimal custom logic |
| Platform-specific toolchains (Swift) | Low | Document platform requirements, provide Docker images |

### Maintenance Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| 10 plugins to maintain | High | Share common code, automated testing |
| LSP protocol changes | Medium | Version pinning, upgrade testing |
| Breaking changes in parsers | Low | Use stable tree-sitter releases |

## Alternatives Considered

### Option A: LSP-Only (No Language Plugins)

**Pros**: Faster implementation, less maintenance
**Cons**: Missing features for import analysis, manifest updates, advanced refactoring
**Verdict**: ❌ Rejected - plugins provide critical value-add

### Option B: Focus on Top 5 Only

**Pros**: Faster to market, less maintenance
**Cons**: Misses Swift/Kotlin (mobile), PHP (web legacy)
**Verdict**: 🤔 Possible compromise - prioritize based on user demand

### Option C: Community Plugin System

**Pros**: Crowdsourced development, faster expansion
**Cons**: Quality inconsistency, support burden
**Verdict**: 🟡 Future enhancement - start with official plugins, open to community later

## Completed Language Plugins (7/10)

1. ✅ TypeScript/JavaScript - Full support with SWC parser
2. ✅ Rust - Full support with tree-sitter
3. ✅ Go - Full support with tree-sitter
4. ✅ Python - Full support with tree-sitter
5. ✅ Java - Full support with custom Roslyn-based parser
6. ✅ C# - Full support with custom Roslyn-based parser (regex fallback)
7. ✅ Swift - Full support with custom Swift parser

## Implementation Insights

- Dual-mode parsing (AST + regex fallback) provides robustness
- External native parsers (Java, C#, Swift) offer superior accuracy over tree-sitter for complex grammars
- Runtime loading of parsers avoids build-time environment dependencies
- Consistent `LanguagePlugin` trait ensures uniform API across all languages
