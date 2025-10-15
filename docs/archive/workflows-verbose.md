# Intent-Based Workflow Engine

Automate complex, multi-step operations with high-level intents. Instead of calling individual tools, declare what you want to achieve and let Codebuddy plan and execute the workflow.

**Note:** This document describes the legacy intent-based workflow system. For refactoring operations, prefer the **Unified Refactoring API** which provides a simpler, safer two-step pattern: `*.plan` to preview changes, then `workspace.apply_edit` to execute. See the [Unified Refactoring API](#unified-refactoring-api) section below.

## Table of Contents
- [Unified Refactoring API](#unified-refactoring-api)
  - [Pattern Overview](#pattern-overview)
  - [Available Refactorings](#available-refactorings)
  - [Safety and Preview Benefits](#safety-and-preview-benefits)
- [Legacy Intent-Based Workflows](#legacy-intent-based-workflows)
  - [Overview](#overview)
  - [Core Concepts](#core-concepts)
  - [Using the achieve_intent Tool](#using-the-achieve_intent-tool)
  - [Built-in Intents](#built-in-intents)
  - [Workflow State Management](#workflow-state-management)
  - [Error Handling](#error-handling)
  - [Best Practices](#best-practices)

## Unified Refactoring API

The Unified Refactoring API provides a safe, consistent pattern for all refactoring operations. Instead of legacy workflow-based tools, all refactorings now follow a simple two-step pattern that emphasizes safety and preview-ability.

### Pattern Overview

Every refactoring operation follows this pattern:

1. **Plan** - Generate a preview of changes without modifying files
2. **Apply** - Execute the planned changes with workspace.apply_edit

```json
// Step 1: Preview the refactoring
{
  "method": "tools/call",
  "params": {
    "name": "rename.plan",
    "arguments": {
      "file_path": "src/api.ts",
      "line": 10,
      "character": 5,
      "new_name": "getData"
    }
  }
}

// Response includes detailed preview of all changes
{
  "edit_id": "550e8400-e29b-41d4-a716-446655440000",
  "changes": {
    "src/api.ts": [...],
    "src/client.ts": [...],
    "src/tests/api.test.ts": [...]
  },
  "summary": "Rename 'fetchData' to 'getData' (3 files, 12 occurrences)"
}

// Step 2: Apply the changes
{
  "method": "tools/call",
  "params": {
    "name": "workspace.apply_edit",
    "arguments": {
      "edit_id": "550e8400-e29b-41d4-a716-446655440000"
    }
  }
}
```

### Available Refactorings

All refactoring operations use this unified pattern:

#### `rename.plan`
Rename a symbol across the entire workspace with LSP-powered accuracy.

**Parameters:**
- `file_path` (string): Path to the file containing the symbol
- `line` (number): Line number of the symbol
- `character` (number): Character position of the symbol
- `new_name` (string): Desired new name

**Example:**
```json
{
  "name": "rename.plan",
  "arguments": {
    "file_path": "src/database.ts",
    "line": 15,
    "character": 9,
    "new_name": "establishDatabaseConnection"
  }
}
```

#### `extract.plan`
Extract a block of code into a new function or method.

**Parameters:**
- `file_path` (string): Path to the file
- `range` (object): Range of code to extract
  - `start` (object): { `line`: number, `character`: number }
  - `end` (object): { `line`: number, `character`: number }
- `new_name` (string): Name for the new function

**Example:**
```json
{
  "name": "extract.plan",
  "arguments": {
    "file_path": "src/validator.ts",
    "range": {
      "start": { "line": 45, "character": 0 },
      "end": { "line": 62, "character": 0 }
    },
    "new_name": "validateEmailFormat"
  }
}
```

#### `inline.plan`
Inline a variable, replacing all its usages with its value.

**Parameters:**
- `file_path` (string): Path to the file
- `line` (number): Line number of the variable
- `character` (number): Character position of the variable

**Example:**
```json
{
  "name": "inline.plan",
  "arguments": {
    "file_path": "src/utils.ts",
    "line": 20,
    "character": 6
  }
}
```

#### `workspace.apply_edit`
Apply a previously planned refactoring edit.

**Parameters:**
- `edit_id` (string): UUID from the plan operation
- `options` (object, optional):
  - `dry_run` (boolean): Preview final application without writing

**Example:**
```json
{
  "name": "workspace.apply_edit",
  "arguments": {
    "edit_id": "550e8400-e29b-41d4-a716-446655440000",
    "options": {
      "dry_run": false
    }
  }
}
```

### Safety and Preview Benefits

The unified API provides multiple layers of safety:

#### 1. Mandatory Preview
Every refactoring requires an explicit preview step. You can't accidentally apply changes without seeing them first.

```json
// This is safe - only generates a plan
{ "name": "rename.plan", "arguments": {...} }

// This requires the edit_id from the plan
{ "name": "workspace.apply_edit", "arguments": { "edit_id": "..." } }
```

#### 2. Detailed Change Preview
Plan operations return comprehensive information:
- All files affected
- Exact line-by-line changes
- Total count of modifications
- Human-readable summary

#### 3. Double Preview with Dry Run
Even after planning, you can do a final dry-run before committing:

```json
{
  "name": "workspace.apply_edit",
  "arguments": {
    "edit_id": "550e8400-e29b-41d4-a716-446655440000",
    "options": { "dry_run": true }
  }
}
```

#### 4. Atomic Operations
All changes in a workspace edit are applied atomically:
- Either all files are updated successfully
- Or no files are modified (transaction rollback on error)

#### 5. Edit Caching
Planned edits are cached for 5 minutes, allowing:
- Time to review changes carefully
- Discussion with team members
- Validation against test suites

### Workflow Integration

The unified API integrates seamlessly with automated workflows:

```json
// Automated safe refactoring workflow
{
  "steps": [
    {
      "tool": "rename.plan",
      "params": {
        "file_path": "{file_path}",
        "line": "{line}",
        "character": "{character}",
        "new_name": "{new_name}"
      },
      "description": "Plan the rename operation"
    },
    {
      "tool": "workspace.apply_edit",
      "params": {
        "edit_id": "$steps.0.edit_id"
      },
      "description": "Apply the rename changes",
      "requires_confirmation": true
    }
  ]
}
```

### Migration from Legacy Tools

The unified API replaces these legacy tools:

| Legacy Tool | Unified API |
|-------------|-------------|
| `rename_symbol` | `rename.plan` + `workspace.apply_edit` |
| `extract_function` | `extract.plan` + `workspace.apply_edit` |
| `inline_variable` | `inline.plan` + `workspace.apply_edit` |
| `refactor.renameSymbol` intent | `rename.plan` + `workspace.apply_edit` |
| `refactor.extractFunction` intent | `extract.plan` + `workspace.apply_edit` |

---

## Legacy Intent-Based Workflows

**Note:** The workflow system below is maintained for non-refactoring operations (research, documentation, etc.). For refactoring operations, use the Unified Refactoring API above.

## Overview

The Intent-Based Workflow Engine is a powerful automation system that enables AI agents to execute complex, multi-step analysis and documentation operations. Instead of performing individual tool calls, agents can declare high-level intents that are automatically planned into executable workflows with proper state management, error handling, and user confirmation for destructive operations.

**Note:** This workflow system is now primarily used for non-refactoring operations like documentation generation and research. For refactoring operations, use the [Unified Refactoring API](#unified-refactoring-api) which provides better safety guarantees and a simpler two-step pattern.

## Core Concepts

### Intent

An **Intent** represents a high-level user or AI goal. It consists of:
- **name**: A unique identifier for the type of operation (e.g., `"refactor.renameSymbol"`)
- **params**: A JSON object containing the parameters needed to execute the intent

Example:
```json
{
  "name": "refactor.renameSymbol",
  "params": {
    "file_path": "src/utils.ts",
    "old_name": "calculateTotal",
    "new_name": "computeSum"
  }
}
```

### Workflow

A **Workflow** is a concrete, executable plan that fulfills an intent. It contains:
- **name**: A human-readable description of the workflow
- **metadata**: Information about the workflow's complexity
- **steps**: An ordered sequence of tool calls to execute

Workflows are automatically generated from intents by the Planner service.

### Step

A **Step** is a single, atomic action within a workflow. Each step corresponds to a tool call and includes:
- **tool**: The name of the tool to execute
- **params**: Parameters for the tool (may include placeholders)
- **description**: Human-readable explanation of what this step does
- **requires_confirmation**: Optional flag to pause workflow for user approval

## Using the `achieve_intent` Tool

The `achieve_intent` tool is the primary interface to the workflow engine.

### Planning a Workflow (Dry Run)

To see what steps a workflow would execute without making any changes:

```json
{
  "intent": {
    "name": "refactor.renameSymbol",
    "params": {
      "file_path": "src/api.ts",
      "old_name": "fetchData",
      "new_name": "getData"
    }
  }
}
```

**Response**: Returns the planned workflow with all steps listed.

### Executing a Workflow

To plan and immediately execute a workflow:

```json
{
  "intent": {
    "name": "refactor.renameSymbol",
    "params": {
      "file_path": "src/api.ts",
      "old_name": "fetchData",
      "new_name": "getData"
    }
  },
  "execute": true
}
```

**Response**: Executes the workflow and returns a detailed execution log with results.

### Dry-Run Mode

To execute a workflow in preview mode (no file modifications):

```json
{
  "intent": {
    "name": "refactor.renameSymbol",
    "params": {
      "file_path": "src/api.ts",
      "old_name": "fetchData",
      "new_name": "getData"
    }
  },
  "execute": true,
  "dry_run": true
}
```

**Response**: Executes the workflow with `dry_run: true` passed to all tools, allowing you to preview changes.

### Interactive Workflows with Confirmation

Some workflows require user confirmation before proceeding with potentially destructive operations. When a workflow pauses for confirmation, you'll receive:

```json
{
  "status": "awaiting_confirmation",
  "workflow_id": "550e8400-e29b-41d4-a716-446655440000",
  "workflow": "Rename 'fetchData' to 'getData'",
  "step_index": 1,
  "step_description": "Apply the rename changes across all found references.",
  "log": [
    "[Step 1/2] SUCCESS: find_references - Find all references to the symbol 'fetchData'.",
    "[Step 2/2] PAUSED: apply_workspace_edit - Apply the rename changes. Awaiting user confirmation."
  ]
}
```

### Resuming a Paused Workflow

To resume a workflow after reviewing and approving the changes:

```json
{
  "workflow_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response**: Continues execution from the paused step and completes the workflow.

## Parameter Templating

The workflow engine supports two types of placeholders for dynamic parameter substitution:

### 1. Intent Parameter Placeholders: `{param_name}`

Used in workflow templates to substitute values from the intent parameters.

Example template:
```json
{
  "tool": "find_references",
  "params": {
    "file_path": "{file_path}",
    "symbol_name": "{old_name}"
  }
}
```

When executed with `old_name: "fetchData"`, becomes:
```json
{
  "tool": "find_references",
  "params": {
    "file_path": "src/api.ts",
    "symbol_name": "fetchData"
  }
}
```

### 2. Step Result Placeholders: `$steps.{index}.{path}`

Used to pass data from previous steps to subsequent steps.

Format: `$steps.{step_index}.{json_path}`
- **step_index**: Zero-based index of the step whose result you want to reference
- **json_path**: Dot-separated path into the result JSON

Example:
```json
{
  "tool": "web_fetch",
  "params": {
    "url": "$steps.0.results.0.url"
  }
}
```

This retrieves the `url` field from the first result of step 0's output.

### Complex Example

```json
{
  "tool": "get_hover",
  "params": {
    "file_path": "{file_path}",
    "line": "$steps.0.symbols.0.range.start.line",
    "character": "$steps.0.symbols.0.range.start.character"
  }
}
```

This combines both types:
- `{file_path}` is replaced with the intent parameter
- `$steps.0.symbols.0.range.start.line` is replaced with data from step 0's result

## Available Recipes

### Refactoring Intents (Deprecated)

**Note:** The refactoring intents below are deprecated. Use the [Unified Refactoring API](#unified-refactoring-api) instead for better safety and preview capabilities.

#### ~~`refactor.renameSymbol`~~ (Deprecated)

**Use instead:** `rename.plan` + `workspace.apply_edit`

Legacy workflow that renames a symbol across the entire project.

**Migration Example:**
```json
// OLD: refactor.renameSymbol intent
{
  "name": "refactor.renameSymbol",
  "params": {
    "file_path": "src/database.ts",
    "old_name": "connectDB",
    "new_name": "establishDatabaseConnection"
  }
}

// NEW: Unified API (two steps)
// Step 1: Plan
{
  "name": "rename.plan",
  "arguments": {
    "file_path": "src/database.ts",
    "line": 15,
    "character": 9,
    "new_name": "establishDatabaseConnection"
  }
}

// Step 2: Apply
{
  "name": "workspace.apply_edit",
  "arguments": {
    "edit_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

---

#### ~~`refactor.extractFunction`~~ (Deprecated)

**Use instead:** `extract.plan` + `workspace.apply_edit`

Legacy workflow that extracts a block of code into a new function.

**Migration Example:**
```json
// OLD: refactor.extractFunction intent
{
  "name": "refactor.extractFunction",
  "params": {
    "file_path": "src/validator.ts",
    "start_line": 45,
    "end_line": 62,
    "function_name": "validateEmailFormat"
  }
}

// NEW: Unified API (two steps)
// Step 1: Plan
{
  "name": "extract.plan",
  "arguments": {
    "file_path": "src/validator.ts",
    "range": {
      "start": { "line": 45, "character": 0 },
      "end": { "line": 62, "character": 0 }
    },
    "new_name": "validateEmailFormat"
  }
}

// Step 2: Apply
{
  "name": "workspace.apply_edit",
  "arguments": {
    "edit_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

---

### Non-Refactoring Intents

#### 1. `docs.generateDocstring`

Generates documentation for a function or method.

**Required Parameters:**
- `file_path` (string): Path to the file
- `symbol_name` (string): Name of the function/method to document

**Workflow Steps:**
1. Find the symbol location using `get_document_symbols`
2. Retrieve signature information using `get_hover`
3. Insert a placeholder docstring above the symbol using `apply_workspace_edit`

**Example:**
```json
{
  "name": "docs.generateDocstring",
  "params": {
    "file_path": "src/auth.ts",
    "symbol_name": "authenticateUser"
  }
}
```

**Complexity:** 3 steps

---

#### 2. `research.topic`

Researches a topic by searching the web and fetching content from the top result.

**Required Parameters:**
- `topic` (string): The subject to research

**Workflow Steps:**
1. Search for the topic using `google_search`
2. Fetch the content of the top result using `web_fetch`

**Example:**
```json
{
  "name": "research.topic",
  "params": {
    "topic": "TypeScript generics best practices"
  }
}
```

**Complexity:** 2 steps

**Note:** The `web_fetch` tool automatically converts HTML to Markdown for easier AI processing.

---

## Workflow Execution Flow

1. **Planning Phase**
   - Intent is received by the `achieve_intent` tool
   - Planner service looks up the workflow template from `workflows.json`
   - Template placeholders (`{param}`) are replaced with intent parameters
   - Workflow is validated and returned

2. **Execution Phase** (if `execute: true`)
   - Executor processes steps sequentially
   - For each step:
     - Check if `requires_confirmation` is true → pause if needed
     - Resolve `$steps` placeholders from previous step results
     - Execute the tool via PluginManager
     - Store the result for future steps
     - Log the outcome

3. **Completion**
   - Return final result with execution log
   - Include workflow statistics (steps executed, complexity, dry-run status)

## Error Handling

The workflow engine provides robust error handling:

- **Planning Errors**: Missing required parameters, unknown intent names
- **Execution Errors**: Tool failures halt the workflow and return detailed error information
- **State Management**: Failed workflows preserve logs up to the failure point
- **Validation**: All placeholders are validated before execution

Example error response:
```json
{
  "error": "Workflow 'Rename X to Y' failed at step 2/2 (apply_workspace_edit): Apply the rename changes. Error: No write permission"
}
```

## Configuration

Workflows are externalized in `.codebuddy/workflows.json`. This allows for:
- Easy addition of new workflow recipes
- Modification of existing workflows without code changes
- Custom workflows for project-specific patterns

## Advanced Features

### State Management
- Each step's output is stored in a `HashMap<usize, Value>`
- Results are accessible via `$steps.{index}` placeholders
- State is preserved across workflow pause/resume

### Dry-Run Mode
- Pass `"dry_run": true` to preview changes
- Executor injects `"dry_run": true` into all tool parameters
- No files are modified, but all steps execute

### Workflow Metadata
- **Complexity Score**: Number of steps in the workflow
- Used by AI agents to estimate execution time and impact
- Helps users understand workflow scope before execution

### Interactive Workflows
- Steps with `requires_confirmation: true` pause execution
- User can review changes before proceeding
- Paused workflows cached with UUID for reliable resumption
- State fully preserved including logs and step results

## Best Practices

### For Refactoring Operations

1. **Use the Unified API**: Always prefer `*.plan` + `workspace.apply_edit` over legacy workflow intents
2. **Always Preview First**: Call `*.plan` to see exactly what will change before applying
3. **Review Changes Carefully**: Examine the detailed change preview returned by plan operations
4. **Use Dry Run for Final Check**: Even after planning, use `workspace.apply_edit` with `dry_run: true` for a final review
5. **Leverage Atomic Operations**: Trust that all changes succeed together or none are applied

### For Legacy Workflow Engine

1. **Start with Planning**: Always preview a workflow before executing
2. **Use Dry-Run**: Test destructive workflows with `dry_run: true` first
3. **Review Confirmations**: Carefully review changes when workflows pause
4. **Check Logs**: Execution logs provide detailed information about each step
5. **Handle Errors**: Workflows fail fast - check error messages for debugging
6. **Prefer Unified API**: For refactoring, migrate to the Unified Refactoring API

## Future Enhancements

Planned features for future releases:
- User input collection during workflow execution (via `resume_data`)
- Conditional step execution based on previous results
- Parallel step execution for independent operations
- Workflow composition (calling workflows from other workflows)
- Custom validators for step outputs