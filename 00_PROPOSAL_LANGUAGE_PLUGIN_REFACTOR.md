# Proposal: Language Plugin Architecture Refactor

**STATUS**: ✅ Phase 1 Complete (Commit: 3662626), ✅ Phase 2 Complete (Commit: 3aa4956)

Phase 1 (A-G): Trait architecture refactored
Phase 2 (Waves 1-3): Capability trait integration complete
Phase 3: Java validation - PENDING (requires network access to merge feat/java-language-support)

---

**Created**: 2025-01-06
**Author**: Architecture Review
**Phases**: 7 phases (1A-1G for trait refactor, then Phase 2-3 for helpers/validation)
**Impact**: -573 LOC immediate, -200 LOC per future language

---

## Executive Summary

Refactor the monolithic `LanguageIntelligencePlugin` trait (22 methods) into a capability-based architecture with composable helper utilities. This change will:

- **Reduce boilerplate by 29-42% per language** (-187 to -192 LOC for TS/Go)
- **Cut new language implementation time in half** (3 days → 1.5 days)
- **Enable opt-in capabilities** instead of NotSupported shims
- **Fix async boundary overhead** for sync string operations
- **Improve maintainability** as we scale to 10+ languages

---

## Problem Statement

### Current Architecture Issues

1. **Monolithic trait surface** - 22 methods intimidate new language implementers
2. **48% boilerplate** - Metadata methods, import regex, manifest parsing duplicated across all plugins
3. **Async overhead** - 6+ methods are sync operations wrapped in unnecessary async
4. **Feature detection via errors** - Must await `NotSupported` errors instead of checking capability flags
5. **Parity gaps** - Python missing 12 methods, TypeScript/Go missing 6 workspace methods

### By The Numbers

| Language | Current LOC | Boilerplate | Custom Logic |
|----------|-------------|-------------|--------------|
| Rust     | 720         | 350 (49%)   | 370 (51%)    |
| TypeScript | 475       | 280 (59%)   | 195 (41%)    |
| Go       | 460         | 260 (57%)   | 200 (43%)    |
| Python   | 310         | 230 (74%)   | 80 (26%)     |

**Total**: 1,965 LOC with **1,120 LOC (57%) boilerplate**

---

## Proposed Solution

### Three-Part Architecture

#### 1. Capability-Based Subtraits

Split monolithic trait into focused capabilities:

```rust
// Core trait - REQUIRED (4 methods)
trait LanguagePlugin: Send + Sync {
    fn metadata(&self) -> &LanguageMetadata;
    fn parse(&self, source: &str) -> PluginResult<ParsedSource>;
    fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>;
    fn capabilities(&self) -> LanguageCapabilities;

    // Optional capabilities
    fn import_support(&self) -> Option<&dyn ImportSupport> { None }
    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> { None }
}

// Import operations - OPTIONAL (6 methods, all SYNC)
trait ImportSupport: Send + Sync {
    fn parse_imports(&self, content: &str) -> Vec<String>;
    fn locate_module_files(&self, path: &Path, module: &str) -> Vec<PathBuf>;
    fn rewrite_import(&self, old: &str, new: &str) -> String;
    fn rewrite_imports_for_rename(&self, content: &str, ...) -> (String, usize);
    fn find_module_references(&self, content: &str, ...) -> Vec<ModuleReference>;
}

// Workspace operations - OPTIONAL (5 methods, all SYNC)
trait WorkspaceSupport: Send + Sync {
    fn is_workspace_manifest(&self, content: &str) -> bool;
    fn generate_workspace_manifest(&self, members: &[&str], root: &Path) -> String;
    fn add_workspace_member(&self, content: &str, member: &str) -> String;
    fn add_manifest_path_dependency(&self, content: &str, ...) -> String;
    fn remove_module_declaration(&self, source: &str, module: &str) -> String;
}
```

**Benefits**:
- Core trait: 22 methods → 6 methods
- Optional capabilities: Implement only what you support
- Sync methods: No async overhead for string operations
- Capability flags: O(1) feature detection

