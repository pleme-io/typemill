# MCP Tools Reference

All 25 MCP tools with practical examples for AI assistants.

## Core Navigation & Analysis

### `find_definition`

Find the definition of a symbol by name and kind in a file. Returns definitions for all matching symbols.

**Parameters:**
- `file_path`: The path to the file
- `symbol_name`: The name of the symbol
- `symbol_kind`: The kind of symbol (function, class, variable, method, etc.) (optional)

**When to use:** Finding where functions, classes, or variables are defined

**Example prompt:** "Find the definition of handleLogin function"

**Returns:** File path and exact line number where symbol is defined

### `find_references`

Find all references to a symbol by name and kind in a file. Returns references for all matching symbols.

**Parameters:**
- `file_path`: The path to the file
- `symbol_name`: The name of the symbol
- `symbol_kind`: The kind of symbol (function, class, variable, method, etc.) (optional)
- `include_declaration`: Whether to include the declaration (optional, default: true)

**When to use:** Finding all places where a symbol is used

**Example prompt:** "Find all references to UserService class"

**Returns:** List of all files and line numbers using the symbol

## Code Modification

### `rename_symbol`

Rename a symbol by name and kind in a file. **This tool applies the rename to all affected files by default.** If multiple symbols match, returns candidate positions and suggests using rename_symbol_strict.

**Parameters:**
- `file_path`: The path to the file
- `symbol_name`: The name of the symbol
- `symbol_kind`: The kind of symbol (function, class, variable, method, etc.) (optional)
- `new_name`: The new name for the symbol
- `dry_run`: If true, only preview the changes without applying them (optional, default: false)

**Note:** When `dry_run` is false (default), the tool will:
- Apply the rename to all affected files
- Create backup files with `.bak` extension
- Return the list of modified files

**Examples:**
```bash
# Preview rename changes
{
  "tool": "rename_symbol",
  "arguments": {
    "file_path": "/path/to/file.ts", 
    "symbol_name": "getUserData",
    "new_name": "fetchUserProfile",
    "dry_run": true
  }
}

# Apply rename across codebase
{
  "tool": "rename_symbol",
  "arguments": {
    "file_path": "/path/to/file.ts",
    "symbol_name": "getUserData", 
    "new_name": "fetchUserProfile"
  }
}
```

### `rename_symbol_strict`

Rename a symbol at a specific position in a file. Use this when rename_symbol returns multiple candidates. **This tool applies the rename to all affected files by default.**

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (1-indexed)
- `new_name`: The new name for the symbol
- `dry_run`: If true, only preview the changes without applying them (optional, default: false)

**Example:**
```bash
# Rename symbol at specific position
{
  "tool": "rename_symbol_strict",
  "arguments": {
    "file_path": "/path/to/file.ts",
    "line": 45,
    "character": 10,
    "new_name": "userData"
  }
}
```

### `format_document`

Format a document according to the language server's formatting rules.

**Parameters:**
- `file_path`: The path to the file to format
- `options`: Formatting options (optional)
- `dry_run`: If true, only preview changes without applying them (optional, default: false)

### `get_code_actions`

Get available code actions (fixes, refactors) for a specific range in a file.

**Parameters:**
- `file_path`: The path to the file
- `start_line`: Start line number (1-indexed)
- `start_character`: Start character position (0-indexed)  
- `end_line`: End line number (1-indexed)
- `end_character`: End character position (0-indexed)

### `apply_workspace_edit`

Apply workspace edits (file changes) across multiple files.

**Parameters:**
- `changes`: Record mapping file paths to arrays of text edits
- `validate_before_apply`: Whether to validate changes before applying (optional, default: false)

### `create_file`

Create a new file with specified content.

**Parameters:**
- `file_path`: Path where the new file should be created
- `content`: Content for the new file

### `delete_file`

Delete a file from the filesystem.

**Parameters:**
- `file_path`: Path to the file to delete
- `dry_run`: If true, only preview the action without executing (optional, default: false)

### `rename_file`

Rename a file and update all import statements that reference it. Includes circular dependency detection to prevent unsafe moves.

**Parameters:**
- `old_path`: Current file path
- `new_path`: New file path
- `dry_run`: If true, only preview changes without applying them (optional, default: false)

**When to use:** Moving individual files with automatic import updates

**Example prompt:** "Rename user-service.ts to account-service.ts"

**Safety features:** Automatically detects and prevents circular dependencies

### `rename_directory`

Rename an entire directory and update all import statements that reference files within it. Includes circular dependency detection for safe directory moves.

**Parameters:**
- `old_path`: Current directory path
- `new_path`: New directory path
- `dry_run`: If true, only preview changes without applying them (optional, default: false)

**When to use:** Moving entire folders/packages with automatic import updates for all contained files

**Example prompt:** "Move the utils directory to lib/utilities"

**Safety features:** Checks for circular dependencies across all files in the directory before moving

**Returns:** Summary of all files moved and import updates applied

### `update_package_json`

Update package.json files by adding, removing, or modifying dependencies, scripts, and configuration. Preserves formatting and validates changes.

