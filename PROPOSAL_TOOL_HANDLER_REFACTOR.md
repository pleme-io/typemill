# Proposal: Tool Handler Architecture Refactor

**Status:** Draft
**Date:** 2025-10-02
**Author:** Architecture Review
**Effort:** 1-2 weeks
**Risk:** Low

---

## Problem Statement

The current codebase has three entry points (CLI, stdio, WebSocket) with the following issues:

1. **Dispatcher initialization duplicated 3x** across `main.rs` and `cli.rs`
2. **Hardcoded tool routing** with 7+ if/else branches in `plugin_dispatcher.rs`
3. **2306-line god object** (`plugin_dispatcher.rs`) handling all special operations
4. **CLI lacks parity** - cannot list tools or use full MCP protocol
5. **Adding a tool requires editing multiple files** (dispatcher routing, category helpers, etc.)

**Impact:** High maintenance burden, error-prone changes, limited extensibility.

---

## Proposed Solution

Introduce a **ToolHandler trait pattern** alongside the existing `LanguagePlugin` system, creating a two-tier architecture:

- **Tier 1:** `LanguagePlugin` - LSP-based operations (existing, unchanged)
- **Tier 2:** `ToolHandler` - Special operations (new, for file ops, workflows, system tools)

**Key Components:**

1. **Shared dispatcher factory** - eliminates initialization duplication
2. **ToolHandler trait** - standardized interface for non-LSP operations
3. **ToolRegistry** - automatic routing based on registered handlers
4. **Dedicated handlers** - file operations, workflows, system tools, refactoring

---

## Design

### Architecture Diagram

```
┌─────────────────┐
│  MCP Tool Call  │
└────────┬────────┘
         │
         ▼
┌─────────────────────────┐
│  PluginDispatcher       │
│  ├─ tool_registry       │──┐
│  └─ plugin_manager      │──┤
└─────────────────────────┘  │
                              │
         ┌────────────────────┴────────────────┐
         │                                     │
         ▼                                     ▼
┌────────────────────┐          ┌─────────────────────┐
│  ToolHandler       │          │  LanguagePlugin     │
│  (special ops)     │          │  (LSP ops)          │
├────────────────────┤          ├─────────────────────┤
│ • FileOps          │          │ • find_definition   │
│ • Workflows        │          │ • rename_symbol     │
│ • System           │          │ • get_hover         │
│ • Refactoring      │          │ (existing, stable)  │
└────────────────────┘          └─────────────────────┘
```

### Core Interfaces

```rust
// New trait for special operations
trait ToolHandler: Send + Sync {
    fn supported_tools(&self) -> Vec<&'static str>;
    async fn handle_tool(&self, tool_call: ToolCall, context: Arc<ToolContext>) -> Result<Value>;
}

// Registry for automatic routing
struct ToolRegistry {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

// Context provided to handlers
struct ToolContext {
    pub app_state: Arc<AppState>,
    pub plugin_manager: Arc<PluginManager>,
}
```

---

## Implementation Plan

### Phase 1: Foundation (Days 1-3)

**New Files:**
- `apps/server/src/dispatcher_factory.rs` - Shared initialization
- `crates/cb-server/src/tool_handler.rs` - ToolHandler trait
- `crates/cb-server/src/tool_registry.rs` - Registry implementation

**Modifications:**
- `apps/server/src/main.rs` - Use factory (stdio, WebSocket)
- `apps/server/src/cli.rs` - Use factory + add `tools` command for CLI parity

**Impact:** Eliminates 3x duplication, adds CLI parity (40 lines removed)

### Phase 2: Handler Migration (Days 4-8)

**New Files:**
- `crates/cb-server/src/handlers/file_operations_handler.rs` (6 tools)
- `crates/cb-server/src/handlers/workflow_handler.rs` (2 tools)
- `crates/cb-server/src/handlers/system_handler.rs` (6 tools)
- `crates/cb-server/src/handlers/refactoring_handler.rs` (4 tools)

**Modifications:**
- `crates/cb-server/src/handlers/plugin_dispatcher.rs`
  - Add `tool_registry` field
  - Replace if/else routing with registry lookup
  - Remove individual handler methods (moved to dedicated handlers)
  - **Reduces from 2306 → ~1700 lines**

**Impact:** Clean separation of concerns, extensible architecture

### Phase 3: Testing & Documentation (Days 9-10)

**New Files:**
- `crates/cb-server/tests/tool_handler_tests.rs` - Integration tests

**Modifications:**
- `SUPPORT_MATRIX.md` - Update architecture notes
- `CLAUDE.md` - Add developer guide for new tools

---

## Benefits

### Developer Experience

**Before (adding a new tool):**
1. Add handler method to `plugin_dispatcher.rs`
2. Update routing if/else chain
3. Add to category helper function
4. Update tool definitions in plugin

