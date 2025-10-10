# Proposal: Reduce Language Support to TypeScript + Rust

**Status**: Draft
**Author**: Project Team
**Date**: 2025-10-10

---

## Executive Summary

Temporarily reduce language support from 7 languages to **2 languages (TypeScript + Rust)** to accelerate unified API implementation. Multi-language support preserved in git tag `pre-language-reduction` for easy restoration after refactoring is complete.

**Impact**: 71% reduction in test surface, faster iteration, simpler debugging.

---

## Problem

**Current state**: 7 languages supported
- TypeScript (`.ts`, `.tsx`, `.js`, `.jsx`)
- Python (`.py`, `.pyi`)
- Go (`.go`)
- Rust (`.rs`)
- Java (`.java`)
- Swift (`.swift`)
- C# (`.cs`)

**Complexity burden**:
- 7 LSP server configurations to maintain
- 7 language plugin implementations
- 7× test matrix for every refactoring operation
- Debugging across 7 different language server protocols
- Each language has unique quirks and edge cases

**Current focus**: Implementing unified refactoring + analysis APIs (PROPOSAL_UNIFIED_REFACTORING_API.md, PROPOSAL_UNIFIED_ANALYSIS_API.md)

**Reality**: We're the only users, primarily working in TypeScript and Rust.

---

## Solution

### Keep Only TypeScript + Rust

**Why these two?**

1. **TypeScript**:
   - Dynamic, interpreted language
   - Rich refactoring scenarios (async/await, arrow functions, React patterns)
   - `typescript-language-server` is well-maintained and feature-complete
   - Represents frontend/dynamic language use cases

2. **Rust**:
   - Static, compiled language with complex type system
   - **Primary language of this codebase**
   - `rust-analyzer` is extremely powerful
   - Ownership and lifetime analysis
   - Represents systems/static language use cases

**Coverage**: These extremes (dynamic vs static, GC vs ownership) validate the API works across different language paradigms.

---

## Implementation Plan

### 1. Create Preservation Tag ✅

**Already done**:
```bash
git tag -a pre-language-reduction -m "Multi-language snapshot"
```

**Restoration later**:
```bash
git checkout pre-language-reduction
# Cherry-pick unified API changes
# Restore Python, Go, Java, Swift, C# support
```

---

### 2. Remove Language Support

**Files/directories to remove or modify**:

#### **Language Plugins** (`crates/languages/`)
- **Keep**: `typescript/`, `rust/`
- **Remove**: `python/`, `go/`, `java/`, `swift/`, `csharp/`

#### **LSP Configurations**
- **Keep**: `typescript-language-server`, `rust-analyzer` configs
- **Remove**: `pylsp`, `gopls`, `jdtls`, `sourcekit-lsp`, `omnisharp` configs

#### **Test Fixtures**
- **Keep**: TypeScript and Rust test files
- **Remove**: Python, Go, Java, Swift, C# test fixtures

#### **Documentation**
- Update README.md, API_REFERENCE.md to reflect TS + Rust only
- Add note: "Multi-language support coming soon (Python, Go, Java, Swift, C#)"

#### **Integration Tests**
- **Keep**: Cross-language tests with TS ↔ Rust interactions
- **Remove**: Python, Go, Java, Swift, C# specific tests

---

### 3. Update Configuration Schema

**Before** (`.codebuddy/config.json`):
```json
{
  "servers": [
    { "extensions": ["ts", "tsx", "js", "jsx"], "command": ["typescript-language-server", "--stdio"] },
    { "extensions": ["py"], "command": ["pylsp"] },
    { "extensions": ["go"], "command": ["gopls"] },
    { "extensions": ["rs"], "command": ["rust-analyzer"] },
    { "extensions": ["java"], "command": ["jdtls"] },
    { "extensions": ["swift"], "command": ["sourcekit-lsp"] },
    { "extensions": ["cs"], "command": ["omnisharp"] }
  ]
}
```

**After**:
```json
{
  "servers": [
    { "extensions": ["ts", "tsx", "js", "jsx"], "command": ["typescript-language-server", "--stdio"] },
    { "extensions": ["rs"], "command": ["rust-analyzer"] }
  ]
}
```

---

### 4. Update Language Support Matrix

**Before** (from API_REFERENCE.md):

| Tool | TS | Python | Go | Rust | Java | Swift | C# |
|------|----|----|----|----|----|----|---|
| rename_symbol | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| extract_function | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ |
| ... | ... | ... | ... | ... | ... | ... | ... |

**After**:

