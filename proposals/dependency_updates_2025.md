# Dependency Update Proposals - 2025

## Summary

This document outlines proposed major version updates for external dependencies in the TypeMill project. Minor and patch updates have already been applied automatically.

**Status:**
- ✅ **Completed:** 17 minor/patch updates applied (see Applied Updates section)
- 📋 **Pending:** 3 version consolidations + 7 major version updates requiring code changes

---

## Applied Updates (Completed)

The following minor/patch updates were automatically applied via `cargo update`:

```text
icu_collections: 2.0.0 → 2.1.1
icu_locale_core: 2.0.0 → 2.1.1
icu_normalizer: 2.0.0 → 2.1.1
icu_normalizer_data: 2.0.0 → 2.1.1
icu_properties: 2.0.1 → 2.1.1
icu_properties_data: 2.0.1 → 2.1.1
icu_provider: 2.0.0 → 2.1.1
litemap: 0.8.0 → 0.8.1
potential_utf: 0.1.3 → 0.1.4
rustls-webpki: 0.103.7 → 0.103.8
tinystr: 0.8.1 → 0.8.2
writeable: 0.6.1 → 0.6.2
yoke: 0.8.0 → 0.8.1
yoke-derive: 0.8.0 → 0.8.1
zerotrie: 0.2.2 → 0.2.3
zerovec: 0.11.4 → 0.11.5
zerovec-derive: 0.11.1 → 0.11.2
```text
**Verification:** ✅ All updates verified with `cargo check --workspace` (successful build)

---

## Priority 1: Version Consolidation (Low Effort)

These dependencies have multiple version specifications in the codebase. Consolidating to the latest version is recommended.

### 1.1 toml: Consolidate to 0.9

**Current:** `^0.8` and `^0.9` (mixed usage)
**Target:** `^0.9.8` (latest)
**Effort:** Low
**Breaking Changes:** Minimal API changes between 0.8 and 0.9

**Action Items:**
1. Update all `Cargo.toml` files using `toml = "^0.8"` to `toml = "^0.9"`
2. Search for `toml::` usage in code and verify compatibility
3. Run full test suite to verify no breaking changes

**Files to Update:**
```bash
grep -r "toml.*0\.8" --include="Cargo.toml" .
```text
### 1.2 thiserror: Consolidate to 2.0

**Current:** `^1.0` and `^2.0` (mixed usage)
**Target:** `^2.0.17` (latest)
**Effort:** Low
**Breaking Changes:** thiserror 2.0 has minimal breaking changes (primarily related to error trait implementations)

**Action Items:**
1. Update all `Cargo.toml` files using `thiserror = "^1.0"` to `thiserror = "^2.0"`
2. Verify error handling code compiles without changes
3. Run error handling tests

**Migration Guide:** https://github.com/dtolnay/thiserror/releases/tag/2.0.0

### 1.3 which: Consolidate to 7.0

**Current:** `^6.0` and `^7.0` (mixed usage)
**Target:** `^7.0.3` (latest, before considering 8.0 upgrade)
**Effort:** Low
**Breaking Changes:** Minimal between 6.0 and 7.0

**Action Items:**
1. Update all `Cargo.toml` files using `which = "^6.0"` to `which = "^7.0"`
2. Test LSP server discovery functionality (main usage of `which`)
3. Verify mill CLI tool detection works correctly

---

## Priority 2: Major Version Updates (Code Changes Required)

### 2.1 petgraph: 0.6 → 0.8.3

**Current:** `0.6.x`
**Latest:** `0.8.3`
**Impact:** Moderate - Used in dependency graph analysis
**Effort:** Medium

**Breaking Changes:**
- API changes in graph construction and traversal
- Some trait bounds have changed
- Iterator APIs may have different signatures

**Action Items:**
1. Review petgraph 0.7 and 0.8 changelogs for breaking changes
2. Update `analysis/mill-analysis-graph/` and `analysis/mill-analysis-circular-deps/`
3. Update graph construction code in dependency analysis
4. Run analysis test suite (`cargo nextest run -p mill-analysis-graph -p mill-analysis-circular-deps`)
5. Verify dependency graph visualization still works

**Files to Update:**
- `analysis/mill-analysis-graph/Cargo.toml`
- `analysis/mill-analysis-circular-deps/Cargo.toml`
- `Cargo.toml` (workspace dependencies)

**References:**
- v0.7.0 release notes: https://github.com/petgraph/petgraph/releases/tag/petgraph-0.7.0
- v0.8.0 release notes: https://github.com/petgraph/petgraph/releases/tag/petgraph-0.8.0

### 2.2 dashmap: 5.5 → 7.0 (when stable)

