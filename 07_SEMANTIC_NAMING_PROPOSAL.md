# PROPOSAL: Semantic Naming Improvements

**Status:** Proposal
**Date:** 2025-10-09
**Effort:** ~4 hours
**Context:** Cherry-picked improvements from Proposal 04, compatible with Proposal 06

---

## Summary

Rename 4 tools to use semantic terminology instead of LSP/UI-specific terms. These changes complement Proposal 06's consolidation strategy.

---

## Changes

| Current Name | New Name | Reason | Effort |
|--------------|----------|--------|--------|
| `get_hover` | `get_symbol_info` | "Hover" is UI-specific LSP term; "symbol_info" describes what users get | 1h |
| `rename_file` | `move_file` | Actually moves files across directories, not just renames | 1h |
| `rename_directory` | `move_directory` | Handles cross-directory moves + consolidation mode | 1h |
| `search_workspace_symbols` | `search_symbols` | "workspace" is implied; shorter and clearer | 1h |

**Total Effort:** ~4 hours

---

## Benefits

1. **Semantic over Technical:** API describes intent, not LSP implementation details
2. **Accurate Naming:** "Move" correctly describes cross-directory operations
3. **Consistency:** File and directory operations use same verb pattern
4. **Brevity:** Remove redundant "workspace" qualifier

---

## Implementation

### Files to Modify
- `crates/cb-handlers/src/handlers/intelligence.rs` - Hover rename
- `crates/cb-handlers/src/handlers/file_ops.rs` - File/directory renames
- `crates/cb-handlers/src/lib.rs` - Tool registration
- `crates/cb-protocol/src/types.rs` - Type name updates
- `API_REFERENCE.md` - Documentation
- `CLAUDE.md` - Tool listings

### Tool Signatures (Unchanged)
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
```

---

## Compatibility with Proposal 06

These renames apply to tools that survive Proposal 06's consolidation:
- ✅ `get_hover` - Not affected by merges
- ✅ `rename_file` - Not affected by merges
- ✅ `rename_directory` - Not affected by merges
- ✅ `search_workspace_symbols` - Not affected by merges

Can be implemented independently or bundled with Proposal 06.

---

**Approval Required:** @maintainer
