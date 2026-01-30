# TypeMill Tools Verification Report

## Environment
- **Project**: Redux (cloned from https://github.com/reduxjs/redux)
- **Language**: TypeScript
- **Toolchain**: mill 0.8.4 (dev build)

## Summary

| Tool | Status | Issues |
|------|--------|--------|
| `inspect_code` | ✅ Working | returned self-reference for imports (LSP behavior) |
| `search_code` | ❌ Failed | "No Project" error from typescript-language-server |
| `rename_all` | ⚠️ Issues | Operation succeeds, but summary reports "0 files renamed" (JSON parsing bug) |
| `relocate` | ✅ Working | Correctly moved files and updated imports |
| `prune` | ✅ Working | Correctly deleted file |
| `refactor` | ❌ Broken | `dryRun` fails with JSON error; Execution produces broken code (missing definition) |
| `workspace` | ⚠️ Issues | Works, but `scope` parameter requires object (not string as per some docs) |

## Detailed Findings

### 1. search_code
The tool failed to return results for a known symbol `createStore`.
**Error**:
```
<syntax> TypeScript Server Error (5.9.3)
No Project.
Error: No Project.
...
```
**Cause**: The `typescript-language-server` instance started by the plugin does not seem to recognize the project root or load the project context correctly, even when `rootDir` is configured.

### 2. rename_all
The operation successfully renamed `src/utils/warning.ts` to `src/utils/warning_renamed.ts` and updated references in `src/combineReducers.ts`.
**Issue**: The output JSON reported:
```json
"summary": "Successfully renamed 0 file(s)"
```
despite `appliedFiles` containing 2 entries.
**Cause**: Likely a mismatch between snake_case `applied_files` in the handler's lookup and the actual camelCase `appliedFiles` in the result object from the service.

### 3. refactor (extract)
**Issue 1**: `dryRun: true` fails.
```json
"error": "Internal error: Internal error: Unexpected plan format: missing ExtractPlan or InlinePlan"
```
**Cause**: `RefactorHandler` expects tagged enum JSON (`{"ExtractPlan": {...}}`) but `RefactorPlan` uses `#[serde(untagged)]`, producing flat JSON.

**Issue 2**: Execution (`dryRun: false`) produces broken code.
Attempted to extract a block from `src/createStore.ts`.
Result:
- The extracted function definition was **missing** from the file.
- The replacement call was malformed/misplaced.
- The original code was partially deleted/mangled.

### 4. workspace (find_replace)
**Issue**: passing `"scope": "workspace"` (string) fails with:
```
invalid type: string "workspace", expected struct ScopeConfig
```
**Fix**: Must pass `"scope": {}` (object). CLI examples/docs might be outdated.
