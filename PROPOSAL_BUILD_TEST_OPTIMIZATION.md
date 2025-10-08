# Build & Test Performance Optimization Proposal

**Date:** 2025-10-08
**Author:** Code Analysis Team
**Status:** 📋 PROPOSAL - Awaiting Approval
**Impact:** High - Expected 2-3x faster builds, 10-15x faster test iteration

---

## Executive Summary

This proposal addresses build time and test execution performance issues in the CodeBuddy workspace. Through targeted optimizations to build configuration, dependency management, and test parallelization, we can achieve:

- **Build time:** 6s → **4-5s** (incremental)
- **Fast test suite:** 170s → **8-12s** (mock tests only)
- **Full test suite:** 170s → **60-80s** (with parallelization)
- **Disk usage:** 46GB → **~30GB** (with cleanup)

---

## Current State Analysis

### Build Performance
- **18 workspace crates** with deep dependency trees
- **46GB target directory** (excessive build artifacts)
- **~6 second incremental builds** (already decent, but improvable)
- Already using `sccache` and `mold` linker ✓

### Test Performance
- **Total test time:** ~170+ seconds for full suite
- **Slowest test:** `e2e_analysis_features.rs` (88 seconds)
- **Integration tests:** 19 test files with sequential execution
- Heavy use of real LSP servers in tests (adds overhead)
- Sequential loops through test fixtures (major bottleneck)

### Key Bottlenecks Identified

1. **Build:** Single-threaded codegen units (default: 16)
2. **Build:** Legacy crates.io registry protocol
3. **Tests:** Sequential execution of parallelizable test cases
4. **Tests:** Heavy dependencies (criterion, proptest) compiled unnecessarily
5. **Tests:** All tests run together (no fast/slow categorization)

---

## Proposed Changes

### Phase 1: Build Optimizations (Immediate - 30 min)

#### 1.1 Increase Build Parallelism

**File:** `.cargo/config.toml`

**Change:**
```toml
[profile.dev]
incremental = true
codegen-units = 256  # ADD: More parallelism for faster dev builds (default: 16)
```

**Rationale:** Increases parallel compilation units from 16 to 256, allowing better CPU utilization on multi-core machines.

**Trade-offs:**
- ✅ Faster incremental builds (10-20% improvement)
- ⚠️ Slightly larger binary sizes (acceptable for dev builds)

---

#### 1.2 Enable Sparse Registry Protocol

**File:** `.cargo/config.toml`

**Change:**
```toml
# Add at end of file
[registries.crates-io]
protocol = "sparse"
```

**Rationale:** Uses modern sparse index protocol instead of legacy git-based index. Significantly faster dependency resolution.

**Impact:** Faster `cargo update` and initial builds, especially in CI environments.

---

### Phase 2: Test Performance Optimization (High Impact - 2-3 hours)

#### 2.1 Feature-Gated Test Categories

**File:** `integration-tests/Cargo.toml`

**Changes:**

1. **Make heavy dependencies optional:**
```toml
# Before:
criterion = "0.5"
proptest = "1.0"
serial_test = "3.0"

# After:
criterion = { version = "0.5", optional = true }
proptest = { version = "1.0", optional = true }
serial_test = { version = "3.0", optional = true }
```

2. **Add feature flags:**
```toml
[features]
default = ["fast-tests"]

# Fast tests run on every `cargo test` (mock-based, no LSP)
fast-tests = []

# Tests requiring real LSP servers installed
lsp-tests = []

# Slow end-to-end workflow tests
e2e-tests = ["serial_test"]

# Performance benchmarks and property-based testing
heavy-tests = ["criterion", "proptest"]
```

**Rationale:**
- Developers can run fast tests by default (`cargo test` = 8-12s)
- Full validation available when needed (`cargo test --all-features`)
- Reduces compilation of heavy dependencies during normal development