#### 2. Metadata Consolidation

Replace 7 methods with 1 struct:

```rust
pub struct LanguageMetadata {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub manifest_filename: &'static str,
    pub source_dir: &'static str,
    pub entry_point: &'static str,
    pub module_separator: &'static str,
    pub language: ProjectLanguage,
}

impl LanguageMetadata {
    pub const RUST: Self = Self {
        name: "Rust",
        extensions: &["rs"],
        manifest_filename: "Cargo.toml",
        source_dir: "src",
        entry_point: "lib.rs",
        module_separator: "::",
        language: ProjectLanguage::Rust,
    };

    pub const PYTHON: Self = /* ... */;
    pub const TYPESCRIPT: Self = /* ... */;
    pub const GO: Self = /* ... */;
}
```

**Savings**: 32 LOC per plugin (7 methods → 1 field access)

#### 3. Composable Helper Utilities

New crate: `crates/cb-plugin-helpers/`

```rust
// Regex-based import support (reusable by Python, Go, TypeScript fallback)
pub struct RegexImportSupport {
    patterns: Vec<Regex>,
}

impl ImportSupport for RegexImportSupport {
    // Implements all 6 ImportSupport methods
    // ~200 LOC, shared by 3+ languages
}

// JSON workspace support (TypeScript, potentially Go modules)
pub struct JsonWorkspaceSupport;

impl WorkspaceSupport for JsonWorkspaceSupport {
    // Implements all 5 WorkspaceSupport methods
    // ~150 LOC, shared by TS/Node/Go
}

// TOML workspace support (Rust)
pub struct TomlWorkspaceSupport;

impl WorkspaceSupport for TomlWorkspaceSupport {
    // Implements all 5 WorkspaceSupport methods
    // ~180 LOC, Rust-specific
}
```

**Benefits**:
- Write once, reuse across languages
- Tested once, works everywhere
- Language plugins delegate instead of reimplement

---

## Implementation Example: Python Plugin

### Before (Current)

```rust
impl LanguageIntelligencePlugin for PythonPlugin {
    fn name(&self) -> &'static str { "Python" }
    fn file_extensions(&self) -> Vec<&'static str> { vec!["py", "pyi"] }
    fn manifest_filename(&self) -> &'static str { "requirements.txt" }
    fn source_dir(&self) -> &'static str { "" }
    fn entry_point(&self) -> &'static str { "__init__.py" }
    fn module_separator(&self) -> &'static str { "." }
    fn language(&self) -> ProjectLanguage { ProjectLanguage::Python }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        parser::extract_symbols(source)
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::load_requirements_txt(path).await
    }

    // 12 methods return NotSupported errors (180+ LOC)
    async fn parse_imports(&self, _file: &Path) -> PluginResult<Vec<String>> {
        Err(PluginError::not_supported("parse_imports not supported for Python"))
    }
    // ... 11 more NotSupported methods
}

// TOTAL: 310 LOC
```

### After (Proposed)

```rust
use cb_plugin_helpers::{LanguageMetadata, RegexImportSupport};

pub struct PythonPlugin {
    metadata: LanguageMetadata,
    imports: RegexImportSupport,
}

impl PythonPlugin {
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::PYTHON,
            imports: RegexImportSupport::new(vec![
                r"^import\s+(\w+)",
                r"^from\s+([\w.]+)\s+import",
            ]),
        }
    }
}

impl LanguagePlugin for PythonPlugin {
    fn metadata(&self) -> &LanguageMetadata { &self.metadata }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        parser::extract_symbols(source)
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::load_requirements_txt(path).await
    }

    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            supports_import_rewriting: true,
            supports_workspaces: false,
            supports_ast_fallback: true,
        }
    }

    fn import_support(&self) -> Option<&dyn ImportSupport> {
        Some(&self.imports)  // Delegate to helper
    }

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        None  // Python has no workspace concept
    }
}

// TOTAL: 223 LOC (-87 LOC, -28%)
// BONUS: Gains full import rewriting functionality
```

