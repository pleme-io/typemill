# Workspace Tools

Package management operations for multi-crate Rust workspaces. Create new packages, extract dependencies for crate extraction workflows, and manage workspace member lists in Cargo.toml.

**Tool count:** 3 tools
**Related categories:** [Refactoring](refactoring.md) (rename for crate consolidation), [Analysis](analysis.md) (analyze.module_dependencies for dependency analysis)

## Table of Contents

- [Tools](#tools)
  - [workspace.create_package](#workspacecreate_package)
  - [workspace.extract_dependencies](#workspaceextract_dependencies)
  - [workspace.update_members](#workspaceupdate_members)
- [Common Patterns](#common-patterns)
  - [Crate Extraction Workflow](#crate-extraction-workflow)
  - [Package Creation with Dependencies](#package-creation-with-dependencies)
  - [Workspace Reorganization](#workspace-reorganization)
  - [Dependency Audit Before Extraction](#dependency-audit-before-extraction)
- [Integration with Other Tools](#integration-with-other-tools)
  - [With rename.plan (Crate Consolidation)](#with-renameplan-crate-consolidation)
  - [With analyze.module_dependencies](#with-analyzemodule_dependencies)

---

## Tools

### workspace.create_package

**Purpose:** Create a new Rust package (library or binary) with proper manifest and source structure.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| package_path | string | Yes | Absolute or workspace-relative path for new package (e.g., "crates/my-lib") |
| package_type | string | No | Package type: "library" or "binary" (default: "library") |
| options | object | No | Creation options |
| options.dry_run | boolean | No | Preview operation (not yet supported - returns error) |
| options.add_to_workspace | boolean | No | Add to workspace members list (default: true) |
| options.template | string | No | Template: "minimal" or "full" (default: "minimal") |

**Returns:**

Object with creation details:
- `created_files` (string[]): Absolute paths to all created files
- `workspace_updated` (boolean): Whether workspace manifest was updated
- `package_info` (object): Package metadata (name, version, manifest_path)
- `dry_run` (boolean): Whether this was a dry-run

**Example:**

```json
// MCP request - Create library with minimal template
{
  "method": "tools/call",
  "params": {
    "name": "workspace.create_package",
    "arguments": {
      "package_path": "/workspace/crates/my-util",
      "package_type": "library",
      "options": {
        "dry_run": false,
        "add_to_workspace": true,
        "template": "minimal"
      }
    }
  }
}

// Response
{
  "result": {
    "created_files": [
      "/workspace/crates/my-util/Cargo.toml",
      "/workspace/crates/my-util/src/lib.rs"
    ],
    "workspace_updated": true,
    "package_info": {
      "name": "my-util",
      "version": "0.1.0",
      "manifest_path": "/workspace/crates/my-util/Cargo.toml"
    },
    "dry_run": false
  }
}
```

**Example - Binary package:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace.create_package",
    "arguments": {
      "package_path": "/workspace/crates/my-cli",
      "package_type": "binary",
      "options": {
        "add_to_workspace": true,
        "template": "minimal"
      }
    }
  }
}

// Creates src/main.rs instead of src/lib.rs
```

**Example - Full template:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace.create_package",
    "arguments": {
      "package_path": "/workspace/crates/full-lib",
      "package_type": "library",
      "options": {
        "template": "full"
      }
    }
  }
}

// Creates additional structure:
// - README.md
// - tests/integration_test.rs
// - examples/basic.rs
```

**Notes:**
- Automatically creates Cargo.toml with proper package metadata
- Library packages get `src/lib.rs`, binary packages get `src/main.rs`
- Updates workspace `Cargo.toml` members array if `add_to_workspace: true`
- Package name derived from final path component (converts hyphens to underscores for crate name)
- Template "minimal" creates basic structure, "full" adds README, tests, examples
- Dry-run mode not yet supported - returns error if `dry_run: true`
- Standalone packages: Set `add_to_workspace: false` to skip workspace registration

---

### workspace.extract_dependencies

**Purpose:** Extract specific dependencies from source Cargo.toml and add them to target Cargo.toml (used in crate extraction workflow).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| source_manifest | string | Yes | Path to source Cargo.toml |
| target_manifest | string | Yes | Path to target Cargo.toml |
| dependencies | string[] | Yes | List of dependency names to extract |
| options | object | No | Extraction options |
| options.dry_run | boolean | No | Preview without modifying target (default: false) |
| options.preserve_versions | boolean | No | Preserve version constraints (default: true) |
| options.preserve_features | boolean | No | Preserve features array (default: true) |
| options.section | string | No | Section: "dependencies", "dev-dependencies", "build-dependencies" (default: "dependencies") |

**Returns:**

Object with extraction results:
- `dependencies_extracted` (number): Count of dependencies extracted
- `dependencies_added` (object[]): Details of each dependency (name, version, features, optional, already_exists)
- `target_manifest_updated` (boolean): Whether target file was modified
- `dry_run` (boolean): Whether this was a dry-run
- `warnings` (string[]): Warnings about conflicts or missing dependencies

**Example:**

```json
// MCP request - Extract tokio and serde
{
  "method": "tools/call",
  "params": {
    "name": "workspace.extract_dependencies",
    "arguments": {
      "source_manifest": "/workspace/crates/big-crate/Cargo.toml",
      "target_manifest": "/workspace/crates/new-crate/Cargo.toml",
      "dependencies": ["tokio", "serde"],
      "options": {
        "dry_run": false,
        "preserve_versions": true,
        "preserve_features": true,
        "section": "dependencies"
      }
    }
  }
}

// Response
{
  "result": {
    "dependencies_extracted": 2,
    "dependencies_added": [
      {
        "name": "tokio",
        "version": "1.0",
        "features": ["full"]
      },
      {
        "name": "serde",
        "version": "1.0",
        "features": ["derive"]
      }
    ],
    "target_manifest_updated": true,
    "dry_run": false,
    "warnings": []
  }
}
```

**Example - Extract dev-dependencies:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace.extract_dependencies",
    "arguments": {
      "source_manifest": "/workspace/crates/source/Cargo.toml",
      "target_manifest": "/workspace/crates/target/Cargo.toml",
      "dependencies": ["tempfile", "criterion"],
      "options": {
        "section": "dev-dependencies"
      }
    }
  }
}

// Extracts from [dev-dependencies] section
```

**Example - Conflict detection:**

```json
// If target already has a dependency with different version
{
  "result": {
    "dependencies_extracted": 2,
    "dependencies_added": [
      {
        "name": "tokio",
        "version": "1.0"
      },
      {
        "name": "serde",
        "version": "1.0",
        "already_exists": true
      }
    ],
    "warnings": [
      "Dependency 'serde' already exists in target with different version (0.9 vs 1.0)"
    ]
  }
}
```

**Example - Dry-run preview:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace.extract_dependencies",
    "arguments": {
      "source_manifest": "/workspace/source/Cargo.toml",
      "target_manifest": "/workspace/target/Cargo.toml",
      "dependencies": ["anyhow"],
      "options": {
        "dry_run": true
      }
    }
  }
}

// Result shows what would be extracted, but target file is NOT modified
```

**Notes:**
- Preserves dependency metadata: versions, features, optional flag, workspace references
- Supports workspace dependencies (`{ workspace = true }`)
- Supports path dependencies (`{ path = "../other" }`)
- Handles optional dependencies (`{ version = "1.0", optional = true }`)
- Conflict detection: Warns when target already has dependency with different version
- Missing dependencies: Warns when requested dependency not found in source
- Idempotent: Safe to run multiple times (skips already-existing dependencies)
- Section-aware: Can extract from dependencies, dev-dependencies, or build-dependencies
- Creates target section if missing (e.g., adds `[dev-dependencies]` if extracting dev deps)

---

### workspace.update_members

**Purpose:** Add, remove, or list workspace members in the root Cargo.toml manifest.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| workspace_manifest | string | Yes | Path to workspace Cargo.toml |
| action | string | Yes | Action: "add", "remove", or "list" |
| members | string[] | Conditional | Member paths (required for add/remove, ignored for list) |
| options | object | No | Update options |
| options.dry_run | boolean | No | Preview without modifying file (default: false) |
| options.create_if_missing | boolean | No | Create [workspace] section if missing (default: false) |

**Returns:**

Object with update results:
- `action` (string): Action performed
- `members_before` (string[]): Members list before operation
- `members_after` (string[]): Members list after operation
- `changes_made` (number): Count of changes made
- `workspace_updated` (boolean): Whether file was modified
- `dry_run` (boolean): Whether this was a dry-run

**Example - Add members:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "workspace.update_members",
    "arguments": {
      "workspace_manifest": "/workspace/Cargo.toml",
      "action": "add",
      "members": ["crates/new-crate1", "crates/new-crate2"],
      "options": {
        "dry_run": false,
        "create_if_missing": false
      }
    }
  }
}

// Response
{
  "result": {
    "action": "add",
    "members_before": ["crates/existing-crate"],
    "members_after": [
      "crates/existing-crate",
      "crates/new-crate1",
      "crates/new-crate2"
    ],
    "changes_made": 2,
    "workspace_updated": true,
    "dry_run": false
  }
}
```

**Example - Remove member:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace.update_members",
    "arguments": {
      "workspace_manifest": "/workspace/Cargo.toml",
      "action": "remove",
      "members": ["crates/deprecated"],
      "options": {
        "dry_run": false
      }
    }
  }
}

// Response
{
  "result": {
    "action": "remove",
    "members_before": ["crates/foo", "crates/deprecated", "crates/bar"],
    "members_after": ["crates/foo", "crates/bar"],
    "changes_made": 1,
    "workspace_updated": true,
    "dry_run": false
  }
}
```

**Example - List members:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace.update_members",
    "arguments": {
      "workspace_manifest": "/workspace/Cargo.toml",
      "action": "list"
    }
  }
}

// Response (no modifications)
{
  "result": {
    "action": "list",
    "members_before": ["crates/a", "crates/b", "crates/c"],
    "members_after": ["crates/a", "crates/b", "crates/c"],
    "changes_made": 0,
    "workspace_updated": false,
    "dry_run": false
  }
}
```

**Example - Create workspace section:**

```json
// If Cargo.toml has no [workspace] section
{
  "method": "tools/call",
  "params": {
    "name": "workspace.update_members",
    "arguments": {
      "workspace_manifest": "/workspace/Cargo.toml",
      "action": "add",
      "members": ["crates/first-member"],
      "options": {
        "create_if_missing": true
      }
    }
  }
}

// Creates [workspace] section with members array
```

**Example - Dry-run preview:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace.update_members",
    "arguments": {
      "workspace_manifest": "/workspace/Cargo.toml",
      "action": "add",
      "members": ["crates/preview"],
      "options": {
        "dry_run": true
      }
    }
  }
}

// Shows what would change, but file is NOT modified
{
  "result": {
    "changes_made": 1,
    "workspace_updated": false,
    "dry_run": true
  }
}
```

**Notes:**
- Idempotent operations: Adding existing member or removing non-existent member is no-op
- Automatically normalizes paths to forward slashes (even on Windows)
- Preserves TOML formatting and comments
- Duplicate detection: Adding an already-present member returns `changes_made: 0`
- Missing workspace section: Returns error unless `create_if_missing: true`
- Error handling: Returns error if manifest file not found
- Member validation: For "add" action, ensures paths are valid
- Atomic operations: File only modified if action succeeds

---

## Common Patterns

### Crate Extraction Workflow

Complete workflow for extracting a module into a standalone crate:

```bash
# 1. Analyze dependencies for the module
codebuddy tool analyze.module_dependencies '{
  "target": {
    "kind": "directory",
    "path": "crates/big-crate/src/analysis"
  }
}'

# 2. Create new package
codebuddy tool workspace.create_package '{
  "package_path": "crates/cb-analysis",
  "package_type": "library",
  "options": {
    "add_to_workspace": true,
    "template": "minimal"
  }
}'

# 3. Extract dependencies to new package
codebuddy tool workspace.extract_dependencies '{
  "source_manifest": "crates/big-crate/Cargo.toml",
  "target_manifest": "crates/cb-analysis/Cargo.toml",
  "dependencies": ["tokio", "serde", "anyhow"],
  "options": {
    "preserve_versions": true,
    "preserve_features": true
  }
}'

# 4. Move code files (using rename.plan + workspace.apply_edit)
codebuddy tool rename.plan '{
  "target": {
    "kind": "directory",
    "path": "crates/big-crate/src/analysis"
  },
  "new_name": "crates/cb-analysis/src"
}'

# 5. Apply the move
codebuddy tool workspace.apply_edit '{
  "plan": "<plan from step 4>"
}'
```

### Package Creation with Dependencies

Create a new package and set up dependencies in one workflow:

```bash
# 1. Create package
codebuddy tool workspace.create_package '{
  "package_path": "crates/new-service",
  "package_type": "binary"
}'

# 2. Extract common dependencies from existing package
codebuddy tool workspace.extract_dependencies '{
  "source_manifest": "crates/common/Cargo.toml",
  "target_manifest": "crates/new-service/Cargo.toml",
  "dependencies": ["tokio", "tracing", "serde"]
}'
```

### Workspace Reorganization

Remove deprecated crates from workspace:

```bash
# 1. List current members
codebuddy tool workspace.update_members '{
  "workspace_manifest": "Cargo.toml",
  "action": "list"
}'

# 2. Remove deprecated crates
codebuddy tool workspace.update_members '{
  "workspace_manifest": "Cargo.toml",
  "action": "remove",
  "members": ["crates/deprecated-a", "crates/deprecated-b"]
}'

# 3. Verify removal
codebuddy tool workspace.update_members '{
  "workspace_manifest": "Cargo.toml",
  "action": "list"
}'
```

### Dependency Audit Before Extraction

Preview dependencies before extracting a module:

```bash
# 1. Dry-run dependency extraction
codebuddy tool workspace.extract_dependencies '{
  "source_manifest": "crates/source/Cargo.toml",
  "target_manifest": "crates/target/Cargo.toml",
  "dependencies": ["dep1", "dep2", "dep3"],
  "options": {
    "dry_run": true
  }
}'

# 2. Review warnings for conflicts

# 3. Execute if no issues
codebuddy tool workspace.extract_dependencies '{
  "source_manifest": "crates/source/Cargo.toml",
  "target_manifest": "crates/target/Cargo.toml",
  "dependencies": ["dep1", "dep2", "dep3"],
  "options": {
    "dry_run": false
  }
}'
```

---

## Integration with Other Tools

### With rename.plan (Crate Consolidation)

Workspace tools integrate with `rename.plan` for crate consolidation:

```bash
# Extract module to new crate, then consolidate back
# See CLAUDE.md "Rust Crate Consolidation" section
codebuddy tool rename.plan '{
  "target": {"kind": "directory", "path": "crates/source-crate"},
  "new_name": "crates/target-crate/src/module",
  "options": {"consolidate": true}
}'

# This automatically:
# - Moves source-crate/src/* to target-crate/src/module/*
# - Merges dependencies (uses workspace.extract_dependencies internally)
# - Removes from workspace members (uses workspace.update_members)
# - Updates imports across workspace
```

### With analyze.module_dependencies

Use dependency analysis before extraction:

```bash
# 1. Analyze what dependencies a module needs
codebuddy tool analyze.module_dependencies '{
  "target": {"kind": "directory", "path": "crates/big-crate/src/module"}
}'

# Returns: external_dependencies, workspace_dependencies, std_dependencies

# 2. Use results to extract exact dependencies needed
codebuddy tool workspace.extract_dependencies '{
  "source_manifest": "crates/big-crate/Cargo.toml",
  "target_manifest": "crates/new-crate/Cargo.toml",
  "dependencies": ["<deps from analysis>"]
}'
```

---

**Related Documentation:**
- [Refactoring Tools](refactoring.md) - rename.plan for crate consolidation
- [Analysis Tools](analysis.md) - analyze.module_dependencies for dependency analysis
- [Main API Reference](../api_reference.md) - Complete API documentation
