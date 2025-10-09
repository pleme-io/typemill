# MCP API Cleanup (Breaking Changes)

## Summary

Aggressive cleanup of MCP API surface to consolidate redundant tools, remove/internalize non-essential tools, improve semantic naming, and standardize the API.

**Impact:** 44 tools → 31 tools (-30%)

**⚠️ IMPORTANT: This is a COMPLETE implementation specification. All changes listed below must be implemented together as a single cohesive refactoring. Do NOT implement only part of this - it's all or nothing.**

The changes are organized by type (MERGE, DELETE, RENAME, etc.) for clarity, but ALL items must be completed to achieve the final API design of 31 tools.

---

## Complete Changes Table

| Action | Tool(s) | New State | Reason |
|--------|---------|-----------|--------|
| **MERGE** | `optimize_imports` + `organize_imports` | `organize_imports(remove_unused: bool)` | Single tool, remove_unused=true by default |
| **MERGE** | `system_status` + `health_check` | `health_check(level: "basic"\|"full")` | Same operation, different detail levels |
| **MERGE** | `suggest_refactoring` + `analyze_complexity` | `analyze_code(include_suggestions: bool)` | Suggestions use complexity metrics anyway |
| **MERGE** | `find_complexity_hotspots` + `analyze_project_complexity` | `analyze_project(output: "full"\|"hotspots"\|"summary", limit: int)` | Same scan, different views |
| **MERGE** | `prepare_call_hierarchy` + `get_call_hierarchy_incoming_calls` + `get_call_hierarchy_outgoing_calls` | `get_call_hierarchy(file, line, char, direction: "incoming"\|"outgoing"\|"both")` | Hide LSP 2-step protocol |
| **DELETE** | `web_fetch` | ❌ Removed | Claude has built-in WebFetch, security risk |
| **DELETE** | `rename_symbol_strict` | ❌ Removed | `rename_symbol` handles position already |
| **INTERNAL** | `get_completions` | 🔒 Internal only | AI doesn't need autocomplete |
| **INTERNAL** | `get_signature_help` | 🔒 Internal only | Not useful for AI agents |
| **RENAME** | `apply_edits` | `execute_edits` | Verb-noun consistency |
| **RENAME** | `batch_execute` | `execute_batch` | Verb-noun consistency |
| **RENAME** | `get_hover` | `get_symbol_info` | Semantic over LSP-specific term |
| **RENAME** | `rename_file` | `move_file` | Accurately describes cross-directory moves |
| **RENAME** | `rename_directory` | `move_directory` | Accurately describes cross-directory moves + consolidation |
| **RENAME** | `search_workspace_symbols` | `search_symbols` | "workspace" is implied; shorter and clearer |
| **KEEP** | `update_dependency` + `update_dependencies` | No change | Different operations (pin vs bulk) |

---

## Before/After Comparison

### Before: 44 Public Tools

- **Navigation & Intelligence:** 13 tools
- **Editing & Refactoring:** 10 tools
- **Code Analysis:** 5 tools
- **File Operations:** 6 tools
- **Workspace Operations:** 5 tools
- **Advanced Operations:** 2 tools
- **System & Health:** 3 tools

### After: 31 Public Tools

- **Navigation & Intelligence:** 7 tools (-6)
  - Merged: 3 call hierarchy tools → 1
  - Internal: `get_completions`, `get_signature_help`
  - Renamed: `get_hover` → `get_symbol_info`, `search_workspace_symbols` → `search_symbols`

- **Editing & Refactoring:** 7 tools (-3)
  - Merged: `optimize_imports` into `organize_imports`
  - Removed: `rename_symbol_strict`
  - Renamed: `apply_edits` → `execute_edits`

- **Code Analysis:** 2 tools (-3)
  - Merged: `analyze_complexity` + `suggest_refactoring` → `analyze_code`
  - Merged: `analyze_project_complexity` + `find_complexity_hotspots` → `analyze_project`

- **File Operations:** 5 tools (-1)
  - Renamed: `rename_file` → `move_file`

- **Workspace Operations:** 4 tools (-1)
  - Renamed: `rename_directory` → `move_directory`, `batch_execute` → `execute_batch`

- **Advanced Operations:** 1 tool (-1)
  - Renamed: `apply_edits` → `execute_edits` (moved to Editing)

