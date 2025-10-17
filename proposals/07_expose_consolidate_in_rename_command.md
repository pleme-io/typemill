# Proposal 07: Expose Consolidate Feature in Rename Command

**Status:** Draft
**Created:** 2025-10-17
**Author:** AI Assistant (via dogfooding session)

---

## Problem Statement

The codebase has **excellent consolidation functionality** for merging Rust crates, but it's:

1. **Hidden from the public API** - Only accessible via internal `rename_directory` tool
2. **Not exposed in the new Unified Refactoring API** - The `rename.plan` and `rename` commands don't support `consolidate: true`
3. **Requires workarounds** - Users must manually call internal tools instead of using the clean public API

### Current State

**Consolidation exists in two places:**

**A. Internal tool (works, but hidden):**
```bash
# This works but is internal-only:
codebuddy tool rename_directory '{
  "old_path": "crates/cb-types",
  "new_path": "crates/codebuddy-core/src/types",
  "consolidate": true,
  "dry_run": true
}'
```

**B. FileService implementation (backend):**
```rust
// In cb-services/src/services/file_service/cargo.rs
pub(super) async fn consolidate_rust_package(
    &self,
    old_package_path: &Path,
    new_package_path: &Path,
    dry_run: bool,
) -> ServerResult<DryRunnable<Value>>
```

**What consolidation does:**
1. Moves `source-crate/src/*` → `target-crate/src/module/*`
2. Merges dependencies from source Cargo.toml into target
3. Removes source crate from workspace members
4. Updates all imports: `use source::*` → `use target::module::*`
5. Deletes source crate directory

