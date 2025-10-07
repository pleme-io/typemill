# Proposal: Extend batch_execute to Support All MCP Operations

**Status:** Draft
**Created:** 2025-10-04
**Goal:** Make `batch_execute` work with all MCP tools, enabling removal of redundant `update_dependency` and `batch_update_dependencies` tools

---

## Problem

`batch_execute` currently only supports 4 file operations:
- CreateFile, WriteFile, DeleteFile, RenameFile

This limitation makes it impossible to batch dependency updates with file changes, leaving `update_dependency` and `batch_update_dependencies` as redundant standalone tools.

---

## Proposed Solution

Extend the `BatchOperation` enum to include dependency management operations.

### Code Changes

**Location:** `/workspace/crates/cb-handlers/src/handlers/tools/advanced.rs`

**Add to BatchOperation enum:**

```rust
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BatchOperation {
    // Existing file operations
    CreateFile { path: String, content: Option<String>, dry_run: Option<bool> },
    DeleteFile { path: String, dry_run: Option<bool> },
    WriteFile { path: String, content: String, dry_run: Option<bool> },
    RenameFile { old_path: String, new_path: String, dry_run: Option<bool> },

    // NEW: Dependency operations
    UpdateDependency {
        manifest_path: Option<String>,
        dependency_name: String,
        version: String,
        dry_run: Option<bool>,
    },
}
```

**Add handling in batch_execute match statement:**

```rust
BatchOperation::UpdateDependency {
    manifest_path,
    dependency_name,
    version,
    dry_run,
} => {
    // Delegate to WorkspaceHandler's existing update_dependency logic
    let tool_call = ToolCall {
        name: "update_dependency".to_string(),
        arguments: Some(json!({
            "manifest_path": manifest_path,
            "dependency_name": dependency_name,
            "version": version,
            "dry_run": dry_run.unwrap_or(false),
        })),
    };

    context.app_state
        .workspace_handler
        .handle_update_dependency(context, &tool_call)
        .await?
}
```

---

## Usage Example

```json
{
  "method": "tools/call",
  "params": {
    "name": "batch_execute",
    "arguments": {
      "operations": [
        {
          "type": "update_dependency",
          "dependency_name": "tokio",
          "version": "1.35.0"
        },
        {
          "type": "update_dependency",
          "dependency_name": "serde",
          "version": "1.0.195"
        },
        {
          "type": "write_file",
          "path": "CHANGELOG.md",
          "content": "Updated tokio and serde"
        }
      ]
    }
  }
}
```

---

## Benefits

1. **Single batch tool** for all operations (file + dependency)
2. **Removes redundancy** - can delete `update_dependency` and `batch_update_dependencies`
3. **Simpler API** - 40 → 38 public tools
4. **Better UX** - batch dependency updates with related file changes atomically

---

## Implementation Notes

- Reuse existing WorkspaceHandler methods (no new logic needed)
- Dry-run support already exists in dependency handlers
- Estimated effort: ~50 lines of code
- No breaking changes to existing batch_execute usage