---

## Impact Analysis

### Existing Languages - LOC Reduction

| Language | Current LOC | After LOC | Reduction | Percentage |
|----------|-------------|-----------|-----------|------------|
| TypeScript | 475 | 288 | **-187** | **-39%** |
| Go | 460 | 268 | **-192** | **-42%** |
| Python | 310 | 223 | **-87** | **-28%** ⭐ gains import support |
| Rust | 720 | 613 | **-107** | **-15%** |
| **TOTAL** | **1,965** | **1,392** | **-573** | **-29%** |

### New Languages - Java Example

| Approach | LOC | Time | Effort |
|----------|-----|------|--------|
| Current architecture | 630 | 3 days | High cognitive load (22 methods) |
| Proposed architecture | 425 | 1.5 days | Low cognitive load (4-6 methods) |
| **Savings** | **-205 (-33%)** | **-50%** | **Focus on custom logic only** |

### Capability Detection - Performance

```rust
// Current: Try/catch with async overhead
match plugin.add_workspace_member(...).await {
    Err(PluginError::NotSupported) => { /* skip */ }
    Ok(result) => { /* use it */ }
}

// Proposed: O(1) capability check
if plugin.capabilities().supports_workspaces {
    if let Some(ws) = plugin.workspace_support() {
        ws.add_workspace_member(...)?;  // Sync, no await
    }
}
```

**Performance**: Skip unsupported operations without async overhead

---

## Implementation Plan - Phased Approach

The refactor is broken into **7 sequential phases** to ensure clean validation points and manageable scope.

---

### **Phase 1A: Foundation - Trait Definitions**

**Goal**: Create new trait architecture without touching existing code

**Files to Create (3)**:
- `crates/cb-plugin-api/src/metadata.rs` - LanguageMetadata struct with pre-defined constants
- `crates/cb-plugin-api/src/import_support.rs` - ImportSupport trait (6 sync methods)
- `crates/cb-plugin-api/src/workspace_support.rs` - WorkspaceSupport trait (5 sync methods)

**Files to Modify (1)**:
- `crates/cb-plugins/src/capabilities.rs` - Add LanguageCapabilities struct (extend existing)

**Deliverable**: New traits compile independently
**Validation**: `cargo check -p cb-plugin-api` passes
**Impact**: Zero - new code sits unused, existing code unaffected

---

### **Phase 1B: Core API Split**

**Goal**: Refactor main trait to use new structure

**Files to Modify (1)**:
- `crates/cb-plugin-api/src/lib.rs` - Split monolithic 22-method trait into 6-method core trait

**Key Changes**:
- Remove 7 metadata methods → replace with `metadata()` returning struct
- Remove 6 import methods → replace with `import_support()` returning optional trait
- Remove 5 workspace methods → replace with `workspace_support()` returning optional trait
- Add module declarations and re-exports

**Deliverable**: Core trait compiles but nothing implements it yet
**Validation**: `cargo check -p cb-plugin-api` passes
**Impact**: ⚠️ **Breaking** - Workspace build will fail (expected), plugins don't implement new trait yet

---

### **Phase 1C: Plugin Migration - Rust Template**

**Goal**: Migrate ONE plugin as reference implementation

**Files to Modify (2)**:
- `crates/languages/cb-lang-rust/src/lib.rs` - Implement new trait structure
- `crates/languages/cb-lang-rust/Cargo.toml` - Update if needed

**Key Changes**:
- Add `metadata` field to plugin struct
- Implement new `LanguagePlugin` trait (6 methods)
- Implement `ImportSupport` trait (move existing methods, remove async)
- Implement `WorkspaceSupport` trait (move existing methods, remove async)
- Add `capabilities()` method

**Deliverable**: Rust plugin works with new trait system
**Validation**: `cargo check -p cb-lang-rust` passes
**Impact**: Rust plugin compiles, creates template for others

---

