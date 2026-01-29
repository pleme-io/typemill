# Workspace Tool

> The workspace tool is a unified action-based tool. Use the `action` field to select an operation.

Package management and workspace operations for multi-language workspaces. Create packages, extract dependencies, perform workspace-wide text operations, and verify project health.

**Supported languages:** Rust (Cargo), TypeScript (npm/yarn/pnpm), Python (PDM/Poetry/Hatch)
**Tool:** 1 unified tool with 5 actions
**Related categories:** [rename_all](rename_all.md) (for crate consolidation)

**Language-specific guides:**
- **[Rust/Cargo](workspace-rust.md)** - Cargo.toml, crate structure, Rust conventions
- **[TypeScript](workspace-typescript.md)** - package.json, tsconfig.json, npm/yarn/pnpm workspaces
- **[Python](workspace-python.md)** - pyproject.toml, src layout, PDM/Poetry/Hatch

## Table of Contents

- [Overview](#overview)
- [Actions](#actions)
  - [create_package](#create_package)
  - [extract_dependencies](#extract_dependencies)
  - [find_replace](#find_replace)
  - [update_members](#update_members)
  - [verify_project](#verify_project)
- [Common Patterns](#common-patterns)
  - [Crate Extraction Workflow](#crate-extraction-workflow)
  - [Package Creation with Dependencies](#package-creation-with-dependencies)
  - [Dependency Audit Before Extraction](#dependency-audit-before-extraction)
  - [Project-Wide Text Replacement](#project-wide-text-replacement)
- [Integration with Other Tools](#integration-with-other-tools)
  - [With rename_all (Crate Consolidation)](#with-rename_all-crate-consolidation)

---

## Overview

The `workspace` tool provides a unified interface for workspace-level operations through action-based dispatching. All actions follow a consistent pattern:

```json
{
  "name": "workspace",
  "arguments": {
    "action": "action_name",
    "params": {
      // action-specific parameters
    },
    "options": {
      "dryRun": true  // where applicable
    }
  }
}
```

**Available actions:**
- `create_package` - Create new packages (library or binary)
- `extract_dependencies` - Extract dependencies from one manifest to another
- `find_replace` - Find and replace text workspace-wide
- `update_members` - Update workspace member list
- `verify_project` - Verify project health and configuration

---

## Actions

### create_package

**Purpose:** Create a new package (library or binary) with proper manifest and source structure. Language auto-detected from workspace or specified via file extension.

**Supported:** Rust (Cargo), TypeScript (npm/yarn/pnpm), Python (PDM/Poetry/Hatch)
**Language-specific details:** See [Rust](workspace-rust.md), [TypeScript](workspace-typescript.md), [Python](workspace-python.md)

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| package_path | string | Yes | Absolute or workspace-relative path for new package (e.g., "crates/my-lib", "packages/utils") |
| packageType | string | No | Package type: "library" or "binary" (default: "library") |
| options | object | No | Creation options |
| options.dryRun | boolean | No | Preview operation (not yet supported - returns error) |
| options.add_to_workspace | boolean | No | Add to workspace members list (default: true) |
| options.template | string | No | Template: "minimal" or "full" (default: "minimal") |

**Returns:**

Object with creation details:
- `created_files` (string[]): Absolute paths to all created files
- `workspace_updated` (boolean): Whether workspace manifest was updated
- `package_info` (object): Package metadata (name, version, manifest_path)
- `dryRun` (boolean): Whether this was a dry-run

**Templates:**

| Template | All Languages | Language-Specific Extras |
|----------|---------------|-------------------------|
| `minimal` | manifest + config + entry file + README + .gitignore + tests | None |
| `full` | minimal + | Rust: examples/, TS: .eslintrc.json, Python: setup.py |

**Example - Create library:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "create_package",
      "params": {
        "packagePath": "packages/my-util",
        "packageType": "library"
      },
      "options": {
        "template": "minimal",
        "addToWorkspace": true
      }
    }
  }
}

// Creates manifest, entry file, README, .gitignore, tests
```
**Example - Create binary:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "create_package",
      "params": {
        "packagePath": "packages/my-cli",
        "packageType": "binary"
      },
      "options": {
        "template": "full"
      }
    }
  }
}

// Creates minimal files + language-specific extras
```
**Notes:**
- Language auto-detected from workspace structure
- Package name derived from final path component
- Minimal template includes README, .gitignore, and starter tests (as of recent updates)
- Full template adds language-specific files (see language-specific guides)
- Dry-run mode not yet supported - returns error if `dryRun: true`
- Standalone packages: Set `add_to_workspace: false` to skip workspace registration
- Workspace paths normalized to forward slashes (cross-platform compatibility)

---

### extract_dependencies

**Purpose:** Extract specific dependencies from source Cargo.toml and add them to target Cargo.toml (used in crate extraction workflow).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| source_manifest | string | Yes | Path to source Cargo.toml |
| target_manifest | string | Yes | Path to target Cargo.toml |
| dependencies | string[] | Yes | List of dependency names to extract |
| options | object | No | Extraction options |
| options.dryRun | boolean | No | Preview without modifying target (default: false) |
| options.preserve_versions | boolean | No | Preserve version constraints (default: true) |
| options.preserve_features | boolean | No | Preserve features array (default: true) |
| options.section | string | No | Section: "dependencies", "dev-dependencies", "build-dependencies" (default: "dependencies") |

**Returns:**

Object with extraction results:
- `dependencies_extracted` (number): Count of dependencies extracted
- `dependencies_added` (object[]): Details of each dependency (name, version, features, optional, already_exists)
- `target_manifest_updated` (boolean): Whether target file was modified
- `dryRun` (boolean): Whether this was a dry-run
- `warnings` (string[]): Warnings about conflicts or missing dependencies

**Example:**

```json
// MCP request - Extract tokio and serde
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "extract_dependencies",
      "params": {
        "source_manifest": "/workspace/crates/big-crate/Cargo.toml",
        "target_manifest": "/workspace/crates/new-crate/Cargo.toml",
        "dependencies": ["tokio", "serde"]
      },
      "options": {
        "dryRun": false,
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
    "dryRun": false,
    "warnings": []
  }
}
```
**Example - Extract dev-dependencies:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "extract_dependencies",
      "params": {
        "source_manifest": "/workspace/crates/source/Cargo.toml",
        "target_manifest": "/workspace/crates/target/Cargo.toml",
        "dependencies": ["tempfile", "criterion"]
      },
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
    "name": "workspace",
    "arguments": {
      "action": "extract_dependencies",
      "params": {
        "source_manifest": "/workspace/source/Cargo.toml",
        "target_manifest": "/workspace/target/Cargo.toml",
        "dependencies": ["anyhow"]
      },
      "options": {
        "dryRun": true
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

### find_replace

**Purpose:** Find and replace text across the workspace with support for literal/regex patterns, case preservation, and file scope filtering.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| pattern | string | Yes | Pattern to search for (literal or regex) |
| replacement | string | Yes | Replacement text (may contain $1, $2 for regex) |
| mode | string | No | Search mode: "literal" or "regex" (default: "literal") |
| whole_word | boolean | No | For literal mode: match whole words only (default: false) |
| preserve_case | boolean | No | Preserve case style when replacing (default: false) |
| scope | object | No | File scope configuration |
| scope.include_patterns | string[] | No | Glob patterns to include (e.g., ["**/*.rs"]) (default: []) |
| scope.exclude_patterns | string[] | No | Glob patterns to exclude (default: see below) |
| dryRun | boolean | No | Preview changes without applying (default: true) |

**Default Excludes:** `**/target/**`, `**/node_modules/**`, `**/.git/**`, `**/build/**`, `**/dist/**`

**Returns:**

When `dryRun: true` (default), returns an EditPlan:
- `source_file` (string): Always "workspace" for workspace operations
- `edits` (TextEdit[]): Array of text edits with file paths, locations, and changes
- `metadata` (object): Operation metadata (intent, complexity, impact areas)

When `dryRun: false`, returns ApplyResult:
- `success` (boolean): Whether operation succeeded
- `files_modified` (string[]): List of modified file paths
- `matches_found` (number): Total number of matches discovered
- `matches_replaced` (number): Total number of matches replaced

**Example - Simple literal replacement:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "find_replace",
      "params": {
        "pattern": "username",
        "replacement": "userid",
        "mode": "literal"
      }
    }
  }
}

// Response (dryRun: true by default)
{
  "result": {
    "source_file": "workspace",
    "edits": [
      {
        "file_path": "/workspace/src/auth.rs",
        "edit_type": "Replace",
        "location": {
          "start_line": 42,
          "start_column": 10,
          "end_line": 42,
          "end_column": 18
        },
        "original_text": "username",
        "new_text": "userid",
        "description": "Replace 'username' with 'userid' (literal)"
      }
    ],
    "metadata": {
      "intent_name": "find_replace",
      "complexity": 3
    }
  }
}
```
**Example - Whole word matching:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "find_replace",
      "params": {
        "pattern": "user",
        "replacement": "account",
        "mode": "literal",
        "wholeWord": true
      }
    }
  }
}

// Matches "user" but NOT "username" or "userInfo"
```
**Example - Regex with capture groups:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "find_replace",
      "params": {
        "pattern": "TYPEMILL_([A-Z_]+)",
        "replacement": "TYPEMILL_$1",
        "mode": "regex"
      }
    }
  }
}

// Converts:
// TYPEMILL_ENABLE_LOGS → TYPEMILL_ENABLE_LOGS
// TYPEMILL_DEBUG_MODE → TYPEMILL_DEBUG_MODE
```
**Example - Case-preserving replacement:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "find_replace",
      "params": {
        "pattern": "user_name",
        "replacement": "account_id",
        "mode": "literal",
        "preserveCase": true
      }
    }
  }
}

// Converts:
// user_name → account_id (snake_case)
// userName → accountId (camelCase)
// UserName → AccountId (PascalCase)
// USER_NAME → ACCOUNT_ID (SCREAMING_SNAKE)
```
**Example - Scoped replacement (Rust files only):**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "find_replace",
      "params": {
        "pattern": "old_function",
        "replacement": "new_function",
        "scope": {
          "includePatterns": ["**/*.rs"],
          "excludePatterns": ["**/target/**", "**/tests/**"]
        }
      }
    }
  }
}

// Only searches .rs files, excluding target/ and tests/
```
**Example - Named capture groups:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "find_replace",
      "params": {
        "pattern": "(?P<module>\\w+)::(?P<function>\\w+)",
        "replacement": "${function}_from_${module}",
        "mode": "regex"
      }
    }
  }
}

// Converts: utils::format → format_from_utils
```
**Example - Execute replacement (dryRun: false):**

```json
{
  "method": "tools/call",
  "params": {
    "name": "workspace",
    "arguments": {
      "action": "find_replace",
      "params": {
        "pattern": "old_config_path",
        "replacement": "new_config_path"
      },
      "options": {
        "dryRun": false
      }
    }
  }
}

// Response
{
  "result": {
    "success": true,
    "files_modified": [
      "/workspace/src/config.rs",
      "/workspace/README.md"
    ],
    "matches_found": 5,
    "matches_replaced": 5
  }
}
```
**Error Cases:**

| Error | Cause | Solution |
|-------|-------|----------|
| InvalidRequest: "Pattern cannot be empty" | Empty pattern string | Provide non-empty pattern |
| InvalidRequest: "Regex error: ..." | Invalid regex syntax | Fix regex pattern syntax |
| InvalidRequest: "Invalid exclude pattern" | Malformed glob pattern | Fix glob pattern syntax |
| Internal: "Failed to read file" | Permission denied or file locked | Check file permissions |

**Notes:**
- **Safety-first design:** `dryRun` defaults to `true` to prevent accidental mass replacements
- **Case preservation limitations:** May not handle acronyms perfectly (e.g., "HTTPServer")
- **Regex mode:** Does not support case preservation (use literal mode instead)
- **Binary files:** Automatically skipped during file discovery
- **Large files:** Files over 100MB may be slow to process
- **Atomic operations:** When `dryRun: false`, all changes are applied atomically (all succeed or all rollback)
- **Git-aware:** Respects `.gitignore` patterns during file discovery
- **Performance:** For 1000+ matches, consider using include_patterns to reduce scope

**Best Practices:**

1. **Always preview first:** Use default `dryRun: true` to review changes
2. **Scope appropriately:** Use include/exclude patterns to target specific file types
3. **Test regex patterns:** Validate complex regex on small samples before workspace-wide replacement
4. **Case preservation:** Review changes when using preserve_case with acronyms or mixed-case identifiers
5. **Large replacements:** For 100+ file modifications, consider breaking into smaller scoped operations

**Related Tools:**
- `rename_all` - For renaming files, directories, and symbols (more semantic than text replacement)

---

## Common Patterns

### Crate Extraction Workflow

Complete workflow for extracting a module into a standalone crate:

```bash
# 1. Gather dependencies for the module (manual review)

# 2. Create new package
mill tool workspace '{
  "action": "create_package",
  "params": {
    "packagePath": "crates/mill-core",
    "packageType": "library"
  },
  "options": {
    "addToWorkspace": true,
    "template": "minimal"
  }
}'

# 3. Extract dependencies to new package
mill tool workspace '{
  "action": "extract_dependencies",
  "params": {
    "source_manifest": "crates/big-crate/Cargo.toml",
    "target_manifest": "crates/mill-core/Cargo.toml",
    "dependencies": ["tokio", "serde", "anyhow"]
  },
  "options": {
    "preserve_versions": true,
    "preserve_features": true
  }
}'

# 4. Preview directory move
mill tool rename_all '{
  "target": {
    "kind": "directory",
    "filePath": "crates/big-crate/src/core"
  },
  "newName": "crates/mill-core/src"
}'

# 5. Execute the move (explicit dryRun: false)
mill tool rename_all '{
  "target": {
    "kind": "directory",
    "filePath": "crates/big-crate/src/core"
  },
  "newName": "crates/mill-core/src",
  "options": { "dryRun": false }
}'
```
### Package Creation with Dependencies

Create a new package and set up dependencies in one workflow:

```bash
# 1. Create package
mill tool workspace '{
  "action": "create_package",
  "params": {
    "packagePath": "crates/new-service",
    "packageType": "binary"
  }
}'

# 2. Extract common dependencies from existing package
mill tool workspace '{
  "action": "extract_dependencies",
  "params": {
    "source_manifest": "crates/common/Cargo.toml",
    "target_manifest": "crates/new-service/Cargo.toml",
    "dependencies": ["tokio", "tracing", "serde"]
  }
}'
```
### Dependency Audit Before Extraction

Preview dependencies before extracting a module:

```bash
# 1. Dry-run dependency extraction
mill tool workspace '{
  "action": "extract_dependencies",
  "params": {
    "source_manifest": "crates/source/Cargo.toml",
    "target_manifest": "crates/target/Cargo.toml",
    "dependencies": ["dep1", "dep2", "dep3"]
  },
  "options": {
    "dryRun": true
  }
}'

# 2. Review warnings for conflicts

# 3. Execute if no issues
mill tool workspace '{
  "action": "extract_dependencies",
  "params": {
    "source_manifest": "crates/source/Cargo.toml",
    "target_manifest": "crates/target/Cargo.toml",
    "dependencies": ["dep1", "dep2", "dep3"]
  },
  "options": {
    "dryRun": false
  }
}'
```
### Project-Wide Text Replacement

Safe workflow for replacing text across the workspace:

```bash
# 1. Preview replacement (dryRun defaults to true)
mill tool workspace '{
  "action": "find_replace",
  "params": {
    "pattern": "TYPEMILL_([A-Z_]+)",
    "replacement": "TYPEMILL_$1",
    "mode": "regex",
    "scope": {
      "includePatterns": ["**/*.rs", "**/*.toml", "**/*.md"]
    }
  }
}'

# 2. Review the EditPlan output showing all changes

# 3. Execute the replacement
mill tool workspace '{
  "action": "find_replace",
  "params": {
    "pattern": "TYPEMILL_([A-Z_]+)",
    "replacement": "TYPEMILL_$1",
    "mode": "regex",
    "scope": {
      "includePatterns": ["**/*.rs", "**/*.toml", "**/*.md"]
    }
  },
  "options": {
    "dryRun": false
  }
}'
```
**Common use cases:**
- Environment variable prefix changes (TYPEMILL_* → TYPEMILL_*)
- Configuration path updates (.typemill → .typemill)
- CLI command updates in documentation (mill → typemill)
- Dependency renames in manifests
- API endpoint path changes

---

## Integration with Other Tools

### With rename_all (Crate Consolidation)

Workspace tools integrate with `rename_all` for crate consolidation:

```bash
# Extract module to new crate, then consolidate back
# See CLAUDE.md "Rust Crate Consolidation" section
mill tool rename_all '{
  "target": {"kind": "directory", "filePath": "crates/source-crate"},
  "newName": "crates/target-crate/src/module",
  "options": {"consolidate": true}
}'

# This automatically:
# - Moves source-crate/src/* to target-crate/src/module/*
# - Merges dependencies (uses workspace extract_dependencies action internally)
# - Removes source from workspace members
# - Updates imports across workspace
```
**Related Documentation:**
- [rename_all](rename_all.md) - Rename tool for crate consolidation
- [Main API Reference](README.md) - Complete API documentation
