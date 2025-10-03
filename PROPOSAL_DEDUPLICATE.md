# Code Deduplication Proposal

**Status:** Draft - Build Fix Required ⚠️
**Created:** 2025-10-03
**Author:** Code Quality Initiative
**Current Duplication:** 4.67% (1224 lines / 10733 tokens across 90 clones)
**Target Duplication:** ~2.7% (~420-550 lines reduction)
**Confidence Level:** 92.0%

---

## 🚨 CRITICAL: Prerequisites Before Refactoring

### Build Status: BLOCKED

The codebase currently **does not compile** due to API signature mismatches. This **MUST** be fixed before any refactoring begins.

#### Compilation Errors
```
error[E0061]: this method takes 5 arguments but 4 arguments were supplied
   --> crates/cb-services/src/services/file_service.rs:141:18
    |
141 |                 .update_imports_for_rename(&old_abs, &new_abs, None, true)
    |                  ^^^^^^^^^^^^^^^^^^^^^^^^^-------------------------------- argument #5 of type `std::option::Option<ScanScope>` is missing
```

**Root Cause:** The `ImportService::update_imports_for_rename()` method signature was updated to include a new `scan_scope: Option<cb_ast::language::ScanScope>` parameter, but not all call sites were updated.

**Affected Files:**
- `crates/cb-services/src/services/file_service.rs` (lines 141, 176, 300+)

#### Required Fix

Add `scan_scope: None` parameter to all `update_imports_for_rename` calls:

```rust
// Line 141 - Fix:
.update_imports_for_rename(&old_abs, &new_abs, None, true, None)

// Line 176 - Fix:
.update_imports_for_rename(&old_abs, &new_abs, None, false, None)

// Line 300+ - Fix: Add scan_scope parameter
```

**Status Check Required:**
```bash
# 1. Verify fix
cargo build --all

# 2. Run tests
cargo test --all

# 3. Quality checks
make check

# 4. Verify duplication baseline
make check-duplicates
```

---

## Executive Summary

This proposal outlines a systematic plan to reduce code duplication in the codebuddy codebase from **4.67%** to approximately **2.7%**, eliminating 420-550 duplicate lines across 10 high-impact refactoring phases. The focus is on critical shared logic including workflow execution (90 duplicate lines), AppState construction (60+ lines), and remote command execution patterns.

### Why 92% Confidence?

**Confidence reduced from 98.5% → 92.0% due to:**
1. **Build failure discovered** (-5%): Indicates some churn in codebase
2. **Type name correction** (-1%): Used incorrect type reference in initial analysis
3. **Partial implementation** (-0.5%): Phase 6 partially obsolete

**Path to 100% Confidence:**
1. ✅ Fix build errors (prerequisite)
2. ✅ Run full test suite to establish baseline
3. ✅ Verify no other API changes in flight
4. ✅ Get stakeholder confirmation that no major refactoring is ongoing

### Critical Findings Summary

1. **Build Failure:** Three missing `scan_scope: None` parameters in file_service.rs
2. **Type Correction:** All references to `OperationResult` should be `DryRunnable<Value>` from `cb_core::dry_run`
3. **Phase 6 Update:** `file_operation_handler.rs` already has `execute_remote_command` implemented - only `refactoring_handler.rs` needs updating
4. **Phase 7 Update:** Should use existing `DryRunnable<T>` type instead of creating new wrapper

---

## Current State Analysis

### Duplicate Code Distribution

| Area | Lines | Tokens | Priority | Risk | Notes |
|------|-------|--------|----------|------|-------|
| Workflow Executor | 90 | 626 | Critical | Low | Phase 2 - Highest ROI |
| AppState Construction | 60-80 | 450-500 | High | Medium | Phase 3 - Needs build fix first |
| Remote Command Execution | 30-40 | 200-300 | Critical | Low | Phase 6 - Already partially done |
| Dry-Run Result Wrapping | 40-50 | 300-350 | Medium | Very Low | Phase 7 - Use existing DryRunnable |
| Refactoring Handler Patterns | 40-50 | 300-350 | Medium | Medium | Phase 9 |
| Context Conversion (Handlers) | 60-80 | 400-500 | Low | Very Low | Phase 8 |
| Manifest Update Logic | 37 | 305 | Medium | Low | Phase 5 |
| Command Existence Checks | 12 | 108 | High | Very Low | Phase 4 - Quick win |
| Test Helpers | 30-40 | 200-300 | Low | Very Low | Phase 10 |

