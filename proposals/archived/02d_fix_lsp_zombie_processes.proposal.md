# Proposal 02d: Fix LSP Zombie Process Leak

**Status**: Draft
**Created**: 2025-10-18
**Priority**: 🔴 CRITICAL
**Dependencies**: None

---

## Problem

**Current State**: 222 zombie LSP processes accumulating in the system

```bash
$ ps aux | grep -E "rust-analyzer|typescript-language-server" | grep -v grep | wc -l
222

$ ps aux | grep rust-analyzer | head -3
developer  410  0.0  0.0      0     0 ?        Z    14:26   0:00 [rust-analyzer] <defunct>
developer 1527  0.5  0.0      0     0 ?        Z    14:26   0:05 [rust-analyzer] <defunct>
developer 2185  0.0  0.0      0     0 ?        Z    01:06   0:00 [rust-analyzer] <defunct>
```

All processes are in `Z` (zombie) state with `<defunct>` status, indicating they've exited but haven't been reaped by the parent process.

### Impact

- **Resource Leak**: Each zombie consumes a process table entry
- **System Degradation**: Eventually hits process limit (typically 32768)
- **Production Risk**: Long-running servers will accumulate thousands of zombies
- **Test Suite Impact**: Every test run leaks processes (tests/e2e runs 822 tests)

---

## Root Cause Analysis

### Investigation Summary

The codebase already has zombie prevention mechanisms that SHOULD work:

1. ✅ **Zombie Reaper** (`cb-lsp/src/lsp_system/zombie_reaper.rs`):
   - Background thread checks registered PIDs every 100ms
   - Calls `waitpid(WNOHANG)` to reap zombies
   - Has comprehensive tests that pass

2. ✅ **Proper Shutdown Method** (`LspClient::shutdown()`):
   - Sends LSP shutdown request
   - Sends exit notification
   - Calls `kill()` on process
   - Calls `wait()` with timeout
   - Well-implemented, correct sequence

3. ✅ **PIDs Registered** (`client.rs:215-222`):
   - All spawned processes registered with zombie reaper
   - Confirmed via debug logs

**So why do we still have zombies?**

### The Actual Problem: Arc Reference Counting + Drop Timing

#### Location: `cb-handlers/src/handlers/lsp_adapter.rs` (Drop implementation)

```rust
impl Drop for DirectLspAdapter {
    fn drop(&mut self) {
        let clients = self.lsp_clients.clone();
        tokio::spawn(async move {
            for (extension, client) in clients_map.drain() {
                match Arc::try_unwrap(client) {
                    Ok(client) => {
                        // ✅ We have exclusive ownership - can call shutdown()
                        client.shutdown().await;
                    }
                    Err(client) => {
                        // ❌ OTHER REFERENCES EXIST - cannot call shutdown()
                        // Just logs: "relying on zombie reaper"
                        // But the process is STILL ALIVE, not a zombie yet!
                    }
                }
            }
        });
    }
}
```

#### The Problem Chain

1. **LSP Client Created**: Stored as `Arc<LspClient>` in HashMap
2. **References Held Elsewhere**: Test harness, concurrent requests, or response futures hold Arc clones
3. **DirectLspAdapter Drops**: When adapter is dropped (test cleanup, server shutdown)
4. **Arc::try_unwrap Fails**: Because `Arc::strong_count(&client) > 1`
5. **No Shutdown Called**: Process never receives kill signal
6. **Process Still Running**: Zombie reaper sees `WaitStatus::StillAlive` and does nothing
7. **Process Eventually Exits**: On its own, or crashes, or test completes
8. **Becomes Zombie**: Now it's a zombie, but...
9. **Race Condition**: If parent doesn't call `wait()` before child fully exits, zombie persists
10. **Accumulation**: 222+ zombies over time

### Why the Zombie Reaper Doesn't Help