- **System & Health:** 1 tool (-2)
  - Merged: `system_status` into `health_check`
  - Removed: `web_fetch`

---

## New/Updated Tool Signatures

### Merged Tools

#### `organize_imports`
```json
{
  "file_path": "src/app.ts",
  "remove_unused": true  // NEW - default: true
}
```

#### `health_check`
```json
{
  "level": "basic" | "full"  // NEW - default: "full"
}
```

#### `analyze_code`
```json
{
  "file_path": "src/app.ts",
  "include_suggestions": true  // NEW - default: true
}
```

#### `analyze_project`
```json
{
  "directory_path": "src/",
  "output": "full" | "hotspots" | "summary",  // NEW - default: "full"
  "limit": 10  // Optional: For hotspots mode
}
```

#### `get_call_hierarchy`
```json
{
  "file_path": "src/app.ts",
  "line": 10,
  "character": 5,
  "direction": "incoming" | "outgoing" | "both"  // NEW
}
```

### Renamed Tools (signatures unchanged)

```json
// get_symbol_info (was get_hover)
{
  "file_path": "src/app.ts",
  "line": 10,
  "character": 5
}

// move_file (was rename_file)
{
  "old_path": "src/old.ts",
  "new_path": "src/new.ts",
  "dry_run": false
}

// move_directory (was rename_directory)
{
  "old_path": "crates/old",
  "new_path": "crates/new",
  "consolidate": false,
  "dry_run": false
}

// search_symbols (was search_workspace_symbols)
{
  "query": "MyClass",
  "limit": 20
}

// execute_edits (was apply_edits)
{
  "edits": [/* TextEdit array */]
}

// execute_batch (was batch_execute)
{
  "operations": [/* BatchOperation array */]
}
```

---

## Implementation Checklist

**⚠️ ALL items below must be completed. This is not incremental.**

### Category 1: Tool Renames (Verb-Noun Consistency) - 6 items

- [ ] `apply_edits` → `execute_edits`
- [ ] `batch_execute` → `execute_batch`
- [ ] `get_hover` → `get_symbol_info`
- [ ] `rename_file` → `move_file`
- [ ] `rename_directory` → `move_directory`
- [ ] `search_workspace_symbols` → `search_symbols`

**Implementation Notes:**
- Update handler implementations in respective files
- Update all string literals in tool registration/dispatch
- Update workflow files (`.codebuddy/workflows.json`)
- Update all documentation references

### Category 2: Tool Deletions - 2 items

- [ ] Delete `web_fetch` (Claude has built-in WebFetch, security risk)
- [ ] Delete `rename_symbol_strict` (redundant with `rename_symbol`)

**Implementation Notes:**
- Remove from `crates/cb-plugins/src/system_tools_plugin.rs` (web_fetch)
- Remove from `crates/cb-handlers/src/handlers/tools/editing.rs` (rename_symbol_strict)
- Remove all handler code and tests
- Clean up documentation references
- Remove workflows that depend on deleted tools

### Category 3: Internalize Tools - 2 items

- [ ] Mark `get_completions` as internal-only
- [ ] Mark `get_signature_help` as internal-only

**Implementation Notes:**
- Create new `crates/cb-handlers/src/handlers/tools/internal_intelligence.rs`
- Move implementations to new handler
- Set `is_internal() = true` so they don't appear in public MCP listings
- Keep functionality for potential backend use

### Category 4: Simple Tool Merges - 2 items

- [ ] Merge `optimize_imports` into `organize_imports(remove_unused: bool)`
  - Add `remove_unused` parameter (default: true)
  - When true, performs optimization (remove unused imports)
  - When false, only organizes (sorts/groups) imports

- [ ] Merge `system_status` into `health_check(level: "basic"|"full")`
  - Add `level` parameter (default: "full")
  - "basic" returns lightweight status (was `system_status`)
  - "full" returns comprehensive health info (was `health_check`)

**Implementation Notes:**
- Update `crates/cb-handlers/src/handlers/tools/editing.rs` (organize_imports)
- Update `crates/cb-handlers/src/handlers/tools/system.rs` (health_check)
- Keep backward compatibility in parameter defaults
- Update parameter types in `crates/cb-protocol/src/types.rs`

### Category 5: Complex Tool Merges - 3 items

- [ ] Merge `analyze_complexity` + `suggest_refactoring` → `analyze_code(include_suggestions: bool)`
  - Add `include_suggestions` parameter (default: true)
  - Always returns complexity metrics
  - When true, also includes refactoring suggestions based on metrics

