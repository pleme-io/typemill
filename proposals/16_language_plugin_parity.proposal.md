# Language Plugin Feature Parity

**Status**: Superseded by language-specific proposals (see below)

## ✅ Split into Language-Specific Proposals

This proposal has been split into detailed, language-specific proposals based on comprehensive code audit:

- **[16a: C# Parity Completion](16a_csharp_parity_completion.md)** - Missing 5/12 traits (58% → 100%)
- **[16b: Swift Parity Completion](16b_swift_parity_completion.md)** - Missing 7/12 traits (42% → 100%)
- **[16c: Go Parity Completion](16c_go_parity_completion.md)** - Missing 7/12 traits + critical bugs (42% → 100%)
- **[16d: C Parity Completion](16d_c_parity_completion.md)** - Missing 6/12 traits + stubs (42% → 60% experimental)
- **[16e: C++ Parity Completion](16e_cpp_parity_completion.md)** - Missing 2/12 traits + zero tests (83% → 100%)

## Problem

Language plugins have significant feature gaps preventing unified testing and consistent user experience across languages. Current parity status shows Rust (17/17 traits), TypeScript (15/15), and Python (14/15) as complete, while Java (11/15), C# (10/15), Swift (8/15), and Go (7/15) have missing core functionality.

**Update (2025-10-30)**: Comprehensive code audit revealed actual state differs from claimed state:
- Python: **100%** (12/12 traits) ✅
- Java: **100%** (12/12 traits) ✅
- Rust: **100%** (12/12 traits + 3 Rust-specific) ✅
- TypeScript: **100%** (12/12 traits + 1 TS-specific) ✅
- C#: **58%** (7/12 traits) ⚠️
- Swift: **42%** (5/12 traits) ⚠️
- Go: **42%** (5/12 traits + critical bugs) ⚠️
- C++: **83%** (10/12 traits claimed, zero tests) ⚠️
- C: **42%** (5/12 traits + stub refactoring) ⚠️

**Critical Gaps:**
- Java/Go/Swift/C# lack ManifestUpdater (cannot update dependencies programmatically)
- Go/Swift/C#/C lack WorkspaceSupport (multi-package project operations)
- Go/Swift/C lack RefactoringProvider (AST-based code transformations)
- Java/Go/Swift/C# lack ImportAnalyzer/ModuleReferenceScanner (dependency analysis)
- All non-core languages lack LspInstaller (auto-setup for LSP servers)

**Known Bugs:**
- Java ImportRenameSupport fails to update package imports (`com.example.utils` → `com.example.helpers`)
- C RefactoringProvider implemented but returns false for all operations (misleading stub)
- C++ plugin registration broken (fails inventory discovery despite mill_plugin! macro)

**Test Coverage Issues:**
- Import harness: 6/9 languages (C/C++ lack ImportMutationSupport, C# has tree-sitter conflict)
- Workspace harness: 4/9 languages (Go/Swift/C#/C/C++ missing WorkspaceSupport)
- Language-specific tests scattered across plugin crates instead of unified harness

## Solution

Achieve 100% feature parity for production languages (TypeScript, Rust, Python, Java, Go, Swift, C#) by implementing missing traits and fixing bugs. Establish unified test harness coverage for all languages.

**Implementation Order (by completeness gap):**
1. Go (7/15 → 15/15): +8 traits
2. Swift (8/15 → 15/15): +7 traits
3. C# (10/15 → 15/15): +5 traits
4. Java (11/15 → 15/15): +4 traits

**Experimental Languages (C/C++):**
- Fix C RefactoringProvider stub (implement or remove)
- Fix C++ plugin discovery issue
- Document limitations clearly

## Checklists

### Phase 1: Bug Fixes (Blockers)

- [ ] Fix Java ImportRenameSupport package import bug
  - [ ] Debug why `com.example.utils` → `com.example.helpers` rename fails
  - [ ] Add test case to Java plugin tests
  - [ ] Verify import_harness test passes
- [ ] Fix C RefactoringProvider misleading stub
  - [ ] Either implement inline_variable/extract_function OR remove trait entirely
  - [ ] Document decision in C plugin README
- [ ] Fix C++ plugin discovery issue
  - [ ] Debug why extern crate mill_lang_cpp doesn't enable discovery
  - [ ] Check mill_plugin! macro registration in C++ plugin
  - [ ] Add test to verify plugin loads in test harness
- [ ] Resolve C# tree-sitter version conflict (0.20 vs 0.25)
  - [ ] Upgrade C# plugin to tree-sitter 0.25
  - [ ] Update dependencies in Cargo.toml
  - [ ] Run full test suite to verify compatibility

### Phase 2: Go Plugin Completion (7/15 → 15/15)

- [ ] Add WorkspaceSupport to Go plugin
  - [ ] Implement is_workspace_manifest (detect go.work files)
  - [ ] Implement list_workspace_members (parse `use` directives)
  - [ ] Implement add_workspace_member (add to go.work)
  - [ ] Implement remove_workspace_member (remove from go.work)
  - [ ] Implement update_package_name (update module directive in go.mod)
  - [ ] Add Go to workspace_harness (fixtures already exist)
  - [ ] Verify all 7 workspace tests pass
- [ ] Add RefactoringProvider to Go plugin
  - [ ] Implement inline_variable (AST-based)
  - [ ] Implement extract_function (AST-based)
  - [ ] Implement extract_variable (AST-based)
  - [ ] Add refactoring tests to Go plugin
- [ ] Add ImportAdvancedSupport to Go plugin (currently returns None)
  - [ ] Implement dependency graph construction
  - [ ] Handle Go package aliases
- [ ] Add ImportAnalyzer to Go plugin
  - [ ] Implement dependency analysis
  - [ ] Handle internal/external package detection
- [ ] Add ModuleReferenceScanner to Go plugin
  - [ ] Scan for package references
  - [ ] Handle vendor directory scanning
- [ ] Add ManifestUpdater to Go plugin
  - [ ] Parse go.mod using go-mod crate or tree-sitter
  - [ ] Implement add_dependency
  - [ ] Implement remove_dependency
  - [ ] Implement update_version
- [ ] Add LspInstaller to Go plugin
  - [ ] Implement gopls detection and download
  - [ ] Add to ~/.mill/lsp/gopls cache
  - [ ] Verify gopls starts correctly

### Phase 3: Swift Plugin Completion (8/15 → 15/15)

- [ ] Add WorkspaceSupport to Swift plugin
  - [ ] Implement is_workspace_manifest (detect Package.swift with multiple targets)
  - [ ] Implement list_workspace_members (parse Package.swift targets)
  - [ ] Implement add_workspace_member
  - [ ] Implement remove_workspace_member
  - [ ] Implement update_package_name
  - [ ] Add Swift to workspace_harness
  - [ ] Verify all 7 workspace tests pass
- [ ] Add RefactoringProvider to Swift plugin
  - [ ] Implement inline_variable
  - [ ] Implement extract_function
  - [ ] Implement extract_variable
- [ ] Add ImportAnalyzer to Swift plugin
  - [ ] Analyze import dependencies
  - [ ] Handle Swift module system
- [ ] Add ModuleReferenceScanner to Swift plugin
  - [ ] Scan for module references
  - [ ] Handle SPM package dependencies
- [ ] Add ManifestUpdater to Swift plugin
  - [ ] Parse Package.swift
  - [ ] Implement dependency updates
- [ ] Add LspInstaller to Swift plugin
  - [ ] Implement sourcekit-lsp detection
  - [ ] Auto-install if Xcode not present

### Phase 4: C# Plugin Completion (10/15 → 15/15)

- [ ] Add WorkspaceSupport to C# plugin
  - [ ] Implement is_workspace_manifest (detect solution .sln files)
  - [ ] Implement list_workspace_members (parse .sln)
  - [ ] Implement add_workspace_member
  - [ ] Implement remove_workspace_member
  - [ ] Implement update_package_name (update .csproj)
  - [ ] Add C# to workspace_harness
- [ ] Add ImportAnalyzer to C# plugin
  - [ ] Analyze using directives
  - [ ] Handle NuGet package references
- [ ] Add ModuleReferenceScanner to C# plugin
  - [ ] Scan for namespace references
  - [ ] Handle assembly references
- [ ] Add ManifestUpdater to C# plugin
  - [ ] Parse .csproj XML
  - [ ] Implement PackageReference updates
- [ ] Add LspInstaller to C# plugin
  - [ ] Implement csharp-ls detection and download
  - [ ] Handle OmniSharp alternative

### Phase 5: Java Plugin Completion (11/15 → 15/15)

- [ ] Add ImportAnalyzer to Java plugin
  - [ ] Analyze import dependencies
  - [ ] Handle Maven/Gradle dependency trees
- [ ] Add ModuleReferenceScanner to Java plugin
  - [ ] Scan for package references
  - [ ] Handle module-info.java (Java 9+)
- [ ] Add ManifestUpdater to Java plugin
  - [ ] Parse pom.xml (Maven)
  - [ ] Implement dependency updates
  - [ ] Handle build.gradle (Gradle) as secondary
- [ ] Add LspInstaller to Java plugin
  - [ ] Implement jdtls detection and download
  - [ ] Configure Eclipse JDT Language Server

### Phase 6: Test Harness Unification

- [ ] Expand import_harness to all 9 languages
  - [ ] Add C/C++ after mutation support implemented
  - [ ] Add C# after tree-sitter upgrade
  - [ ] Verify all 7 import tests pass for all languages
- [ ] Expand workspace_harness to all 7 production languages
  - [ ] Add Go after WorkspaceSupport implemented
  - [ ] Add Swift after WorkspaceSupport implemented
  - [ ] Add C# after WorkspaceSupport implemented
  - [ ] Verify all 7 workspace tests pass
- [ ] Create refactoring_harness for unified refactoring tests
  - [ ] Define RefactoringScenarios (inline_variable, extract_function, extract_variable)
  - [ ] Add fixtures for all languages with RefactoringProvider
  - [ ] Create refactoring_harness_integration.rs
  - [ ] Verify tests pass for all 7 production languages
- [ ] Create analysis_harness for ImportAnalyzer/ModuleReferenceScanner
  - [ ] Define AnalysisScenarios (dependency graph, module scanning)
  - [ ] Add fixtures for all languages
  - [ ] Create analysis_harness_integration.rs
- [ ] Document test coverage in mill-test-support README
  - [ ] List which languages pass which harnesses
  - [ ] Document known limitations per language

### Phase 7: Documentation and Validation

- [ ] Update CLAUDE.md language parity table
  - [ ] Show 15/15 for all production languages
  - [ ] Document experimental status of C/C++
  - [ ] Add trait implementation matrix
- [ ] Create docs/architecture/language_plugin_traits.md
  - [ ] Document all 17 capability traits
  - [ ] Show implementation requirements
  - [ ] Provide examples from Rust/TypeScript reference implementations
- [ ] Run full test suite to validate parity
  - [ ] `cargo nextest run --workspace`
  - [ ] Verify 0 failures across all harnesses
  - [ ] Check all language plugins tested

## Success Criteria

- [ ] All 7 production languages implement 15/15 common traits (Rust has 2 additional language-specific traits)
- [ ] Import harness: 9/9 languages passing all 7 tests (or documented reason for exclusion)
- [ ] Workspace harness: 7/7 production languages passing all 7 tests
- [ ] Refactoring harness: 7/7 production languages passing all 3 operation tests
- [ ] Analysis harness: 7/7 production languages passing all analysis tests
- [ ] Zero language-specific bug workarounds in test harnesses
- [ ] All known bugs fixed (Java rename, C stub, C++ discovery, C# tree-sitter)

## Benefits

- **Unified Testing**: Single test harness validates all languages identically
- **Consistent UX**: All languages support same features through MCP tools
- **Reduced Duplication**: Eliminate 200+ lines of language-specific tests
- **Clear Gaps**: Matrix shows exactly what's missing for each language
- **Easy Additions**: Adding new languages follows established trait pattern
- **Quality Assurance**: Comprehensive harness tests catch regressions immediately