### Root Causes

1. **Evolutionary Architecture:** Legacy handlers duplicated before unified architecture
2. **Missing Abstractions:** No shared utilities module for common operations
3. **Copy-Paste Development:** Similar patterns repeated across handlers
4. **Test Code Duplication:** AppState setup repeated in multiple test files
5. **Pre-Refactoring State:** Some handlers await migration to unified pattern

---

## Detailed Refactoring Plan

### Phase 1: Create Shared Utilities Module ⭐ Foundation

**Objective:** Establish infrastructure for shared code

**Files to CREATE:**
- `crates/cb-core/src/utils/mod.rs`
  ```rust
  //! Common utilities for codebuddy
  pub mod system;
  pub mod app_state_factory;
  ```

- `crates/cb-core/src/utils/system.rs`
  ```rust
  //! System-level utilities

  /// Check if a command exists on the system's PATH
  pub fn command_exists(cmd: &str) -> bool {
      std::process::Command::new(if cfg!(target_os = "windows") {
          "where"
      } else {
          "command"
      })
      .arg("-v")
      .arg(cmd)
      .stdout(std::process::Stdio::null())
      .stderr(std::process::Stdio::null())
      .status()
      .is_ok_and(|status| status.success())
  }
  ```

- `crates/cb-core/src/utils/app_state_factory.rs`
  ```rust
  //! Factory for creating AppState instances
  //! Consolidates duplicate initialization logic

  use cb_ast::AstCache;
  use cb_services::services::*;
  use std::path::PathBuf;
  use std::sync::Arc;

  /// Bundle of core services used by AppState
  pub struct ServicesBundle {
      pub ast_service: Arc<dyn cb_protocol::AstService>,
      pub file_service: Arc<FileService>,
      pub lock_manager: Arc<LockManager>,
      pub operation_queue: Arc<OperationQueue>,
      pub planner: Arc<dyn planner::Planner>,
      pub workflow_executor: Arc<dyn workflow_executor::WorkflowExecutor>,
  }

  /// Create services bundle with default configuration
  pub fn create_services_bundle(
      project_root: &PathBuf,
      cache_settings: cb_ast::CacheSettings,
      plugin_manager: Arc<cb_plugins::PluginManager>,
  ) -> ServicesBundle {
      let ast_cache = Arc::new(AstCache::with_settings(cache_settings));
      let ast_service = Arc::new(DefaultAstService::new(ast_cache.clone()));
      let lock_manager = Arc::new(LockManager::new());
      let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));
      let file_service = Arc::new(FileService::new(
          project_root,
          ast_cache.clone(),
          lock_manager.clone(),
          operation_queue.clone(),
      ));
      let planner = Arc::new(planner::DefaultPlanner::new());
      let workflow_executor = Arc::new(
          workflow_executor::DefaultWorkflowExecutor::new(plugin_manager)
      );

      ServicesBundle {
          ast_service,
          file_service,
          lock_manager,
          operation_queue,
          planner,
          workflow_executor,
      }
  }

  /// Register MCP proxy plugin if feature is enabled
  #[cfg(feature = "mcp-proxy")]
  pub async fn register_mcp_proxy_if_enabled(
      plugin_manager: &Arc<cb_plugins::PluginManager>,
      external_mcp_config: Option<&cb_core::config::ExternalMcpConfig>,
  ) -> Result<(), cb_protocol::ApiError> {
      if let Some(config) = external_mcp_config {
          use cb_mcp_proxy::McpProxyPlugin;
          use cb_plugins::LanguagePlugin;

          tracing::info!(
              servers_count = config.servers.len(),
              "Registering MCP proxy plugin"
          );

          let mut plugin = McpProxyPlugin::new(config.servers.clone());
          plugin.initialize().await.map_err(|e| {
              cb_protocol::ApiError::plugin(
                  format!("Failed to initialize MCP proxy plugin: {}", e)
              )
          })?;

          plugin_manager
              .register_plugin("mcp-proxy", Arc::new(plugin))
              .await
              .map_err(|e| {
                  cb_protocol::ApiError::plugin(
                      format!("Failed to register MCP proxy plugin: {}", e)
                  )
              })?;
      }
      Ok(())
  }
  ```

