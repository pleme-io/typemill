# Proposal: LSP Progress Notification Support

**Status**: Draft

## Executive Summary

Implement proper support for LSP `$/progress` notifications to enable event-driven coordination with language servers instead of polling/sleeping. This addresses the root cause of test flakiness in `test_search_symbols_rust_workspace` and makes the entire LSP integration more robust.

## Problem Statement

### Current Architecture Flaw

The `LspClient` in `crates/cb-lsp/src/lsp_system/client.rs` **discards all server notifications**:

```rust
// Line ~200 in handle_message
if message.get("method").is_some() {
    if message.get("id").is_some() {
        // Request - handle it
        Self::handle_server_request(&message, message_tx).await;
    } else {
        // Notification - just log and discard!
        debug!("Received notification from LSP server: {:?}", message);
    }
}
```

### Why This Matters

Language servers like rust-analyzer send critical notifications:
- **`$/progress`** - Long-running operations (indexing, building)
- **`textDocument/publishDiagnostics`** - Error/warning updates
- **`window/logMessage`** - Server logs
- **`window/showMessage`** - User-facing messages

By ignoring these, we:
- ❌ Cannot know when indexing completes
- ❌ Must use arbitrary `tokio::sleep()` timeouts
- ❌ Have flaky tests that fail in CI
- ❌ Cannot show diagnostics in real-time
- ❌ Miss important server status updates

### Concrete Example: rust-analyzer Indexing

When rust-analyzer starts indexing, it sends:

```json
{
  "jsonrpc": "2.0",
  "method": "$/progress",
  "params": {
    "token": "rustAnalyzer/Indexing",
    "value": {
      "kind": "begin",
      "title": "Indexing",
      "message": "0/42 packages",
      "percentage": 0
    }
  }
}
```

Then progress updates:

```json
{
  "method": "$/progress",
  "params": {
    "token": "rustAnalyzer/Indexing",
    "value": {
      "kind": "report",
      "message": "21/42 packages",
      "percentage": 50
    }
  }
}
```

Finally completion:

```json
{
  "method": "$/progress",
  "params": {
    "token": "rustAnalyzer/Indexing",
    "value": {
      "kind": "end",
      "message": "Indexing complete"
    }
  }
}
```

**We throw all of these away!**

## Proposed Solution

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                          LspClient                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────┐    ┌─────────────────┐                 │
│  │ Request/      │───▶│ Progress        │                 │
│  │ Response      │    │ Manager         │                 │
│  │ Handler       │    └─────────────────┘                 │
│  │ (existing)    │            │                           │
│  └───────────────┘            │                           │
│         │                     │                           │
│         │              ┌──────▼──────────┐                │
│         │              │ Progress State  │                │
│         │              │ Machine         │                │
│         │              └──────┬──────────┘                │
│         │                     │                           │
│         │                     ▼                           │
│         │          ┌─────────────────────┐               │
│         └─────────▶│ Message Handler     │               │
│                    │ (enhanced)          │               │
│                    └─────────────────────┘               │
│                             │                             │
└─────────────────────────────┼─────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │  LSP Server      │
                    │  (rust-analyzer) │
                    └──────────────────┘
```

### Core Components

#### 1. Progress Notification Types

```rust
// crates/cb-lsp/src/progress.rs

use serde::{Deserialize, Serialize};
use std::fmt;

/// LSP Progress token (string or integer)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    String(String),
    Number(i32),
}

impl fmt::Display for ProgressToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProgressToken::String(s) => write!(f, "{}", s),
            ProgressToken::Number(n) => write!(f, "{}", n),
        }
    }
}

/// Work done progress value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WorkDoneProgressValue {
    #[serde(rename = "begin")]
    Begin {
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        percentage: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cancellable: Option<bool>,
    },
    #[serde(rename = "report")]
    Report {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        percentage: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cancellable: Option<bool>,
    },
    #[serde(rename = "end")]
    End {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
}

/// Progress notification parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressParams {
    pub token: ProgressToken,
    pub value: WorkDoneProgressValue,
}

/// Internal progress state
#[derive(Debug, Clone)]
pub enum ProgressState {
    InProgress {
        title: String,
        message: Option<String>,
        percentage: Option<u32>,
        cancellable: bool,
    },
    Completed {
        message: Option<String>,
    },
}
```

#### 2. Progress Manager

```rust
// crates/cb-lsp/src/progress.rs (continued)

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Manages LSP progress notifications
pub struct ProgressManager {
    /// Active progress tasks by token
    tasks: Arc<DashMap<ProgressToken, ProgressState>>,

