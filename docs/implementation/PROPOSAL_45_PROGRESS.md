# Proposal 45 Implementation Progress

**Status**: Phase 1 Foundation Complete
**Started**: 2025-10-12
**Strategy**: Hybrid Sequential (Phases 1-2 sequential, Phase 3 parallel docs)

## Completed

### ✅ Phase 0: Shared Helpers Module
**Duration**: ~30 minutes
**Commit**: `4b8c6298`

**Created**: `/workspace/crates/cb-handlers/src/handlers/tools/analysis/helpers.rs`
- `filter_analyzable_files()` - File filtering by extensions
- `weighted_average()` - Multi-file aggregation calculations
- `AggregateStats` - Statistical aggregation (count, sum, min, max, avg)
- `WorkspaceAnalysisContext` - Workspace-wide operation support

**Tests**: 4/4 passing
- test_filter_analyzable_files
- test_weighted_average
- test_aggregate_stats
- test_aggregate_stats_merge

---

## ✅ Completed Phases

### Phase 1: analyze_project → analyze.quality (workspace scope)

**Status**: ✅ **COMPLETE**
**Commits**: `4b8c6298` (helpers), `9597a1bb` (workspace scope)
**Duration**: ~2 hours

**What Was Done**:
1. ✅ Added workspace scope detection in `quality.rs:handle_tool_call()`
2. ✅ Implemented `analyze_workspace_maintainability()` method (210 lines)
3. ✅ Used helpers module for file filtering and stats aggregation
4. ✅ Aggregates complexity metrics across all workspace files
5. ✅ Generates workspace-level finding with comprehensive metrics
6. ✅ Error-resilient (continues on file errors)
7. ✅ Build successful, no compilation errors

**Key Features**:
- File scope → Uses existing `run_analysis()` engine (unchanged)
- Workspace scope → New multi-file aggregation logic
- Metrics: total_files, total_functions, total_sloc, avg/max complexity, attention_ratio
- Severity calculation: >30% = high, >10% = medium, else low
- Error array appended to result if any files fail

**Files Modified**:
- `/workspace/crates/cb-handlers/src/handlers/tools/analysis/quality.rs` (+234 lines, -8 lines)

**Usage Example**:
```json
{
  "name": "analyze.quality",
  "arguments": {
    "kind": "maintainability",
    "scope": {
      "type": "workspace",
      "path": "/path/to/project"
    }
  }
}
```

**Legacy Handler Status**: `analyze_project` still exists but superseded. Will be removed in Phase 4.

---

## Current Phase: Phase 1 Tests

### Task: Port e2e tests for analyze_project migration

**Status**: ⏳ In Progress

**Original Plan**:

#### Step 1: Add workspace scope detection to quality.rs
```rust
// In handle_tool_call(), detect scope type
match scope_type.as_str() {
    "file" => { /* existing logic */ }
    "workspace" => { /* new aggregation logic */ }
    _ => return Err(...)
}
```

#### Step 2: Implement workspace aggregation for "maintainability" kind
```rust
async fn analyze_workspace_maintainability(
    context: &ToolHandlerContext,
    directory_path: &str,
    options: &QualityOptions,
) -> ServerResult<AnalysisResult> {
    // 1. Use helpers::WorkspaceAnalysisContext to list files
    // 2. For each file, run analyze_file_complexity()
    // 3. Aggregate using helpers::AggregateStats
    // 4. Build AnalysisResult with workspace scope
    // 5. Add findings for top hotspots
}
```

#### Step 3: Port e2e tests
- Find existing `analyze_project` tests
- Update to use `analyze.quality` with `scope: { type: "workspace" }`
- Verify outputs match

#### Step 4: Mark legacy handler for removal (don't delete yet, wait for Phase 4)

**Files to Modify**:
1. `/workspace/crates/cb-handlers/src/handlers/tools/analysis/quality.rs`
   - Add workspace scope handling in `handle_tool_call()`
   - Add `analyze_workspace_maintainability()` function

2. `/workspace/crates/cb-handlers/src/handlers/tools/analysis/engine.rs`
   - May need workspace-aware `run_analysis()` variant

3. Tests:
   - Find and update `analyze_project` integration tests
   - Add new `analyze.quality` workspace tests

**Estimated Time**: 1-2 days

---

## Next Phases

### Phase 2: analyze_imports → analyze.dependencies (plugin integration)
**Status**: Pending
**Estimated**: 1-2 days

**Current State**:
- `analyze_imports` delegates to SystemToolsPlugin
- Plugin handles import graph construction
- Need to integrate into `analyze.dependencies("imports")`

### Phase 3: find_dead_code → analyze.dead_code (LSP integration)
**Status**: Pending
**Estimated**: 2-3 days

**Requirements**:
- Add workspace scope to `analyze.dead_code`
- File scope: Keep existing regex heuristics (sandbox-safe)
- Workspace scope: Add LSP integration (accurate cross-file)
- Create thin backward-compat shim

### Phase 4: Cleanup & Documentation
**Status**: Pending
**Estimated**: 0.5 days

- Remove all 3 legacy handlers
- Update tool registration (23 → 20 internal tools)
- Update documentation

---

## Key Design Decisions

### 1. LSP Integration Strategy (Phase 3)
**Decision**: Automatic based on scope type
- File scope = regex heuristics (no LSP required)
- Workspace scope = LSP required (accurate cross-file)
- No explicit `use_lsp` flag needed

**Rationale**: Codex feedback - avoids API complexity, scope implies detection mode

### 2. Plugin Integration (Phase 2)
**Decision**: Plugin-backed parsing as default (not optional)
- Maintains TypeScript/Rust parity
- No fallback to regex
- Plugin integration is primary implementation

**Rationale**: Codex feedback - preserve language-specific accuracy

### 3. Shared Helpers
**Decision**: Front-load helper extraction
- Created in Phase 0 before any migrations
- Prevents copy/paste across phases
- Simplifies Phase 4 cleanup

**Rationale**: Codex feedback - bake in early for reuse

---

## Open Questions

1. **analyze_project format compatibility**: Should workspace maintainability output match old ProjectComplexityReport format for backward compat?
   - **Lean**: No, unified API uses AnalysisResult format (breaking change for internal tool)

2. **Error handling**: Should workspace analysis continue on file errors or fail fast?
   - **Current behavior**: project.rs continues with errors array
   - **Lean**: Match existing behavior (resilient)

3. **Test migration**: Port tests or create new ones?
   - **Lean**: Port existing tests, verify parity, then add new workspace tests

---

## Notes

- Token usage at Phase 1 start: 97k/200k
- Helper module: 213 lines added
- Helper tests: 4/4 passing
- Current quality.rs: 1101 lines (large, will need careful modification)
- analyze_maintainability() function already exists (lines 742-924)
  - Good foundation for workspace aggregation
  - Just needs multi-file collection and stats merging

---

## Next Session TODO

1. Read `/workspace/crates/cb-handlers/src/handlers/tools/analysis/engine.rs` to understand `run_analysis()` helper
2. Implement workspace scope detection in quality.rs `handle_tool_call()`
3. Implement `analyze_workspace_maintainability()` using helpers module
4. Find and update e2e tests for analyze_project
5. Verify no regressions with `cargo test`
6. Commit Phase 1 completion