**C. NEW Unified API (doesn't support consolidate):**
```rust
// In cb-handlers/src/handlers/rename_handler/mod.rs
pub(crate) struct RenameOptions {
    strict: Option<bool>,
    validate_scope: Option<bool>,
    update_imports: Option<bool>,
    scope: Option<String>,
    custom_scope: Option<RenameScope>,
    // ❌ Missing: consolidate flag!
}
```

---

## Proposed Solution

### Option 1: Add `consolidate` to RenameOptions (Recommended)

**Add the field to existing options:**

```rust
// In cb-handlers/src/handlers/rename_handler/mod.rs
#[derive(Debug, Deserialize, Default)]
pub(crate) struct RenameOptions {
    #[serde(default)]
    strict: Option<bool>,
    #[serde(default)]
    validate_scope: Option<bool>,
    #[serde(default)]
    update_imports: Option<bool>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub custom_scope: Option<codebuddy_core::rename_scope::RenameScope>,

    /// NEW: Consolidate source package into target (for directory renames only)
    /// When true, merges Cargo.toml dependencies and updates all imports
    #[serde(default)]
    pub consolidate: Option<bool>,
}
```

**Update directory_rename.rs to use it:**

```rust
// In cb-handlers/src/handlers/rename_handler/directory_rename.rs
impl RenameHandler {
    pub(crate) async fn plan_directory_rename(
        &self,
        params: &RenamePlanParams,
        context: &ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.new_name);

        // Check if this is a consolidation
        let consolidate = params.options.consolidate.unwrap_or(false);

        if consolidate {
            // Use consolidate_rust_package instead of rename_directory
            let edit_plan = context
                .app_state
                .file_service
                .plan_consolidate_rust_package(old_path, new_path)
                .await?;

            // ... rest of consolidation logic
        } else {
            // Normal rename logic (existing code)
            let edit_plan = context
                .app_state
                .file_service
                .plan_rename_directory_with_imports(old_path, new_path, rename_scope.as_ref())
                .await?;
        }

        // ... build RenamePlan
    }
}
```

**Usage:**

```bash
# Now users can consolidate via public API:
codebuddy tool rename '{
  "target": {
    "kind": "directory",
    "path": "crates/cb-types"
  },
  "new_name": "crates/codebuddy-core/src/types",
  "options": {
    "consolidate": true
  }
}'
```

**Benefits:**
- ✅ Clean, consistent API
- ✅ Works with both `rename.plan` and `rename` (one-step)
- ✅ Documented in api_reference.md
- ✅ Type-safe

---

### Option 2: Auto-detect Consolidation (Smart Move)

**Automatically detect when a move is a consolidation:**

```rust
fn is_consolidation_move(old_path: &Path, new_path: &Path) -> bool {
    // Auto-detect: moving into another crate's src/ directory
    old_path.join("Cargo.toml").exists() &&  // Source is a crate
    new_path.ancestors().any(|p| p.join("Cargo.toml").exists()) &&  // Target is inside a crate
    new_path.components().any(|c| c.as_os_str() == "src")  // Target is in src/
}
```

**Usage (implicit):**

```bash
# Auto-detects consolidation (no flag needed):
codebuddy tool rename '{
  "target": {"kind": "directory", "path": "crates/cb-types"},
  "new_name": "crates/codebuddy-core/src/types"
}'
```

**Benefits:**
- ✅ Zero configuration
- ✅ "Do what I mean" behavior
- ✅ Simpler for users

**Drawbacks:**
- ❌ Less explicit (magic behavior)
- ❌ Harder to override if detection is wrong
- ❌ Could surprise users

---

### Option 3: Separate `consolidate.plan` Command

**Create a dedicated consolidation command:**

```rust
// New handler: ConsolidateHandler
pub struct ConsolidateHandler;

impl ToolHandler for ConsolidateHandler {
    fn tool_names(&self) -> &[&str] {
        &["consolidate.plan"]
    }

    async fn handle_tool_call(...) -> ServerResult<Value> {
        // Dedicated consolidation logic
    }
}
```

**Usage:**

```bash
codebuddy tool consolidate.plan '{
  "source": "crates/cb-types",
  "target": "crates/codebuddy-core/src/types"
}'
```

**Benefits:**
- ✅ Clear intent
- ✅ Separate documentation
- ✅ Can have consolidation-specific options

**Drawbacks:**
- ❌ More tools to learn
- ❌ Duplicates rename logic
- ❌ Not consistent with rename API

---

## Recommendation

**Implement Option 1 + Option 2 (Hybrid):**

1. **Add `consolidate` flag to `RenameOptions`** (Option 1)
   - Explicit control when needed
   - Works with unified API

2. **Auto-detect consolidation if flag is not set** (Option 2)
   - Smart defaults for common case
   - Users can override with `"consolidate": false` if needed

3. **Also expose `consolidate` as a convenience command alias**
   ```bash
   # These are equivalent:
   codebuddy tool consolidate '{"source": "...", "target": "..."}'
   codebuddy tool rename '{"target": {"kind": "directory", "path": "..."}, "new_name": "...", "options": {"consolidate": true}}'
   ```

### Implementation Steps

1. **Add `consolidate` field to `RenameOptions`** (5 min)
   ```rust
   // cb-handlers/src/handlers/rename_handler/mod.rs
   pub consolidate: Option<bool>,
   ```

2. **Update `plan_directory_rename` to check consolidate flag** (15 min)
   ```rust
   // cb-handlers/src/handlers/rename_handler/directory_rename.rs
   if params.options.consolidate.unwrap_or_else(|| self.is_consolidation(old_path, new_path)) {
       // Use consolidate logic
   }
   ```

3. **Add auto-detection helper** (10 min)
   ```rust
   fn is_consolidation(&self, old: &Path, new: &Path) -> bool {
       // Check if moving crate into another crate's src/
   }
   ```

4. **Add `ConsolidateHandler` alias** (optional, 10 min)
   ```rust
   pub struct ConsolidateHandler(RenameHandler);
   // Delegates to RenameHandler with consolidate=true
   ```

5. **Update documentation** (5 min)
   - api_reference.md
   - CLAUDE.md

**Total time:** ~45 minutes

---

## Examples

### Before (Current State):

```bash
# Must use internal tool:
codebuddy tool rename_directory '{
  "old_path": "crates/cb-types",
  "new_path": "crates/codebuddy-core/src/types",
  "consolidate": true,
  "dry_run": true
}'
```

### After (Proposed):

```bash
# Option A: Explicit flag
codebuddy tool rename '{
  "target": {"kind": "directory", "path": "crates/cb-types"},
  "new_name": "crates/codebuddy-core/src/types",
  "options": {"consolidate": true}
}'

# Option B: Auto-detect (implicit)
codebuddy tool rename '{
  "target": {"kind": "directory", "path": "crates/cb-types"},
  "new_name": "crates/codebuddy-core/src/types"
}'
# Auto-detects: "Oh, you're moving a crate into another crate's src/, I'll consolidate!"

# Option C: Convenience alias
codebuddy tool consolidate '{
  "source": "crates/cb-types",
  "target": "crates/codebuddy-core/src/types"
}'
```

---

## Impact

**Users:**
- ✅ Can consolidate crates via public API
- ✅ Consistent with rename.plan workflow
- ✅ Works with QuickRenameHandler (one-step command)

**Code:**
- ✅ Minimal changes (add one field + conditional)
- ✅ Reuses existing consolidate_rust_package logic
- ✅ No breaking changes

**Documentation:**
- Update api_reference.md with consolidate option
- Add consolidation examples to CLAUDE.md
- Document auto-detection behavior

---

## Testing

```rust
#[test]
fn test_consolidate_via_rename() {
    let result = rename_handler.handle_tool_call(context, ToolCall {
        name: "rename".into(),
        arguments: json!({
            "target": {"kind": "directory", "path": "crates/source"},
            "new_name": "crates/target/src/module",
            "options": {"consolidate": true}
        })
    }).await?;

    // Verify:
    assert!(!Path::new("crates/source").exists());
    assert!(Path::new("crates/target/src/module").exists());
    // Check Cargo.toml merged dependencies
    // Check imports updated across workspace
}
```

---

## Alternative Considered

**Status quo** - Keep consolidation internal-only

**Rejected because:**
- Users need this feature (we needed it for Proposal 06!)
- The code already exists and works well
- Hiding it from public API creates friction
- Inconsistent with "dogfooding" principle

---

## Conclusion

The consolidation feature is **production-ready** but **hidden**. Exposing it via the `rename` command with auto-detection makes it:
- **Discoverable** - Part of the public API
- **Easy to use** - Works with existing rename workflow
- **Smart** - Auto-detects common case
- **Consistent** - Same API as file/directory renames

**Recommendation: Implement hybrid Option 1 + 2 for best UX.**