- `crates/cb-handlers/src/utils/mod.rs`
  ```rust
  //! Handler-specific utilities
  pub mod remote_exec;
  pub mod dry_run;
  ```

- `crates/cb-handlers/src/utils/remote_exec.rs`
  ```rust
  //! Remote command execution utilities

  use cb_core::workspaces::WorkspaceManager;
  use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
  use reqwest;
  use serde_json::json;
  use std::time::Duration;
  use tracing::error;

  /// Execute a command on a remote workspace via the workspace agent
  pub async fn execute_remote_command(
      workspace_manager: &WorkspaceManager,
      workspace_id: &str,
      command: &str,
  ) -> ServerResult<String> {
      // Look up workspace
      let workspace = workspace_manager.get(workspace_id).ok_or_else(|| {
          ServerError::InvalidRequest(format!("Workspace '{}' not found", workspace_id))
      })?;

      // Build agent URL
      let agent_url = format!("{}/execute", workspace.agent_url);

      // Create HTTP client with timeout
      let client = reqwest::Client::builder()
          .timeout(Duration::from_secs(60))
          .build()
          .map_err(|e| {
              error!(error = %e, "Failed to create HTTP client");
              ServerError::Internal("HTTP client error".into())
          })?;

      // Execute command via agent
      let response = client
          .post(&agent_url)
          .json(&json!({ "command": command }))
          .send()
          .await
          .map_err(|e| {
              error!(
                  workspace_id = %workspace_id,
                  agent_url = %agent_url,
                  error = %e,
                  "Failed to send command to workspace agent"
              );
              ServerError::Internal(format!("Failed to reach workspace agent: {}", e))
          })?;

      // Check response status
      if !response.status().is_success() {
          let status = response.status();
          let error_text = response
              .text()
              .await
              .unwrap_or_else(|_| "Unknown error".to_string());
          error!(
              workspace_id = %workspace_id,
              status = %status,
              error = %error_text,
              "Workspace agent returned error"
          );
          return Err(ServerError::Internal(format!(
              "Workspace agent error ({}): {}",
              status, error_text
          )));
      }

      // Parse response
      let result: serde_json::Value = response.json().await.map_err(|e| {
          error!(error = %e, "Failed to parse agent response");
          ServerError::Internal("Failed to parse agent response".into())
      })?;

      // Extract stdout from response
      result
          .get("stdout")
          .and_then(|v| v.as_str())
          .map(|s| s.to_string())
          .ok_or_else(|| {
              error!("Agent response missing stdout field");
              ServerError::Internal("Invalid agent response format".into())
          })
  }
  ```

- `crates/cb-handlers/src/utils/dry_run.rs`
  ```rust
  //! Dry-run result wrapping utilities

  use cb_core::dry_run::DryRunnable;
  use cb_protocol::ApiResult as ServerResult;
  use serde_json::{json, Value};

  /// Wrap an operation result with dry-run status if applicable
  pub fn wrap_dry_run_result(result: DryRunnable<Value>) -> ServerResult<Value> {
      if result.dry_run {
          // Merge status into the result object instead of nesting
          if let Value::Object(mut obj) = result.result {
              obj.insert("status".to_string(), json!("preview"));
              Ok(Value::Object(obj))
          } else {
              // Fallback for non-object results
              Ok(json!({
                  "status": "preview",
                  "result": result.result
              }))
          }
      } else {
          Ok(result.result)
      }
  }
  ```

**Files to EDIT:**
- `crates/cb-core/src/lib.rs`
  - **Add:** `pub mod utils;` after other module declarations

- `crates/cb-handlers/src/lib.rs`
  - **Add:** `pub mod utils;` after other module declarations

**Impact:** 15-20 lines saved, establishes foundation for other phases
**Risk:** Very Low - New code only
**Effort:** 2-3 hours
**Confidence:** 95%

---

### Phase 2: Extract Workflow Executor Step Logic ⭐ Highest ROI

**Objective:** Eliminate 90 duplicate lines in workflow execution