- [ ] Merge `analyze_project_complexity` + `find_complexity_hotspots` → `analyze_project(output: "full"|"hotspots"|"summary", limit: int)`
  - Add `output` parameter (default: "full")
  - "full" = complete complexity analysis of all files
  - "hotspots" = top N most complex areas (controlled by `limit`)
  - "summary" = high-level overview statistics
  - Add optional `limit` parameter (default: 10, only used in "hotspots" mode)

- [ ] Merge `prepare_call_hierarchy` + `get_call_hierarchy_incoming_calls` + `get_call_hierarchy_outgoing_calls` → `get_call_hierarchy(direction: "incoming"|"outgoing"|"both")`
  - Add `direction` parameter (required)
  - Hides LSP's 2-step protocol from users
  - "incoming" = who calls this function
  - "outgoing" = what this function calls
  - "both" = complete call graph

**Implementation Notes:**
- Update `crates/cb-handlers/src/handlers/tools/analysis.rs` (complexity tools)
- Update `crates/cb-handlers/src/handlers/tools/navigation.rs` (call hierarchy)
- Implement unified handlers that branch based on parameters
- Update parameter types in `crates/cb-protocol/src/types.rs`
- Comprehensive test coverage for all parameter combinations

---

## Files to Modify (Complete List)

### Core Handler Files
- `crates/cb-handlers/src/handlers/tools/advanced.rs` - Rename batch_execute → execute_batch
- `crates/cb-handlers/src/handlers/tools/navigation.rs` - Call hierarchy merge, rename search_workspace_symbols → search_symbols, rename get_hover → get_symbol_info
- `crates/cb-handlers/src/handlers/tools/editing.rs` - Remove rename_symbol_strict, merge optimize_imports, rename apply_edits → execute_edits
- `crates/cb-handlers/src/handlers/tools/analysis.rs` - Complexity tools merge
- `crates/cb-handlers/src/handlers/tools/system.rs` - Health check merge
- `crates/cb-handlers/src/handlers/tools/file_ops.rs` - Rename rename_file → move_file
- `crates/cb-handlers/src/handlers/tools/workspace.rs` - Rename rename_directory → move_directory
- `crates/cb-handlers/src/handlers/tools/internal_intelligence.rs` - NEW FILE - Internal tools
- `crates/cb-handlers/src/handlers/tools/mod.rs` - Add internal_intelligence module

### Plugin Files
- `crates/cb-plugins/src/system_tools_plugin.rs` - Remove web_fetch

### Protocol/Types
- `crates/cb-protocol/src/types.rs` - Add new parameter types for merged tools

### Configuration
- `.codebuddy/workflows.json` - Update tool names, remove web_fetch workflows

### Tests (Update for new tool names and merged functionality)
- `crates/cb-server/tests/tool_registration_test.rs` - Update tool counts and names
- `integration-tests/tests/e2e_workflow_execution.rs` - Update workflow tests
- `integration-tests/src/harness/client.rs` - Update test client
- `apps/codebuddy/tests/navigation_tests.rs` - Call hierarchy and symbol search tests
- `apps/codebuddy/tests/editing_tests.rs` - Import and rename tests
- `apps/codebuddy/tests/analysis_tests.rs` - Complexity analysis tests
- `apps/codebuddy/tests/system_tests.rs` - Health check tests
- `apps/codebuddy/tests/file_ops_tests.rs` - File move tests
- `apps/codebuddy/tests/workspace_tests.rs` - Directory move tests

### Documentation (ALL must be updated)
- `API_REFERENCE.md` - Complete tool reference update
- `CLAUDE.md` - Tool listings and examples
- `CONTRIBUTING.md` - Tool development examples
- `TOOLS_QUICK_REFERENCE.md` - Quick reference guide
- `CHANGELOG.md` - Add breaking changes entry

---

## Implementation Strategy

### Recommended Approach

**Manual Implementation (Recommended for correctness)**
- Systematically work through each category in the checklist above
- Start with renames (simpler), then deletions, then merges (most complex)
- Run `cargo test --workspace` after each category
- Run `cargo clippy` to catch any issues
- Update documentation as you go

### Testing Strategy

After implementing ALL changes, verify:

