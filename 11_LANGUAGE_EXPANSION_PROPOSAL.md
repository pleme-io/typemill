# Proposal: Language Support Expansion

> [!WARNING]
> **STATUS: OUTDATED**
> This proposal is kept for historical context. The language support status described herein is no longer accurate. Please refer to `API_REFERENCE.md` for the current support matrix.

**Status**: Proposal - For Discussion
**Date**: 2025-10-05

## 1. Overview

CodeBuddy currently supports 4 languages via LSP integration and language plugins. This proposal outlines a strategic expansion plan to cover the top 10 most popular programming languages in 2025.

## 2. Current vs Target Support Matrix

### Top 10 Languages (2025 Rankings)

| Rank | Language           | Current LSP | Language Plugin | Priority | Notes |
|------|-------------------|-------------|-----------------|----------|-------|
| 1    | **Python**        | ✅          | ✅              | ✅ Done  | Plugin complete (AST + manifest parsing) |
| 2    | **C++**           | ❌          | ❌              | 🟡 Medium | High-performance systems, game engines |
| 3    | **Java**          | ❌          | ✅              | ✅ Done  | Plugin complete (AST + manifest parsing) |
| 4    | **JavaScript/TypeScript** | ✅ | ✅              | ✅ Done  | Fully supported |
| 5    | **C#**            | ❌          | ✅              | ✅ Done  | Plugin complete (AST + manifest parsing) |
| 6    | **C**             | ❌          | ❌              | 🟢 Low   | Systems programming, embedded devices |
| 7    | **Go**            | ✅          | ✅              | ✅ Done  | Fully supported |
| 8    | **Rust**          | ✅          | ✅              | ✅ Done  | Fully supported |
| 9    | **Swift/Kotlin**  | ❌          | ✅/❌           | 🟡 Medium | Swift plugin complete, Kotlin pending |
| 10   | **PHP**           | ❌          | ❌              | 🟢 Low   | Web development, dynamic pages |

### Summary Statistics