### **Phase 1D: Plugin Migration - Remaining Languages**

**Goal**: Apply Rust template to TypeScript, Go, Python

**Files to Modify (6)**:
- `crates/languages/cb-lang-typescript/src/lib.rs` + Cargo.toml
- `crates/languages/cb-lang-go/src/lib.rs` + Cargo.toml
- `crates/languages/cb-lang-python/src/lib.rs` + Cargo.toml

**Key Changes**: Same pattern as Phase 1C for each language
- TypeScript: `import_support() → Some(self)`, `workspace_support() → None`
- Go: `import_support() → Some(self)`, `workspace_support() → None`
- Python: `import_support() → None`, `workspace_support() → None`

**Deliverable**: All 4 plugins use new trait system
**Validation**: `cargo check` in `crates/languages/` passes
**Impact**: All plugins compile, but handler/service consumers still broken

---

### **Phase 1E: Consumer Layer - Handlers**

**Goal**: Update handler layer to use new plugin API

**Files to Modify (6)**:
- `crates/cb-handlers/src/language_plugin_registry.rs` - Update trait bounds
- `crates/cb-handlers/src/handlers/tools/workspace.rs` - Capability checks + sync calls
- `crates/cb-handlers/src/handlers/tools/editing.rs` - Capability checks for imports
- `crates/cb-handlers/src/handlers/tools/navigation.rs` - Use `plugin.metadata().*`
- `crates/cb-handlers/src/handlers/tools/system.rs` - Minor trait updates
- `crates/cb-handlers/src/handlers/tools/internal_workspace.rs` - Capability-aware ops

**Key Pattern**:
```rust
// Before: await NotSupported errors
plugin.add_workspace_member(...).await?;

// After: capability check + sync call
if plugin.capabilities().supports_workspaces {
    let ws = plugin.workspace_support().unwrap();
    ws.add_workspace_member(...)?;  // No .await!
}
```

**Deliverable**: Handler layer compiles with new traits
**Validation**: `cargo check -p cb-handlers` passes
**Impact**: Handlers work, services layer still broken

---

### **Phase 1F: Consumer Layer - Services**

**Goal**: Update service layer to use new plugin API

**Files to Modify (2)**:
- `crates/cb-ast/src/import_updater.rs` - Capability checks, sync imports, remove .await
- `crates/cb-ast/src/package_extractor.rs` - Capability checks, sync workspace, remove .await

**Key Changes**:
- Add capability checks at entry points
- Get trait objects (`ImportSupport`, `WorkspaceSupport`)
- Remove `.await` from plugin method calls (now sync)
- Handle `None` gracefully when capability missing

**Deliverable**: Full codebase compiles
**Validation**: `cargo build` succeeds
**Impact**: Everything compiles, ready for testing

---

### **Phase 1G: Documentation & Validation**

**Goal**: Update documentation and validate full system

**Files to Modify (3)**:
- `API.md` - New trait structure, capability examples, before/after code
- `crates/languages/README.md` - Updated implementation guide with 6-method trait
- `docs/architecture/ARCHITECTURE.md` - Capability system explanation, async/sync rationale

**Testing Checklist**:
- [ ] Unit tests pass for all plugins
- [ ] Integration tests pass
- [ ] Can run `find_definition` on Rust/TypeScript/Go/Python files
- [ ] Can run `rename_symbol` cross-language
- [ ] Python gracefully returns capability errors
- [ ] Documentation builds without warnings

**Deliverable**: Project ready for Phase 2 (helper utilities)
**Validation**: `cargo test` passes
**Impact**: Trait refactor complete, foundation for 50%+ LOC reduction

---

## Phase Summary