**Files to EDIT:**
- `crates/cb-services/src/services/workflow_executor.rs`

**Changes:**

1. **Add new method** (after line 50):
```rust
/// Execute a single workflow step
///
/// This method handles parameter resolution, dry-run injection,
/// plugin request creation, and execution with logging.
async fn execute_workflow_step(
    &self,
    step: &cb_core::model::workflow::WorkflowStep,
    step_index: usize,
    workflow_name: &str,
    total_steps: usize,
    step_results: &mut HashMap<usize, Value>,
    log: &mut Vec<String>,
    dry_run: bool,
) -> ServerResult<Value> {
    debug!(
        step_index = step_index,
        tool = %step.tool,
        description = %step.description,
        "Executing workflow step"
    );

    // Resolve parameters using generic placeholder substitution
    let mut resolved_params = Self::resolve_step_params(&step.params, step_results)?;

    // If dry_run is enabled, add it to the parameters for all tools
    if dry_run {
        if let Value::Object(ref mut map) = resolved_params {
            map.insert("dry_run".to_string(), Value::Bool(true));
        }
    }

    debug!(params = ?resolved_params, dry_run = dry_run, "Resolved step parameters");

    // Create plugin request
    let file_path = resolved_params
        .get("file_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let plugin_request = PluginRequest {
        method: step.tool.clone(),
        file_path,
        position: None,
        range: None,
        params: resolved_params,
        request_id: None,
    };

    // Execute the step
    match self.plugin_manager.handle_request(plugin_request).await {
        Ok(response) => {
            let step_result = response.data.unwrap_or(json!({}));
            debug!(
                step_index = step_index,
                result = ?step_result,
                "Step completed successfully"
            );

            // Log successful step completion
            log.push(format!(
                "[Step {}/{}] SUCCESS: {} - {}",
                step_index + 1,
                total_steps,
                step.tool,
                step.description
            ));

            step_results.insert(step_index, step_result.clone());
            Ok(step_result)
        }
        Err(e) => {
            error!(
                step_index = step_index,
                step_description = %step.description,
                tool = %step.tool,
                workflow = %workflow_name,
                error = %e,
                "Step execution failed - halting workflow"
            );

            // Log the failure
            log.push(format!(
                "[Step {}/{}] FAILED: {} - {}. Error: {}",
                step_index + 1,
                total_steps,
                step.tool,
                step.description,
                e
            ));

            Err(ServerError::Runtime {
                message: format!(
                    "Workflow '{}' failed at step {}/{} ({}): {}. Error: {}",
                    workflow_name,
                    step_index + 1,
                    total_steps,
                    step.tool,
                    step.description,
                    e
                ),
            })
        }
    }
}
```

2. **Replace lines 213-296** in `execute_workflow()`:
```rust
// Replace the entire loop body with:
for (step_index, step) in workflow.steps.iter().enumerate() {
    // Check for pause request
    if step.requires_confirmation.unwrap_or(false) {
        // ... existing pause logic stays ...
    }

    let step_result = self.execute_workflow_step(
        step,
        step_index,
        &workflow.name,
        workflow.steps.len(),
        &mut step_results,
        &mut log,
        dry_run,
    ).await?;

    final_result = step_result;
}
```

3. **Replace lines 363-444** in `resume_workflow()`:
```rust
// Replace the entire loop body with:
for (step_index, step) in workflow
    .steps
    .iter()
    .enumerate()
    .skip(paused_state.step_index)
{
    let step_result = self.execute_workflow_step(
        step,
        step_index,
        &workflow.name,
        workflow.steps.len(),
        &mut step_results,
        &mut log,
        dry_run,
    ).await?;

    final_result = step_result;
}
```

**Impact:** 90 lines eliminated, improved maintainability
**Risk:** Low - Well-isolated logic with clear inputs/outputs
**Effort:** 3-4 hours including testing
**Confidence:** 95%

---

### Phase 3: Consolidate AppState Construction

**Objective:** Reduce 60-80 duplicate lines across initialization code

**Files to EDIT:**
- `crates/cb-server/src/lib.rs`

