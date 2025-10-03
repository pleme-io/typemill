# Bug Report & Known Issues

This document tracks known bugs, limitations, and areas for improvement in Codebuddy.

## ✅ Resolved Issues

### Cargo Dependency Management (Issues #2, #6) - RESOLVED
**Resolution Date:** 2025-10-03 (Phase 4)
**Tool Added:** `update_dependency`

**Original Problem:**
- `rename_directory` didn't update Cargo.toml dependency names or paths
- Moving packages broke relative path dependencies
- Required manual sed/grep to fix all Cargo.toml files

**Solution:**
Created language-agnostic `update_dependency` tool using Manifest trait:
- Supports Cargo.toml (Rust) and package.json (JavaScript/TypeScript)
- Automatically updates dependency names and paths
- Extensible to other package managers (Python, Go, etc.)

**Usage:**
```bash
codebuddy tool update_dependency '{
  "manifest_path": "crates/my-crate/Cargo.toml",
  "old_dep_name": "old-package",
  "new_dep_name": "new-package",
  "new_path": "../new-package"
}'
```

**Note:** Currently requires exact filename match (`Cargo.toml` or `package.json`). Optional parameters like `optional = true` still need manual adjustment.

---

### Git-Aware File Operations (Issue #2) - RESOLVED
**Resolution Date:** 2025-10-03 (Pre-Phase 4 Sprint)
**Tool Enhanced:** All file operation tools now use git when available

**Original Problem:**
- File operations didn't use `git mv`, losing git history tracking
- Required manual `git add -A` to let git detect renames
- Files showed as deleted + new instead of renamed

**Solution:**
Created GitService and integrated into FileService:
- Auto-detects git repositories on initialization
- Uses `git mv` for tracked files automatically
- Falls back to filesystem operations when git unavailable
- Runs git commands in `spawn_blocking` to avoid blocking async runtime

**Files Modified:**
- `crates/cb-services/src/services/git_service.rs` (new)
- `crates/cb-services/src/services/file_service.rs` (enhanced)

**Impact:** File operations now preserve git history automatically!

---

### Batch Dependency Updates (Enhancement #2) - PARTIALLY RESOLVED
**Resolution Date:** 2025-10-03 (Pre-Phase 4 Sprint)
**Tool Added:** `batch_update_dependencies` (44th MCP tool)

**What's Implemented:**
- ✅ Batch update mode for multi-file refactorings
- ✅ Auto-detect from workspace root and update all referencing crates
- ✅ Aggregated result reporting with success/failure counts

**What's Still Needed:**
- ❌ Support inline dependency features (`optional = true`, `features = [...]`)
- ❌ Preserve existing metadata when renaming dependencies

**Usage:**
```bash
codebuddy tool batch_update_dependencies '{
  "updates": [
    {"old_dep_name": "old-pkg", "new_dep_name": "new-pkg", "new_path": "../new-pkg"}
  ]
}'
# Auto-discovers all Cargo.toml files and updates them
```

---

### E2E Test Config Loading (Issue #1) - RESOLVED
**Resolution Date:** ce965a5
**Root Cause:** Incorrect CacheConfig JSON structure in test fixtures
**Fix:** Corrected field names (`maxSizeBytes` vs `maxSizeMb`) and added required fields

---

## ✅ More Resolved Issues

### Enhanced Import Scanning (Issue #1) - RESOLVED
**Resolution Date:** 2025-10-03 (Phase IS - Import Scanning)
**Tool Enhanced:** `rename_directory` now supports configurable import scanning

**Original Problem:**
- Only top-level `use` statements were updated
- Missed function-scoped imports (`#[test]` functions)
- Missed qualified paths in code (`old_module::function()`)
- No support for deep scanning

**Solution:**
Implemented multi-level AST-based import scanning with EditPlan integration:
- ✅ Added `ScanScope` enum (TopLevelOnly, AllUseStatements, QualifiedPaths, All)
- ✅ Added `find_module_references()` to LanguageAdapter trait
- ✅ Fully implemented TypeScript/JavaScript scanning with SWC visitor pattern
- ✅ Added user-facing `update_mode` parameter (Conservative/Standard/Aggressive)
- ✅ Integrated with EditPlan for precise surgical text edits
- ✅ Backward compatible - defaults to Conservative mode

