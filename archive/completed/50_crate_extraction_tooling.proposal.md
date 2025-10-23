# Proposal 50: Crate Extraction Tooling (Dogfooding Enhancement)

## Problem

While codebuddy has excellent **consolidation** capabilities (merging crates together via `rename.plan` with `consolidate: true`), it lacks the reverse operation: **extracting modules from a crate into a new standalone crate**. This makes the "split god crate" workflow (Proposal 09) only ~50% automated, requiring significant manual intervention.

**Current gaps when splitting `codebuddy-core` into smaller crates:**
1. ❌ No tool to create a new Rust crate with proper structure (Cargo.toml, src/lib.rs)
2. ❌ No tool to analyze which dependencies a module needs
3. ❌ No tool to extract dependencies FROM a crate into a new Cargo.toml
4. ❌ No public MCP tool to update workspace members (internal-only)
5. ❌ No "split mode" in `rename.plan` (reverse of consolidation)

**Dogfooding opportunity:** This proposal enables codebuddy to perform the god crate decomposition workflow using its own tools, demonstrating MCP tool capabilities and identifying real-world gaps.

## Solution

Add 3 new public MCP tools + 1 enhancement to existing tool for complete crate extraction workflow.

### 1. New Tool: `workspace.create_crate`

**What it does:** Create a new Rust crate with proper structure and register it in workspace.

**Parameters:**
```json
{
  "crate_path": "../../crates/mill-auth",     // Required
  "crate_type": "lib",                       // Optional: "lib" (default) or "bin"
  "options": {
    "dryRun": true,                         // Optional: preview mode
    "addToWorkspace": true,                // Optional: auto-register (default: true)
    "template": "minimal"                    // Optional: "minimal" (default) or "full"
  }
}
```

**Returns:**
```json
{
  "created_files": [
    "../../crates/mill-auth/Cargo.toml",
    "../../crates/mill-auth/src/lib.rs"
  ],
  "workspace_updated": true,
  "cargo_toml": {
    "package_name": "mill-auth",
    "version": "0.1.0"
  }
}
```

**Implementation:**
- Use `RustWorkspaceSupport::add_workspace_member()` (already exists)
- Generate Cargo.toml with minimal dependencies
- Create `src/lib.rs` with standard header
- Support dry-run mode

### 2. New Tool: `analyze.module_dependencies`

**What it does:** Analyze which Cargo dependencies a module/file needs by parsing imports.

**Parameters:**
```json
{
  "target": {
    "kind": "file",                          // "file" or "directory"
    "path": "../../../../crates/mill-foundation/src/core/src/auth.rs"
  },
  "options": {
    "include_dev_dependencies": false,       // Optional
    "include_workspace_deps": true           // Optional: include workspace-internal deps
  }
}
```

**Returns:**
```json
{
  "external_dependencies": {
    "tokio": { "version": "1.0", "features": ["full"] },
    "jsonwebtoken": { "version": "9.0" },
    "serde": { "version": "1.0", "features": ["derive"] }
  },
  "workspace_dependencies": [
    "cb-types",
    "cb-protocol"
  ],
  "import_analysis": {
    "total_imports": 12,
    "external_crates": 3,
    "workspace_crates": 2
  }
}
```

**Implementation:**
- Use existing `RustPlugin::analyze_imports()`
- Cross-reference with workspace Cargo.toml to identify external vs internal deps
- Parse version requirements from workspace manifest

### 3. New Tool: `workspace.extract_dependencies`

**What it does:** Extract required dependencies from one Cargo.toml and add them to another (reverse of merge).

**Parameters:**
```json
{
  "source_manifest": "../../../../crates/mill-foundation/src/core/Cargo.toml",
  "target_manifest": "../../crates/mill-auth/Cargo.toml",
  "dependencies": ["tokio", "jsonwebtoken", "serde"],  // From analyze.module_dependencies
  "options": {
    "dryRun": true,
    "preserve_versions": true,                         // Copy exact version specs
    "preserve_features": true                          // Copy feature flags
  }
}
```