**Changes in `create_dispatcher()`** (lines 60-139):
```rust
// Replace lines 63-116 with:
use cb_core::utils::app_state_factory::{create_services_bundle, register_mcp_proxy_if_enabled};

let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

let cache_settings = cb_ast::CacheSettings::from_config(
    options.config.cache.enabled,
    options.config.cache.ttl_seconds,
    options.config.cache.max_size_bytes,
);

let plugin_manager = Arc::new(cb_plugins::PluginManager::new());

// Register MCP proxy plugin if feature enabled
#[cfg(feature = "mcp-proxy")]
register_mcp_proxy_if_enabled(&plugin_manager, options.config.external_mcp.as_ref()).await?;

let services = create_services_bundle(&project_root, cache_settings, plugin_manager.clone());

let workspace_manager = Arc::new(cb_core::workspaces::WorkspaceManager::new());

let app_state = Arc::new(AppState {
    ast_service: services.ast_service,
    file_service: services.file_service,
    planner: services.planner,
    workflow_executor: services.workflow_executor,
    project_root,
    lock_manager: services.lock_manager,
    operation_queue: services.operation_queue,
    start_time: std::time::Instant::now(),
    workspace_manager,
});
```

**Changes in `create_dispatcher_with_workspace()`** (lines 176-229):
```rust
// Similar replacement pattern
```

- `crates/cb-handlers/src/handlers/plugin_dispatcher.rs`

**Changes in `create_test_dispatcher()`** (lines 510-547):
```rust
// Replace lines 511-544 with:
use cb_core::utils::app_state_factory::create_services_bundle;

let temp_dir = tempfile::TempDir::new().unwrap();
let project_root = temp_dir.path().to_path_buf();
let cache_settings = cb_ast::CacheSettings::default();
let plugin_manager = Arc::new(PluginManager::new());

let services = create_services_bundle(&project_root, cache_settings, plugin_manager.clone());

let workspace_manager = Arc::new(WorkspaceManager::new());

let app_state = Arc::new(AppState {
    ast_service: services.ast_service,
    file_service: services.file_service,
    planner: services.planner,
    workflow_executor: services.workflow_executor,
    project_root,
    lock_manager: services.lock_manager,
    operation_queue: services.operation_queue,
    start_time: std::time::Instant::now(),
    workspace_manager,
});

PluginDispatcher::new(app_state, plugin_manager)
```

**Impact:** 60-80 lines eliminated
**Risk:** Medium - Affects initialization paths
**Effort:** 4-5 hours including thorough testing
**Confidence:** 90% - Needs build fix first

---

### Phase 4: Unify Command Existence Checking

**Objective:** Consolidate duplicate `command_exists` functions

**Files to EDIT:**
- `apps/codebuddy/src/cli.rs`

**Replace function** (lines 377-390):
```rust
// Remove function definition, replace calls with:
use cb_core::utils::system::command_exists;
```

- `crates/cb-client/src/commands/doctor.rs`

**Replace method** (lines 72-84):
```rust
// Change method to:
fn command_exists(&self, cmd: &str) -> bool {
    cb_core::utils::system::command_exists(cmd)
}
```

**Impact:** 12 lines eliminated, single source of truth
**Risk:** Very Low - Simple utility function
**Effort:** 30 minutes
**Confidence:** 98%

---

### Phase 5: Extract Workspace Manifest Update Logic

**Objective:** Reduce 37 duplicate lines in workspace handler

**Files to EDIT:**
- `crates/cb-handlers/src/handlers/tools/workspace.rs`

**Add helper method** (after line 139):
```rust
/// Update a dependency in a manifest file
async fn update_manifest_dependency(
    manifest_path: &str,
    old_dep_name: &str,
    new_dep_name: &str,
    new_path: Option<&str>,
) -> ServerResult<()> {
    use std::path::Path;
    use tokio::fs;

    // Read the manifest file
    let content = fs::read_to_string(manifest_path).await.map_err(|e| {
        cb_protocol::ApiError::Internal(format!(
            "Failed to read manifest file at {}: {}",
            manifest_path, e
        ))
    })?;

    // Use the manifest factory to get the correct handler
    let path = Path::new(manifest_path);
    let mut manifest = cb_ast::manifest::load_manifest(path, &content).map_err(|e| {
        cb_protocol::ApiError::Internal(format!("Failed to load manifest: {}", e))
    })?;

    // Update the dependency using the generic trait method
    manifest
        .rename_dependency(old_dep_name, new_dep_name, new_path)
        .map_err(|e| {
            cb_protocol::ApiError::Internal(format!("Failed to update dependency: {}", e))
        })?;

    // Write the updated content back
    let updated_content = manifest.to_string().map_err(|e| {
        cb_protocol::ApiError::Internal(format!("Failed to serialize manifest: {}", e))
    })?;

    fs::write(manifest_path, updated_content)
        .await
        .map_err(|e| {
            cb_protocol::ApiError::Internal(format!(
                "Failed to write manifest file at {}: {}",
                manifest_path, e
            ))
        })?;

    Ok(())
}
```

