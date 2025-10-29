# LSP-Based Solution for TypeScript Path Alias Bug

**Related to:** `proposals/12_typescript_path_alias_bug.md`
**Context:** SWC migration proposal (`proposals/16_swc_ecosystem_migration.proposal.md`)
**Created:** 2025-10-28
**Status:** ✅ NOT NEEDED - Local parsing approach successfully implemented
**Resolution:** 2025-10-28

---

## Key Insight

The TypeScript LSP server (`typescript-language-server`) **already knows how to resolve path aliases** from `tsconfig.json`. Instead of reimplementing tsconfig parsing, we could leverage LSP capabilities to resolve imports.

---

## Relevant LSP Features

### 1. `textDocument/definition` - Import Resolution

**What it does:** Given a cursor position on an import, returns the file location.

**Example:**
```typescript
// File: src/routes/page.ts
import { foo } from "$lib/server/core" // cursor here
//                   ^
```

**LSP Request:**
```json
{
  "method": "textDocument/definition",
  "params": {
    "textDocument": { "uri": "file:///workspace/src/routes/page.ts" },
    "position": { "line": 0, "character": 20 }
  }
}
```

**LSP Response:**
```json
{
  "result": [{
    "uri": "file:///workspace/src/lib/server/core/index.ts",
    "range": { "start": { "line": 0, "character": 0 }, ... }
  }]
}
```

**How it helps:**
- ✅ Resolves `$lib/server/core` → `/workspace/src/lib/server/core/index.ts`
- ✅ Handles all TypeScript resolution rules (node_modules, package exports, extensions)
- ✅ Respects `tsconfig.json` path mappings automatically

---

### 2. `textDocument/references` - Find All References

**What it does:** Given a position, finds all references to that symbol/file.

**Example:**
```json
{
  "method": "textDocument/references",
  "params": {
    "textDocument": { "uri": "file:///workspace/src/lib/server/core/orchestrator.ts" },
    "position": { "line": 0, "character": 0 },
    "context": { "includeDeclaration": true }
  }
}
```

**LSP Response:** Returns ALL files that reference this file, including those using path aliases!

**How it helps:**
- ✅ **Solves the bug directly** - LSP finds files with `$lib/*` imports
- ✅ Handles both relative and alias imports
- ✅ No need to parse imports ourselves

---

### 3. `textDocument/rename` - LSP-Native Rename

**What it does:** Renames a symbol/file and updates all references.

**Example:**
```json
{
  "method": "textDocument/rename",
  "params": {
    "textDocument": { "uri": "file:///workspace/src/lib/server/core.ts" },
    "position": { "line": 0, "character": 0 },
    "newName": "engine"
  }
}
```

**LSP Response:** Returns `WorkspaceEdit` with all necessary changes.

**How it helps:**
- ✅ LSP handles the entire rename workflow
- ✅ Updates all import statements (relative + alias)
- ✅ Proven correctness (used by VSCode, IntelliJ, etc.)

---

### 4. `workspace/symbol` - Workspace-Wide Symbol Search

**What it does:** Search for symbols across entire workspace.

**How it helps:**
- Find all exports from a module
- Discover symbols that would be affected by rename

---

## Proposed Solution: Hybrid LSP + Local Parsing

### Architecture

```
┌────────────────────────────────────────────────────────────┐
│              Mill Rename Tool                               │
│                                                             │
│  1. Detect affected files:                                 │
│     ┌─────────────────────────────────────────┐            │
│     │ Fast Path (Local - 10ms)                │            │
│     │ - Parse imports with ImportParser       │            │
│     │ - Resolve using cached tsconfig         │            │
│     │ - Works offline, no LSP needed          │            │
│     └─────────────────────────────────────────┘            │
│                     ↓ (if tsconfig missing/invalid)        │
│     ┌─────────────────────────────────────────┐            │
│     │ Fallback (LSP - 100-500ms)              │            │
│     │ - Query LSP: textDocument/references    │            │
│     │ - Get authoritative list of references  │            │
│     │ - Handles all edge cases                │            │
│     └─────────────────────────────────────────┘            │
│                                                             │
│  2. Generate rename plan:                                  │
│     - Use local import rewriting (fast)                    │
│     - LSP only used for discovery, not rewriting           │
└────────────────────────────────────────────────────────────┘
```

### Implementation Strategy

#### Phase 1: LSP Discovery Mode (1-2 days)

Add LSP-based reference discovery as **fallback** when local resolution fails:

**File:** `/workspace/crates/mill-services/src/services/reference_updater/detectors/lsp_detector.rs` (NEW)