**Returns:**
```json
{
  "dependencies_extracted": 3,
  "dependencies_added": [
    { "name": "tokio", "version": "1.0", "features": ["full"] },
    { "name": "jsonwebtoken", "version": "9.0" },
    { "name": "serde", "version": "1.0", "features": ["derive"] }
  ],
  "target_manifest_updated": true
}
```

**Implementation:**
- Reuse `merge_cargo_dependencies` logic (extract subset instead of merge all)
- Use `toml_edit::DocumentMut` for structure-preserving edits
- Support dry-run preview

### 4. Enhancement: `rename.plan` Split Mode

**What it does:** Add `split: true` option to `rename.plan` for extracting modules into new crates (reverse of consolidation).

**Parameters:**
```json
{
  "target": {
    "kind": "file",
    "path": "../../../../crates/mill-foundation/src/core/src/auth.rs"
  },
  "newName": "../../crates/mill-auth/src/lib.rs",
  "options": {
    "split": true,                           // NEW: Extract module into new crate
    "analyze_dependencies": true             // NEW: Include dependency analysis in plan
  }
}
```

**Enhanced Plan Output:**
```json
{
  "plan_type": "rename",
  "metadata": {
    "operation": "split_to_new_crate",
    "old_module": "codebuddy_core::auth",
    "new_crate": "codebuddy_auth"
  },
  "dependency_analysis": {                    // NEW SECTION
    "required_dependencies": ["tokio", "jsonwebtoken"],
    "workspace_dependencies": ["cb-types"],
    "suggested_cargo_toml": "..."
  },
  "edits": [
    /* ... LSP WorkspaceEdit with import updates ... */
  ],
  "warnings": [
    "After extraction, remove 'pub mod auth;' from ../../../../crates/mill-foundation/src/core/src/lib.rs"
  ]
}
```

**Implementation:**
- Detect split mode when source is inside `src/` and target is in different crate
- Call `analyze.module_dependencies` during planning phase
- Include dependency requirements in plan metadata
- Auto-detect similar to consolidation mode

### 5. Expose `workspace.update_members` (Public API)

**What it does:** Make workspace member management publicly accessible via MCP.

**Parameters:**
```json
{
  "operation": "add",                        // "add" or "remove"
  "member": "../../crates/mill-auth",
  "manifest_path": "./Cargo.toml",           // Optional: defaults to workspace root
  "options": {
    "dryRun": true
  }
}
```

**Returns:**
```json
{
  "operation": "add",
  "member": "../../crates/mill-auth",
  "manifest_updated": true,
  "current_members": [
    "../../../../crates/mill-foundation/src/core",
    "../../crates/mill-auth",
    // ... etc
  ]
}
```

**Implementation:**
- Expose existing `RustWorkspaceSupport::{add,remove}_workspace_member()` via MCP
- Currently internal-only, should be public for workspace management
- Move from internal tools to public API

## End-to-End Workflow Example

**Goal:** Extract `auth` module from `codebuddy-core` into new `mill-auth` crate.

### Step 1: Create new crate structure
```json
{
  "tool": "workspace.create_crate",
  "arguments": {
    "crate_path": "../../crates/mill-auth",
    "options": { "dryRun": false }
  }
}
```
✅ Creates `Cargo.toml`, `src/lib.rs`, adds to workspace members

### Step 2: Analyze dependencies
```json
{
  "tool": "analyze.module_dependencies",
  "arguments": {
    "target": {
      "kind": "file",
      "path": "../../../../crates/mill-foundation/src/core/src/auth.rs"
    }
  }
}
```
✅ Returns: `["tokio", "jsonwebtoken", "serde"]` + workspace deps

### Step 3: Extract dependencies to new crate
```json
{
  "tool": "workspace.extract_dependencies",
  "arguments": {
    "source_manifest": "../../../../crates/mill-foundation/src/core/Cargo.toml",
    "target_manifest": "../../crates/mill-auth/Cargo.toml",
    "dependencies": ["tokio", "jsonwebtoken", "serde"],
    "options": { "dryRun": false }
  }
}
```
✅ Copies dependencies with versions/features