**Modify `handle_rename_dependency()`** (lines 180-215):
```rust
// Replace with call to helper:
Self::update_manifest_dependency(manifest_path, old_dep_name, new_dep_name, new_path).await?;
```

**Modify `update_single_dependency()`** (lines 348-385):
```rust
// Replace entire method body with:
Self::update_manifest_dependency(manifest_path, old_dep_name, new_dep_name, new_path).await
```

**Impact:** 37 lines eliminated
**Risk:** Low - Well-scoped logic
**Effort:** 1-2 hours
**Confidence:** 95%

---

### Phase 6: Consolidate Remote Command Execution ⚠️ PARTIALLY COMPLETE

**Objective:** Eliminate duplicate remote execution code (30-40 lines remaining)

**Current Status:** `file_operation_handler.rs` already has `execute_remote_command` implemented. Only `refactoring_handler.rs` needs updating.

**Files to EDIT:**
- `crates/cb-handlers/src/handlers/refactoring_handler.rs`

**Add import:**
```rust
use crate::utils::remote_exec::execute_remote_command;
```

**Replace duplicate calls** in `handle_refactoring()`:
```rust
// Replace Self::execute_remote_command(...) with:
execute_remote_command(&context.app_state.workspace_manager, workspace_id, &command).await
```

**Impact:** 30-40 lines eliminated (reduced from 50-60)
**Risk:** Low - Moving to shared module
**Effort:** 1 hour (reduced from 2 hours)
**Confidence:** 90%

---

### Phase 7: Extract Dry-Run Result Wrapping

**Objective:** Consolidate dry-run result formatting (40-50 lines)

**Note:** Uses existing `DryRunnable<T>` type from `cb_core::dry_run` instead of creating new wrapper.

**Files to EDIT:**
- `crates/cb-handlers/src/handlers/file_operation_handler.rs`

**Add import:**
```rust
use crate::utils::dry_run::wrap_dry_run_result;
```

**Replace logic in methods:**
- `handle_rename_file()` (lines 178-192)
- `handle_rename_directory()` (lines 223-237)
- `handle_create_file()`
- `handle_delete_file()`

**Replace with:**
```rust
let result = context.app_state.file_service
    .rename_file_with_imports(Path::new(old_path), Path::new(new_path), dry_run)
    .await?;
wrap_dry_run_result(result)
```

**Impact:** 40-50 lines eliminated
**Risk:** Very Low - Simple formatting logic
**Effort:** 1 hour (reduced from 1-2 hours)
**Confidence:** 92%

---

### Phase 8: Refactor Tool Handler Context Conversion

**Objective:** Use macro to eliminate repetitive context conversion (60-80 lines)

**Files to CREATE:**
- `crates/cb-handlers/src/macros.rs`
```rust
//! Macros for reducing handler boilerplate

/// Delegate tool call to legacy handler with automatic context conversion
#[macro_export]
macro_rules! delegate_to_legacy {
    ($self:expr, $context:expr, $tool_call:expr) => {{
        let legacy_context = $crate::handlers::compat::ToolContext {
            app_state: $context.app_state.clone(),
            plugin_manager: $context.plugin_manager.clone(),
            lsp_adapter: $context.lsp_adapter.clone(),
        };
        $self.legacy_handler
            .handle_tool($tool_call.clone(), &legacy_context)
            .await
    }};
}
```

**Files to EDIT:**
- `crates/cb-handlers/src/lib.rs`
  - **Add:** `#[macro_use] mod macros;`

- `crates/cb-handlers/src/handlers/tools/file_ops.rs` (lines 38-54):
```rust
async fn handle_tool_call(
    &self,
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    delegate_to_legacy!(self, context, tool_call)
}
```