**Current:** `5.5.x`
**Latest:** `7.0.0-rc2` (Release Candidate)
**Impact:** High - Used extensively for concurrent data structures
**Effort:** Medium-High
**Status:** ⚠️ WAIT FOR STABLE RELEASE

**Recommendation:** Monitor dashmap 7.0 release status. The RC version should not be used in production. Wait for stable 7.0.0 release.

**Breaking Changes (when stable):**
- API changes in concurrent map operations
- Possible changes to shard count and hashing strategy
- Iterator behavior may differ

**Action Items (when 7.0.0 stable is released):**
1. Review dashmap 7.0 migration guide and changelog
2. Update all concurrent map usage in:
   - LSP manager (server tracking)
   - AST cache
   - Import cache
   - Session management
3. Review lock-free algorithm changes
4. Run concurrent stress tests
5. Performance benchmarks to ensure no regression

**Risk Assessment:** High - Core infrastructure dependency

### 2.3 SWC Ecosystem Update (TypeScript/JavaScript Parsing)

**Impact:** High - Core functionality for TypeScript language plugin
**Effort:** High
**Risk:** Medium-High

The SWC (Speedy Web Compiler) crates need coordinated updates:

| Crate | Current | Latest | Versions Behind |
|-------|---------|--------|-----------------|
| swc_common | 14.x | 16.0.0 | 2 major versions |
| swc_ecma_ast | 15.x | 17.0.0 | 2 major versions |
| swc_ecma_codegen | 17.x | 19.0.0 | 2 major versions |
| swc_ecma_parser | 24.x | 26.0.0 | 2 major versions |
| swc_ecma_visit | 15.x | 17.0.0 | 2 major versions |

**Breaking Changes:**
- AST node structure changes (impacts all TypeScript parsing)
- Visitor pattern API changes
- Code generation API modifications
- Possible changes to parser options and configuration

**Action Items:**
1. **Research phase:**
   - Review all SWC changelog entries from current to latest versions
   - Identify specific breaking changes affecting our usage
   - Check for deprecation warnings in current code

2. **Update dependencies:**
   - Update all SWC crates together (must stay synchronized)
   - Update `crates/mill-lang-typescript/Cargo.toml`

3. **Code migration:**
   - Update TypeScript plugin AST handling (`crates/mill-lang-typescript/src/`)
   - Update import analysis for TypeScript
   - Update refactoring operations (extract, inline, rename)
   - Update code generation for TypeScript transformations

4. **Testing:**
   - Run TypeScript plugin tests: `cargo nextest run -p mill-lang-typescript`
   - Test all TypeScript refactoring operations
   - Test TypeScript import resolution
   - Run E2E tests with TypeScript fixtures
   - Test against real-world TypeScript projects

5. **Validation:**
   - Verify AST parsing correctness
   - Check code generation produces valid TypeScript
   - Ensure refactoring preserves semantics

**Files Affected:**
- `crates/mill-lang-typescript/Cargo.toml`
- `crates/mill-lang-typescript/src/*.rs` (all files)
- TypeScript test fixtures in `tests/e2e/test-fixtures/typescript/`

**Estimated Effort:** 2-3 days of development + testing

### 2.4 which: 7.0 → 8.0

**Current:** `7.0.3` (after consolidation)
**Latest:** `8.0.0`
**Impact:** Low - Used only for LSP executable discovery
**Effort:** Low

**Prerequisite:** Complete 1.3 (consolidate to 7.0) first

**Breaking Changes:**
- API signature changes in executable search functions
- Possible changes to path resolution behavior

**Action Items:**
1. Review which 8.0 changelog for breaking changes
2. Update workspace dependency to `which = "^8.0"`
3. Update LSP discovery code in `crates/mill-lsp-manager/`
4. Test `mill setup` command (LSP detection)
5. Test `mill install-lsp` command
6. Verify cross-platform behavior (Linux, macOS, Windows)

**Testing Focus:**
- LSP server discovery
- Path environment variable handling
- Error handling for missing executables

---

## Priority 3: Refactoring Opportunities

### 3.1 once_cell → std::sync::OnceLock Migration

**Current:** `once_cell = "1.21.3"`
**Target:** Use `std::sync::OnceLock` (stable since Rust 1.70)
**Rust Version:** We're on 1.90, well above minimum
**Impact:** Code modernization, reduce dependencies
**Effort:** Medium

**Benefits:**
- Remove external dependency
- Use stdlib (better maintenance)
- Same performance characteristics
- Simpler dependency tree

**Breaking Changes:**
- API is slightly different from once_cell
- Migration requires code changes