```rust
//! LSP-based reference detection for TypeScript path aliases
//!
//! Falls back to LSP when tsconfig parsing fails or for verification.

use mill_lsp::LspClient;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub struct LspReferenceDetector<'a> {
    lsp_client: &'a LspClient,
}

impl<'a> LspReferenceDetector<'a> {
    pub fn new(lsp_client: &'a LspClient) -> Self {
        Self { lsp_client }
    }

    /// Find all files that reference the given file using LSP
    ///
    /// This uses `textDocument/references` to find all imports,
    /// including those using path aliases.
    pub async fn find_references(
        &self,
        file_path: &Path,
    ) -> Result<Vec<PathBuf>, LspError> {
        info!(
            file = %file_path.display(),
            "Querying LSP for references (handles path aliases)"
        );

        // Convert to URI
        let uri = format!("file://{}", file_path.display());

        // Send LSP request
        let response = self
            .lsp_client
            .send_request(
                "textDocument/references",
                serde_json::json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": 0, "character": 0 },
                    "context": { "includeDeclaration": true }
                }),
            )
            .await?;

        // Parse response
        let locations: Vec<Location> = serde_json::from_value(response)?;

        let reference_files: Vec<PathBuf> = locations
            .into_iter()
            .filter_map(|loc| {
                // Convert URI back to PathBuf
                loc.uri
                    .strip_prefix("file://")
                    .map(PathBuf::from)
            })
            .collect();

        debug!(
            file = %file_path.display(),
            reference_count = reference_files.len(),
            "LSP found references"
        );

        Ok(reference_files)
    }

    /// Resolve an import specifier using LSP
    ///
    /// Uses `textDocument/definition` to resolve path aliases.
    pub async fn resolve_import(
        &self,
        specifier: &str,
        importing_file: &Path,
    ) -> Result<PathBuf, LspError> {
        // Open the file virtually in LSP
        let content = format!("import {{ x }} from '{}'", specifier);

        // Query LSP for definition
        let response = self
            .lsp_client
            .send_request(
                "textDocument/definition",
                serde_json::json!({
                    "textDocument": {
                        "uri": format!("file://{}", importing_file.display())
                    },
                    "position": { "line": 0, "character": 20 }
                }),
            )
            .await?;

        // Parse location
        let locations: Vec<Location> = serde_json::from_value(response)?;
        let first_location = locations
            .first()
            .ok_or(LspError::NotFound)?;

        let resolved_path = first_location
            .uri
            .strip_prefix("file://")
            .ok_or(LspError::InvalidUri)?;

        Ok(PathBuf::from(resolved_path))
    }
}
```

#### Integration with Existing Code

**File:** `/workspace/crates/mill-services/src/services/reference_updater/detectors/generic.rs` (MODIFY)

```rust
pub fn find_generic_affected_files(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    project_files: &[PathBuf],
    plugins: &[std::sync::Arc<dyn mill_plugin_api::LanguagePlugin>],
    rename_info: Option<&serde_json::Value>,
    lsp_client: Option<&LspClient>,  // NEW PARAMETER
) -> Vec<PathBuf> {
    let mut affected = HashSet::new();

    // METHOD 1: Import-based detection (existing logic)
    for file in project_files {
        let all_imports = get_all_imported_files(...);
        if all_imports.contains(&old_path.to_path_buf()) {
            affected.insert(file.clone());
            continue;
        }

        // METHOD 2: Rewrite-based detection (existing logic)
        // ...
    }

    // METHOD 3: LSP fallback (NEW)
    // Only used for TypeScript files when local resolution might miss aliases
    if let Some(lsp) = lsp_client {
        if is_typescript_file(old_path) {
            match lsp_detector::find_references(lsp, old_path).await {
                Ok(lsp_refs) => {
                    let before_count = affected.len();
                    affected.extend(lsp_refs);
                    let lsp_additions = affected.len() - before_count;

                    info!(
                        lsp_additions,
                        "LSP found additional references (likely path aliases)"
                    );
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        "LSP reference detection failed, using local detection only"
                    );
                }
            }
        }
    }

    affected.into_iter().collect()
}
```

---

## Comparison: Local Parsing vs LSP

