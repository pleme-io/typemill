# Handler Naming Cleanup Proposal

**Status:** Proposed  
**Goal:** Eliminate `Legacy*` naming from handler wrappers and align the codebase around the unified `ToolHandler` trait.

---

## Background

During the compat layer migration we retained “legacy” prefixes (e.g., `LegacySystemHandler`) to avoid name collisions while the new trait rollout finished. With the compat module gone and all handlers now implementing `ToolHandler`, the remaining `Legacy*` aliases and fields create noise and confuse the API surface.

Key issues:
- Wrapper structs in `handlers/tools/` still import core handlers with `as Legacy*`, despite using the new trait (§ `crates/cb-handlers/src/handlers/tools/system.rs`, etc.).
- The top-level `handlers/mod.rs` re-exports `SystemHandler as LegacySystemHandler`, propagating the legacy vocabulary into downstream code.
- Field names like `legacy_handler` linger even though the wrapped handler is now the primary implementation.

Removing these artifacts improves readability, simplifies imports, and prevents new contributors from assuming multiple code paths still exist.

---

## Proposal

Adopt **Option 1 (“Rename Wrapper Handlers”)** from the internal discussion:

1. **Rename wrapper types** to disambiguate them from the primary handlers:
   - `SystemHandler` → `SystemToolsHandler`
   - `FileOpsHandler` → `FileToolsHandler` (or `FileOperationsHandler`—pick the clearest)
   - `EditingHandler` → `EditingToolsHandler`
   - `InternalEditingHandler` → `InternalEditingToolsHandler`, etc.
2. **Remove `Legacy*` aliases** in `handlers/mod.rs`; import concrete types directly where needed.
3. **Rename struct fields** (`legacy_handler` → `system_handler`, `refactoring_handler`, …) to reflect actual usage.
4. **Update registration sites** (`handlers/plugin_dispatcher.rs`, `tools/mod.rs`) to reference the new type names.

The rename is mechanical but touches multiple files; doing it once avoids lingering hybrid naming.

---

## Scope & Impact

**Files to update:**
1. `crates/cb-handlers/src/handlers/tools/system.rs`
2. `crates/cb-handlers/src/handlers/tools/file_ops.rs`
3. `crates/cb-handlers/src/handlers/tools/editing.rs`
4. `crates/cb-handlers/src/handlers/tools/internal_editing.rs`
5. `crates/cb-handlers/src/handlers/tools/workspace.rs`
6. `crates/cb-handlers/src/handlers/tools/advanced.rs`
7. `crates/cb-handlers/src/handlers/tools/mod.rs`
8. `crates/cb-handlers/src/handlers/mod.rs`
9. `crates/cb-handlers/src/handlers/plugin_dispatcher.rs`

No runtime behaviour changes are expected; this is a refactor focused on naming consistency.

---

## Plan

1. **Establish name mapping** – confirm final wrapper names (prefer consistent suffix like `*ToolsHandler`).
2. **Apply renames**
   - Update struct and file-level `pub use` statements.
   - Adjust field names inside each wrapper.
   - Fix registration macro invocations to log the new names.
3. **Run `cargo fmt` + `cargo check --package cb-handlers`** to ensure the rename is complete.
4. **Document** the naming convention in `docs/architecture/ARCHITECTURE.md` if necessary (optional).

Implementation can be done with `rustfix`/`sed` helpers or IDE rename tools to minimize typos.

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Missed rename causing compile error | Medium | Build (`cargo check`) after the sweep. |
| Downstream crates relying on `Legacy*` exports | Low | We removed the compat re-exports; audit usages during rename and update imports. |
| Reviewer confusion about behavioural changes | Low | Emphasize the change is naming-only; include diff summary in PR. |

---

## Validation

- `cargo fmt`
- `cargo check --package cb-handlers`
- Optional: `cargo nextest run --package cb-handlers`

Since behaviour should remain identical, no new tests are required beyond existing coverage.

---

## Outcome

Adopting this proposal yields a cleaner handler layer:
- No references to “legacy” remain.
- Wrapper names clearly indicate their role (`*ToolsHandler` vs. core handler).
- Future contributors won’t assume a deprecated code path still exists.

This aligns the handler module structure with the unified trait architecture achieved in the compat removal effort.