**Usage:**
```bash
# Fast iteration (DEFAULT)
cargo test --workspace

# With LSP tests (requires servers installed)
cargo test --workspace --features lsp-tests -- --include-ignored

# Full suite including heavy tests
cargo test --workspace --all-features -- --include-ignored
```

---

#### 2.2 Parallelize Test Fixtures

**File:** `integration-tests/tests/lsp_features.rs`

**Problem:** Tests currently loop sequentially through fixtures:
```rust
// CURRENT (SLOW - Sequential):
for (idx, case) in GO_TO_DEFINITION_TESTS.iter().enumerate() {
    run_go_to_definition_test(case, false).await;
}
// Total time: N × test_duration
```

**Solution:** Run test cases concurrently using `join_all`:
```rust
// PROPOSED (FAST - Concurrent):
use futures::future::join_all;

let futures: Vec<_> = GO_TO_DEFINITION_TESTS
    .iter()
    .map(|case| run_go_to_definition_test(case, false))
    .collect();

join_all(futures).await;
// Total time: ~max(test_duration) on multi-core
```

**Why This Pattern (Not tokio::spawn)?**

✅ **Correct Pattern:**
- No `Clone` trait requirement
- Panics propagate correctly with full assertion messages
- No task spawning overhead
- Borrows work naturally

❌ **Incorrect Pattern (DO NOT USE):**
```rust
// This was in the original proposal but is WRONG:
let handles: Vec<_> = tests
    .iter()
    .map(|case| tokio::spawn(run_test(case.clone(), false)))  // ❌ Requires Clone
    .collect();
join_all(handles).await;  // ❌ Swallows errors silently
```

**Impact:**
- **Expected speedup:** 3-5x on 4+ core machines
- `e2e_analysis_features.rs`: 88s → **15-30s**
- Total mock tests: 170s → **8-12s**

**Tests to parallelize:**
- `test_go_to_definition_mock/real`
- `test_find_references_mock/real`
- `test_hover_mock/real`
- `test_document_symbols_mock/real`
- `test_workspace_symbols_mock/real`
- `test_completion_mock/real`
- `test_rename_mock/real`
- `test_lsp_compliance_suite` (already ignored)

**Gate real LSP tests:**
```rust
#[tokio::test]
#[ignore]
#[cfg(feature = "lsp-tests")]  // ADD THIS
async fn test_go_to_definition_real() {
    // ... parallel execution
}
```

---

### Phase 3: Tooling & Documentation (1 hour)

#### 3.1 Add `cargo-nextest` Support (Recommended)

**File:** `Makefile`

**Changes:**
```makefile
# Add to setup target
setup:
	@echo "📦 Installing build optimization tools..."
	@cargo install sccache 2>/dev/null || echo "✓ sccache already installed"
	@cargo install cargo-watch 2>/dev/null || echo "✓ cargo-watch already installed"
	@cargo install cargo-nextest 2>/dev/null || echo "✓ cargo-nextest already installed"
	@./scripts/setup-dev-tools.sh
	@echo "✅ Setup complete!"

# Add new test targets
test-fast:
	cargo nextest run --workspace

test-full:
	cargo nextest run --workspace --all-features -- --include-ignored

test-lsp:
	cargo nextest run --workspace --features lsp-tests -- --include-ignored
```

**Rationale:** `cargo-nextest` provides:
- Better parallelization (per-test instead of per-file)
- Cleaner output with progress bars
- 20-40% faster than `cargo test`
- Industry best practice for Rust test execution

**Impact:** This is highly recommended, not optional. The 20-40% speedup compounds with our other optimizations.

---

#### 3.2 Add Target Directory Cleanup

**File:** `Makefile`

**Changes:**
```makefile
# Add new cleanup target
clean-cache:
	@echo "🧹 Cleaning build cache..."
	cargo clean
	@echo "💡 Tip: Install cargo-sweep for smarter cleanup: cargo install cargo-sweep"

# Add to help menu
help:
	# ... existing help text ...
	@echo "  make clean-cache - Remove all build artifacts (frees ~30-40GB)"
```

