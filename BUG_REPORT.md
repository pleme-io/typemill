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

### E2E Test Config Loading (Issue #1) - RESOLVED
**Resolution Date:** ce965a5
**Root Cause:** Incorrect CacheConfig JSON structure in test fixtures
**Fix:** Corrected field names (`maxSizeBytes` vs `maxSizeMb`) and added required fields

---

## 🐛 Active Issues

### 1. Incomplete Import Path Updates During `rename_directory`
**Severity:** Medium

Only top-level `use` statements are updated. Missed references:
- Imports inside function bodies (`#[test]` functions)
- Qualified paths in code (`old_module::function()`)
- Module references in strings

**Workaround:** Manual find-and-replace after `rename_directory`

---

### 2. Batch File Operations Don't Use Git
**Severity:** Low
**Component:** `batch_execute`, all file operation tools

**Description:**
File operations (rename_file, etc.) copy/move files without using `git mv`, losing Git history tracking.

**Impact:**
- Git shows files as deleted + new instead of renamed
- Lose file history in `git log --follow`
- Requires `git add -A` to let Git auto-detect renames

**Example:**
```bash
# codebuddy creates:
deleted:    old/file.rs
new file:   new/file.rs

# But git add -A recovers rename detection:
renamed:    old/file.rs -> new/file.rs
```

**Enhancement Request:**
Detect if working directory is a git repository and use `git mv` instead of filesystem operations. Fallback to regular fs ops if not in git repo.

**Implementation Idea:**
```rust
fn is_git_repo() -> bool {
    Command::new("git").args(&["rev-parse", "--git-dir"]).status().is_ok()
}

fn rename_file(old: &Path, new: &Path) -> Result<()> {
    if is_git_repo() {
        Command::new("git").args(&["mv", old, new]).status()?;
    } else {
        std::fs::rename(old, new)?;
    }
}
```

---

### 3. Test Flakiness
**Severity:** Low
**Affected Test:** `resilience_tests::test_basic_filesystem_operations`

Intermittent timeouts and JSON parsing errors ("trailing characters"). Likely timing/initialization issue with integration test infrastructure, not a regression.

---

## 📋 Enhancement Requests

### 1. Enhanced Import Scanning
- Update qualified paths (`module::function`) in addition to `use` statements
- Scan function-scoped imports
- Configurable scope with pattern matching

### 2. update_dependency Tool Improvements
- Support inline dependency features (e.g., `optional = true`, `features = [...]`)
- Batch update mode for multi-file refactorings
- Auto-detect from workspace root and update all referencing crates

### 3. Post-Operation Validation
- Run `cargo check` after refactoring operations
- Report compilation errors with suggestions
- Optional rollback on validation failure

### 4. Better MCP Error Reporting
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

**Last Updated:** Phase 4 complete (2025-10-03)
**Tool Count:** 43 MCP tools registered
**Test Coverage:** 244 library tests, 13 CLI integration tests