| Tool | TypeScript | Rust |
|------|-----------|------|
| rename_symbol | ✅ | ✅ |
| extract_function | ✅ | ✅ |
| inline_variable | ✅ | ✅ |
| ... | ... | ... |

---

## Benefits

### 1. **71% Reduction in Test Surface**

**Before**:
- 14 refactoring commands × 7 languages = **98 test combinations**
- 6 analysis commands × 7 languages = **42 test combinations**
- **Total**: 140 test combinations

**After**:
- 14 refactoring commands × 2 languages = **28 test combinations**
- 6 analysis commands × 2 languages = **12 test combinations**
- **Total**: 40 test combinations

**Result**: 71% fewer tests to write and maintain.

---

### 2. **Faster Iteration**

- Debug only 2 LSP servers instead of 7
- No "works in TS but breaks in Python" issues during refactoring
- Simpler CI/CD pipeline
- Faster test runs

---

### 3. **Simpler Codebase**

- Fewer language plugin implementations
- Less conditional logic for language-specific quirks
- Easier to reason about behavior
- Smaller binary size

---

### 4. **Easy Restoration**

All removed code preserved in tag:
```bash
# Restore multi-language support
git checkout pre-language-reduction

# Or cherry-pick specific language back
git show pre-language-reduction:crates/languages/python/ > ...
```

---

## Risks & Mitigations

### Risk 1: Need Python/Go Support Mid-Refactoring

**Impact**: Medium - Would need to pause and restore language support.

**Mitigation**:
- Tag preserves all language code
- Restoration is just `git checkout pre-language-reduction`
- Can selectively restore individual languages without full restoration

---

### Risk 2: Lose Multi-Language Test Coverage

**Impact**: Low - Tests will need updating when languages are restored.

**Mitigation**:
- Language plugin architecture unchanged
- Tests can be re-enabled when languages are restored
- TypeScript + Rust validate core API across language paradigms

---

### Risk 3: API Design Doesn't Scale to Other Languages

**Impact**: Low - TS + Rust cover dynamic vs static extremes.

**Mitigation**:
- If API works for both TS (dynamic) and Rust (static), it should work for Python, Go, Java
- Language-specific quirks isolated to plugins
- Can validate design with community feedback before restoring all languages

---

## Restoration Strategy (Future)

When ready to restore multi-language support:

### Option 1: Full Restoration
```bash
git checkout pre-language-reduction
git cherry-pick <unified-api-commits>
# Resolve conflicts, update tests
```

### Option 2: Selective Restoration
```bash
# Add back Python first
git show pre-language-reduction:crates/languages/python/ > crates/languages/python/
# Update to new unified API
# Test
# Repeat for Go, Java, Swift, C#
```

### Option 3: Fresh Implementation
- Use new unified API as foundation
- Reimplement language plugins one at a time
- Cleaner than merging old code
- Opportunity to simplify based on lessons learned

---

## Changes Required

### Code Changes

1. **Remove language plugin crates**:
   ```bash
   rm -rf crates/languages/python
   rm -rf crates/languages/go
   rm -rf crates/languages/java
   rm -rf crates/languages/swift
   rm -rf crates/languages/csharp
   ```

2. **Update `crates/languages/Cargo.toml`**:
   - Remove python, go, java, swift, csharp from workspace members

3. **Update `crates/cb-plugins/src/manager.rs`**:
   - Remove non-TS/Rust language plugin registrations

4. **Update LSP adapter** (`crates/cb-plugins/src/adapters/lsp_adapter/`):
   - Keep only TS and Rust server configurations

5. **Update test fixtures**:
   - Remove `integration-tests/fixtures/python/`, `go/`, `java/`, etc.
   - Keep `typescript/` and `rust/`

6. **Update documentation**:
   - README.md language support section
   - API_REFERENCE.md language matrix
   - CLAUDE.md language list
   - Add restoration note

---

### Documentation Changes

**README.md**:
```diff
- Supported languages: TypeScript, Python, Go, Rust, Java, Swift, C#
+ Supported languages: TypeScript, Rust
+ (Python, Go, Java, Swift, C# temporarily removed for unified API refactoring)
+ (Multi-language support preserved in git tag `pre-language-reduction`)
```

**API_REFERENCE.md**:
```diff
# Language Support Matrix

- Current implementation supports 7 languages across LSP and AST providers.
+ Current implementation supports 2 languages (TypeScript, Rust).
+ Additional languages (Python, Go, Java, Swift, C#) will be restored after
+ unified API refactoring is complete.
```

