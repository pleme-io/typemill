# Bug Report: Batch Rename Not Working

**Date:** 2025-10-22
**Session:** Dogfooding mill-* rename migration
**Affected Commands:** `rename.plan` (batch mode), `rename` (batch mode)
**Status:** 🐛 Issue #1 Confirmed - Needs Fix | ✅ Issue #2 False Alarm - Already Working

---

## Summary

One critical issue discovered while dogfooding the rename tool during the mill-* migration:

1. **Batch rename feature returns 0 affected files and doesn't execute**

**Note:** Issue #2 (Cargo.toml feature definitions) was initially reported but investigation proved it's NOT a bug - feature refs ARE being updated correctly by existing code.

---

## Issue #1: Batch Rename Not Working

### What We Tried

Used the batch rename feature (added in commit 15d46fe4) to rename two crates simultaneously:

```bash
./target/release/codebuddy tool rename.plan '{
  "targets": [
    {
      "kind": "directory",
      "path": "crates/codebuddy-config",
      "new_name": "crates/mill-config"
    },
    {
      "kind": "directory",
      "path": "crates/codebuddy-auth",
      "new_name": "crates/mill-auth"
    }
  ],
  "options": {
    "scope": "all"
  }
}'
```

### Expected Behavior

Should return a plan with all files that would be affected by both renames:
- Workspace `Cargo.toml` updates
- Dependency updates in dependent crates
- Import statement updates
- Documentation updates
- etc.

### Actual Behavior

```json
{
  "content": {
    "edits": {
      "changes": {}  // ← Empty!
    },
    "summary": {
      "affected_files": 0,  // ← Zero files!
      "created_files": 0,
      "deleted_files": 0
    },
    "metadata": {
      "kind": "batch_rename"  // ← Correctly identified as batch
    }
  }
}
```

**Result:** Plan generated but shows 0 affected files, no edits planned.

### Also Tried Quick Rename

```bash
./target/release/codebuddy tool rename '{
  "targets": [...same...]
}'
```

**Result:**
```json
{
  "content": {
    "applied_files": [],  // ← Empty!
    "success": true  // ← Claims success but did nothing
  }
}
```

No directories were renamed, no files updated.

### Workaround Used

Fell back to individual renames (which work perfectly):

```bash
# First rename - WORKS ✅
./target/release/codebuddy tool rename \
  --target "directory:crates/codebuddy-config" \
  --new-name "crates/mill-config" \
  --update-all
# Result: 53 files updated

# Second rename - WORKS ✅
./target/release/codebuddy tool rename \
  --target "directory:crates/codebuddy-auth" \
  --new-name "crates/mill-auth" \
  --update-all
# Result: 16 files updated
```

### Root Cause Investigation Needed

**Files to Check:**
- `crates/mill-handlers/src/handlers/rename_handler/mod.rs` (lines 332-450)
  - `plan_batch_rename()` method
  - Validates all targets have `new_name` ✅
  - Plans each rename individually
  - Merges WorkspaceEdits
- `crates/mill-handlers/src/handlers/quick_rename_handler.rs`
  - Does it support batch mode parameters?

**Possible Causes:**
1. **Parameter parsing issue:** Batch `targets` array not being parsed correctly
2. **Empty edits:** Individual plans generating empty edits that get merged to empty
3. **WorkspaceEdit merging bug:** Edits being lost during merge
4. **Quick rename limitation:** Quick rename doesn't support batch mode at all

### Test Case

**Input:**
```json
{
  "targets": [
    {"kind": "directory", "path": "crates/codebuddy-config", "new_name": "crates/mill-config"},
    {"kind": "directory", "path": "crates/codebuddy-auth", "new_name": "crates/mill-auth"}
  ],
  "options": {"scope": "all"}
}
```

**Expected Output:**
- Plan with 50+ affected files (based on individual rename results)
- WorkspaceEdit with changes for both crates
- Summary showing total affected files

**Actual Output:**
- Plan with 0 affected files
- Empty WorkspaceEdit
- Summary shows 0 changes

---

## Issue #2: Cargo.toml Feature Definitions Not Updated

### Status: ✅ FALSE ALARM - Already Fixed

**Investigation Result:** This is NOT a bug. Feature refs ARE being updated correctly.

### What Actually Happened

During the `codebuddy-config → mill-config` rename, feature definitions **were updated correctly**:

