# Proposal: Fix Tool Listing for AdvancedHandler Tools

## Problem

`batch_execute` and `apply_edits` tools are **callable** but **not discoverable** via `codebuddy tools` list.

**Root Cause:** Two parallel routing systems exist:
- **ToolRegistry** (legacy) - Used for `tools/call` execution only
- **PluginManager** (new) - Used for BOTH `tools/list` discovery AND execution

AdvancedHandler tools are registered ONLY with ToolRegistry, making them invisible to tool discovery.

## Solution

Create `AdvancedToolsPlugin` following the same pattern as `SystemToolsPlugin`.

**Architecture:**
- Plugin provides tool definitions for discovery
- ToolRegistry continues to handle execution
- Zero breaking changes

## Changes Required

### 1. CREATE: `crates/cb-plugins/src/advanced_tools_plugin.rs`

New LanguagePlugin implementation exposing tool schemas for `batch_execute` and `apply_edits`.

**Pattern:** Identical to `SystemToolsPlugin` - provides `tool_definitions()`, delegates execution to AdvancedHandler.

### 2. EDIT: `crates/cb-plugins/src/lib.rs`

Add module export:
```rust
pub mod advanced_tools_plugin;
pub use advanced_tools_plugin::AdvancedToolsPlugin;
```

### 3. EDIT: `crates/cb-server/src/handlers/plugin_dispatcher.rs`

Register plugin in `initialize()` after SystemToolsPlugin (line ~206):
```rust
let advanced_plugin = Arc::new(cb_plugins::AdvancedToolsPlugin::new());
self.plugin_manager
    .register_plugin("advanced", advanced_plugin)
    .await
    .map_err(|e| ServerError::Internal(format!("Failed to register Advanced tools plugin: {}", e)))?;
```

## Verification

```bash
# Should now show batch_execute and apply_edits
cargo run --bin codebuddy -- tools | grep -E "(batch_execute|apply_edits)"

# Should still execute successfully
cargo run --bin codebuddy -- tool batch_execute '{"operations":[...]}'
```

## Impact

- ✅ Tools become discoverable
- ✅ Execution remains unchanged
- ✅ No breaking changes
- ✅ Follows established patterns
- ✅ Future-proof for ToolRegistry removal

**Estimated effort:** 30 minutes implementation + testing