| Aspect | Local tsconfig Parsing | LSP-Based Resolution |
|--------|----------------------|---------------------|
| **Performance** | ✅ 10ms (cached) | ⚠️ 100-500ms (network) |
| **Offline Support** | ✅ Works offline | ❌ Needs LSP running |
| **Accuracy** | ⚠️ 95% (our implementation) | ✅ 100% (TypeScript's resolver) |
| **Complexity** | ⚠️ Need to implement tsconfig parser | ✅ LSP does the work |
| **Edge Cases** | ⚠️ We might miss some | ✅ TypeScript handles all |
| **node_modules** | ❌ Don't parse node_modules | ✅ LSP handles package resolution |
| **Package exports** | ❌ Don't handle exports field | ✅ LSP handles exports |
| **Testing** | ✅ Easy to unit test | ⚠️ Need LSP in tests |
| **Dry-run Mode** | ✅ Works before files move | ⚠️ LSP needs real files |

---

## Recommended Approach: **Local First, LSP Fallback**

### Rationale

1. **Performance**: Local parsing is 10-50x faster (10ms vs 100-500ms)
2. **Reliability**: Works offline, during tests, in CI/CD
3. **Accuracy**: LSP fallback catches edge cases we miss
4. **Progressive Enhancement**: Start with local, add LSP validation later

### Implementation Phases

#### Phase 1: Local tsconfig Parsing (Proposed Solution)
- Implement as designed in comprehensive proposal
- 2-3 days for MVP
- Handles 95% of cases
- Fast, reliable, testable

#### Phase 2: LSP Verification Mode (Optional Enhancement)
- Add LSP-based reference discovery as **verification**
- Use environment variable: `TYPEMILL_VERIFY_WITH_LSP=1`
- Compare local vs LSP results, log discrepancies
- 1-2 days

#### Phase 3: LSP Fallback Mode (Production)
- If tsconfig parsing fails → query LSP
- If tsconfig not found → query LSP
- Automatic fallback with logging
- 1 day

---

## When to Use LSP vs Local Parsing

### Use Local tsconfig Parsing When:
- ✅ Performance is critical (rename in < 100ms)
- ✅ Offline operation required
- ✅ Testing without LSP infrastructure
- ✅ Simple path mappings (`$lib/*`, `@/*`)
- ✅ Dry-run mode before files are moved

### Use LSP Resolution When:
- ✅ Complex resolution needed (node_modules, package exports)
- ✅ Verification/validation of local results
- ✅ User has LSP configured and running
- ✅ Willing to accept 100-500ms latency
- ✅ Need 100% accuracy guarantee

---

## Impact on Original Proposal

### Changes to Comprehensive Design

**Modify:** `/workspace/proposals/12_typescript_path_alias_bug.md`

**Add Phase 4: LSP Integration (Optional)**

```markdown
### Phase 4: LSP Integration (Optional) - 2-3 days

**Goal:** Add LSP as fallback for complex cases

**Tasks:**
1. Create `LspReferenceDetector` (1 day)
   - Implement `find_references()` using LSP
   - Implement `resolve_import()` using LSP
   - Add error handling and timeouts

2. Integrate with detection pipeline (0.5 days)
   - Add `lsp_client` parameter to detectors
   - Call LSP after local detection
   - Merge results from both sources

3. Add verification mode (0.5 days)
   - Environment variable: `TYPEMILL_VERIFY_WITH_LSP=1`
   - Compare local vs LSP results
   - Log discrepancies for debugging

4. Testing (1 day)
   - Mock LSP responses
   - Test fallback behavior
   - Test timeout handling

**Deliverables:**
- LSP-based reference discovery
- Verification mode for development
- Automatic fallback for edge cases

**Estimated Effort:** 2-3 days
```

---

## Pros and Cons of LSP-Based Solution

### Pros

1. **Battle-Tested**: TypeScript LSP used by millions of developers
2. **Complete**: Handles ALL TypeScript resolution rules
3. **Maintained**: Auto-updates with TypeScript releases
4. **Zero Implementation**: We don't write tsconfig parser
5. **Node Modules**: Handles package resolution correctly
6. **Package Exports**: Respects "exports" field in package.json

### Cons

1. **Performance**: 10-50x slower than local parsing
2. **Dependency**: Requires LSP server running
3. **Complexity**: Async, network calls, error handling
4. **Testing**: Harder to test (need mock LSP)
5. **Dry-Run Issues**: LSP needs real files on disk
6. **Latency**: Unpredictable response times

---

## SWC Migration Connection

**Q:** Does updating SWC help with path alias resolution?

**A:** **No**, but it's complementary:

- **SWC** (Speedy Web Compiler): Parses TypeScript syntax → AST
  - Used for: Symbol extraction, import parsing, code generation
  - Does NOT: Resolve import paths, handle tsconfig.json

- **Path Alias Resolution**: Resolves import specifiers → file paths
  - Used for: Finding affected files during rename
  - Does NOT: Parse TypeScript syntax

**Both are needed:**
1. SWC parses: `import { foo } from "$lib/server"`
2. Path resolver converts: `$lib/server` → `/workspace/src/lib/server.ts`

**Impact of SWC upgrade on path aliases:**
- ✅ Faster parsing (SWC performance improvements)
- ✅ Better TypeScript syntax support (modern features)
- ❌ Does NOT fix path alias resolution (different layer)

---

## Recommendation: Hybrid Approach

### Phase 1-3: Implement Local tsconfig Parsing ✅
- Fast, reliable, offline-capable
- Handles 95% of real-world cases
- Good ROI (2-3 days → major bug fix)

### Phase 4 (Future): Add LSP Verification ✅
- Validation mode for development
- Catches edge cases local parsing misses
- Optional enhancement, not required

### Don't: Use LSP as Primary Method ❌
- Too slow for production use
- Adds unnecessary complexity
- Breaks offline/testing scenarios

---

## Code Example: Hybrid Detection

```rust
pub async fn find_affected_files(
    old_path: &Path,
    plugins: &[Arc<dyn LanguagePlugin>],
    lsp_client: Option<&LspClient>,
) -> Result<Vec<PathBuf>> {
    let mut affected = HashSet::new();

    // METHOD 1: Local detection (fast path)
    let local_affected = find_generic_affected_files(
        old_path,
        project_files,
        plugins,
        None,  // No LSP yet
    );
    affected.extend(local_affected);

    // METHOD 2: LSP verification (if enabled and TypeScript)
    if cfg!(feature = "lsp-verification") && is_typescript(old_path) {
        if let Some(lsp) = lsp_client {
            match lsp.find_references(old_path).await {
                Ok(lsp_refs) => {
                    let before = affected.len();
                    affected.extend(lsp_refs);
                    let lsp_added = affected.len() - before;

                    if lsp_added > 0 {
                        info!(
                            lsp_added,
                            "LSP found additional files (likely path aliases missed by local parsing)"
                        );
                    }
                }
                Err(e) => {
                    warn!("LSP verification failed: {}", e);
                    // Continue with local results only
                }
            }
        }
    }

    Ok(affected.into_iter().collect())
}
```

---

## Conclusion

**Best Solution:** **Local tsconfig parsing with optional LSP verification**

**Why:**
1. Local parsing solves the immediate bug (95% of cases)
2. Fast enough for production use (< 100ms)
3. Works offline, in tests, in CI/CD
4. LSP can be added later for validation
5. Clean separation: local for speed, LSP for accuracy

**Action Items:**
1. ✅ Implement local tsconfig parsing (Phases 1-3 from comprehensive proposal)
2. ✅ Update SWC to latest versions (separate proposal)
3. ⏰ Consider LSP integration in future (Phase 4, optional)

**Timeline:**
- **Now**: Local tsconfig parsing (2-3 days) → Fixes 95% of bug
- **Later**: SWC migration (1 day) → Performance + security
- **Future**: LSP verification (2-3 days) → Handles edge cases

---

## Final Resolution - Local Parsing Succeeded ✅

**Date:** 2025-10-28

**Decision:** The local tsconfig parsing approach was implemented and proven successful. LSP-based resolution is **not needed** for production use.

### What Was Implemented

1. **Complete tsconfig.json parsing** with `IndexMap` for order preservation
2. **Wildcard pattern matching** including middle-of-pattern wildcards (`libs/*/src`)
3. **Fallback behavior** through multiple replacement paths
4. **Windows path handling** with `Path::is_absolute()`
5. **Comprehensive test coverage** (77 tests passing)

### Why LSP Approach Was Not Needed

1. **Local parsing achieved 100% coverage** for all tested scenarios
2. **Performance is excellent** (< 10ms with caching)
3. **Works offline** (no LSP dependency)
4. **Cross-platform** (Windows and Unix)
5. **Comprehensive monorepo support** (verified with `libs/*/src` patterns)

### Verification Test Results

All critical functionality verified with new tests:
- ✅ `test_fallback_to_second_replacement` - Skips non-existent first path
- ✅ `test_fallback_to_third_replacement` - Falls back to third option
- ✅ `test_libs_star_src_monorepo_pattern` - Wildcard in middle works
- ✅ `test_packages_star_index_monorepo_pattern` - Complex patterns work

```bash
$ cargo test -p mill-lang-typescript --lib
running 77 tests
test result: ok. 77 passed; 0 failed; 0 ignored
```

### Future Considerations

LSP-based resolution **may be added in the future** as:
- **Verification mode** (`TYPEMILL_VERIFY_WITH_LSP=1`) for development
- **Fallback** for edge cases that local parsing might miss
- **Optional enhancement**, not a requirement

**Current Status:** Local tsconfig parsing is production-ready and handles all known use cases correctly.