| Phase | Focus | Files | Validation Command | Can Break Build? |
|-------|-------|-------|-------------------|------------------|
| **1A** | New trait definitions | 4 CREATE/EDIT | `cargo check -p cb-plugin-api` | ❌ No |
| **1B** | Core API split | 1 EDIT | `cargo check -p cb-plugin-api` | ✅ Yes (expected) |
| **1C** | Rust plugin template | 2 EDIT | `cargo check -p cb-lang-rust` | ✅ Yes (partial) |
| **1D** | Other plugins | 6 EDIT | `cargo check` in languages/ | ✅ Yes (partial) |
| **1E** | Handler updates | 6 EDIT | `cargo check -p cb-handlers` | ✅ Yes (partial) |
| **1F** | Service updates | 2 EDIT | `cargo build` | ❌ Should pass |
| **1G** | Docs & tests | 3 EDIT | `cargo test` | ❌ Should pass |

**Total Files**: 3 new, 21 modified across 7 phases

---

### **Phase 2: Helper Utilities** (Future Work)

After Phase 1G completes, Phase 2 will create the `cb-plugin-helpers` crate with:
- `RegexImportSupport` - Reusable for Python, Go, TypeScript
- `JsonWorkspaceSupport` - Reusable for TypeScript
- `TomlWorkspaceSupport` - Reusable for Rust
- Edge case hardening and fuzzing

**Expected Impact**: 50%+ LOC reduction per plugin

---

### **Phase 3: Java Plugin Validation** (Future Work)

Prove the new architecture works for greenfield languages:
- Implement Java plugin using helpers (~425 LOC)
- Validate implementation speed improvement
- Document best practices

**Success Metric**: Java plugin in <250 LOC, implemented faster than current approach

---

## Risk Assessment & Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **Breaking changes disrupt consumers** | High | High | Phased rollout (7 phases), validation checkpoints |
| **Trait object lifetime issues** | Medium | Medium | Rust plugin template (Phase 1C) validates pattern early |
| **Plugin migration introduces bugs** | Low | Medium | Copy-paste from working Rust template, isolated testing |
| **Capability detection breaks flows** | Medium | High | Extensive integration tests in Phase 1G |
| **Helper complexity in Phase 2** | Medium | Medium | Phase 1 establishes trait contracts first |

---

## Success Metrics

### After Phase 1G (Trait Refactor Complete)
- [ ] All 4 existing plugins migrated to new architecture
- [ ] Core trait reduced from 22 methods to 6 methods
- [ ] All integration tests pass
- [ ] Documentation updated and accurate
- [ ] Zero functionality regression

### After Phase 2 (Helper Utilities)
- [ ] Helper crate with >90% test coverage
- [ ] 50%+ LOC reduction in plugin implementations
- [ ] All 4 languages using helpers

### After Phase 3 (Java Validation)
- [ ] Java plugin <250 LOC in lib.rs
- [ ] All refactoring operations work cross-language
- [ ] Implementation time significantly reduced vs current approach

### Long-term (After 5+ new languages)
- [ ] Average plugin LOC < 450 (vs 600 current)
- [ ] New languages implemented faster
- [ ] Reduced maintenance burden for trait changes

---

## Return on Investment

### Investment
- **Phase 1 (A-G)**: Trait architecture refactor
- **Phase 2**: Helper crate creation
- **Phase 3**: Java validation

### Immediate Return
- **573 LOC removed** from existing codebase
- **O(1) capability detection** instead of try/catch
- **No more 4× plugin updates** when adding trait methods
- **Reduced async overhead** for string operations

### Ongoing Return (Per New Language)
- **~200 LOC saved** per plugin implementation
- **Faster implementation** via helpers and clear patterns
- **Lower maintenance** when adding new capabilities
- **Better developer experience** with focused traits

---

## Alternatives Considered

### Alternative 1: Keep Single Trait + Helpers Only
**Pros**: No breaking changes
**Cons**: Still 22 methods, async overhead remains, NotSupported shims
**Verdict**: ❌ Doesn't address root cause

### Alternative 2: Macro-Generated Implementations
**Pros**: Minimal boilerplate
**Cons**: Macro complexity, less flexible, harder to debug
**Verdict**: ⚠️ Consider for future, not initial refactor

