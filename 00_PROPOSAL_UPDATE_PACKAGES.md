# Proposal: Dependency Update Strategy

**Status:** Draft
**Created:** 2025-10-04
**Goal:** Systematic approach to updating outdated dependencies with minimal risk

---

## Current State

Multiple outdated dependencies identified across the workspace, ranging from safe patch updates to major version upgrades requiring code changes.

---

## Phase 1: Immediate Updates (Low Risk)

**Safe patch/minor releases with no breaking changes:**

```bash
cargo update quote axum axum-extra swc_ecma_parser console dialoguer indicatif
cargo test --workspace
cargo build --release
```

**Expected Impact:** Zero breaking changes, bug fixes only
**Estimated Time:** 30 minutes
**Risk Level:** ✅ Low

---

## Phase 2: Major Updates (Requires Testing & Code Changes)

### Priority 1: TOML Libraries (High Impact)

**Packages:**
- `toml` 0.8.23 → 0.9.7
- `toml_edit` 0.22.27 → 0.23.6

**Required Actions:**
1. Review changelog: https://github.com/toml-rs/toml/releases
2. Update configuration parsing code in:
   - `cb-core` (config loading)
   - `cb-handlers` (TOML manipulation)
   - `cb-server` (config validation)
   - `cb-lang-rust` (Cargo.toml editing)
3. Test all config file operations
4. Verify Cargo.toml editing functionality

**Estimated Effort:** 2-4 hours
**Risk Level:** ⚠️ Medium-High (critical for config system)

---

### Priority 2: Tree-sitter Ecosystem (Medium Impact)

**Packages:**
- `tree-sitter` 0.20.10 → 0.25.10
- `tree-sitter-go` (update to match)
- `tree-sitter-java` (update to match)

**Required Actions:**
1. Update tree-sitter core library
2. Update ALL language bindings together (coordinated release)
3. Review AST parsing code in `cb-ast`
4. Test parsing for Go and Java files
5. Verify AST-based refactoring tools still work

**Estimated Effort:** 3-5 hours
**Risk Level:** ⚠️ Medium (affects parsing accuracy)

---

### Priority 3: WebSocket Stack (Medium Impact)

**Package:**
- `tokio-tungstenite` 0.21.0 → 0.28.0

**Required Actions:**
1. Review changelog: https://github.com/snapview/tokio-tungstenite/releases
2. Update WebSocket connection handling in:
   - `cb-client` (WebSocket client)
   - `integration-tests` (WebSocket test harness)
3. Test real-time communication
4. Verify authentication flow still works

**Estimated Effort:** 2-3 hours
**Risk Level:** ⚠️ Medium (affects client communication)

---

### Priority 4: Lower Priority Updates

| Package           | Current → Target | Effort | Risk | Notes                                        |
|-------------------|------------------|--------|------|----------------------------------------------|
| `petgraph`        | TBD              | Medium | Low  | Used in AST analysis - test graph operations |
| `rustpython-parser` | TBD            | Low    | Low  | Python parsing - isolated to cb-ast          |
| `dirs`            | TBD              | Low    | Low  | Directory utilities - straightforward API    |
| `criterion`       | TBD              | Low    | Low  | Dev dependency - only affects benchmarks     |
| `simd-json`       | TBD              | Low    | Low  | Dev dependency - only affects benchmarks     |

**Estimated Effort:** 1-2 hours each
**Risk Level:** ✅ Low

---

## Recommended Schedule

### Immediate Action (Week 1)

```bash
# Safe updates - execute now
cargo update quote axum axum-extra swc_ecma_parser console dialoguer indicatif
cargo test --workspace
cargo build --release
```

### Scheduled Major Updates

| Week | Priority | Package(s) | Rationale |
|------|----------|------------|-----------|
| 2    | Priority 1 | TOML libraries | Critical for config system |
| 3    | Priority 2 | Tree-sitter ecosystem | Coordinated update required |
| 4    | Priority 3 | WebSocket stack | Affects client communication |
| 5    | Priority 4 | Remaining updates | Lower priority, isolated impact |

---

## Total Estimated Effort

- **Immediate updates:** 30 minutes
- **Major updates:** 10-15 hours total
- **Testing & validation:** 3-5 hours

**Total:** ~15-20 hours

---

## Risk Mitigation

1. **Test extensively** after each priority update
2. **Commit separately** for each priority (easy rollback)
3. **Update staging environment first** before production
4. **Monitor for regressions** in CI/CD pipeline
5. **Document breaking changes** in commit messages

---

## Success Criteria

- [ ] All tests pass after updates
- [ ] No performance regressions
- [ ] CI/CD pipeline remains green
- [ ] No breaking changes to MCP API
- [ ] Config parsing still works correctly
- [ ] WebSocket connections remain stable
