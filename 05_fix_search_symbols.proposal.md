# Proposal: Fix search_symbols and Add Workspace-Level Analysis

**Status**: Draft
**Dependencies**: None (can run in parallel with other proposals)
**Priority**: High (blocks dogfooding for TypeMill rename - Proposal 04)

---

## Problem

Two critical issues block dogfooding CodeBuddy for the TypeMill rename (Proposal 04):

### Issue 1: search_symbols Returns Empty Results

`search_symbols` returns empty results and creates zombie processes because:

1. **LSP servers hang during initialization** - `crates/cb-lsp/src/lsp_system/client.rs:738` treats all messages with `id` as responses, ignoring server-to-client requests (workspace/configuration, client/registerCapability, window/workDoneProgress/create)
2. **Servers crash silently** - When initialization hangs, servers give up and exit, explaining empty symbol results
3. **Zombie process accumulation** - 647+ defunct processes because `lsp_clients` cache in `crates/cb-handlers/src/handlers/lsp_adapter.rs:29-71` holds Arc references forever, preventing Child Drop/reaping
4. **Test coverage gap** - `apps/codebuddy/tests/e2e_workspace_operations.rs:600` uses harness that also ignores server requests, never exposing the bug

### Issue 2: Workspace-Level Analysis Not Supported

The unified `analyze.*` API only supports file-level analysis, preventing workspace-wide operations:

```bash
$ codebuddy tool analyze.dependencies '{"kind":"graph","scope":{"type":"workspace"}}'
Error: Invalid request: Missing file path. For MVP, only file-level analysis is supported
```

**Impact:**
- Cannot analyze cross-crate dependencies for rename planning
- Cannot find all dead code across entire workspace
- Requires manual per-file analysis for large refactorings
- Limits utility for production-scale operations

## Root Causes

### Issue 1: LSP Message Handling Bug

```rust
// crates/cb-lsp/src/lsp_system/client.rs:738
// Current: Assumes all messages with 'id' are responses
if let Some(id) = msg.get("id") {
    // Only checks pending_requests
    // Drops server-initiated requests → servers hang
}
```

### Issue 2: Missing Scope Infrastructure

All 6 unified analysis tools (`analyze.quality`, `analyze.dead_code`, `analyze.dependencies`, `analyze.structure`, `analyze.documentation`, `analyze.tests`) only accept file-level scope. No workspace aggregation infrastructure exists.

## Solution

### Part A: Fix search_symbols LSP Integration

#### 1. Implement Server Request Handling

Add bidirectional JSON-RPC support to distinguish server responses from server requests:

**Required Server Requests:**
- `workspace/configuration` - Server queries config
- `client/registerCapability` - Dynamic capability registration
- `window/workDoneProgress/create` - Progress token creation
- `workspace/workspaceFolders` - Workspace folder queries

