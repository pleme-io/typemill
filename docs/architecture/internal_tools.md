# Internal Tools Policy

Understanding which tools are visible to AI agents and which are reserved for backend operations.

## Table of Contents
- [Overview](#overview)
- [Internal vs Public Tools](#internal-vs-public-tools)
- [Current Internal Tools](#current-internal-tools)
- [Marking Tools as Internal](#marking-tools-as-internal)
- [When to Make a Tool Internal](#when-to-make-a-tool-internal)
- [Implementation Details](#implementation-details)

## Overview

TypeMill distinguishes between **public tools** (exposed to AI agents via MCP) and **internal tools** (backend-only, hidden from MCP listings).

## Internal vs Public Tools

### Public Tools (Visible to AI Agents)
- Listed in MCP `tools/list` requests
- Designed for direct use by AI agents
- Well-documented, user-friendly APIs
- Examples: `find_definition`, `rename`, `extract`, `read_file`

### Internal Tools (Hidden from AI Agents)
- **Hidden** from MCP `tools/list` requests
- **Still callable** via direct tool invocation (for backend use)
- Used for system plumbing, protocol interop, and backend coordination
- Examples: `notify_file_opened`, `rename_symbol_with_imports`

## Current Internal Tools

### Lifecycle Hooks (3 tools)
**Handler**: `LifecycleHandler`

- `notify_file_opened`
- `notify_file_saved`
- `notify_file_closed`

**Rationale**: These are backend hooks for editors/IDEs to notify LSP servers and trigger plugin lifecycle events. AI agents don't "open/close files" in the editor sense - they directly read/write files. No value for AI code modification tasks.

## Marking Tools as Internal

### In Code

Implement `is_internal()` in your `ToolHandler`:

```rust
# [async_trait]
impl ToolHandler for MyHandler {
    fn tool_names(&self) -> &[&str] {
        &["my_internal_tool"]
    }

    fn is_internal(&self) -> bool {
        true  // Hide from MCP listings
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Implementation
    }
}
```text
### Documentation Requirements

When marking a tool as internal, you **must**:

1. **Add an entry to this file** under "Current Internal Tools"
2. **Document the rationale** - why is it internal?
3. **Add inline code comments** explaining the decision
4. **Update claude.md** tool count if necessary

## When to Make a Tool Internal

A tool should be internal if it meets **any** of these criteria:

### 1. Backend Plumbing
- Used for internal system coordination
- No direct value to end users
- Examples: lifecycle hooks, health check internals

### 2. Protocol Leakage
- Exposes low-level protocol details (LSP, language-specific internals)
- AI agents should use higher-level abstractions
- Example: `apply_workspace_edit` (LSP protocol format)

### 3. Redundancy
- Functionality fully covered by other public tools
- Internal convenience for specific backend use cases
- Example: `rename_symbol_with_imports` (covered by unified refactoring API: `rename` with `dryRun` option)

### 4. Dangerous/Unstable
- Experimental features not ready for general use
- Could cause data loss or corruption if misused
- Should be reviewed before making public

## Current Internal Tools (Continued)

### Workflow Orchestration (2 tools)
**Handler**: `InternalEditingHandler`, `InternalWorkspaceHandler`

- `rename_symbol_with_imports`
- `apply_workspace_edit`

**Rationale**: These are legacy workflow wrappers and low-level LSP protocol interop tools. AI agents should use the unified refactoring API instead:
- Instead of `rename_symbol_with_imports`: Use `rename` with `options.dryRun: false`
- Instead of `apply_workspace_edit` (low-level): Use the public refactoring tools (`rename`, `extract`, `inline`, etc.) with built-in execution mode

The internal `apply_workspace_edit` accepts raw LSP `WorkspaceEdit` format, while the public refactoring tools accept structured parameters with enhanced safety features (checksums, validation, rollback, default preview mode).

## Implementation Details

### ToolHandler Trait

```rust
pub trait ToolHandler: Send + Sync {
    fn tool_names(&self) -> &[&str];

    fn is_internal(&self) -> bool {
        false  // Default: tools are public
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value>;
}
```text
### ToolRegistry Behavior

- `list_tools()` - Returns **only public tools** (for MCP)
- `list_internal_tools()` - Returns internal tools (for diagnostics)
- `list_public_tools_with_handlers()` - Returns public tools with handler info (for CLI)
- `list_tools_with_handlers()` - Returns **all tools** with handler info (for debugging)
- `handle_tool()` - Executes **all tools** (public and internal)

This ensures:
- AI agents only see relevant, user-friendly tools
- Backend code can still call internal tools when needed
- Clear separation between public API and internal plumbing

## CLI Tool Listing

The `mill tools` command lists **only public tools** (those visible to AI agents via MCP).

Internal tools are hidden from this listing but remain:
- Callable via `mill tool <internal-tool-name> <args>` (for testing/debugging)
- Registered in the system for backend workflows
- Documented in this file

**Example output:**
```bash
$ mill tools
┌────────────────────────────────┬────────────────────┐
│ TOOL NAME                      │ HANDLER            │
├────────────────────────────────┼────────────────────┤
│ find_definition                │ NavigationHandler  │
│ rename                         │ RenameHandler      │
│ extract                        │ ExtractHandler     │
└────────────────────────────────┴────────────────────┘

Public tools: 28 across handlers
(Internal tools hidden - backend-only tools not shown)
```text
**To see internal tools programmatically**, use the Rust API:
```rust
let registry = dispatcher.tool_registry.lock().await;
let internal_tools = registry.list_internal_tools();
// Returns: ["notify_file_opened", "notify_file_saved", ...]

let all_tools = registry.list_tools_with_handlers();
// Returns all 44 tools (public + internal) with handler names
```text
## Testing

### Public Tool Tests
Test that tools are **listed** in MCP responses:
```rust
let tools = registry.list_tools();
assert!(tools.contains(&"find_definition".to_string()));
```text
### Internal Tool Tests
Test that internal tools are:
1. **Not listed** in public listings
2. **Still callable** via handle_tool

```rust
let tools = registry.list_tools();
assert!(!tools.contains(&"notify_file_opened".to_string()));

let internal_tools = registry.list_internal_tools();
assert!(internal_tools.contains(&"notify_file_opened".to_string()));

// But still callable
let result = registry.handle_tool(
    ToolCall {
        name: "notify_file_opened".to_string(),
        arguments: Some(json!({"file_path": "test.rs"})),
    },
    &context,
).await;
assert!(result.is_ok());
```text
## Migration Guide

### Making an Existing Tool Internal

1. **Add `is_internal()` override** to the handler
2. **Update this documentation** with rationale
3. **Update test expectations** (tool count, listings)
4. **Update claude.md** if tool count changed
5. **Announce in changelog** if user-facing

### Making an Internal Tool Public

Same steps, but document why it's now useful for AI agents.

## FAQ

**Q: Can internal tools still be called by backend code?**
A: Yes! Internal tools are only hidden from MCP `tools/list`. They're still fully functional and callable via `handle_tool()`.

**Q: What happens if an AI agent tries to call an internal tool?**
A: It works! The tool executes normally. We just don't advertise it in the tool list.

**Q: Should I hide tools that are rarely used?**
A: No. "Internal" is about purpose, not frequency. If a tool is legitimately useful for AI agents, keep it public even if rarely used.

**Q: Can I have a handler with both public and internal tools?**
A: No. `is_internal()` applies to all tools in a handler. If you need mixed visibility, split into separate handlers.

## See Also

- **[Tools Visibility Specification](tools_visibility_spec.md)** - Detailed specification for tool visibility rules
- [`ToolHandler` trait documentation](../../../../crates/mill-handlers/src/handlers/tools/mod.rs)
- [`ToolRegistry` implementation](../../../../crates/mill-handlers/src/handlers/tool_registry.rs)
- [Tool registration tests](../../../../crates/mill-server/tests/tool_registration_test.rs)