**Parameters:**
- `file_path`: Path to package.json file (default: "./package.json")
- `add_dependencies`: Object of dependencies to add to "dependencies" section (optional)
- `add_dev_dependencies`: Object of dependencies to add to "devDependencies" section (optional)
- `remove_dependencies`: Array of dependency names to remove (optional)
- `add_scripts`: Object of scripts to add to "scripts" section (optional)
- `remove_scripts`: Array of script names to remove (optional)
- `update_version`: New version string (optional)
- `workspace_config`: Workspace configuration with workspaces array (optional)
- `dry_run`: If true, only preview changes without applying them (optional, default: false)

**When to use:** Managing project dependencies, scripts, and workspace configuration

**Example prompt:** "Add lodash as a dependency and a build script"

**Returns:** Summary of all changes made to the package.json file

### `fix_imports`

Fix import paths in a file that has been moved to a new location. Updates relative imports to maintain correct references.

**Parameters:**
- `file_path`: Path to the file with imports to fix
- `old_path`: The file's previous location (for calculating new relative paths)

**When to use:** After manually moving files or when import paths are broken

**Example prompt:** "Fix the imports in user-service.ts after it was moved from utils/ to services/"

**Returns:** List of import statements that were updated

### `analyze_imports`

Analyze import relationships for a file, showing both what it imports and what imports it.

**Parameters:**
- `file_path`: Path to the file to analyze
- `include_importers`: Whether to show files that import this file (optional, default: true)
- `include_imports`: Whether to show what this file imports (optional, default: true)

**When to use:** Understanding dependencies before refactoring or debugging circular imports

**Example prompt:** "Show me what imports user-service.ts and what it imports"

**Returns:** Comprehensive import relationship analysis

### `find_dead_code`

Find potentially unused code (functions, classes, variables) with no external references.

**Parameters:**
- `files`: Specific files to analyze (optional, defaults to common source files)
- `exclude_tests`: Skip test files in analysis (optional, default: true)
- `min_references`: Minimum references to not be considered dead (optional, default: 1)

**When to use:** Code cleanup, bundle size optimization, identifying unused exports

**Example prompt:** "Find dead code in my TypeScript project"

**Returns:** Detailed report of potentially unused symbols with locations and reference counts

### `health_check`

Get health status of LSP servers and system resources.

**Parameters:**
- `include_details`: Include detailed server information (optional, default: false)

**When to use:** Debugging LSP issues, monitoring system performance, troubleshooting

**Example prompt:** "Check the health of all language servers"

**Returns:** System health report with server status and resource usage

### `execute_workflow`

Execute a dependency-orchestrated workflow chain with multiple tool operations.

**Parameters:**
- `chain`: Workflow definition with dependency relationships
- `inputs`: Input parameters for the workflow execution

**When to use:** Complex multi-step operations with dependencies between steps

**Example prompt:** "Execute a workflow to refactor and test changes"

**Returns:** Workflow execution results with step-by-step status

## Code Intelligence

### `get_hover`

Get hover information (documentation, types, signatures) for a symbol at a specific position.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (0-indexed)

### `get_completions`

Get code completion suggestions at a specific position in a file.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed) 
- `character`: The character position in the line (0-indexed)


### `get_signature_help`

Get function signature help at a specific position in the code.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (0-indexed)

## Code Structure Analysis

### `prepare_call_hierarchy`

Prepare call hierarchy items for a symbol at a specific position.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (0-indexed)

### `get_call_hierarchy_incoming_calls`

Get incoming calls for a call hierarchy item.

**Parameters:**
- `item`: Call hierarchy item from prepare_call_hierarchy

### `get_call_hierarchy_outgoing_calls`

Get outgoing calls for a call hierarchy item.

**Parameters:**
- `item`: Call hierarchy item from prepare_call_hierarchy

## Workspace Operations

### `get_document_symbols`

Get all symbols in a document for code outline and navigation.

**Parameters:**
- `file_path`: The path to the file

### `search_workspace_symbols`

Search for symbols across the entire workspace.

**Parameters:**
- `query`: Search query string

## Batch Operations

### `batch_execute`

Execute multiple MCP operations in a single transaction with options for parallel execution, dry-run previews, and automatic rollback on failure.

**Parameters:**
- `operations`: Array of tool operations to execute
  - `tool`: Name of the MCP tool to execute (e.g., "find_definition", "rename_file", "get_diagnostics")
  - `args`: Arguments for the specific tool (same as individual tool arguments)
  - `id`: Optional identifier for correlating results
- `options`: Execution options for the batch operation
  - `atomic`: If true, all operations succeed or all fail with automatic rollback (default: false)
  - `parallel`: If true, execute operations in parallel for better performance (default: false)
  - `dry_run`: If true, preview what would happen without executing (default: false)
  - `stop_on_error`: If true, stop execution when first error occurs (default: true)

