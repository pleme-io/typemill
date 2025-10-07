# Proposal: Language Support Expansion

**Status**: Proposal - For Discussion
**Date**: 2025-10-05

## 1. Overview

CodeBuddy currently supports 4 languages via LSP integration and language plugins. This proposal outlines a strategic expansion plan to cover the top 10 most popular programming languages in 2025.

## 2. Current vs Target Support Matrix

### Top 10 Languages (2025 Rankings)

| Rank | Language           | Current LSP | Language Plugin | Priority | Notes |
|------|-------------------|-------------|-----------------|----------|-------|
| 1    | **Python**        | ✅          | ❌              | 🔴 High  | LSP only, missing AST plugin |
| 2    | **C++**           | ❌          | ❌              | 🟡 Medium | High-performance systems, game engines |
| 3    | **Java**          | ❌          | ❌              | 🟡 Medium | Enterprise apps, Android development |
| 4    | **JavaScript/TypeScript** | ✅ | ✅              | ✅ Done  | Fully supported |
| 5    | **C#**            | ❌          | ❌              | 🟢 Low   | Game dev (Unity), enterprise software |
| 6    | **C**             | ❌          | ❌              | 🟢 Low   | Systems programming, embedded devices |
| 7    | **Go**            | ✅          | ✅              | ✅ Done  | Fully supported |
| 8    | **Rust**          | ✅          | ✅              | ✅ Done  | Fully supported |
| 9    | **Swift/Kotlin**  | ❌          | ❌              | 🟢 Low   | Mobile app development (iOS/Android) |
| 10   | **PHP**           | ❌          | ❌              | 🟢 Low   | Web development, dynamic pages |

### Summary Statistics

- **Fully Supported**: 3/10 (TypeScript/JS, Go, Rust)
- **Partial Support**: 1/10 (Python - LSP only)
- **Not Supported**: 6/10 (C++, Java, C#, C, Swift/Kotlin, PHP)
- **Coverage**: 40% complete

## 3. Implementation Checklist

### Phase 1: Complete Existing Support (🔴 High Priority)

- [ ] **Python Language Plugin** (`crates/languages/cb-lang-python`)
  - AST parsing for import analysis
  - Manifest parsing (`requirements.txt`, `pyproject.toml`, `setup.py`)
  - Extract function/variable refactoring support
  - Estimated effort: 2-3 weeks

### Phase 2: Enterprise Languages (🟡 Medium Priority)

- [ ] **Java Support** (`crates/cb-lang-java`)
  - LSP: `jdtls` (Eclipse JDT Language Server)
  - AST parsing via `tree-sitter-java`
  - Manifest: `pom.xml`, `build.gradle`, `build.gradle.kts`
  - Package manager: Maven, Gradle
  - Estimated effort: 3-4 weeks

- [ ] **C++ Support** (`crates/languages/cb-lang-cpp`)
  - LSP: `clangd`
  - AST parsing via `tree-sitter-cpp`
  - Build systems: CMake, Makefile, Bazel
  - Package managers: Conan, vcpkg
  - Estimated effort: 4-5 weeks (complex build systems)

### Phase 3: Additional Languages (🟢 Low Priority)

- [ ] **C# Support** (`crates/languages/cb-lang-csharp`)
  - LSP: `OmniSharp`
  - AST parsing via `tree-sitter-c-sharp`
  - Build: MSBuild, .NET CLI
  - Package manager: NuGet
  - Estimated effort: 3-4 weeks

- [ ] **C Support** (`crates/languages/cb-lang-c`)
  - LSP: `clangd` (shared with C++)
  - AST parsing via `tree-sitter-c`
  - Build systems: Make, CMake
  - Estimated effort: 2-3 weeks

- [ ] **Swift Support** (`crates/languages/cb-lang-swift`)
  - LSP: `sourcekit-lsp`
  - AST parsing via `tree-sitter-swift`
  - Build: Swift Package Manager
  - Estimated effort: 3-4 weeks

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
| C#         | `omnisharp`       | Download from OmniSharp |
| C          | `clangd`          | Same as C++ |
| Swift      | `sourcekit-lsp`   | Included with Swift toolchain |
| Kotlin     | `kotlin-language-server` | Download from GitHub |
| PHP        | `intelephense`    | `npm install -g intelephense` |

## 5. Effort Estimation

### Total Development Time

- **Phase 1 (Python)**: 2-3 weeks
- **Phase 2 (Java + C++)**: 7-9 weeks
- **Phase 3 (C#, C, Swift, Kotlin, PHP)**: 13-18 weeks

**Total**: 22-30 weeks (5.5-7.5 months) for full top-10 coverage

### Resource Requirements

- 1 developer full-time: ~7 months
- 2 developers full-time: ~3.5 months
- With community contributions: could be faster

## 6. Priority Justification

### 🔴 High Priority: Python Plugin

- Already has LSP support (50% done)
- Rank #1 language globally
- AI/ML/data science dominance
- Completes existing partial support
- **Impact**: Unlocks full refactoring capabilities for Python users

### 🟡 Medium Priority: Java & C++

- **Java**: #3 globally, massive enterprise adoption
  - Android development (billions of devices)
  - Enterprise backend systems
  - Spring Framework ecosystem

- **C++**: #2 globally, performance-critical domains
  - Game engines (Unreal Engine)
  - Financial trading platforms
  - Embedded systems, robotics

### 🟢 Low Priority: C#, C, Swift, Kotlin, PHP

- Smaller user bases relative to top priorities
- More specialized use cases
- Can be added incrementally based on demand

## 7. Business Impact

### Market Coverage

| Phase | Languages Added | Market Coverage | Cumulative Coverage |
|-------|----------------|-----------------|---------------------|
| Current | TypeScript/JS, Go, Rust, Python* | 40% | 40% |
| Phase 1 | Python (complete) | +10% | 50% |
| Phase 2 | Java, C++ | +25% | 75% |
| Phase 3 | C#, C, Swift, Kotlin, PHP | +25% | 100% |

*Python currently partial (LSP only)

### Target User Segments

- **Phase 1**: Data scientists, ML engineers, Python web developers
- **Phase 2**: Enterprise developers, game developers, systems programmers
- **Phase 3**: Mobile developers, web developers, systems programmers

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

- **Coverage**: 100% of top 10 languages supported
- **Quality**: All MCP tools work across all languages
- **Performance**: AST parsing <100ms for typical files
- **Adoption**: User growth in new language segments

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

1. **Immediate**: Implement Phase 1 (Python plugin) - completes partial support
2. **Q1 2025**: Implement Phase 2 (Java + C++) - enterprise & performance markets
3. **Q2 2025**: Assess demand for Phase 3 languages via user surveys
4. **Q3 2025**: Implement top-requested Phase 3 languages
5. **Q4 2025**: Evaluate community plugin system for long-tail languages

## 12. Open Questions

- Should we support multiple versions of language servers (e.g., Java 8 vs 17)?
- Should we bundle LSP servers or require user installation?
- Should we provide pre-configured Docker images per language?
- What's the minimum LSP feature set required for "supported" status?

---

**Next Steps**: Gather user feedback on language priorities via GitHub discussions/surveys before committing to Phase 2/3 priorities.
