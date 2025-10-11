# Implementation Sequencing Guide

**Status**: Phase 0-1 ✅ **COMPLETE** | Phase 2-3 🔄 **PENDING** | Phase 4 🟡 **PARTIAL**

**Purpose**: Define the correct implementation order for unified API features to avoid dependency conflicts and enable incremental delivery.

**Last Updated**: 2025-10-11

---

## Implementation Status Summary

### ✅ Completed Phases (2025-10-11)

**Phase 0: Foundation (Self-Registration)** - All plugins expose capabilities for dynamic validation

**Phase 1A: Refactoring API Core** - All 7 plan commands + workspace.apply_edit with validation and rollback
- `rename.plan`, `extract.plan`, `inline.plan`, `move.plan`, `reorder.plan`, `transform.plan`, `delete.plan`
- Unified `workspace.apply_edit` executor with checksum validation
- Atomic rollback mechanism

**Phase 1B: Refactoring API Config** - Project-level presets via `.codebuddy/refactor.toml`
- Preset system with override support
- Configuration validation against plugin registry
- Integration test coverage

**Phase 1C: Post-Apply Validation** - Validation command execution with automatic rollback
- Post-apply validation with timeout handling
- Automatic rollback on validation failure
- Comprehensive test coverage (pass/fail/timeout)

**Phase 4: Client Utilities (Partial)** - `formatPlan` utility for Rust
- Human-readable plan descriptions
- Handles all 7 plan types with proper pluralization
- Exported from `crates/cb-client`

### 🔄 Pending Phases

**Phase 2A-C: Unified Analysis API** - Not started (separate proposal)
**Phase 3: Batch Operations** - Not started
**Phase 4 (Remaining)**: Plan diff visualization, TypeScript client utilities

---

## Critical Dependencies

### 1. Self-Registration System (PREREQUISITE)

**Why it's needed**:
- Config/preset loading must query plugin capabilities dynamically
- Can't hardcode which `kind` values are valid per category
- Plugins must advertise their supported operations and parameters

**What it provides**:
```rust
// Plugin registry exposes capabilities
trait PluginRegistry {
    fn get_analysis_kinds(&self, category: &str) -> Vec<String>;
    fn get_refactoring_kinds(&self, operation: &str) -> Vec<String>;
    fn validate_analysis_args(&self, category: &str, kind: &str, args: &Value) -> Result<()>;
    fn validate_refactoring_args(&self, operation: &str, kind: &str, args: &Value) -> Result<()>;
}
```

**Without this**:
- Config validation can't verify that preset references valid `kind` values
- Plugin additions require manual updates to config schema
- No way to validate suggestion `refactor_call` arguments dynamically

**Status**: ✅ **COMPLETE** - Self-registration implemented.

---

## Implementation Phases

### Phase 0: Foundation (Self-Registration) ✅ **COMPLETE**

**Status**: ✅ Completed 2025-10-11

**Goal**: Enable dynamic capability discovery

**Deliverables**:
1. ✅ Registry descriptor system for plugins
2. ✅ Plugin capability advertisement (supported kinds, argument schemas)
3. ✅ Runtime validation of commands against registry
4. ✅ CI validation that all plugins expose descriptors

**Timeline**: 2-3 weeks (actual: completed)
**Blockers**: None
**Blocks**: Phase 1 of both unified APIs

**Success criteria**:
- [x] Plugins expose `get_capabilities()` method
- [x] Registry can enumerate all valid `kind` values per category/operation
- [x] Dynamic schema validation for plugin arguments
- [x] CI fails if plugin doesn't provide descriptors

---

### Phase 1A: Refactoring API Core (No Config) ✅ **COMPLETE**

**Status**: ✅ Completed 2025-10-11

**Goal**: Implement plan → apply pattern without presets

**Deliverables**:
1. ✅ All 7 `*.plan` commands (rename, extract, inline, move, reorder, transform, delete)
2. ✅ `workspace.apply_edit` with checksum validation and rollback
3. ✅ Plan structure with checksums and metadata
4. ✅ Rollback mechanism

**Timeline**: 4-5 weeks (actual: completed)
**Blockers**: Phase 0 (self-registration) ✅
**Blocks**: Phase 1B (refactoring config) ✅

**Success criteria**:
- [x] All `*.plan` commands implemented
- [x] `workspace.apply_edit` handles all 7 plan types
- [x] Checksum validation works
- [x] Rollback on error works
- [x] No config/preset support yet (moved to Phase 1B)

---

### Phase 1B: Refactoring API Config ✅ **COMPLETE**

**Status**: ✅ Completed 2025-10-11

**Goal**: Add project-level presets for refactoring