### Alternative 3: Do Nothing
**Pros**: No effort required
**Cons**: Technical debt compounds with each new language
**Verdict**: ❌ Problem gets worse at 10+ languages

---

## Dependencies & Prerequisites

### Required
- [ ] Approval from architecture review
- [ ] 10 days of focused development time
- [ ] Test coverage >85% before merging

### Optional
- [ ] Pause new language additions during Week 1-2
- [ ] Coordinated announcement for breaking change

---

## Follow-up Work

### Immediate (After Week 3)
- [ ] Add Java AST subprocess (`ast_tool.java`) - 4h
- [ ] Performance benchmarks for capability detection - 2h
- [ ] Migration guide for external plugins (if any) - 2h

### Future Enhancements
- [ ] C# plugin using helpers - 1.5 days
- [ ] Ruby plugin using helpers - 1.5 days
- [ ] Kotlin plugin using helpers - 1.5 days
- [ ] Macro derive support for trivial plugins - 3 days

---

## Approval & Sign-off

**Proposed by**: Architecture Review Team
**Date**: 2025-01-06

**Approved by**: _________________
**Date**: _________________

**Notes**:

---

## Appendix A: File Impact Summary

### New Files (16)

**Week 1 (3)**:
- `crates/cb-plugin-api/src/metadata.rs`
- `crates/cb-plugin-api/src/import_support.rs`
- `crates/cb-plugin-api/src/workspace_support.rs`

**Week 2 (7)**:
- `crates/cb-plugin-helpers/Cargo.toml`
- `crates/cb-plugin-helpers/src/lib.rs`
- `crates/cb-plugin-helpers/src/imports.rs`
- `crates/cb-plugin-helpers/src/json_workspace.rs`
- `crates/cb-plugin-helpers/src/toml_workspace.rs`
- `crates/cb-plugin-helpers/tests/imports_smoke.rs`
- `crates/cb-plugin-helpers/tests/workspace_smoke.rs`

**Week 3 (6)**:
- `crates/languages/cb-lang-java/Cargo.toml`
- `crates/languages/cb-lang-java/README.md`
- `crates/languages/cb-lang-java/src/lib.rs`
- `crates/languages/cb-lang-java/src/parser.rs`
- `crates/languages/cb-lang-java/src/manifest.rs`
- `crates/languages/cb-lang-java/tests/plugin_parity.rs`

### Modified Files (56)

See detailed breakdown in implementation plan sections.

---

## Appendix B: Code Examples

### Before: Current Plugin Implementation

```rust
// Current: 310 LOC with NotSupported shims
impl LanguageIntelligencePlugin for PythonPlugin {
    // 7 metadata methods (40 LOC)
    fn name(&self) -> &'static str { "Python" }
    // ... 6 more

    // 2 required implementations (80 LOC)
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> { /* ... */ }
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> { /* ... */ }

    // 13 NotSupported shims (190 LOC)
    async fn parse_imports(&self, _file: &Path) -> PluginResult<Vec<String>> {
        Err(PluginError::not_supported("not implemented"))
    }
    // ... 12 more NotSupported methods
}
```

### After: Proposed Plugin Implementation

```rust
// Proposed: 223 LOC with helper delegation
use cb_plugin_helpers::{LanguageMetadata, RegexImportSupport};

pub struct PythonPlugin {
    metadata: LanguageMetadata,
    imports: RegexImportSupport,
}

impl LanguagePlugin for PythonPlugin {
    fn metadata(&self) -> &LanguageMetadata { &self.metadata }
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> { /* ... */ }
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> { /* ... */ }
    fn capabilities(&self) -> LanguageCapabilities { /* ... */ }
    fn import_support(&self) -> Option<&dyn ImportSupport> { Some(&self.imports) }
}
// No NotSupported methods - capabilities declare what's available
```

---

## Appendix C: References

- [Original parity analysis](conversation context)
- [Codex assessment](conversation context)
- [Language Support Matrix](API.md#language-support-matrix)
- [Plugin README](crates/languages/README.md)