    /// Broadcast channel for progress updates
    /// (token, state) pairs
    updates_tx: broadcast::Sender<(ProgressToken, ProgressState)>,
}

impl ProgressManager {
    pub fn new() -> Self {
        let (updates_tx, _) = broadcast::channel(100);

        Self {
            tasks: Arc::new(DashMap::new()),
            updates_tx,
        }
    }

    /// Handle a progress notification
    pub fn handle_notification(&self, params: ProgressParams) {
        use WorkDoneProgressValue::*;

        let token = params.token.clone();

        match params.value {
            Begin { title, message, percentage, cancellable } => {
                let state = ProgressState::InProgress {
                    title,
                    message,
                    percentage,
                    cancellable: cancellable.unwrap_or(false),
                };

                tracing::debug!(
                    token = %token,
                    "Progress started"
                );

                self.tasks.insert(token.clone(), state.clone());
                let _ = self.updates_tx.send((token, state));
            }

            Report { message, percentage, cancellable } => {
                if let Some(mut entry) = self.tasks.get_mut(&token) {
                    if let ProgressState::InProgress {
                        ref mut message: msg,
                        ref mut percentage: pct,
                        ref mut cancellable: cancel,
                        ..
                    } = *entry {
                        *msg = message.clone();
                        *pct = percentage;
                        if let Some(c) = cancellable {
                            *cancel = c;
                        }

                        tracing::debug!(
                            token = %token,
                            percentage = ?percentage,
                            "Progress update"
                        );

                        let state = entry.clone();
                        let _ = self.updates_tx.send((token, state));
                    }
                }
            }

            End { message } => {
                let state = ProgressState::Completed { message };

                tracing::debug!(
                    token = %token,
                    "Progress completed"
                );

                self.tasks.insert(token.clone(), state.clone());
                let _ = self.updates_tx.send((token.clone(), state));

                // Remove from active tasks after a short delay
                // (allows subscribers to see completion)
                let tasks = self.tasks.clone();
                let token_clone = token.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    tasks.remove(&token_clone);
                });
            }
        }
    }

    /// Get current state of a progress task
    pub fn get_state(&self, token: &ProgressToken) -> Option<ProgressState> {
        self.tasks.get(token).map(|entry| entry.clone())
    }

    /// Check if a task is completed
    pub fn is_completed(&self, token: &ProgressToken) -> bool {
        matches!(
            self.get_state(token),
            Some(ProgressState::Completed { .. }) | None
        )
    }

    /// Wait for a task to complete
    pub async fn wait_for_completion(
        &self,
        token: &ProgressToken,
        timeout: std::time::Duration,
    ) -> Result<(), ProgressError> {
        // Already completed?
        if self.is_completed(token) {
            return Ok(());
        }

        // Subscribe to updates
        let mut rx = self.updates_tx.subscribe();
        let target_token = token.clone();

        let result = tokio::time::timeout(timeout, async move {
            loop {
                match rx.recv().await {
                    Ok((token, state)) if token == target_token => {
                        if matches!(state, ProgressState::Completed { .. }) {
                            return Ok(());
                        }
                    }
                    Ok(_) => continue, // Different token
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // We lagged behind, check current state
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err(ProgressError::ChannelClosed);
                    }
                }
            }
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ProgressError::Timeout(timeout)),
        }
    }

    /// Subscribe to all progress updates
    pub fn subscribe(&self) -> broadcast::Receiver<(ProgressToken, ProgressState)> {
        self.updates_tx.subscribe()
    }

    /// List all active tasks
    pub fn active_tasks(&self) -> Vec<(ProgressToken, ProgressState)> {
        self.tasks
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

/// Progress-related errors
#[derive(Debug, thiserror::Error)]
pub enum ProgressError {
    #[error("Timeout waiting for progress completion: {0:?}")]
    Timeout(std::time::Duration),

    #[error("Progress channel closed")]
    ChannelClosed,
}
```

#### 3. LspClient Integration

```rust
// crates/cb-lsp/src/lsp_system/client.rs

pub struct LspClient {
    // ... existing fields ...

    /// Progress notification manager
    progress_manager: Arc<ProgressManager>,
}

impl LspClient {
    pub async fn new(config: LspServerConfig) -> ServerResult<Self> {
        // ... existing initialization ...

        let progress_manager = Arc::new(ProgressManager::new());

        // ... spawn message handler with progress_manager ...

        Ok(Self {
            // ... existing fields ...
            progress_manager,
        })
    }

    /// Wait for a specific progress task to complete
    pub async fn wait_for_progress(
        &self,
        token: &ProgressToken,
        timeout: std::time::Duration,
    ) -> Result<(), ProgressError> {
        self.progress_manager.wait_for_completion(token, timeout).await
    }

    /// Wait for rust-analyzer indexing to complete
    pub async fn wait_for_indexing(
        &self,
        timeout: std::time::Duration,
    ) -> Result<(), ProgressError> {
        // rust-analyzer uses this standard token
        let token = ProgressToken::String("rustAnalyzer/Indexing".to_string());
        self.wait_for_progress(&token, timeout).await
    }

    /// Get progress manager for advanced use cases
    pub fn progress(&self) -> &Arc<ProgressManager> {
        &self.progress_manager
    }
}

// In handle_message function:
async fn handle_message(
    // ... parameters ...
    progress_manager: Arc<ProgressManager>,
) {
    // ... existing code ...

    if let Some(method) = message.get("method").and_then(|v| v.as_str()) {
        if message.get("id").is_some() {
            // Server-initiated request
            Self::handle_server_request(&message, message_tx).await;
        } else {
            // Server notification - NOW WE HANDLE IT!
            match method {
                "$/progress" => {
                    if let Ok(params) = serde_json::from_value::<ProgressParams>(
                        message.get("params").cloned().unwrap_or_default()
                    ) {
                        progress_manager.handle_notification(params);
                    } else {
                        warn!("Failed to parse $/progress notification");
                    }
                }
                "textDocument/publishDiagnostics" => {
                    // TODO: Handle diagnostics
                    debug!("Received diagnostics notification");
                }
                "window/logMessage" => {
                    // TODO: Handle log messages
                    debug!("Received log message notification");
                }
                _ => {
                    debug!(method = method, "Received unhandled notification");
                }
            }
        }
    }

    // ... existing code ...
}
```

### Test Integration

#### Before (Flaky)

```rust
#[tokio::test]
async fn test_search_symbols_rust_workspace() {
    // ... setup ...

    // Wait for LSP to index
    client.wait_for_lsp_ready(&main_file, 30000).await?;

    // ARBITRARY SLEEP - might not be enough!
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Try workspace symbols
    let response = client.call_tool("search_symbols", json!({"query": "main"})).await?;

    // Might fail if indexing not complete!
    assert!(!symbols.is_empty());
}
```

#### After (Deterministic)

```rust
#[tokio::test]
async fn test_search_symbols_rust_workspace() {
    // ... setup ...

    // Wait for LSP to index (document symbols)
    client.wait_for_lsp_ready(&main_file, 30000).await?;

    // Wait for workspace indexing to ACTUALLY complete
    let lsp_client = get_lsp_client_for_rust(&mut client).await?;
    lsp_client
        .wait_for_indexing(Duration::from_secs(30))
        .await
        .expect("Indexing should complete within 30s");

    // Try workspace symbols - NOW IT'S GUARANTEED TO WORK
    let response = client.call_tool("search_symbols", json!({"query": "main"})).await?;

    // Will always pass!
    assert!(!symbols.is_empty());
}
```

## Checklists

### Core Infrastructure
- [ ] Create `crates/cb-lsp/src/progress.rs` - Types and ProgressManager
- [ ] Export progress module in `crates/cb-lsp/src/lsp_system/mod.rs`
- [ ] Integrate ProgressManager in `crates/cb-lsp/src/lsp_system/client.rs`
- [ ] Add `dashmap` dependency to `crates/cb-lsp/Cargo.toml`
- [ ] Progress types defined
- [ ] ProgressManager implemented
- [ ] Unit tests for ProgressManager
- [ ] LspClient has progress_manager field

### Message Handling
- [ ] Update `handle_message` function in `crates/cb-lsp/src/lsp_system/client.rs`
- [ ] Parse and handle `$/progress` notifications
- [ ] Log other notifications (diagnostics, log messages)
- [ ] Add error handling for malformed notifications
- [ ] Add tracing/logging for debugging

### Public API
- [ ] Add `wait_for_progress()` method to LspClient
- [ ] Add `wait_for_indexing()` helper
- [ ] Add `progress()` getter for advanced use
- [ ] Write API documentation with examples

### Test Integration
- [ ] Update `apps/codebuddy/tests/e2e_workspace_operations.rs` - Replace sleep with wait_for_indexing
- [ ] Add helper to get LspClient in `crates/cb-test-support/src/harness/client.rs`
- [ ] Update `test_search_symbols_rust_workspace` to use proper waiting
- [ ] Document new test pattern

### Testing & Validation
- [ ] Unit tests for progress types
- [ ] Unit tests for ProgressManager
- [ ] Integration test with mock LSP server
- [ ] Verify `test_search_symbols_rust_workspace` passes reliably

## Benefits

### Immediate

- ✅ **Test Reliability** - No more flaky tests due to timing
- ✅ **Deterministic** - Wait exactly as long as needed, no guessing
- ✅ **Fast** - Don't wait longer than necessary
- ✅ **Protocol Compliant** - Use LSP as designed

### Medium-Term

- ✅ **Diagnostics** - Can implement real-time error/warning updates
- ✅ **User Feedback** - Show progress bars for long operations
- ✅ **Debugging** - Better visibility into server state
- ✅ **Cancellation** - Can support cancelling long operations

### Long-Term

- ✅ **Foundation** - Enables many LSP features we're currently missing
- ✅ **Robustness** - Better coordination with language servers
- ✅ **Observability** - Metrics on indexing times, operation duration

## Risks & Mitigation

### Risk 1: Broadcast Channel Lag

**Risk**: High-frequency progress updates could overwhelm subscribers

**Mitigation**:
- Use `broadcast::channel` with sufficient capacity (100)
- Handle `RecvError::Lagged` gracefully
- Check current state on lag

### Risk 2: Token Mismatch

**Risk**: Different servers use different token formats/names

**Mitigation**:
- Support both string and integer tokens
- Document common token names (rust-analyzer, typescript-language-server)
- Provide generic `wait_for_progress()` with manual token

### Risk 3: Progress Never Completes

**Risk**: Server might not send "end" notification

**Mitigation**:
- All wait methods have mandatory timeout
- Return `ProgressError::Timeout` with context
- Log warnings for long-running tasks

### Risk 4: Memory Leak

**Risk**: Completed tasks accumulate in DashMap

**Mitigation**:
- Remove completed tasks after 100ms
- Use DashMap for lock-free cleanup
- Consider TTL-based cleanup for safety

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_begin() {
        let manager = ProgressManager::new();
        let token = ProgressToken::String("test".to_string());

        let params = ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::Begin {
                title: "Testing".to_string(),
                message: None,
                percentage: Some(0),
                cancellable: None,
            },
        };

        manager.handle_notification(params);

        let state = manager.get_state(&token).unwrap();
        assert!(matches!(state, ProgressState::InProgress { .. }));
    }

    #[tokio::test]
    async fn test_wait_for_completion() {
        let manager = Arc::new(ProgressManager::new());
        let token = ProgressToken::String("test".to_string());

        // Start progress
        manager.handle_notification(ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::Begin {
                title: "Test".to_string(),
                message: None,
                percentage: None,
                cancellable: None,
            },
        });

        // Spawn task to complete it after 100ms
        let manager_clone = manager.clone();
        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            manager_clone.handle_notification(ProgressParams {
                token: token_clone,
                value: WorkDoneProgressValue::End {
                    message: Some("Done".to_string()),
                },
            });
        });

        // Wait should succeed
        let result = manager
            .wait_for_completion(&token, Duration::from_secs(1))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_timeout() {
        let manager = ProgressManager::new();
        let token = ProgressToken::String("never-completes".to_string());

        // Start but never complete
        manager.handle_notification(ProgressParams {
            token: token.clone(),
            value: WorkDoneProgressValue::Begin {
                title: "Stuck".to_string(),
                message: None,
                percentage: None,
                cancellable: None,
            },
        });

        // Should timeout
        let result = manager
            .wait_for_completion(&token, Duration::from_millis(100))
            .await;

        assert!(matches!(result, Err(ProgressError::Timeout(_))));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_rust_analyzer_progress() {
    // Create real rust-analyzer LSP client
    let config = LspServerConfig {
        extensions: vec!["rs".to_string()],
        command: vec!["rust-analyzer".to_string()],
        root_dir: None,
        restart_interval: None,
        initialization_options: None,
    };

    let client = LspClient::new(config).await.unwrap();

    // Subscribe to progress updates
    let mut rx = client.progress().subscribe();

    // Open a Rust file (triggers indexing)
    client.send_notification(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": "file:///tmp/test.rs",
                "languageId": "rust",
                "version": 1,
                "text": "fn main() {}"
            }
        })
    ).await.unwrap();

    // Wait for indexing progress
    let indexing_token = ProgressToken::String("rustAnalyzer/Indexing".to_string());

    let result = client
        .wait_for_progress(&indexing_token, Duration::from_secs(30))
        .await;

    assert!(result.is_ok(), "Indexing should complete");

    // Verify we can see workspace symbols now
    let symbols = client
        .send_request("workspace/symbol", json!({"query": "main"}))
        .await
        .unwrap();

    assert!(symbols.as_array().unwrap().len() > 0);
}
```

## Migration Path

### Backward Compatibility

- ✅ Existing code continues to work unchanged
- ✅ Progress notifications are handled transparently
- ✅ No breaking API changes

### Incremental Adoption

1. **Phase 1**: Infrastructure deployed, no behavior changes
2. **Phase 2**: Tests updated to use new waiting methods
3. **Phase 3**: Production code can optionally use progress API
4. **Phase 4**: Enable progress UI in future versions

### Documentation

- Update `crates/cb-lsp/README.md` with progress notification examples
- Add doc comments to all new public APIs
- Create integration guide in `docs/lsp/progress-notifications.md`
- Update `CLAUDE.md` with new capabilities

## Success Criteria

### Must Have

- ✅ `test_search_symbols_rust_workspace` passes reliably (100% success rate in 50 runs)
- ✅ No increase in test execution time (compared to sleep-based approach)
- ✅ All unit tests pass
- ✅ No memory leaks (verified with valgrind or similar)

### Nice to Have

- ✅ Progress bars in CLI (future work)
- ✅ Diagnostics support (future work)
- ✅ Cancellation support (future work)

## Alternatives Considered

### Alt 1: Polling-Based Wait

**Approach**: Periodically query server capabilities or try operations

**Pros**: Simpler to implement

**Cons**:
- Still relies on timing
- Inefficient (wastes resources)
- Not protocol-compliant

**Verdict**: ❌ Rejected - doesn't solve the root cause

### Alt 2: Skip Test When Not Ready

**Approach**: Current workaround - skip if symbols not found

**Pros**:
- Minimal code change
- Unblocks test suite

**Cons**:
- Doesn't validate the feature works
- Hides potential bugs
- Test becomes less valuable

**Verdict**: ⚠️ Temporary workaround only

### Alt 3: This Proposal

**Approach**: Implement proper progress notification support

**Pros**:
- ✅ Protocol-compliant
- ✅ Deterministic
- ✅ Enables future features
- ✅ Robust

**Cons**:
- More implementation work (8-12 hours)

**Verdict**: ✅ **Recommended**

## Dependencies

### New Crate Dependencies

```toml
# crates/cb-lsp/Cargo.toml
[dependencies]
# ... existing ...
dashmap = "6.0"  # Already in workspace deps
```

### Existing Dependencies

- `tokio` - async runtime
- `serde` / `serde_json` - serialization
- `tracing` - logging
- `thiserror` - error types


## Appendix: LSP Progress Specification

From [LSP Specification - Progress Support](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#progress):

> Progress is reported against a token. The token can be obtained by:
> - A request ID
> - A unique token created by the server
>
> Progress is reported using the `$/progress` notification.

**Work Done Progress**:
- `begin` - operation started
- `report` - progress update
- `end` - operation completed

**Common Tokens**:
- rust-analyzer: `"rustAnalyzer/Indexing"`, `"rustAnalyzer/Building"`
- typescript-language-server: `"semanticTokens/full"`
- Others vary by server

## References

- [LSP Specification v3.17](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
- [rust-analyzer LSP implementation](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/lsp-extensions.md)
- [Tower LSP Progress Example](https://github.com/ebkalderon/tower-lsp/blob/master/examples/progress.rs)
