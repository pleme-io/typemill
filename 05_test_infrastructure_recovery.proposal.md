# Test Infrastructure Recovery

**Status:** Proposed

## Problem

The test infrastructure has degraded to the point of being non-functional:

1. **Zombie Process Accumulation**: 58+ defunct cargo/nextest processes accumulating over multiple days
2. **Complete Test Timeout**: All test runs timeout, including basic unit tests
3. **Environment Corruption**: Test environment is in an unstable state preventing any validation
4. **Blocked Development**: Cannot verify code changes, merge work, or validate fixes

The root cause appears to be a combination of:
- Process cleanup failures leaving zombie processes
- Resource exhaustion from accumulated defunct processes
- Potential deadlocks or infinite loops in test execution
- Corrupted test state from previous failed runs

**Critical Impact**: Development is completely blocked. Cannot validate the suggestion system integration or any other work until test infrastructure is restored.

## Solution

### Phase 1: Environment Recovery (Immediate)

**Goal:** Restore test execution capability

1. **Clean zombie processes**
   - Kill all defunct cargo/nextest processes
   - Clear any lingering test artifacts
   - Reset process table

2. **Identify deadlock/hang source**
   - Run minimal test subset with strict timeout
   - Use process monitoring to identify hanging tests
   - Review recent changes for synchronization issues

3. **Isolate working tests**
   - Identify which test suites can run
   - Create test subset that completes successfully
   - Use as baseline for incremental recovery

### Phase 2: Test Infrastructure Hardening

**Goal:** Prevent recurrence and improve reliability

1. **Add process cleanup safeguards**
   - Implement timeout handlers for all async operations
   - Add explicit process cleanup in test teardown
   - Use process groups for reliable cleanup

2. **Improve test isolation**
   - Ensure tests don't share state
   - Add resource limits per test
   - Implement proper test timeouts

3. **Add monitoring and diagnostics**
   - Add test execution metrics
   - Monitor for zombie process creation
   - Detect and report hung tests early

### Phase 3: Test Suite Validation

**Goal:** Verify all tests work reliably

1. **Progressive validation**
   - Run tests by crate in isolation
   - Identify any remaining problematic tests
   - Fix or quarantine unstable tests

2. **Performance profiling**
   - Identify slow tests
   - Optimize or increase timeouts where appropriate
   - Document expected test durations

3. **Documentation**
   - Document test categories and run times
   - Add troubleshooting guide for test failures
   - Create runbook for test infrastructure issues

## Checklists

### Phase 1: Environment Recovery

- [ ] Kill all zombie cargo/nextest processes
- [ ] Clear target directory test artifacts
- [ ] Reset any test state files
- [ ] Run single minimal test to verify basic execution
- [ ] Run cb-core tests in isolation
- [ ] Run cb-handlers tests in isolation
- [ ] Identify specific hanging test or test suite
- [ ] Review recent commits for synchronization bugs
- [ ] Check for infinite loops in new code
- [ ] Document zombie process creation pattern

### Phase 2: Test Infrastructure Hardening

- [ ] Add timeout handlers to async test utilities
- [ ] Implement proper Drop/cleanup for test harnesses
- [ ] Add process group management for spawned processes
- [ ] Review all tokio::spawn calls for cleanup
- [ ] Add resource limits to nextest configuration
- [ ] Implement per-test timeout enforcement
- [ ] Add test state isolation checks
- [ ] Create shared state detection tooling
- [ ] Add zombie process monitoring to CI
- [ ] Create alerting for hung test runs
- [ ] Add execution time tracking per test
- [ ] Implement progressive timeout warnings

### Phase 3: Test Suite Validation

- [ ] Run full test suite with monitoring
- [ ] Profile test execution times
- [ ] Identify tests >30s duration
- [ ] Review slow tests for optimization opportunities
- [ ] Set appropriate per-test timeouts
- [ ] Document fast-tests vs lsp-tests vs e2e-tests timing
- [ ] Run full suite 3x to verify stability
- [ ] Create test troubleshooting guide
- [ ] Document zombie process cleanup procedure
- [ ] Add test infrastructure section to CONTRIBUTING.md
- [ ] Create runbook for test failures
- [ ] Set up test health dashboard

## Success Criteria

1. **Test Execution Works**
   - `cargo nextest run --workspace` completes without timeout
   - All test categories (fast-tests, lsp-tests, e2e-tests) can run
   - Test runs complete in reasonable time (<5 minutes for fast-tests)

2. **No Process Leaks**
   - No zombie processes accumulate during test runs
   - Process table stays clean after test completion
   - Resource cleanup verified

3. **Stable and Reliable**
   - Test suite can run 10x consecutively without failure
   - No environment-dependent failures
   - Deterministic results across runs

4. **Well Documented**
   - Test infrastructure documented in CONTRIBUTING.md
   - Troubleshooting guide available
   - Runbook for common issues

## Benefits

1. **Unblocked Development**
   - Can validate code changes
   - Can merge completed work
   - Can proceed with new features

2. **Improved Reliability**
   - Prevents future test infrastructure degradation
   - Catches problems early
   - Reduces debugging time

3. **Better Developer Experience**
   - Fast, reliable test feedback
   - Clear documentation
   - Minimal maintenance overhead

4. **Production Readiness**
   - Confidence in code quality
   - Proper validation coverage
   - Professional CI/CD pipeline

## Notes

**Current Baseline** (from `.debug/TEST_STATUS_SUMMARY.md` dated 2025-10-11):
- 38/55 integration tests passing (69%)
- Known failure categories documented
- Test infrastructure was functional at that time

**Regression Window**: Between 2025-10-11 and 2025-10-13, test infrastructure degraded from functional to completely non-functional.

**Priority**: This is a BLOCKING issue. All other work depends on test infrastructure being functional.