**Deliverables**:
1. ✅ `.codebuddy/refactor.toml` loader (`crates/cb-core/src/refactor_config.rs`)
2. ✅ Preset resolution with override support
3. ✅ Config validation against registry (uses Phase 0)
4. ✅ Integration tests for preset loading

**Timeline**: 1-2 weeks (actual: completed)
**Blockers**: Phase 1A (refactoring core) ✅
**Blocks**: None (parallel with Phase 1C) ✅

**Success criteria**:
- [x] Config loader reads `.codebuddy/refactor.toml`
- [x] Presets override defaults correctly
- [x] Per-call options override presets
- [x] CI validates config files in test fixtures

---

### Phase 1C: Post-Apply Validation ✅ **COMPLETE**

**Status**: ✅ Completed 2025-10-11

**Goal**: Add validation command execution with rollback

**Deliverables**:
1. ✅ Command executor in `workspace.apply_edit`
2. ✅ Validation result capture (exit code, stdout, stderr, timing)
3. ✅ Automatic rollback on validation failure
4. ✅ Timeout handling
5. ✅ Integration tests for validation scenarios

**Timeline**: 1-2 weeks (actual: completed)
**Blockers**: Phase 1A (refactoring core) ✅
**Blocks**: None (parallel with Phase 1B) ✅

**Success criteria**:
- [x] Validation command runs after edits applied
- [x] Rollback triggered on non-zero exit
- [x] Timeout enforced (default 60s)
- [x] Validation output captured in result
- [x] Tests cover pass/fail/timeout scenarios

---

### Phase 2A: Analysis API Core (No Config, No Safety)

**Goal**: Implement unified analysis commands with basic results

**Deliverables**:
1. All 6 `analyze.*` commands
2. Unified `AnalysisResult` structure
3. Basic suggestions (no safety metadata yet)
4. Per-category `kind` support

**Timeline**: 3-4 weeks
**Blockers**: Phase 0 (self-registration)
**Blocks**: Phase 2B (analysis config), Phase 2C (safety metadata)

**Success criteria**:
- [ ] All 6 analysis categories implemented
- [ ] Uniform result structure across categories
- [ ] Basic suggestions with `refactor_call`
- [ ] Integration tests per category

---

### Phase 2B: Analysis API Config

**Goal**: Add project-level presets for analysis

**Deliverables**:
1. `.codebuddy/analysis.toml` loader
2. Preset resolution with override support
3. Config validation against registry (uses Phase 0)
4. Integration tests for preset loading

**Timeline**: 1-2 weeks
**Blockers**: Phase 2A (analysis core)
**Blocks**: None (parallel with Phase 2C)

**Success criteria**:
- [ ] Config loader reads `.codebuddy/analysis.toml`
- [ ] Presets define thresholds, filters, scope
- [ ] Per-call options override presets
- [ ] CI validates config files

---

### Phase 2C: Safety Metadata & Ranking

**Goal**: Add safety/confidence/reversible to suggestions

**Deliverables**:
1. Safety classification logic per suggestion type
2. Confidence scoring algorithms
3. Reversibility analysis
4. Safety-first ranking algorithm
5. CI validation of metadata

**Timeline**: 2-3 weeks
**Blockers**: Phase 2A (analysis core)
**Blocks**: None (parallel with Phase 2B)

**Success criteria**:
- [ ] All suggestions include safety/confidence/reversible
- [ ] Suggestions ordered by safety → confidence → impact
- [ ] CI validates metadata presence and ranges
- [ ] Tests cover safety classification logic

---

### Phase 3: Batch Operations

**Goal**: Add `analyze.batch` with shared parsing

**Deliverables**:
1. Batch query executor
2. Shared AST parsing across analyses
3. Cache optimization
4. Performance benchmarks

**Timeline**: 2-3 weeks
**Blockers**: Phase 2A (analysis core)
**Blocks**: None

**Success criteria**:
- [ ] `analyze.batch` accepts multiple queries
- [ ] Files parsed once, AST reused
- [ ] Cache hit metrics in result
- [ ] Performance improvement vs sequential calls

---

### Phase 4: Client Library Utilities 🟡 **PARTIAL**

**Status**: 🟡 Partially Complete (formatPlan done, others pending)

**Goal**: Add helper functions for client convenience

**Deliverables**:
1. ✅ `formatPlan(plan)` utility (Rust implementation complete)
2. 🔄 Plan diff visualization (pending)
3. 🔄 Suggestion filtering helpers (pending - requires Phase 2C)
4. 🔄 Safety decision helpers for AI agents (pending - requires Phase 2C)