**Implementation:**
- Check if message has both `id` AND `method` → server request
- Reply synchronously with appropriate response
- Unknown requests → LSP error response (don't drop silently)

#### 2. Add Process Lifecycle Management

Track and cleanup dead LSP processes:
- Detect EOF on stdout reader → call `try_wait()` on Child
- Evict dead clients from `lsp_clients` cache
- Allow fresh server spawn on next request
- Child Drop properly reaps process

#### 3. Add Test Coverage for LSP

Verify we answer server requests:
- Assert response to `workspace/configuration`
- Assert clean process table after operations
- Detect regressions in bidirectional communication

### Part B: Add Workspace-Level Analysis

#### 1. Add Scope Infrastructure

```rust
pub enum AnalysisScope {
    File { path: PathBuf },
    Directory { path: PathBuf },
    Crate { manifest_path: PathBuf },
    Workspace,
}
```

#### 2. Implement File Discovery & Aggregation

- Use `ignore` crate for workspace file discovery (respects .gitignore)
- Parallel file analysis with bounded concurrency (e.g., 8 concurrent)
- Aggregate results by category with file-level traceability
- Error tolerance (continue on single file failures)

#### 3. Update All 6 Analysis Tools

Each tool (`analyze.*`) must accept new scope types and aggregate results appropriately:
- `analyze.dependencies` - Build cross-file dependency graph
- `analyze.dead_code` - Find unused symbols workspace-wide
- `analyze.quality` - Average metrics, identify worst offenders
- `analyze.structure` - Workspace symbol hierarchy
- `analyze.documentation` - Overall coverage percentage
- `analyze.tests` - Test coverage by crate/directory

#### 4. Add Test Coverage for Workspace Analysis

- Workspace-level tests for each analysis tool
- Performance tests (<10s for ~250 files)
- Error handling (tolerates unparseable files)

## Implementation Checklist

### Part A: Fix search_symbols (Phases 1-4)

#### Phase 1: Server Request Handling
- [ ] Update `LspClient::handle_message()` in `crates/cb-lsp/src/lsp_system/client.rs:738`
  - [ ] Add message type detection: `has_id() && has_method()` → server request
  - [ ] Implement `workspace/configuration` response handler
  - [ ] Implement `client/registerCapability` response handler
  - [ ] Implement `window/workDoneProgress/create` response handler
  - [ ] Implement `workspace/workspaceFolders` response handler
  - [ ] Unknown server requests → return LSP error response (code: -32601 Method not found)
  - [ ] Add structured logging for server requests

#### Phase 2: Process Lifecycle Tracking
- [ ] Add `is_alive()` check to `LspClient` in `crates/cb-lsp/src/lsp_system/client.rs`
- [ ] Detect EOF on stdout reader task
  - [ ] Call `Child::try_wait()` when EOF detected
  - [ ] Emit structured log when process exits
- [ ] Update `lsp_clients` cache in `crates/cb-handlers/src/handlers/lsp_adapter.rs:29-71`
  - [ ] Evict dead clients from cache
  - [ ] Allow fresh spawn on next request
  - [ ] Add cache cleanup method: `cleanup_dead_clients()`
- [ ] Add zombie detection to `codebuddy status` command

#### Phase 3: Testing LSP Fixes
- [ ] Add `test_server_requests_answered()` - Verify bidirectional JSON-RPC
  - [ ] Assert response to `workspace/configuration`
  - [ ] Assert response to `client/registerCapability`
- [ ] Add `test_search_symbols_rust_workspace()` - Verify symbol search works
  - [ ] Create Rust workspace with known symbols
  - [ ] Wait for indexing
  - [ ] Assert `search_symbols` returns results
- [ ] Add `test_process_cleanup()` - Verify no zombies
  - [ ] Run multiple LSP operations
  - [ ] Assert process table clean afterward
- [ ] Update `test_cross_language_project()` - Add zombie assertion
  - [ ] Count processes before test
  - [ ] Count processes after test
  - [ ] Assert no new zombies

#### Phase 4: Validation of search_symbols
- [ ] Manual test: `codebuddy tool search_symbols '{"query": "main"}'` returns results
- [ ] Manual test: No TypeScript "No Project" errors in Rust workspace
- [ ] Manual test: `ps aux | grep -E "(rust-analyzer|typescript)" | grep defunct | wc -l` returns 0
- [ ] All existing tests pass
- [ ] Symbol search completes within 5 seconds after indexing

### Part B: Add Workspace Analysis (Phases 5-7)

#### Phase 5: Scope Infrastructure
- [ ] Add `AnalysisScope` enum to `crates/cb-protocol/src/lib.rs`
  ```rust
  pub enum AnalysisScope {
      File { path: PathBuf },
      Directory { path: PathBuf },
      Crate { manifest_path: PathBuf },
      Workspace,
  }
  ```
- [ ] Update all 6 `analyze.*` handlers to accept new scope types
- [ ] Add file discovery utilities to `crates/cb-services/src/services/`
  - [ ] `discover_files(scope: AnalysisScope, extensions: &[&str]) -> Vec<PathBuf>`
  - [ ] Respect `.gitignore` via `ignore` crate
  - [ ] Filter by language extensions

#### Phase 6: Workspace Analysis Engine
- [ ] Add parallel file processor to `crates/cb-services/src/services/planner.rs`
  - [ ] Bounded parallelism (`tokio::sync::Semaphore` with limit 8)
  - [ ] Progress tracking for long operations
  - [ ] Error tolerance (continue on single file errors)
- [ ] Add result aggregation for each analysis kind:
  - [ ] `analyze.dependencies` - Build dependency graph across files
  - [ ] `analyze.dead_code` - Aggregate unused symbols workspace-wide
  - [ ] `analyze.quality` - Average metrics, worst offenders
  - [ ] `analyze.structure` - Workspace symbol hierarchy
  - [ ] `analyze.documentation` - Overall coverage percentage
  - [ ] `analyze.tests` - Test coverage by crate/directory

#### Phase 7: Testing Workspace Analysis
- [ ] Add workspace-level tests for each analysis tool:
  - [ ] `test_analyze_dependencies_workspace()` - Cross-crate imports
  - [ ] `test_analyze_dead_code_workspace()` - Unused across files
  - [ ] `test_analyze_quality_workspace()` - Aggregate metrics
- [ ] Add performance tests:
  - [ ] Workspace with 100+ files completes in <10s
  - [ ] Memory usage stays under 500MB for large workspaces
- [ ] Add error handling tests:
  - [ ] Workspace analysis tolerates unparseable files
  - [ ] Reports partial results on timeout

## Success Criteria

### Part A: search_symbols Fixed
- [ ] `search_symbols` returns Rust symbols in Rust workspace
- [ ] LSP servers complete initialization without hanging
- [ ] Zero zombie processes after running `search_symbols` 10 times
- [ ] New tests pass: server requests answered, no zombies, symbols found
- [ ] Existing `test_cross_language_project()` still passes
- [ ] `codebuddy status` shows no defunct processes

### Part B: Workspace Analysis Working
- [ ] All 6 `analyze.*` tools accept `"scope": {"type": "workspace"}`
- [ ] `analyze.dependencies` with workspace scope returns cross-crate graph
- [ ] Workspace analysis completes in <10s for ~250 files
- [ ] Results include file-level breakdown for traceability
- [ ] Proposal 04 dogfooding updated to use workspace-level analysis
- [ ] No regression in file-level analysis performance

## Benefits

- **Enables full dogfooding** - Both `search_symbols` and workspace analysis work for TypeMill rename (Proposal 04)
- **Clean process management** - Eliminates 647+ zombie accumulation
- **Better LSP compliance** - Proper bidirectional JSON-RPC support
- **Workspace-scale operations** - Real-world codebases need workspace analysis
- **Production ready** - LSP integration and analysis validated for large projects

## Technical Notes

### Part A: LSP Integration

**LSP JSON-RPC Message Types:**
```json
// Server Response (has id, no method)
{"jsonrpc": "2.0", "id": 1, "result": {...}}

// Server Request (has id AND method)
{"jsonrpc": "2.0", "id": 2, "method": "workspace/configuration", "params": {...}}

// Server Notification (has method, no id)
{"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {...}}
```

**Current Bug:**
```rust
// Treats both responses AND requests as responses
if let Some(id) = msg.get("id") {
    // Only checks pending_requests
    // Server requests get dropped here!
}
```

**Fix:**
```rust
if let Some(id) = msg.get("id") {
    if msg.get("method").is_some() {
        // Server request - answer it!
        handle_server_request(msg);
    } else {
        // Server response - match to pending request
        handle_server_response(msg);
    }
}
```

### Part B: Workspace Analysis

**File Discovery Pattern:**
```rust
use ignore::WalkBuilder;

fn discover_rust_files(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .filter_entry(|e| !e.file_name().starts_with('.'))
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("rs"))
        .map(|e| e.into_path())
        .collect()
}
```

**Parallel Processing:**
```rust
use tokio::sync::Semaphore;

let sem = Arc::new(Semaphore::new(8)); // Max 8 concurrent
let tasks: Vec<_> = files.into_iter().map(|file| {
    let sem = sem.clone();
    tokio::spawn(async move {
        let _permit = sem.acquire().await;
        analyze_file(file).await
    })
}).collect();

let results = futures::future::join_all(tasks).await;
```

**Aggregated Result Format:**
```json
{
  "findings": [...],
  "summary": {
    "files_analyzed": 247,
    "total_findings": 1834,
    "by_severity": {"high": 12, "medium": 89, "low": 1733},
    "analysis_time_ms": 4521
  },
  "metadata": {
    "scope": {"type": "workspace"}
  }
}
```

## References

- Proposal 04: TypeMill Rename (requires both fixes)
- LSP Specification: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
  - Server Requests: Section 4.4 "Server initiated"
  - `workspace/configuration`: Section 9.26
  - `client/registerCapability`: Section 9.27
- `ignore` crate: https://docs.rs/ignore/ (gitignore support for file discovery)