- Apply same pattern to:
  - `editing.rs`
  - `lifecycle.rs`
  - `system.rs`
  - `navigation.rs`

**Impact:** 60-80 lines eliminated
**Risk:** Very Low - Macro just wraps existing pattern
**Effort:** 1-2 hours
**Confidence:** 95%

---

### Phase 9: Consolidate Refactoring Handler Patterns

**Objective:** Extract common file reading and LSP service creation (40-50 lines)

**Files to EDIT:**
- `crates/cb-handlers/src/handlers/refactoring_handler.rs`

**Add helper methods** (after line 127):
```rust
/// Read file content from local filesystem or remote workspace
async fn read_file_content(
    workspace_id: Option<&str>,
    file_path: &str,
    file_service: &cb_services::services::FileService,
    workspace_manager: &cb_core::workspaces::WorkspaceManager,
) -> ServerResult<String> {
    if let Some(workspace_id) = workspace_id {
        let command = format!("cat '{}'", Self::escape_shell_arg(file_path));
        crate::utils::remote_exec::execute_remote_command(
            workspace_manager,
            workspace_id,
            &command,
        )
        .await
    } else {
        file_service
            .read_file(Path::new(file_path))
            .await
    }
}

/// Create LSP refactoring service wrapper from adapter
async fn create_lsp_service(
    lsp_adapter: &Arc<Mutex<Option<Arc<DirectLspAdapter>>>>,
) -> Option<LspRefactoringServiceWrapper> {
    let adapter_guard = lsp_adapter.lock().await;
    adapter_guard
        .as_ref()
        .map(|adapter| LspRefactoringServiceWrapper::new(adapter.clone()))
}
```

**Refactor** `handle_refactoring()`:
- Replace file reading patterns in `extract_function`, `inline_variable`, `extract_variable`
- Replace LSP service creation patterns

**Impact:** 40-50 lines eliminated
**Risk:** Medium - Core refactoring logic
**Effort:** 3-4 hours
**Confidence:** 88% - Most complex phase, needs LSP understanding

---

### Phase 10: Test Helper Consolidation

**Objective:** Reduce test setup duplication (30-40 lines)

**Files to CREATE:**
- `crates/cb-server/tests/common/mod.rs`
```rust
//! Shared test utilities

use cb_handlers::handlers::plugin_dispatcher::AppState;
use std::sync::Arc;
use tempfile::TempDir;

pub fn create_test_app_state() -> (Arc<AppState>, TempDir) {
    use cb_core::utils::app_state_factory::create_services_bundle;

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path().to_path_buf();
    let cache_settings = cb_ast::CacheSettings::default();
    let plugin_manager = Arc::new(cb_plugins::PluginManager::new());

    let services = create_services_bundle(&project_root, cache_settings, plugin_manager);
    let workspace_manager = Arc::new(cb_core::workspaces::WorkspaceManager::new());

    let app_state = Arc::new(AppState {
        ast_service: services.ast_service,
        file_service: services.file_service,
        planner: services.planner,
        workflow_executor: services.workflow_executor,
        project_root,
        lock_manager: services.lock_manager,
        operation_queue: services.operation_queue,
        start_time: std::time::Instant::now(),
        workspace_manager,
    });

    (app_state, temp_dir)
}
```

**Files to EDIT:**
- `crates/cb-server/tests/phase2_integration.rs`
- `crates/cb-server/tests/phase3_integration.rs`
- Replace test setup with: `let (app_state, _temp) = common::create_test_app_state();`

**Impact:** 30-40 lines eliminated
**Risk:** Very Low - Test code only
**Effort:** 1-2 hours
**Confidence:** 98%

---

## Updated Impact Summary

| Phase | Original Lines | Adjusted Lines | Confidence | Notes |
|-------|---------------|----------------|------------|-------|
| 1 | 15-20 | 15-20 | 95% | No changes |
| 2 | 90 | 90 | 95% | No changes |
| 3 | 60-80 | 60-80 | 90% | Needs build fix first |
| 4 | 12 | 12 | 98% | No changes |
| 5 | 37 | 37 | 95% | No changes |
| 6 | 50-60 | 30-40 | 90% | **Already partially done** |
| 7 | 40-50 | 40-50 | 92% | **Use existing DryRunnable** |
| 8 | 60-80 | 60-80 | 95% | No changes |
| 9 | 40-50 | 40-50 | 88% | Most complex, needs LSP understanding |
| 10 | 30-40 | 30-40 | 98% | Test code only |