```bash
# Full test suite must pass
cargo test --workspace --all-features -- --include-ignored

# No clippy warnings
cargo clippy --all-targets --all-features

# Verify tool count
cargo run -- serve &
# Check MCP tools/list shows exactly 31 public tools
# Verify internal tools are hidden but still functional

# Documentation accuracy
# Manually verify API_REFERENCE.md matches actual implementation
# Check all examples still work with renamed tools
```

### Rollback Plan

If issues arise:
1. This is a breaking change, so no backward compatibility required
2. However, keep feature branch until thoroughly tested
3. Consider creating a compatibility shim if clients need migration time

---

## Benefits

1. **Reduced Cognitive Load:** 30% fewer tools to learn (44 → 31)
2. **Clearer Intent:** Tool names match user mental models
3. **Better Discoverability:** Related functionality grouped in parameters
4. **Consistent API:** Verb-noun naming throughout
5. **Simplified Maintenance:** Less code duplication
6. **Semantic Naming:** API describes intent, not implementation details
7. **Production Validation:** Self-refactoring proves tools are production-ready

---

## Risks

- **Breaking Changes:** All existing MCP clients must update
- **Migration Effort:** Users need to update tool calls
- **Testing Burden:** Comprehensive integration tests required for merged tools
- **Self-Refactoring Risk:** Tests tools on real codebase (mitigated by dry-run mode)

---

## Decision Points

1. **Keep `update_dependency` + `update_dependencies`?**
   - ✅ YES - Different operations (pin specific version vs bulk upgrade)

2. **Remove `web_fetch` entirely?**
   - ✅ YES - Claude has built-in WebFetch, security risk, out of scope

3. **Internal vs Delete for `get_completions`?**
   - ✅ INTERNAL - Keep for potential future use, hide from MCP listing

4. **Naming: `analyze_code` vs `analyze_complexity`?**
   - ✅ `analyze_code` - Broader scope, includes suggestions

5. **Use `move_file`/`move_directory` vs `rename_*`?**
   - ✅ `move_*` - Accurately describes cross-directory capability

---

## Success Criteria (All Must Pass)

| Category | Validation | Pass Criteria |
|----------|------------|---------------|
| **Tool Count** | `tools/list` MCP call | Exactly 31 public tools returned |
| **Internal Tools** | Direct handler test | `get_completions` and `get_signature_help` work but don't appear in public list |
| **Compilation** | `cargo build --release` | Clean build with no errors |
| **Tests** | `cargo test --workspace --all-features -- --include-ignored` | 100% pass rate |
| **Code Quality** | `cargo clippy --all-targets --all-features` | Zero warnings |
| **Documentation** | Manual review | All docs reference correct tool names, no broken examples |
| **API Contracts** | Integration tests | All merged tools support new parameters correctly |
| **Backward Compat** | N/A | Breaking change is intentional, no compatibility required |

**Final Verification Checklist:**
- [ ] Tool count reduced from 44 → 31 public tools
- [ ] All 6 renames completed and working
- [ ] Both deletions complete (web_fetch, rename_symbol_strict)
- [ ] Both internalizations complete (get_completions, get_signature_help)
- [ ] All 2 simple merges implemented with new parameters
- [ ] All 3 complex merges implemented with new parameters
- [ ] All documentation updated (5 files minimum)
- [ ] All tests passing
- [ ] Zero clippy warnings
- [ ] CHANGELOG.md updated with breaking changes notice

---

## Implementation Status Tracking

**⚠️ CRITICAL: Do NOT implement this proposal partially. It must be completed in its entirety.**

- [ ] Category 1: Tool Renames (6 items)
- [ ] Category 2: Tool Deletions (2 items)
- [ ] Category 3: Internalize Tools (2 items)
- [ ] Category 4: Simple Merges (2 items)
- [ ] Category 5: Complex Merges (3 items)
- [ ] Documentation Updates (5 files)
- [ ] Test Updates (all test files)
- [ ] Final Verification (10 criteria)

---

## Historical Note

**⚠️ WARNING:** A previous implementation attempted to do only part of this proposal. That partial implementation is INCOMPLETE and should NOT be considered done. This proposal requires ALL categories to be completed together to achieve the final API design.

The branch `feature/mcp-api-cleanup` contains only partial work (renames, deletions, internalizations) but is missing all the tool merges. If continuing from that branch, you must complete Categories 4-5 and ensure all success criteria pass.