**Timeline**: 1-2 weeks (partial: formatPlan completed)
**Blockers**: Phase 1A (refactoring core) ✅, Phase 2C (safety metadata) 🔄
**Blocks**: None

**Success criteria**:
- [x] `formatPlan` generates human-readable descriptions (Rust only)
- [ ] TypeScript/JavaScript `formatPlan` in separate `@codebuddy/client` package
- [ ] AI agent helpers for safety decisions (requires Phase 2C)
- [ ] Documentation with examples (formatPlan documented)

---

## Parallel Work Streams

**Can run concurrently**:
- Phase 1B (refactoring config) + Phase 1C (validation) after Phase 1A
- Phase 2B (analysis config) + Phase 2C (safety metadata) after Phase 2A
- Phase 3 (batch) + Phase 4 (client utils) after Phase 2A

**Cannot parallelize**:
- Phase 0 must complete before Phase 1A or Phase 2A
- Phase 1A must complete before Phase 1B or Phase 1C
- Phase 2A must complete before Phase 2B or Phase 2C

---

## Critical Path

```
Phase 0 (Foundation)
  ↓
Phase 1A (Refactoring Core) ────┬──→ Phase 1B (Refactoring Config)
                                 └──→ Phase 1C (Post-Apply Validation)
  ↓
Phase 2A (Analysis Core) ────────┬──→ Phase 2B (Analysis Config)
                                 └──→ Phase 2C (Safety Metadata)
  ↓
Phase 3 (Batch Operations) ──────┬──→ Phase 4 (Client Utilities)
```

**Total timeline**: ~16-22 weeks (4-5.5 months) with 2-3 parallel work streams

---

## Integration Test Strategy

### Phase 0 Tests
- Plugin capability discovery
- Registry validation
- Dynamic schema checks

### Phase 1 Tests
- All refactoring operations (14 commands)
- Plan validation (checksums, types)
- Rollback scenarios
- Config preset loading and overrides
- Post-apply validation (pass/fail/timeout)

### Phase 2 Tests
- All analysis operations (6 categories × ~4 kinds = 24 operations)
- Suggestion generation and ranking
- Safety metadata validation
- Config preset loading and overrides
- Batch query optimization

### Phase 3 Tests
- Batch query execution
- Shared parsing optimization
- Cache hit verification

### Phase 4 Tests
- Client utility correctness
- Format plan output
- AI agent decision helpers

---

## Risk Mitigation

### Risk: Config schema changes break existing configs
**Mitigation**: Version config files (`.codebuddy/refactor.v1.toml`), support migration

### Risk: Validation command hangs indefinitely
**Mitigation**: Hard timeout enforcement, process kill on timeout

### Risk: Safety classification produces incorrect metadata
**Mitigation**: Conservative defaults (mark as "experimental" if uncertain), CI validation

### Risk: Registry descriptors missing or incomplete
**Mitigation**: CI fails if plugin doesn't provide valid descriptors

### Risk: Performance regression from validation overhead
**Mitigation**: Make validation optional, benchmark before/after

---

## Deployment Strategy

### Rolling Deployment
1. Deploy Phase 0 (self-registration) - no user-facing changes
2. Deploy Phase 1A (refactoring core) - new commands available, no config yet
3. Deploy Phase 1B + 1C - add config and validation support
4. Deploy Phase 2A (analysis core) - new analysis commands
5. Deploy Phase 2B + 2C - add config and safety metadata
6. Deploy Phase 3 + 4 - batch operations and client utilities

### Feature Flags
- `enable_unified_refactoring_api` (Phase 1)
- `enable_unified_analysis_api` (Phase 2)
- `enable_batch_analysis` (Phase 3)

### Backward Compatibility
- Keep legacy commands during rollout
- Remove legacy after unified API stabilizes (Phase 1-2 complete)
- Provide migration guide for users

---

## Open Questions

1. **Config file format**: TOML vs JSON vs YAML?
   - **Decision**: TOML (better for human editing, native Rust support)

2. **Validation command security**: Sandboxing? Resource limits?
   - **Decision**: Run in same environment as server, user responsibility to secure commands

3. **Safety classification logic**: Rule-based vs ML-based?
   - **Decision**: Rule-based for MVP, ML exploration in Phase 5+

4. **Registry storage**: In-memory vs persistent?
   - **Decision**: In-memory, rebuilt on startup from plugin descriptors

5. **Preset inheritance**: Can presets extend other presets?
   - **Decision**: Not in Phase 1, consider in Phase 2+ if requested

---

## Next Steps

1. Review this sequencing with team
2. Create Phase 0 (self-registration) implementation plan
3. Set up project tracking for phases
4. Define interface contracts between phases
5. Create sample configs for testing