**Updated Total Reduction:** 420-550 lines (adjusted from 450-570)
**Updated Target Duplication:** ~2.7% (adjusted from 2.5%)

---

## Implementation Strategy

### Recommended Order

1. **Prerequisites** (CRITICAL)
   - Fix build errors in file_service.rs
   - Run full test suite
   - Verify baseline with `make check-duplicates`
   - ~30 minutes

2. **Quick Wins First** (Phases 1, 4)
   - Build momentum with low-risk changes
   - Establish infrastructure
   - ~2-3 hours total

3. **High-Impact Core** (Phases 2, 6)
   - Address largest duplications
   - Critical shared logic
   - ~4-5 hours total

4. **Medium Complexity** (Phases 3, 5, 7)
   - AppState consolidation
   - Helper extraction
   - ~6-8 hours total

5. **Polish & Refinement** (Phases 8, 9, 10)
   - Macro improvements
   - Test cleanup
   - ~5-7 hours total

**Total Estimated Effort:** 18-24 hours

### Per-Phase Checklist

After each phase:
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo test` all tests pass
- [ ] `make check-duplicates` shows expected reduction
- [ ] No breaking API changes
- [ ] Update CHANGELOG.md
- [ ] Commit with descriptive message

---

## Risk Management

### Identified Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Build failures | High | Critical | **Fix before starting** |
| Test failures | Medium | High | Run full test suite after each phase |
| Breaking changes | Low | High | Careful API review, maintain compatibility |
| Increased complexity | Low | Medium | Keep abstractions simple, document well |
| Merge conflicts | Medium | Low | Small atomic commits |
| Performance regression | Very Low | Medium | Benchmark critical paths |

### Rollback Strategy

- Each phase is independent and can be reverted
- Use feature branches for complex phases
- Tag stable points: `refactor/phase-N-complete`

---

## Success Metrics

### Quantitative
- **Primary:** Duplication reduced from 4.67% to ≤ 2.7%
- Lines of code: -420 to -550 lines
- Clone count: 90 → ~50-55 clones
- Test coverage: Maintained or improved

### Qualitative
- Improved code maintainability
- Clearer separation of concerns
- Easier onboarding for new contributors
- Reduced cognitive load when reading code

---

## Timeline

| Week | Phases | Deliverables |
|------|--------|--------------|
| 0 | Build Fix | Compiling codebase, passing tests |
| 1 | 1, 4 | Foundation + quick wins |
| 2 | 2, 6 | Core deduplication |
| 3 | 3, 5, 7 | Medium complexity items |
| 4 | 8, 9, 10 | Polish + documentation |

**Total Duration:** 4 weeks (part-time) or 2 weeks (full-time focus)

---

## Immediate Next Steps

### Before ANY Refactoring:

1. **Fix Build** (Priority: CRITICAL)
   ```bash
   # Fix the three call sites in file_service.rs
   # Add scan_scope: None parameter
   cargo build --all
   cargo test --all
   ```

2. **Verify Baseline** (Priority: HIGH)
   ```bash
   make check
   make check-duplicates
   ```

3. **Communication** (Priority: HIGH)
   - Inform team of build failure
   - Confirm no other major changes in progress
   - Get green light before starting refactoring

---

## References

- Duplicate Code Analysis: `.build/jscpd-report/html/index.html`
- Current Metrics: 4.67% duplication (1224 lines, 10733 tokens, 90 clones)
- Build Issues: PROPOSAL_DEDUPLICATE_CRITICAL_FINDINGS.md (archived)
- Related Documents: None
- External Resources:
  - [Refactoring Guru - Duplicated Code](https://refactoring.guru/smells/duplicate-code)
  - [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

---

**Document Version:** 2.0 (Consolidated)
**Last Updated:** 2025-10-03
**Next Review:** After build fix + Phase 2 completion
**Status:** BLOCKED - Build must pass before refactoring can proceed