### Step 4: Move module code with import updates
```json
{
  "tool": "rename.plan",
  "arguments": {
    "target": {
      "kind": "file",
      "path": "../../../../crates/mill-foundation/src/core/src/auth.rs"
    },
    "newName": "../../crates/mill-auth/src/lib.rs",
    "options": { "split": true }
  }
}
```
✅ Plan includes all import updates across workspace

### Step 5: Apply the plan
```json
{
  "tool": "workspace.apply_edit",
  "arguments": {
    "plan": "<plan from step 4>",
    "options": { "dryRun": false }
  }
}
```
✅ Moves file, updates all `use codebuddy_core::auth::*` → `use codebuddy_auth::*`

### Step 6: Manual cleanup (minimal)
- Remove `pub mod auth;` from `../../../../crates/mill-foundation/src/core/src/lib.rs`
- Run `cargo check --workspace`

## Success Criteria

1. ✅ New crates can be created programmatically with proper structure
2. ✅ Dependency analysis identifies exact requirements for a module
3. ✅ Dependencies can be extracted from one Cargo.toml to another
4. ✅ `rename.plan` supports split mode with dependency analysis
5. ✅ Workspace members can be managed via public MCP tools
6. ✅ Complete god crate decomposition workflow is **~90% automated** (up from 50%)
7. ✅ All new tools support dry-run mode for safe previews
8. ✅ Codebuddy can dogfood its own tools for crate restructuring

## Benefits

**For Users:**
- Complete automation of crate extraction workflow
- Safe, preview-driven refactoring (dry-run by default)
- Works with any Rust workspace, not just codebuddy

**For Codebuddy Project:**
- Dogfooding proves MCP tool capabilities
- Identifies real-world workflow gaps
- Demonstrates value proposition to users
- Enables Proposal 09 completion using codebuddy itself

**For MCP Ecosystem:**
- Showcases advanced workspace operations
- Provides reference implementation for language-specific refactoring
- Proves MCP can handle complex, multi-step workflows

## Implementation Notes

### Code Reuse
- ✅ `RustWorkspaceSupport` - Already implements workspace member operations
- ✅ `merge_cargo_dependencies` - Reverse logic for extraction
- ✅ `RustPlugin::analyze_imports` - Dependency analysis foundation
- ✅ `consolidate_rust_package` - Reference for split mode logic

### New Code Required
- `analyze.module_dependencies` handler (~200 lines)
- `workspace.create_crate` handler (~150 lines)
- `workspace.extract_dependencies` handler (~100 lines, reuses merge logic)
- `workspace.update_members` handler (~50 lines, exposes existing trait)
- `rename.plan` split mode detection (~100 lines, mirrors consolidation)

**Total estimated LOC:** ~600 lines (minimal, leverages existing infrastructure)

### Testing Strategy
- Unit tests for each new tool
- Integration test for end-to-end extraction workflow
- Dogfooding test: Extract a real module from codebuddy-core
- Verify workspace still builds after extraction

## Timeline

**Phase 1 (Week 1):** Core tools
- Implement `workspace.create_crate`
- Implement `analyze.module_dependencies`
- Implement `workspace.extract_dependencies`

**Phase 2 (Week 2):** Enhanced rename
- Add split mode to `rename.plan`
- Expose `workspace.update_members`

**Phase 3 (Week 3):** Testing & dogfooding
- Integration tests
- Dogfood on real codebuddy module extraction
- Documentation updates

## Related Proposals

- **Proposal 09:** Split `codebuddy-core` God Crate (enables completion)
- **Proposal 06:** Workspace Consolidation (inverse operation)
- **Proposal 45:** Unified Analysis API (provides dependency analysis foundation)

## Open Questions

1. Should `workspace.create_crate` support other languages (Go, TypeScript)?
   - **Answer:** Start with Rust only, extend later if needed

2. Should split mode be auto-detected like consolidation mode?
   - **Answer:** Yes, detect when moving from `src/` to different crate

3. Should we support multi-file extraction (directory split)?
   - **Answer:** Yes, `target.kind: "directory"` already supported in rename.plan

4. Should dependency analysis be a separate tool or integrated into rename.plan?
   - **Answer:** Both - separate tool for flexibility, integrated for convenience