The zombie reaper only calls `waitpid(WNOHANG)` on already-exited processes. It:
- ✅ **Works** if shutdown() was called (process killed → exits → reaped)
- ❌ **Fails** if shutdown() was never called (process still alive → reaper does nothing → eventually crashes → zombie)

### Key Insight

The comment "relying on zombie reaper" is **incorrect** - the reaper can't reap processes that were never killed!

---

## Solution

### Approach: Force Kill + Wait Regardless of Arc Count

The fix is to NOT rely on `Arc::try_unwrap` succeeding. Instead:

1. **Access process directly** through `client.kill()` (doesn't require ownership)
2. **Always call kill()** on all LSP processes during cleanup
3. **Ensure wait() is called** to reap the process

### Implementation Plan

#### Phase 1: Fix DirectLspAdapter Drop (Critical)

**File**: `crates/cb-handlers/src/handlers/lsp_adapter.rs`

```rust
impl Drop for DirectLspAdapter {
    fn drop(&mut self) {
        let clients = self.lsp_clients.clone();
        let adapter_name = self.name.clone();

        tokio::spawn(async move {
            let mut clients_map = clients.lock().await;

            if clients_map.is_empty() {
                return;
            }

            tracing::debug!(
                adapter_name = %adapter_name,
                client_count = clients_map.len(),
                "DirectLspAdapter dropping - forcefully killing all LSP clients"
            );

            // NEW APPROACH: Force kill all clients regardless of Arc count
            for (extension, client) in clients_map.drain() {
                let ext = extension.clone();
                let arc_count = Arc::strong_count(&client);

                tokio::spawn(async move {
                    // Always try to kill the process (doesn't require ownership)
                    match client.kill().await {
                        Ok(_) => {
                            tracing::debug!(
                                extension = %ext,
                                arc_count = arc_count,
                                "Forcefully killed LSP process from DirectLspAdapter drop"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                extension = %ext,
                                arc_count = arc_count,
                                error = %e,
                                "Failed to kill LSP process - may become zombie"
                            );
                        }
                    }

                    // Try graceful shutdown if we can get ownership
                    if let Ok(owned_client) = Arc::try_unwrap(client) {
                        let _ = owned_client.shutdown().await;
                    }
                    // If we can't get ownership, kill() above already terminated it
                    // and the zombie reaper will clean it up
                });
            }
        });
    }
}
```

#### Phase 2: Add Explicit Cleanup on Dead Client Detection

**File**: `crates/cb-handlers/src/handlers/lsp_adapter.rs` (get_or_create_client method)

Currently (lines 48-53):
```rust
if !client.is_alive().await {
    warn!("Found dead LSP client in cache, removing it");
    clients.remove(extension);
    // ❌ Just removes from HashMap - no cleanup!
}
```

**Fix**:
```rust
if !client.is_alive().await {
    warn!("Found dead LSP client in cache, cleaning up before removal");

    // Remove from cache first
    let dead_client = clients.remove(extension);

    // Spawn cleanup task
    if let Some(client) = dead_client {
        tokio::spawn(async move {
            // Try to kill the process if it's still running
            let _ = client.kill().await;

            // If we can get ownership, do full shutdown
            if let Ok(owned) = Arc::try_unwrap(client) {
                let _ = owned.shutdown().await;
            }
        });
    }
}
```

#### Phase 3: Enhance Zombie Reaper (Defense in Depth)

**File**: `crates/cb-lsp/src/lsp_system/zombie_reaper.rs`

Add periodic "force reap" that kills orphaned processes:

```rust
// Add to reaper_loop after the normal reaping logic
if iteration_count % 100 == 0 {  // Every 10 seconds
    // Check for processes that have been alive too long
    // and are likely orphaned
    force_reap_old_processes(&pids_guard);
}
```

---

## Checklists

### Phase 1: Fix DirectLspAdapter Drop ✅ CRITICAL
- [ ] Modify `impl Drop for DirectLspAdapter` to always call `kill()`
- [ ] Remove reliance on `Arc::try_unwrap` succeeding
- [ ] Add tracing for Arc count to monitor multiple references
- [ ] Test that processes are actually killed during drop

### Phase 2: Cleanup Dead Clients ✅ HIGH
- [ ] Modify `get_or_create_client` dead client detection
- [ ] Call `kill()` on dead clients before removing from cache
- [ ] Add cleanup spawn task for dead clients

### Phase 3: Testing & Verification ✅ CRITICAL
- [ ] Run test suite and verify no new zombies created
- [ ] Check `ps aux | grep rust-analyzer | wc -l` before and after tests
- [ ] Add integration test that verifies cleanup
- [ ] Test with simulated Arc reference leaks

### Phase 4: Zombie Reaper Enhancement (Optional) 🟡 LOW
- [ ] Add periodic force-reap for old processes
- [ ] Add metrics for zombie count
- [ ] Add alerts for zombie accumulation

---

## Success Criteria

### Before
```bash
$ ps aux | grep rust-analyzer | wc -l
222  # 222 zombie processes
```

### After (Running Full Test Suite)
```bash
$ cargo nextest run --workspace
# ... 822 tests pass ...

$ ps aux | grep rust-analyzer | wc -l
0-2  # Maximum 1-2 processes (currently running), zero zombies
```

### Verification Commands

```bash
# Before fix
ps aux | grep -E "rust-analyzer|typescript" | grep defunct | wc -l
# Should be high (100+)

# Run tests
cargo nextest run --workspace

# After fix
ps aux | grep -E "rust-analyzer|typescript" | grep defunct | wc -l
# Should be 0

# Check for any LSP processes
ps aux | grep -E "rust-analyzer|typescript" | grep -v grep
# Should show 0-2 active processes, ZERO defunct
```

---

## Benefits

### Immediate
- ✅ Eliminates 222+ accumulated zombies
- ✅ Prevents future zombie accumulation
- ✅ Test suite no longer leaks processes
- ✅ Production servers remain stable long-term

### Long-term
- 🎯 Proper resource cleanup patterns established
- 🎯 Arc reference counting pitfalls documented
- 🎯 Defense-in-depth approach (multiple cleanup layers)
- 🎯 Observable metrics for process management

---

## Risk Assessment

**Risk Level**: 🟢 **LOW**

**Why low risk?**
1. Changes are additive (adding more cleanup, not removing existing)
2. Existing `kill()` and `shutdown()` methods are well-tested
3. Multiple safety layers (Drop + zombie reaper + dead client detection)
4. Test suite provides immediate feedback

**Failure Modes**:
- ⚠️ If kill() fails → Logged + zombie reaper as backup
- ⚠️ If shutdown() fails → Logged + kill() already sent
- ⚠️ If both fail → Zombie reaper still running

**Mitigation**: Multiple independent cleanup mechanisms ensure at least one succeeds.

---

## Implementation Time Estimate

- **Phase 1** (Drop fix): 30 minutes
- **Phase 2** (Dead client cleanup): 15 minutes
- **Phase 3** (Testing): 30 minutes
- **Total**: ~1.5 hours

**Note**: Phase 4 (Zombie Reaper Enhancement) is optional and can be deferred.

---

## References

**Files to Modify**:
- `crates/cb-handlers/src/handlers/lsp_adapter.rs` (Drop + dead client detection)

**Files to Review**:
- `crates/cb-lsp/src/lsp_system/client.rs` (LspClient implementation)
- `crates/cb-lsp/src/lsp_system/zombie_reaper.rs` (Zombie reaper)

**Related Issues**:
- Drop implementation relying on Arc::try_unwrap
- Missing cleanup for dead clients
- Zombie reaper not proactive enough

---

## Follow-up Work (Optional)

- [ ] Add metrics/telemetry for LSP process lifecycle
- [ ] Add alerts for zombie accumulation
- [ ] Periodic audit of Arc reference counts
- [ ] Document Arc lifetime management patterns
- [ ] Consider replacing Arc with weak references where appropriate
