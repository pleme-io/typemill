# Test Reliability Improvements

## Problem

The test suite has several reliability and maintenance issues identified by audit:

1. **Remaining hardcoded sleeps** in feature-gated performance tests cause flakiness
2. **550+ unwrap/expect calls** in E2E tests create panic risks and poor error messages
3. **Debug print statements** in 58 test files add noise to test output
4. **Magic numbers** without documentation make tests hard to maintain
5. **Repetitive test patterns** in scope tests could be consolidated

## Solution(s)

### A. Replace remaining performance test sleeps with polling
Use existing `wait_for_lsp_ready()` pattern in heavy-tests feature-gated tests.

### B. Reduce unwrap/expect usage in test helpers
Convert critical test infrastructure to return `Result` types with meaningful errors.

### C. Replace debug prints with structured logging or remove
Use `tracing::debug!` for necessary debugging, remove noise.

### D. Extract magic numbers to named constants
Document timeout values, port numbers, and thresholds.

### E. Parameterize repetitive scope tests
Use table-driven tests instead of duplicate test functions.

## Checklists

### Performance Test Sleep Replacement
- [x] `e2e_performance.rs:68` - Replace 2000ms with `wait_for_lsp_ready`
- [x] `e2e_performance.rs:185` - Replace 5000ms with `wait_for_lsp_ready`
- [x] `e2e_performance.rs:812` - Replace 8000ms with `wait_for_lsp_ready`

### Test Helper Error Handling
- [x] Audit `mill-test-support/src/harness/test_helpers.rs` for unwrap calls (clean - no issues)
- [x] Improve `TestWorkspace` error messages with file paths and context
- [x] Add context to `TestClient` setup errors (already has good messages)
- [ ] Review `fixtures.rs` for panic-prone patterns

### Debug Output Cleanup
- [ ] Audit test files for `println!`/`eprintln!` statements
- [ ] Replace necessary debug output with `tracing::debug!`
- [ ] Remove redundant print statements
- [ ] Keep only CI-useful output (test summaries, failure info)

### Magic Number Documentation
- [x] Extract LSP timeout values to constants in `client.rs`
- [ ] Document port numbers in config tests
- [ ] Add comments explaining timing thresholds

### Test Consolidation
- [ ] Refactor `rename_scope_test.rs` to use parameterized tests
- [ ] Consolidate similar edge case tests where appropriate

## Success Criteria

- [x] All performance tests use polling instead of fixed sleeps
- [ ] Test failures produce actionable error messages (not just "unwrap failed")
- [ ] `cargo test 2>&1 | grep -c println` returns < 10 (minimal debug noise)
- [x] All timing constants have associated documentation
- [ ] No test functions with near-identical assertion patterns

## Benefits

- **Faster CI**: Polling completes in 200-500ms vs fixed 2-8s sleeps
- **Better debugging**: Error messages show what failed, not just that it failed
- **Cleaner output**: Test logs show meaningful information only
- **Maintainability**: Constants are documented and centralized
- **Less code**: Parameterized tests reduce duplication
