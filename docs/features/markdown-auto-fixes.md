# Markdown Auto-Fix Guide

**Comprehensive documentation for markdown auto-fix capabilities**

TypeMill provides automatic fixes for common markdown issues through a trait-based fixer architecture. Each fixer can preview changes (dry-run mode) or apply them with conflict detection.

## Table of Contents

- [Overview](#overview)
- [Available Fixers](#available-fixers)
  - [auto_toc](#auto_toc)
  - [trailing_whitespace](#trailing_whitespace)
  - [missing_code_language_tag](#missing_code_language_tag)
  - [malformed_heading](#malformed_heading)
  - [reversed_link_syntax](#reversed_link_syntax)
- [Usage Patterns](#usage-patterns)
  - [Preview Mode (Dry Run)](#preview-mode-dry-run)
  - [Execute Mode (Apply Changes)](#execute-mode-apply-changes)
  - [Multiple Fixers](#multiple-fixers)
  - [Custom Configuration](#custom-configuration)
- [Architecture](#architecture)
- [CLI Examples](#cli-examples)

## Overview

**Key Features:**
- **Safe defaults**: Preview mode is the default (no file modifications)
- **Conflict detection**: SHA-256 optimistic locking prevents overwriting concurrent edits
- **Unified diffs**: Preview mode shows exact changes before applying
- **Per-fixer options**: Configure each fixer independently
- **Atomic operations**: All fixes succeed or all rollback

**Integration:**
- Auto-fixes are integrated into `analyze.quality` with `kind: "markdown_structure"` or `kind: "markdown_formatting"`
- Findings include `fix_id` metadata linking issues to available fixers
- Works with file, directory, and workspace scopes

## Available Fixers

### auto_toc

**Purpose:** Generate or update table of contents based on document headings.

**Fixer ID:** `auto_toc`

**Kind:** `markdown_structure`

**Configuration Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `marker` | string | `"## Table of Contents"` | TOC marker to search for |
| `max_depth` | number | `3` | Maximum heading depth to include (1-6) |
| `include_h1` | boolean | `false` | Include H1 headings in TOC |
| `exclude_patterns` | array[string] | `["^TOC$", "^Contents$", "^Table of Contents$"]` | Regex patterns for headings to exclude |

**Algorithm:**
1. Find TOC marker in document
2. Extract all headings (respecting depth/exclusion filters)
3. Generate GitHub-compatible anchor slugs:
   - Lowercase text
   - Replace spaces with hyphens
   - Remove special characters (keep alphanumeric, hyphens, underscores)
   - Handle duplicates by appending `-1`, `-2`, etc.
4. Format TOC with proper indentation (2 spaces per level)
5. Replace old TOC section with new content

**Example:**

```json
{
  "kind": "markdown_structure",
  "scope": {"type": "file", "path": "README.md"},
  "options": {
    "fix": ["auto_toc"],
    "apply": false,
    "fix_options": {
      "auto_toc": {
        "marker": "## Contents",
        "max_depth": 4,
        "include_h1": true
      }
    }
  }
}
```

**Finding Kind:** `toc_out_of_sync` - Detected when TOC marker exists but content doesn't match generated TOC

### trailing_whitespace

**Purpose:** Remove trailing spaces and tabs from line endings.

**Fixer ID:** `trailing_whitespace`

**Kind:** `markdown_formatting`

**Configuration Options:** None

**Algorithm:**
1. Scan all lines for trailing whitespace
2. Remove spaces/tabs from line endings
3. Preserve line content and line breaks

**Example:**

```json
{
  "kind": "markdown_formatting",
  "scope": {"type": "directory", "path": "docs"},
  "options": {
    "fix": ["trailing_whitespace"],
    "apply": true
  }
}
```

**Finding Kind:** `trailing_whitespace` - Detected when any line ends with spaces or tabs

### missing_code_language_tag

**Purpose:** Add language tags to code blocks for syntax highlighting.

**Fixer ID:** `missing_code_language_tag`

**Kind:** `markdown_formatting`

**Configuration Options:** None

**Algorithm:**
1. Find opening code fences (` ``` `) without language tags
2. Attempt to infer language from context or content
3. Add appropriate language tag (defaults to `text` if unable to infer)

**Example:**

```json
{
  "kind": "markdown_formatting",
  "scope": {"type": "file", "path": "guide.md"},
  "options": {
    "fix": ["missing_code_language_tag"],
    "apply": true
  }
}
```

**Finding Kind:** `missing_code_language_tag` - Detected when code block starts with ` ``` ` (no language tag)

### malformed_heading

**Purpose:** Fix headings without space after hash marks.

**Fixer ID:** `malformed_heading`

**Kind:** `markdown_structure`

**Configuration Options:** None

**Algorithm:**
1. Find headings matching pattern `^#{1,6}[^\s#]`
2. Insert space between hash marks and heading text
3. Preserve heading level and content

**Example:**

```json
{
  "kind": "markdown_structure",
  "scope": {"type": "file", "path": "README.md"},
  "options": {
    "fix": ["malformed_heading"],
    "apply": true
  }
}
```

**Before:** `##Section Title`
**After:** `## Section Title`

**Finding Kind:** `malformed_heading` - Detected when heading has no space after `#`

### reversed_link_syntax

**Purpose:** Fix reversed markdown link syntax.

**Fixer ID:** `reversed_link_syntax`

**Kind:** `markdown_formatting`

**Configuration Options:** None

**Algorithm:**
1. Find patterns matching `(url)[text]`
2. Swap to correct markdown syntax: `[text](url)`
3. Preserve URL and link text

**Example:**

```json
{
  "kind": "markdown_formatting",
  "scope": {"type": "file", "path": "README.md"},
  "options": {
    "fix": ["reversed_link_syntax"],
    "apply": true
  }
}
```

**Before:** `(https://example.com)[Click here]`
**After:** `[Click here](https://example.com)`

**Finding Kind:** `reversed_link_syntax` - Detected when link uses `(url)[text]` instead of `[text](url)`

## Usage Patterns

### Preview Mode (Dry Run)

**Default behavior** - Returns unified diffs without modifying files.

```json
{
  "kind": "markdown_structure",
  "scope": {"type": "file", "path": "README.md"},
  "options": {
    "fix": ["auto_toc"],
    "apply": false  // Can omit - false is default
  }
}
```

**Response:**
```json
{
  "result": {
    "findings": [...],
    "summary": {
      "fix_actions": {
        "preview_only": true,
        "applied": false,
        "previews": 1,
        "files_modified": 0,
        "diffs": {
          "README.md": "--- a/README.md\n+++ b/README.md\n@@ ... @@"
        }
      }
    }
  }
}
```

### Execute Mode (Apply Changes)

**Explicit opt-in** - Applies fixes with conflict detection.

```json
{
  "kind": "markdown_structure",
  "scope": {"type": "file", "path": "README.md"},
  "options": {
    "fix": ["auto_toc"],
    "apply": true  // Required to write files
  }
}
```

**Response:**
```json
{
  "result": {
    "findings": [...],
    "summary": {
      "fix_actions": {
        "preview_only": false,
        "applied": true,
        "files_modified": 1,
        "total_edits": 3
      }
    }
  }
}
```

**Conflict Detection:**
- Each file's content is hashed (SHA-256) before analysis
- Before writing, current content is re-hashed and compared
- If hash mismatch: operation aborts with error
- Prevents overwriting concurrent edits

### Multiple Fixers

Apply multiple fixers in a single operation:

```json
{
  "kind": "markdown_structure",
  "scope": {"type": "directory", "path": "docs"},
  "options": {
    "fix": ["auto_toc", "malformed_heading", "trailing_whitespace"],
    "apply": true
  }
}
```

**Execution Order:**
1. All fixers run independently on each file
2. Edits are merged (sorted by position, applied in reverse order)
3. Combined changes applied atomically
4. If any file fails conflict check, entire operation rolls back

### Custom Configuration

Configure individual fixers via `fix_options`:

```json
{
  "kind": "markdown_structure",
  "scope": {"type": "workspace", "path": "/workspace"},
  "options": {
    "fix": ["auto_toc"],
    "apply": true,
    "fix_options": {
      "auto_toc": {
        "marker": "## Contents",
        "max_depth": 4,
        "include_h1": true,
        "exclude_patterns": ["^TOC$", "^Index$"]
      }
    }
  }
}
```

## Architecture

### Trait-Based Design

```rust
pub trait MarkdownFixer: Send + Sync {
    fn id(&self) -> &'static str;
    fn apply(&self, ctx: &MarkdownContext, config: &Value) -> FixOutcome;
}
```

**Benefits:**
- Extensible: Add new fixers without modifying core code
- Testable: Each fixer is independently unit-tested
- Configurable: Per-fixer options via JSON
- Safe: Preview mode validates changes before applying

### Data Structures

**MarkdownContext:**
```rust
pub struct MarkdownContext {
    pub content: String,        // File content
    pub file_path: PathBuf,     // Absolute path
    pub content_hash: String,   // SHA-256 hash for conflict detection
}
```

**FixOutcome:**
```rust
pub struct FixOutcome {
    pub edits: Vec<TextEdit>,   // List of text replacements
    pub preview: Option<String>, // Unified diff (for dry-run)
    pub warnings: Vec<String>,   // Non-fatal issues
}
```

**TextEdit:**
```rust
pub struct TextEdit {
    pub range: Range,           // Start/end positions
    pub old_text: String,       // Original text
    pub new_text: String,       // Replacement text
}
```

### Unified Diff Generation

All fixers generate unified diffs manually (no external dependencies):

```
--- a/README.md
+++ b/README.md
@@ -3,7 +3,8 @@
 ## Table of Contents

-Old content
+- [Section 1](#section-1)
+- [Section 2](#section-2)
```

Format follows standard unified diff conventions with 3 lines of context.

## CLI Examples

### Preview TOC Update

```bash
mill tool analyze.quality '{
  "kind": "markdown_structure",
  "scope": {"type": "file", "path": "/workspace/README.md"},
  "options": {
    "fix": ["auto_toc"],
    "apply": false
  }
}'
```

### Apply TOC Update

```bash
mill tool analyze.quality '{
  "kind": "markdown_structure",
  "scope": {"type": "file", "path": "/workspace/README.md"},
  "options": {
    "fix": ["auto_toc"],
    "apply": true
  }
}'
```

### Fix All Formatting Issues

```bash
mill tool analyze.quality '{
  "kind": "markdown_formatting",
  "scope": {"type": "directory", "path": "/workspace/docs"},
  "options": {
    "fix": [
      "trailing_whitespace",
      "missing_code_language_tag",
      "reversed_link_syntax"
    ],
    "apply": true
  }
}'
```

### Custom TOC Configuration

```bash
mill tool analyze.quality '{
  "kind": "markdown_structure",
  "scope": {"type": "file", "path": "/workspace/README.md"},
  "options": {
    "fix": ["auto_toc"],
    "apply": true,
    "fix_options": {
      "auto_toc": {
        "marker": "## Contents",
        "max_depth": 4,
        "include_h1": true
      }
    }
  }
}'
```

## Integration with Findings

All fixable issues include `fix_id` metadata:

```json
{
  "id": "toc-out-of-sync-README.md",
  "kind": "toc_out_of_sync",
  "severity": "low",
  "message": "Table of Contents is out of sync with document headings",
  "location": {
    "file_path": "README.md",
    "range": {"start": {"line": 3, "character": 0}, "end": {"line": 3, "character": 22}}
  },
  "metrics": {
    "fix_id": "auto_toc"  // Links to auto_toc fixer
  }
}
```

This enables closed-loop workflows:
1. Run analysis to detect issues
2. Examine findings with `fix_id` metadata
3. Apply corresponding fixer to resolve issues
4. Re-run analysis to verify fixes

## See Also

- [Analysis Tools](../tools/analysis.md#analyzequality) - Complete analyze.quality API reference
- [User Guide](../user-guide/cheatsheet.md) - Quick reference with examples
- Proposal 17 (`proposals/17_markdown_autofix_architecture.proposal.md`) - Architecture design document