**Files Modified:**
- `crates/cb-ast/src/language.rs` - Enhanced trait with find_module_references()
- `crates/cb-ast/src/import_updater.rs` - Returns EditPlan with precise TextEdits
- `crates/cb-services/src/services/import_service.rs` - Passes through EditPlan
- `crates/cb-services/src/services/file_service.rs` - Applies EditPlan via apply_edit_plan()
- `crates/cb-handlers/src/handlers/tools/workspace.rs` - UpdateMode API

**Usage:**
```json
{
  "name": "rename_directory",
  "arguments": {
    "old_path": "src/old_module",
    "new_path": "src/new_module",
    "update_mode": "aggressive"
  }
}
```

**Impact:** `rename_directory` now finds and updates:
- ✅ Top-level imports (all modes)
- ✅ Function-scoped imports (Standard/Aggressive)
- ✅ Qualified paths like `module.method()` (Aggressive)
- ✅ Works with TypeScript/JavaScript via SWC AST visitor
- ✅ Precise line/column edits instead of full-file rewrites

---

## 🐛 Active Issues

### 1. Test Flakiness
**Severity:** Low
**Affected Test:** `resilience_tests::test_basic_filesystem_operations`

Intermittent timeouts and JSON parsing errors ("trailing characters"). Likely timing/initialization issue with integration test infrastructure, not a regression.

---

## 📋 Enhancement Requests

### 1. update_dependency Tool - Metadata Preservation
- Support inline dependency features (e.g., `optional = true`, `features = [...]`)
- Preserve existing metadata when renaming dependencies
- **Note:** Batch mode and auto-discovery are already implemented (see Resolved Issues)

### 2. Post-Operation Validation
- Run `cargo check` after refactoring operations
- Report compilation errors with suggestions
- Optional rollback on validation failure

### 3. Better MCP Error Reporting
- The update_dependency tool returns JSON errors but CLI expects string messages
- Need consistent error format across all tools

---

## 🔧 Phase 4 Refactoring Experience (2025-10-03)

### Successful Dogfooding! 🎉

**Tools Used:**
1. ✅ `batch_execute` - Moved 7 files from cb-mcp-proxy to cb-plugins/mcp
2. ✅ `update_dependency` - Updated Cargo.toml files (cb-client, cb-server)

**Manual Adjustments Still Needed:**
1. Feature flags in Cargo.toml (mcp-proxy feature)
2. Import path updates (crate:: → super:: for mcp module)
3. Exposing new module in lib.rs

**Key Learning:**
Our new `update_dependency` tool worked perfectly for basic dependency renaming! The refactoring went much smoother than Phase 3 because we could automate the Cargo.toml updates.

**Git Rename Detection:**
Even though batch_execute doesn't use git mv, running `git add -A` allowed Git to properly detect all 7 file renames. No history lost.

---

## 📝 Notes

### Best Practices for Large Refactorings

1. **Always use dry_run first:**
   ```bash
   codebuddy tool rename_directory '{"old_path":"...","new_path":"...","dry_run":true}'
   ```

2. **For package renames:**
   ```bash
   # 1. Move files
   codebuddy tool rename_directory ...

   # 2. Update dependencies
   codebuddy tool update_dependency '{"manifest_path":"...","old_dep_name":"...","new_dep_name":"..."}'

   # 3. Fix imports
   cargo check --workspace 2>&1 | grep error

   # 4. Stage all changes
   git add -A  # Let git detect renames
   ```

3. **Validate continuously:**
   - Run `cargo check` after each major step
   - Run tests before committing
   - Check git diff to ensure renames are detected

---

**Last Updated:** Pre-Phase 4 Sprint complete (2025-10-03)
**Tool Count:** 44 MCP tools registered (added batch_update_dependencies)
**Test Coverage:** 244 library tests, 13 CLI integration tests