**Action Items:**
1. **Audit usage:**
   ```bash
   grep -r "once_cell::" --include="*.rs" .
   grep -r "OnceCell\|Lazy" --include="*.rs" .
   ```

2. **Migration mapping:**
   - `once_cell::sync::OnceCell` → `std::sync::OnceLock`
   - `once_cell::sync::Lazy` → `std::sync::LazyLock` (Rust 1.80+)
   - Update initialization patterns

3. **Update code:**
   - Replace imports
   - Update initialization syntax
   - Update access patterns

4. **Testing:**
   - Full test suite
   - Verify thread safety
   - Check for initialization race conditions

5. **Cleanup:**
   - Remove `once_cell` from all `Cargo.toml` files
   - Update documentation

**Estimated Effort:** 1 day

**References:**
- LazyLock stabilization: https://releases.rs/docs/1.80.0/#lazycell-and-lazylock
- OnceLock docs: https://doc.rust-lang.org/std/sync/struct.OnceLock.html

---

## Implementation Roadmap

### Phase 1: Version Consolidation (Week 1)
**Effort:** 1-2 days
**Risk:** Low

1. ✅ toml: 0.8 → 0.9 consolidation
2. ✅ thiserror: 1.0 → 2.0 consolidation
3. ✅ which: 6.0 → 7.0 consolidation

**Testing:** Run full test suite after each consolidation

### Phase 2: Low-Risk Major Updates (Week 2)
**Effort:** 2-3 days
**Risk:** Low-Medium

1. ✅ petgraph: 0.6 → 0.8.3
2. ✅ which: 7.0 → 8.0
3. ✅ once_cell → std::sync::OnceLock migration

**Testing:** Run analysis tests, LSP tests, full suite

### Phase 3: High-Risk Updates (Week 3-4)
**Effort:** 5-7 days
**Risk:** Medium-High

1. ✅ SWC ecosystem update (all crates together)
2. ⏸️ dashmap: 5.5 → 7.0 (WAIT FOR STABLE)

**Testing:** Extensive TypeScript testing, performance benchmarks

### Phase 4: Monitoring
**Ongoing**

- Monitor dashmap 7.0 stable release
- Check for security advisories on all dependencies
- Quarterly dependency review

---

## Testing Strategy

### For Each Update:

1. **Unit Tests:**
   ```bash
   cargo nextest run --workspace
   ```

2. **LSP Integration Tests:**
   ```bash
   cargo nextest run --workspace --features lsp-tests
   ```

3. **E2E Tests:**
   ```bash
   cargo nextest run -p e2e --all-features
   ```

4. **Performance Benchmarks:**
   ```bash
   cargo nextest run --workspace --features heavy-tests
   ```

5. **Manual Testing:**
   - Test `mill setup` command
   - Test refactoring operations (rename, extract, inline)
   - Test analysis tools
   - Test WebSocket server

### Regression Prevention:

- Run full test suite before and after each phase
- Performance benchmarks to detect regressions
- Manual smoke tests of critical paths
- Review deprecation warnings

---

## Rollback Plan

Each phase should be committed separately to allow easy rollback:

```bash
# If issues occur, rollback to previous commit
git revert <commit-hash>
cargo update
cargo build --workspace
cargo nextest run --workspace
```text
**Critical:** Always commit working state before major updates.

---

## Estimated Timeline

| Phase | Duration | Risk Level | Dependencies |
|-------|----------|------------|--------------|
| Phase 1: Consolidation | 1-2 days | Low | None |
| Phase 2: Low-Risk Updates | 2-3 days | Low-Medium | Phase 1 complete |
| Phase 3: SWC Update | 3-5 days | Medium-High | Phase 1-2 complete |
| Phase 3: dashmap | TBD | High | Wait for stable release |
| **Total (excluding dashmap)** | **6-10 days** | | |

---

## Approval Required

Please review and approve each phase before implementation:

- [ ] **Phase 1:** Version Consolidation (toml, thiserror, which to 7.0)
- [ ] **Phase 2:** Low-Risk Updates (petgraph, which to 8.0, once_cell migration)
- [ ] **Phase 3:** SWC Ecosystem Update (TypeScript parsing)
- [ ] **Phase 4:** dashmap update (when 7.0.0 stable is released)

---

## Notes

- All updates preserve backward compatibility at the API level where possible
- Breaking changes are isolated to internal implementation details
- No changes to MCP tools API or CLI interface required
- Performance impact expected to be neutral or positive
- Security: All updates include latest security patches

---

## References

- Rust Edition Guide: https://doc.rust-lang.org/edition-guide/
- Cargo Book (SemVer): https://doc.rust-lang.org/cargo/reference/semver.html
- crates.io: https://crates.io/