**CLAUDE.md** (project instructions):
```diff
- Supported language servers (configurable):
-   - TypeScript: `typescript-language-server`
-   - Python: `pylsp`
-   - Go: `gopls`
-   - Rust: `rust-analyzer`
-   - Java: `jdtls` (Eclipse JDT Language Server)
+ Supported language servers:
+   - TypeScript: `typescript-language-server`
+   - Rust: `rust-analyzer`
+
+ Note: Python, Go, Java, Swift, C# support temporarily removed during
+ unified API refactoring. Multi-language support preserved in git tag
+ `pre-language-reduction` and will be restored after refactoring.
```

**CONTRIBUTING.md** (contributor guide):
```diff
- Adding new language plugins (TypeScript, Python, Go, Rust, Java, Swift, C#)
+ Adding new language plugins (TypeScript, Rust)
+ Note: Guide shows TypeScript + Rust patterns. Python, Go, Java, Swift, C#
+ plugins temporarily removed but preserved in `pre-language-reduction` tag.
```

**crates/languages/README.md** (language plugin documentation):
```diff
- This crate provides language-specific plugins for 7 languages.
+ This crate provides language-specific plugins for TypeScript and Rust.
+
+ Previously supported languages (Python, Go, Java, Swift, C#) have been
+ temporarily removed to accelerate unified API implementation. All code
+ preserved in git tag `pre-language-reduction` for future restoration.
```

**docs/architecture/ARCHITECTURE.md**:
```diff
- Language support: 7 languages (TypeScript, Python, Go, Rust, Java, Swift, C#)
+ Language support: 2 languages (TypeScript, Rust)
+ Multi-language support (Python, Go, Java, Swift, C#) will be restored
+ after unified API refactoring is complete.
```

**integration-tests/TESTING_GUIDE.md**:
```diff
- Cross-language test matrix: 7 languages × N operations
+ Cross-language test matrix: 2 languages × N operations
+ (TypeScript + Rust only during unified API refactoring)
```

---

## Success Criteria

- [ ] Git tag `pre-language-reduction` created ✅
- [ ] All Python, Go, Java, Swift, C# code removed
- [ ] Only TypeScript + Rust plugins remain
- [ ] LSP configurations updated (2 servers only)
- [ ] Test fixtures updated (TS + Rust only)
- [ ] Documentation updated to reflect TS + Rust support
- [ ] All tests pass with TS + Rust only
- [ ] CI/CD updated (remove multi-language test matrix)
- [ ] Restoration documentation written
- [ ] Binary size reduced (no unused language plugins)

---

## Restoration Checklist (Future)

When ready to restore multi-language support:

- [ ] Checkout tag: `git checkout pre-language-reduction`
- [ ] Cherry-pick unified API commits
- [ ] Resolve conflicts in language plugins
- [ ] Update Python plugin to new API
- [ ] Update Go plugin to new API
- [ ] Update Java plugin to new API
- [ ] Update Swift plugin to new API
- [ ] Update C# plugin to new API
- [ ] Restore test fixtures for each language
- [ ] Update documentation (remove "temporary" notes)
- [ ] Verify all 7 languages work with unified API
- [ ] Update language support matrix

---

## Timeline

**No timeline** - we're the only users, move at our own pace.

**Suggested order**:
1. Create tag ✅
2. Remove non-TS/Rust code (this proposal)
3. Implement unified refactoring + analysis APIs (existing proposals)
4. Validate APIs work well with TS + Rust
5. Restore languages one-by-one after API is stable

---

## Conclusion

Temporarily reducing language support to TypeScript + Rust accelerates unified API implementation by 71% while preserving all multi-language code in a git tag for easy restoration.

**Recommendation**: Approve and proceed with language reduction. Restore Python, Go, Java, Swift, C# after unified API is stable and validated with TypeScript + Rust.

---

## Appendix: Tag Restoration Commands

### View tag contents
```bash
git tag -l pre-language-reduction -n20
```

### Restore full multi-language codebase
```bash
git checkout pre-language-reduction
```

### Restore single language (example: Python)
```bash
# Extract Python plugin from tag
git show pre-language-reduction:crates/languages/python/src/lib.rs > crates/languages/python/src/lib.rs

# Extract tests
git show pre-language-reduction:integration-tests/fixtures/python/sample.py > integration-tests/fixtures/python/sample.py
```

### Cherry-pick unified API changes onto multi-language base
```bash
git checkout pre-language-reduction
git cherry-pick <commit-range-of-unified-api>
```

### Compare current state with tagged state
```bash
git diff pre-language-reduction..HEAD
```