**Git Diff (commit eab9c9c8):**
```diff
 [dependencies]
-codebuddy-config = { path = "../codebuddy-config", optional = true }
+mill-config = { path = "../mill-config", optional = true }

 [features]
-runtime = ["codebuddy-foundation", "codebuddy-config", "codebuddy-ast"]
-mcp-proxy = ["runtime", "codebuddy-config/mcp-proxy"]
+runtime = ["codebuddy-foundation", "mill-config", "codebuddy-ast"]
+mcp-proxy = ["runtime", "mill-config/mcp-proxy"]
```

Both the dependency declaration AND feature refs were updated in the same operation.

### Why This Works

Feature flag updates were fixed in previous sessions:
- **Oct 19** (commit 4c429304): Added feature update logic to `manifest.rs` lines 196-219
- **Oct 21** (commit b296b6fb): Additional feature flag fixes in `cargo_util.rs`

The `rename_dependency()` function already handles:
- ✅ Exact matches: `"codebuddy-config"` → `"mill-config"`
- ✅ Feature references: `"codebuddy-config/feature"` → `"mill-config/feature"`

### Conclusion

No fix needed. This issue can be CLOSED.

---

## Impact

### Issue #1: Batch Rename
- **Severity:** Medium
- **Impact:** Cannot efficiently rename multiple crates at once
- **Workaround:** Use individual renames (works fine, just slower)

### Issue #2: Feature Refs
- **Severity:** N/A (False Alarm)
- **Status:** ✅ Already Working Correctly
- **No Action Needed:** Feature refs ARE being updated by existing code (commits 4c429304, b296b6fb)

---

## Reproduction Steps

### For Issue #1 (Batch Rename):

1. Build latest: `cargo build --release --bin codebuddy`
2. Run batch rename:
   ```bash
   ./target/release/codebuddy tool rename.plan '{
     "targets": [
       {"kind": "directory", "path": "crates/A", "new_name": "crates/B"},
       {"kind": "directory", "path": "crates/C", "new_name": "crates/D"}
     ]
   }'
   ```
3. **Expected:** Plan with multiple affected files
4. **Actual:** Plan with 0 affected files

### For Issue #2 (Feature Refs):

**N/A - This is not a reproducible bug.** Feature refs ARE updated correctly. See Investigation section above for proof via git diff.

---

## Proposed Fix Plan

### Fix #1: Batch Rename Investigation

1. **Add debug logging** to `plan_batch_rename()`:
   - Log each individual plan result
   - Log WorkspaceEdit merge process
   - Check if individual plans are empty

2. **Test individual plans** within batch context:
   - Are they generating edits?
   - Are edits being merged correctly?

3. **Check parameter parsing:**
   - Verify `targets` array is parsed
   - Verify each `new_name` is extracted

4. **Review WorkspaceEdit merge logic:**
   - Are changes being combined correctly?
   - Any deduplication removing valid edits?

### Fix #2: Feature Refs Update

**NO FIX NEEDED** - Feature refs are already working correctly. Code exists at:
- `crates/cb-lang-rust/src/manifest.rs` lines 196-219
- `crates/cb-lang-rust/src/workspace/cargo_util.rs` (additional handling)

Unit test `test_rename_dependency_updates_features` already validates this behavior.

---

## Success Criteria

### Fix #1 Complete When:
- ✅ Batch rename plan shows correct affected files count
- ✅ Batch rename generates proper WorkspaceEdit
- ✅ Batch rename can be applied successfully
- ✅ Test case passes with 2+ directory renames

### Fix #2 Complete When:
- ✅ Feature definitions updated during rename
- ✅ Both simple refs (`"crate"`) and feature refs (`"crate/feature"`) work
- ✅ Unit test added and passing
- ✅ Build passes after rename without manual fixes

---

## Next Steps

1. **Investigate & fix batch rename** (Issue #1)
2. **Implement feature refs update** (Issue #2)
3. **Re-test with same 2 crates** (codebuddy-config, codebuddy-auth)
4. **If successful, use batch rename for remaining 5 codebuddy-* crates**

---

## Related Files

**Batch Rename:**
- `crates/mill-handlers/src/handlers/rename_handler/mod.rs` (lines 332-450)
- `crates/mill-handlers/src/handlers/quick_rename_handler.rs`

**Feature Refs:**
- `crates/cb-lang-rust/src/manifest.rs`
- `crates/codebuddy-plugin-system/Cargo.toml` (test case)

**Test Files:**
- Add: `crates/cb-lang-rust/src/manifest.rs` (unit test for feature refs)
- Add: Integration test for batch rename

---

**Discovered By:** Dogfooding mill-* migration (session 2025-10-22)
**Reported By:** Claude Code
**Priority:** High (blocks efficient bulk renaming)