**Rationale:** The `target/` directory grows to 46GB over time. Providing an easy cleanup command helps developers manage disk space.

**Alternative (advanced):**
```bash
# Install cargo-sweep for smarter cleanup
cargo install cargo-sweep

# Add to Makefile
clean-old:
	cargo-sweep --time 30  # Remove artifacts older than 30 days
```

---

#### 3.3 Update Documentation

**File:** `CLAUDE.md` (Development Commands section)

**Add:**
```markdown
## Testing Workflow

The test suite is organized into categories for fast iteration:

```bash
# Fast tests only (mock-based, ~10s)
cargo test --workspace
cargo nextest run --workspace  # Recommended (faster)

# With LSP server tests (~60s, requires LSP servers installed)
cargo test --workspace --features lsp-tests -- --include-ignored

# Full suite with heavy tests (~80s)
cargo test --workspace --all-features -- --include-ignored

# Performance benchmarks
cargo test --workspace --features heavy-tests
```

**Test Categories:**
- `fast-tests` (default): Mock-based unit and integration tests
- `lsp-tests`: Tests requiring real LSP servers (TypeScript, Python, Rust)
- `e2e-tests`: End-to-end workflow tests
- `heavy-tests`: Performance benchmarks and property-based tests
```

---

## Implementation Plan

### Step 1: Immediate Build Optimizations (15 min)
1. Edit `.cargo/config.toml`:
   - Add `codegen-units = 256` to `[profile.dev]`
   - Add `[registries.crates-io]` section with `protocol = "sparse"`
2. Test: Run `cargo check --workspace` and verify faster builds
3. Commit: `chore: optimize build performance with parallel codegen and sparse registry`

### Step 2: Test Dependency Optimization (30 min)
1. Edit `integration-tests/Cargo.toml`:
   - Make `criterion`, `proptest`, `serial_test` optional
   - Add `[features]` section with test categories
2. Test: Run `cargo test --workspace` (should be faster to compile)
3. Commit: `refactor: add feature flags for test categorization`

### Step 3: Test Parallelization (1-2 hours)
1. Edit `integration-tests/tests/lsp_features.rs`:
   - Add `use futures::future::join_all;` import
   - Convert all sequential `for` loops to parallel `join_all` pattern
   - Add `#[cfg(feature = "lsp-tests")]` to real LSP tests
2. Test: Run `cargo test --workspace` and verify faster execution
3. Verify: Check that all tests still pass and panics are visible
4. Commit: `perf: parallelize LSP feature test fixtures for 3-5x speedup`

### Step 4: Tooling & Documentation (45 min)
1. Install `cargo-nextest`: `cargo install cargo-nextest`
2. Update `Makefile`:
   - Add `cargo-nextest` targets (test-fast, test-full, test-lsp)
   - Add `clean-cache` target for disk cleanup
3. Update `CLAUDE.md` with new testing workflow
4. Test all new make targets
5. Commit: `chore: add nextest support and cleanup tooling`

---

## Expected Results

### Build Performance
| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Incremental build | 6s | 4-5s | 17-33% faster |
| Dependency updates | Variable | Faster | Sparse protocol |
| `cargo check` | 5.8s | 4-5s | 15-30% faster |

### Test Performance
| Test Suite | Before | After | Improvement |
|------------|--------|-------|-------------|
| Fast tests (default) | 170s | 8-12s | **14x faster** |
| LSP tests | 170s | 60-80s | 2-3x faster |
| Full suite | 170s | 60-90s | 2-3x faster |
| Mock fixture tests | 88s | 15-30s | **3-5x faster** |

### Developer Experience
| Workflow | Before | After |
|----------|--------|-------|
| Quick iteration | Run all tests (170s) | Fast tests only (10s) |
| Pre-commit check | 170s | 10s (fast) or 60s (full) |
| CI pipeline | 170s | 60-80s |

