# Fix LSP Client Zombie Process Leaks

## Problem

The system accumulates **2,502+ zombie processes** due to `LspClient` failing to properly reap child processes. Root causes:

1. **`tokio::process::Child` does not auto-reap** - Unlike `std::process::Child`, tokio's version requires explicit `wait()` call
2. **Async Drop anti-pattern** - `LspClient::drop()` uses `tokio::spawn()` for cleanup, which provides no execution guarantee during runtime shutdown
3. **Test failure** - `test_no_zombie_processes_on_lsp_failure` expects LSP initialization to fail for invalid servers but receives success

Current zombie breakdown:
- 1,427 mold (linker processes)
- 600 node (TypeScript LSP servers)
- 154 rust-analyzer (Rust LSP servers)
- 109 cargo + 108 rustc (build processes)
- 104 misc (test infrastructure)

## Solution

Implement hybrid approach combining explicit shutdown with global reaper safety net:

1. Add `LspClient::shutdown()` async method for proper cleanup
2. Implement global zombie reaper thread to catch missed cleanups
3. Update `Drop` to warn and rely on reaper as fallback
4. Fix LSP initialization health check to properly detect dead servers

## Checklists

- [ ] Add `nix` crate dependency for Unix process management
- [ ] Create `zombie_reaper` module in `cb-lsp/src/lsp_system/`
- [ ] Implement `ZombieReaper` struct with background thread
- [ ] Thread polls registered PIDs with `waitpid(WNOHANG)` every 100ms
- [ ] Expose `register(pid: i32)` function via static `ZOMBIE_REAPER`
- [ ] Add tests verifying reaper cleans up zombies within 200ms
- [ ] Add `shutdown(&mut self)` async method to `LspClient`
- [ ] Send LSP `shutdown` request and `exit` notification
- [ ] Call `process.kill()` and `process.wait()` with 5s timeout
- [ ] Return error if process doesn't exit gracefully
- [ ] Register child PID with `ZOMBIE_REAPER` in `LspClient::new()`
- [ ] Update `Drop` to log warning instead of spawning cleanup task
- [ ] Investigate why `call_tool()` returns Ok for `/bin/sh -c "exit 1"`
- [ ] Verify health check at `client.rs:428-434` properly detects dead servers
- [ ] Ensure `initialize()` timeout actually fails for non-responsive servers
- [ ] Trace error handling path from `LspClient::new()` to test assertion
- [ ] Fix error swallowing if present in handler chain
- [ ] Audit all `LspClient` creation sites in codebase
- [ ] Update `DirectLspAdapter` to call `shutdown()` on client drop
- [ ] Update test infrastructure to call `shutdown()` after tests
- [ ] Add shutdown to server lifecycle in `cb-server`
- [ ] Verify all paths through handlers properly clean up clients
- [ ] Enable `test_no_zombie_processes_on_lsp_failure` and verify it passes
- [ ] Add test that spawns 10 LSP clients and verifies zero zombies after cleanup
- [ ] Add test that drops clients without shutdown and verifies reaper cleans up
- [ ] Run full test suite and verify zombie count remains at zero
- [ ] Add zombie count assertion to test framework teardown

## Success Criteria

- [ ] `test_no_zombie_processes_on_lsp_failure` passes
- [ ] Full test suite produces zero zombie processes
- [ ] `ps aux | grep defunct | wc -l` shows 0 after test runs
- [ ] All existing tests continue to pass
- [ ] No performance regression from reaper thread

## Benefits

- Eliminates 2,500+ zombie process leak
- Prevents PID exhaustion (32K limit on Linux)
- Fixes failing test that detects the zombie issue
- Provides safety net for future process management bugs
- Enables long-running server deployments without zombie accumulation