**Examples:**
```bash
# Execute multiple operations sequentially
{
  "tool": "batch_execute",
  "arguments": {
    "operations": [
      {
        "tool": "find_definition",
        "args": {"file_path": "src/index.ts", "symbol_name": "main"},
        "id": "find-main"
      },
      {
        "tool": "rename_file",
        "args": {"old_path": "old.ts", "new_path": "new.ts"},
        "id": "rename-1"
      },
      {
        "tool": "get_diagnostics",
        "args": {"file_path": "new.ts"},
        "id": "check-errors"
      }
    ],
    "options": {
      "atomic": true,
      "parallel": false,
      "dry_run": false
    }
  }
}

# Preview operations without executing
{
  "tool": "batch_execute",
  "arguments": {
    "operations": [
      {
        "tool": "format_document",
        "args": {"file_path": "src/utils.ts"}
      },
      {
        "tool": "get_diagnostics",
        "args": {"file_path": "src/utils.ts"}
      }
    ],
    "options": {
      "dry_run": true
    }
  }
}
```

## System Operations

### `get_diagnostics`

Get language diagnostics (errors, warnings, hints) for a file. Uses LSP textDocument/diagnostic to pull current diagnostics.

**Parameters:**
- `file_path`: The path to the file to get diagnostics for

**Example:**
```bash
# Check for errors and warnings
{
  "tool": "get_diagnostics",
  "arguments": {
    "file_path": "/path/to/file.ts"
  }
}
```

### `restart_server`

Manually restart LSP servers and retry any previously failed servers. Can restart servers for specific file extensions or all running servers.

**Parameters:**
- `extensions`: Array of file extensions to restart servers for (e.g., ["ts", "tsx"]). If not provided, all servers will be restarted (optional)

**Examples:**
```bash
# Restart TypeScript server
{
  "tool": "restart_server",
  "arguments": {
    "extensions": ["ts", "tsx"]
  }
}

# Restart all servers
{
  "tool": "restart_server",
  "arguments": {}
}
```

## Real-world Usage Examples

### Finding Function Definitions

When Claude needs to understand how a function works:

```
Claude: Let me find the definition of the `processRequest` function
> Using codebuddy.find_definition with symbol_name="processRequest", symbol_kind="function"

Result: Found definition at src/handlers/request.ts:127:1
```

### Finding All References

When refactoring or understanding code impact:

```
Claude: I'll find all places where `CONFIG_PATH` is used
> Using codebuddy.find_references with symbol_name="CONFIG_PATH"

Results: Found 5 references:
- src/config.ts:10:1 (declaration)
- src/index.ts:45:15
- src/utils/loader.ts:23:8
- tests/config.test.ts:15:10
- tests/config.test.ts:89:12
```

### Safe Renaming Across Codebase

Preview changes before applying:

```
Claude: Let me first preview what will be renamed
> Using codebuddy.rename_symbol with symbol_name="getUserData", new_name="fetchUserProfile", dry_run=true

Result: [DRY RUN] Would rename getUserData (function) to "fetchUserProfile":
File: src/api/user.ts
  - Line 55, Column 10 to Line 55, Column 21: "fetchUserProfile"
File: src/services/auth.ts
  - Line 123, Column 15 to Line 123, Column 26: "fetchUserProfile"
... (12 files total)
```

Apply the rename:

```
Claude: I'll rename `getUserData` to `fetchUserProfile`
> Using codebuddy.rename_symbol with symbol_name="getUserData", new_name="fetchUserProfile"

Result: Successfully renamed getUserData (function) to "fetchUserProfile".

Modified files:
- src/api/user.ts
- src/services/auth.ts
- src/components/UserProfile.tsx
... (12 files total)
```

### Handling Multiple Symbol Matches

When multiple symbols match:

```
Claude: I'll rename the `data` variable to `userData`
> Using codebuddy.rename_symbol with symbol_name="data", new_name="userData"

Result: Multiple symbols found matching "data". Please use rename_symbol_strict with one of these positions:
- data (variable) at line 45, character 10
- data (parameter) at line 89, character 25
- data (property) at line 112, character 5

> Using codebuddy.rename_symbol_strict with line=45, character=10, new_name="userData"

Result: Successfully renamed symbol at line 45, character 10 to "userData".

Modified files:
- src/utils/parser.ts
```

### Checking File Diagnostics

When analyzing code quality:

```
Claude: Let me check for any errors or warnings in this file
> Using codebuddy.get_diagnostics

Results: Found 3 diagnostics:
- Error [TS2304]: Cannot find name 'undefinedVar' (Line 10, Column 5)
- Warning [no-unused-vars]: 'config' is defined but never used (Line 25, Column 10)
- Hint: Consider using const instead of let (Line 30, Column 1)
```

### Restarting LSP Servers

When LSP servers become unresponsive:

```
Claude: The TypeScript server seems unresponsive, let me restart it
> Using codebuddy.restart_server with extensions ["ts", "tsx"]

Result: Successfully restarted 1 LSP server(s)
Restarted servers:
• typescript-language-server --stdio

Note: Any previously failed servers have been cleared and will be retried on next access.
```

Or restart all servers:

```
Claude: I'll restart all LSP servers to ensure they're working properly
> Using codebuddy.restart_server

Result: Successfully restarted 2 LSP server(s)
Restarted servers:
• typescript-language-server --stdio (ts, tsx)
• pylsp (py)

Note: Any previously failed servers have been cleared and will be retried on next access.
```