- **Fully Supported**: 7/10 (TypeScript/JS, Go, Rust, Python, Java, C#, Swift)
- **Partial Support**: 0/10
- **Not Supported**: 3/10 (C++, C, Kotlin, PHP)
- **Coverage**: 70% complete (7/10 languages)

## 3. Implementation Checklist

### Phase 1: Complete Existing Support ✅ **COMPLETED**

- [x] **Python Language Plugin** (`crates/cb-lang-python`) ✅ **COMPLETED**
  - AST parsing for import analysis
  - Manifest parsing (`requirements.txt`, `pyproject.toml`, `setup.py`)
  - Extract function/variable refactoring support
  - **Status**: Plugin implemented and ready

### Phase 2: Enterprise Languages (🟡 Medium Priority)

- [x] **Java Support** (`crates/cb-lang-java`) ✅ **COMPLETED**
  - AST parsing via custom Java parser
  - Manifest: `pom.xml`, `build.gradle`
  - Extract `<dependency>` and project references
  - LSP: `jdtls` (Eclipse JDT Language Server) - user-configurable
  - **Status**: Plugin implemented and ready

- [ ] **C++ Support** (`crates/cb-lang-cpp`)
  - LSP: `clangd`
  - AST parsing via `tree-sitter-cpp`
  - Build systems: CMake, Makefile, Bazel
  - Package managers: Conan, vcpkg
  - Estimated effort: 4-5 weeks (complex build systems)

### Phase 3: Additional Languages (🟢 Low Priority)

- [x] **C# Support** (`crates/cb-lang-csharp`) ✅ **COMPLETED**
  - AST parsing via Roslyn-based parser (with regex fallback)
  - Manifest parsing for `.csproj` files
  - Extract `PackageReference` and `ProjectReference` dependencies
  - LSP: `omnisharp-roslyn` or `csharp-ls` (user-configurable)
  - Build: MSBuild, .NET CLI
  - Package manager: NuGet
  - **Status**: Plugin implemented, tested, and ready for merge

- [ ] **C Support** (`crates/languages/cb-lang-c`)
  - LSP: `clangd` (shared with C++)
  - AST parsing via `tree-sitter-c`
  - Build systems: Make, CMake
  - Estimated effort: 2-3 weeks

- [x] **Swift Support** (`crates/cb-lang-swift`) ✅ **COMPLETED**
  - AST parsing via custom Swift parser
  - Manifest parsing for `Package.swift`
  - Extract dependencies from Swift Package Manager
  - LSP: `sourcekit-lsp` - user-configurable
  - **Status**: Plugin implemented and ready

- [ ] **Kotlin Support** (`crates/languages/cb-lang-kotlin`)
  - LSP: `kotlin-language-server`
  - AST parsing via `tree-sitter-kotlin`
  - Build: Gradle
  - Estimated effort: 3-4 weeks

- [ ] **PHP Support** (`crates/languages/cb-lang-php`)
  - LSP: `intelephense` or `phpactor`
  - AST parsing via `tree-sitter-php`
  - Package manager: Composer
  - Estimated effort: 2-3 weeks

## 4. Technical Requirements

### Language Plugin Interface

Each language plugin must implement (from `crates/languages/README.md`):

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

## 5. Effort Estimation

### Total Development Time

- **Phase 1 (Python)**: ✅ Complete
- **Phase 2 (Java + C++)**: ✅ Java complete, C++ pending (4-5 weeks)
- **Phase 3 (C, Kotlin, PHP)**: ~9-12 weeks (C# ✅ Swift ✅ complete)

**Total Remaining**: ~13-17 weeks (~3-4 months) for remaining 4 languages
**Completed**: Python, Java, C#, Swift (estimated 11-14 weeks total effort)

### Resource Requirements

- 1 developer full-time: ~7 months
- 2 developers full-time: ~3.5 months
- With community contributions: could be faster

## 6. Priority Justification

### ✅ Completed: Python Plugin

- ✅ Complete AST parsing and manifest support
- Rank #1 language globally
- AI/ML/data science dominance
- **Impact**: Full refactoring capabilities for Python users

### 🟡 Medium Priority: C++ (Java ✅ complete)

- **Java**: ✅ Complete plugin with AST and manifest parsing
  - #3 globally, massive enterprise adoption
  - Android development, enterprise backend systems
  - Spring Framework ecosystem

- **C++**: #2 globally, performance-critical domains - **PENDING**
  - Game engines (Unreal Engine)
  - Financial trading platforms
  - Embedded systems, robotics
  - **Status**: Next priority for implementation

### Remaining Languages: C, Kotlin, PHP (C# ✅ Swift ✅ complete)

- **C#**: ✅ Complete - Game dev (Unity), enterprise software
- **Swift**: ✅ Complete - iOS development, Apple ecosystem
- **C**: Pending - Systems programming, embedded devices
- **Kotlin**: Pending - Android development, JVM ecosystem
- **PHP**: Pending - Web development, legacy systems
- Can be added incrementally based on user demand

## 7. Business Impact

### Market Coverage

| Phase | Languages Added | Market Coverage | Cumulative Coverage |
|-------|----------------|-----------------|---------------------|
| **Current** ✅ | TypeScript/JS, Go, Rust, Python, Java, C#, Swift | **70%** | **70%** |
| Remaining | C++ | +10% | 80% |
| Future | C, Kotlin, PHP | +20% | 100% |

### Target User Segments

- **✅ Currently Supported**:
  - Data scientists, ML engineers (Python)
  - Enterprise developers (Java, C#)
  - Web developers (TypeScript/JS, Go)
  - Systems programmers (Rust, Go)
  - Mobile developers (Swift for iOS)
  - Game developers (C#, Rust)
- **Remaining**: C++ systems/game developers, C embedded developers, Kotlin/Android developers, PHP web developers

## 8. Risks & Mitigation

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

## 9. Success Metrics

- **Coverage**: ✅ 70% complete (7/10 top languages supported)
  - Target: 100% (add C++, C, Kotlin, PHP)
- **Quality**: ✅ All language plugins implement consistent LanguagePlugin trait
- **Performance**: ✅ AST parsing <100ms for typical files across all plugins
- **Adoption**: User growth observed in Python, Java, C#, Swift segments

## 10. Alternatives Considered

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

## 11. Recommendations

1. ✅ **COMPLETED**: Python, Java, C#, Swift plugins - 70% market coverage achieved
2. **Q4 2024**: Implement C++ support - highest impact remaining language
3. **Q1 2025**: Assess demand for C, Kotlin, PHP via user surveys
4. **Q2 2025**: Implement top-requested remaining languages based on demand
5. **Q3 2025**: Evaluate community plugin system for long-tail languages (Ruby, Scala, Elixir, etc.)

## 12. Open Questions

- Should we support multiple versions of language servers (e.g., Java 8 vs 17)?
  - Current approach: User-configurable via `.codebuddy/config.json`
- Should we bundle LSP servers or require user installation?
  - Current approach: User installation required, documented in setup guide
- Should we provide pre-configured Docker images per language?
  - Current approach: Docker deployment available, see `docs/deployment/DOCKER_DEPLOYMENT.md`
- What's the minimum LSP feature set required for "supported" status?
  - Current standard: AST parsing + manifest parsing + symbol extraction

---

## 13. Achievements Summary

**Completed Language Plugins (7/10)**:
1. ✅ TypeScript/JavaScript - Full support with SWC parser
2. ✅ Rust - Full support with tree-sitter
3. ✅ Go - Full support with tree-sitter
4. ✅ Python - Full support with tree-sitter
5. ✅ Java - Full support with custom Roslyn-based parser
6. ✅ C# - Full support with custom Roslyn-based parser (regex fallback)
7. ✅ Swift - Full support with custom Swift parser

**Implementation Insights**:
- Dual-mode parsing (AST + regex fallback) provides robustness
- External native parsers (Java, C#, Swift) offer superior accuracy over tree-sitter for complex grammars
- Runtime loading of parsers avoids build-time environment dependencies
- Consistent `LanguagePlugin` trait ensures uniform API across all languages

**Next Steps**:
- Prioritize C++ implementation for maximum remaining market impact
- Gather user feedback on C, Kotlin, PHP demand via GitHub discussions
