# Intent-Based Workflow Engine

## Overview

The Intent-Based Workflow Engine is a powerful automation system that enables AI agents to execute complex, multi-step refactoring and analysis operations. Instead of performing individual tool calls, agents can declare high-level intents that are automatically planned into executable workflows with proper state management, error handling, and user confirmation for destructive operations.

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

### 1. `refactor.renameSymbol`

Renames a symbol across the entire project with LSP-powered accuracy.

**Required Parameters:**
- `file_path` (string): Path to the file containing the symbol
- `old_name` (string): Current name of the symbol
- `new_name` (string): Desired new name

**Workflow Steps:**
1. Find all references to the symbol using `find_references`
2. Apply workspace edits to rename all occurrences (requires confirmation)

**Example:**
```json
{
  "name": "refactor.renameSymbol",
  "params": {
    "file_path": "src/database.ts",
    "old_name": "connectDB",
    "new_name": "establishDatabaseConnection"
  }
}
```

**Complexity:** 2 steps

---

### 2. `refactor.extractFunction`

Extracts a block of code into a new function.

**Required Parameters:**
- `file_path` (string): Path to the file
- `start_line` (number): Starting line of code to extract
- `end_line` (number): Ending line of code to extract
- `function_name` (string): Name for the new function

**Workflow Steps:**
1. Extract the code block into a new function using `extract_function`

**Example:**
```json
{
  "name": "refactor.extractFunction",
  "params": {
    "file_path": "src/validator.ts",
    "start_line": 45,
    "end_line": 62,
    "function_name": "validateEmailFormat"
  }
}
```

**Complexity:** 1 step

---

### 3. `docs.generateDocstring`

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

### 4. `research.topic`

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

1. **Start with Planning**: Always preview a workflow before executing
2. **Use Dry-Run**: Test destructive workflows with `dry_run: true` first
3. **Review Confirmations**: Carefully review changes when workflows pause
4. **Check Logs**: Execution logs provide detailed information about each step
5. **Handle Errors**: Workflows fail fast - check error messages for debugging

## Future Enhancements

Planned features for future releases:
- User input collection during workflow execution (via `resume_data`)
- Conditional step execution based on previous results
- Parallel step execution for independent operations
- Workflow composition (calling workflows from other workflows)
- Custom validators for step outputs