**After (adding a new tool):**
1. Implement `ToolHandler`
2. Register in dispatcher init

```rust
// Just 2 lines!
registry.register(Arc::new(MyHandler::new()));
```

### Maintainability

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Dispatcher LoC | 2306 | ~1700 | −26% |
| Init Duplication | 3x | 1x | −67% |
| Tool Routing | Hardcoded | Registry | 100% |
| CLI Parity | Partial | Full | ✅ |

### Extensibility

- **Plugin-style architecture** for all operations
- **Zero dispatcher edits** when adding tools
- **Clear separation** between LSP and special operations
- **Self-documenting** via trait requirements

---

## Migration Strategy

### Backward Compatibility

- ✅ No breaking changes to public APIs
- ✅ Existing tools continue to work
- ✅ Gradual migration (can ship partial implementations)
- ✅ Each phase independently testable

### Rollout

1. **Week 1:** Foundation + FileOpsHandler (prove pattern)
2. **Week 2:** Migrate remaining handlers
3. **Release:** Full refactor with tests and docs

### Validation

- Unit tests for each handler
- Integration tests for registry routing
- End-to-end tests for all 3 entry points
- Performance benchmarks (ensure no regression)

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Trait object overhead | Low | Low | Benchmarking shows negligible impact |
| Migration errors | Low | Medium | Incremental migration with tests at each step |
| Complexity increase | Low | Low | Actually reduces complexity via clear boundaries |
| Breaking changes | Very Low | High | Careful API design, backward compatibility |

---

## Alternatives Considered

### Option A: Do Nothing
- **Pros:** No effort required
- **Cons:** Tech debt accumulates, spiderweb complexity grows
- **Verdict:** ❌ Not sustainable

### Option B: Minimal Touch (Factory Only)
- **Pros:** Fast, low risk
- **Cons:** Doesn't fix routing issues, partial solution
- **Verdict:** ⚠️ Bandaid, not cure

### Option C: Full Restructure (Separate Crates)
- **Pros:** Maximum modularity
- **Cons:** 3-4 weeks, high risk, massive churn
- **Verdict:** ❌ Over-engineering

### Option D: Tool Handler Pattern (This Proposal)
- **Pros:** Clean, extensible, low risk, incremental
- **Cons:** Requires 1-2 weeks effort
- **Verdict:** ✅ **Recommended**

---

## Success Criteria

1. ✅ All 3 entry points use shared dispatcher factory
2. ✅ Zero hardcoded tool routing in dispatcher
3. ✅ CLI has full MCP protocol support (`tools` command)
4. ✅ Adding a tool requires ≤5 lines of code
5. ✅ All existing tests pass
6. ✅ No performance regression (≤5% overhead)
7. ✅ Documentation updated

---

## Timeline

| Phase | Days | Deliverable |
|-------|------|-------------|
| Foundation | 1-3 | Factory + ToolHandler trait + CLI parity |
| Migration | 4-8 | All handlers implemented, registry routing |
| Testing & Docs | 9-10 | Tests passing, docs updated |
| **Total** | **10** | **Clean, extensible architecture** |

---

## Recommendation

**Approve and proceed with implementation.**

This refactor addresses fundamental architectural issues while maintaining backward compatibility and low risk through incremental delivery. The result is a maintainable, extensible codebase that eliminates duplication and enables faster development.

**Next Steps:**
1. Review and approve proposal
2. Create implementation branch
3. Begin Phase 1 (Foundation)
4. Weekly progress check-ins

---

## Appendix: Code Examples

### Before: Adding a Tool

```rust
// 1. Edit plugin_dispatcher.rs - add routing
else if tool_name == "my_tool" {
    self.handle_my_tool(tool_call).await
}

// 2. Edit plugin_dispatcher.rs - add method
async fn handle_my_tool(&self, tool_call: ToolCall) -> ServerResult<Value> {
    // 50 lines of implementation
}

// 3. Edit plugin_dispatcher.rs - update category helper
fn is_my_category(&self, tool_name: &str) -> bool {
    matches!(tool_name, "my_tool" | "other_tool")
}
```

### After: Adding a Tool

```rust
// 1. Create handler
struct MyHandler;
impl ToolHandler for MyHandler {
    fn supported_tools(&self) -> Vec<&'static str> { vec!["my_tool"] }
    async fn handle_tool(&self, call: ToolCall, ctx: Arc<ToolContext>) -> Result<Value> {
        // 50 lines of implementation
    }
}

// 2. Register (in dispatcher::initialize)
registry.register(Arc::new(MyHandler::new()));

// Done! Tool available everywhere.
```

**Result:** From 3 edits across 1 god object → 1 new file + 1 line registration.