### Disk Usage
| Item | Before | After (with cleanup) |
|------|--------|---------------------|
| `target/` | 46GB | ~30GB |

---

## Risk Assessment

### Low Risk ✅
- `.cargo/config.toml` changes (build-only, easily reversible)
- Feature flags (backward compatible, default behavior unchanged for users)

### Medium Risk ⚠️
- Test parallelization (requires careful implementation)
  - **Mitigation:** Use correct `join_all` pattern (not `tokio::spawn`)
  - **Mitigation:** Verify all test fixtures are independent
  - **Mitigation:** Check for race conditions in temp directories (already handled by `TestWorkspace::new()`)

### Known Limitations
- Parallelization speedup depends on CPU cores (best on 4+ cores)
- LSP tests still require external server installation
- Some tests may still need `serial_test` for true sequential execution

---

## Alternatives Considered

### 1. LSP Connection Pooling
**Considered:** Shared LSP server instances across tests
**Rejected for now:** High implementation complexity, save for Phase 2
**Future work:** Could add another 30-50% improvement to LSP tests

### 2. Reduce Test File Sizes
**Considered:** Use smaller fixture files in performance tests
**Rejected:** Not a major bottleneck compared to parallelization
**Future work:** Optimize if specific tests remain slow

### 3. Workspace Consolidation
**Considered:** Merge some of the 18 crates to reduce build graph
**Rejected:** Would break modular architecture
**Future work:** Audit for truly unnecessary crate splits

---

## Success Criteria

✅ **Must Have:**
1. `cargo test --workspace` completes in < 15 seconds (fast tests)
2. All existing tests pass with identical behavior
3. Test panics/failures display clear error messages
4. Build time improves by at least 10%

✅ **Should Have:**
5. Full test suite completes in < 90 seconds
6. Documentation updated with new workflow
7. `cargo-nextest` integrated into Makefile
8. `make clean-cache` command available for disk cleanup

✅ **Nice to Have:**
9. CI pipeline updated to use fast tests by default
10. Pre-commit hook uses fast tests
11. Target directory cleanup automation with cargo-sweep

---

## Rollback Plan

If issues arise:

1. **Revert `.cargo/config.toml`:**
   ```bash
   git checkout HEAD -- .cargo/config.toml
   ```

2. **Revert test parallelization:**
   ```bash
   git checkout HEAD -- integration-tests/tests/lsp_features.rs
   ```

3. **Revert feature flags (if needed):**
   ```bash
   git checkout HEAD -- integration-tests/Cargo.toml
   ```

All changes are isolated and can be reverted independently.

---

## Approval Required

This proposal requires approval before implementation. Key decision points:

1. ✅ **Approve build optimizations** (`.cargo/config.toml`)
2. ✅ **Approve feature-gated tests** (`integration-tests/Cargo.toml`)
3. ✅ **Approve parallelization strategy** (using `join_all`, not `tokio::spawn`)
4. ✅ **Recommended:** Include `cargo-nextest` setup (20-40% additional speedup)
5. 🤔 **Optional:** Update CI/CD workflows?
6. 🤔 **Optional:** Add `cargo-sweep` for advanced cache management?

---

## Next Steps

Once approved:

1. Create feature branch: `git checkout -b perf/build-test-optimization`
2. Implement changes in order (Step 1 → Step 4)
3. Test each change independently
4. Submit PR with before/after benchmarks
5. Update this proposal with actual results

---

## References

- [Cargo Build Configuration](https://doc.rust-lang.org/cargo/reference/config.html)
- [Sparse Registry RFC](https://rust-lang.github.io/rfcs/2789-sparse-index.html)
- [cargo-nextest Documentation](https://nexte.st/)
- [Tokio Runtime Parallelism](https://tokio.rs/tokio/topics/bridging)

---

**End of Proposal**
