# Navigation & Intelligence Tools

LSP-powered code navigation with IDE-quality intelligence. Jump to definitions, find references, search symbols workspace-wide, explore implementations, get diagnostics, and traverse call hierarchies. All tools leverage language server protocol for accurate, project-aware results.

**Tool count:** 8 tools
**Related categories:** [Refactoring](refactoring.md) for code transformations

---

## Tools

### find_definition

**Purpose:** Find where a symbol (function, class, variable, method) is defined.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| filePath | string | Yes | Path to file containing the symbol (also accepts `file_path`) |
| line | number | Yes | Line number (1-indexed) |
| character | number | Yes | Character position (0-indexed) |

**Returns:**

Array of definition locations with URI and range information. Each location includes file path and position range (line/character).

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "find_definition",
    "arguments": {
      "filePath": "src/app.ts",
      "line": 10,
      "character": 5
    }
  }
}

// Response
{
  "content": {
    "definitions": [
      {
        "uri": "file:///workspace/src/utils.ts",
        "range": {
          "start": {"line": 10, "character": 0},
          "end": {"line": 20, "character": 1}
        }
      }
    ]
  },
  "plugin": "lsp",
  "processing_time_ms": 45,
  "cached": false
}
```
**Notes:**
- LSP-based, accuracy depends on language server capabilities
- Uses cursor position (line/character) to identify the symbol
- Supports cross-file navigation in multi-file projects
- Both `filePath` (camelCase) and `file_path` (snake_case) are accepted for compatibility

---

### find_references

**Purpose:** Find all locations where a symbol is used across the codebase.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| filePath | string | Yes | Path to file containing the symbol (also accepts `file_path`) |
| line | number | Yes | Line number (1-indexed) |
| character | number | Yes | Character position (0-indexed) |
| include_declaration | boolean | No | Include definition location (default: true) |

**Returns:**

Array of reference locations with URI and range. Includes total count of references found.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "find_references",
    "arguments": {
      "filePath": "src/utils.ts",
      "line": 5,
      "character": 9,
      "include_declaration": true
    }
  }
}

// Response
{
  "content": {
    "references": [
      {
        "uri": "file:///workspace/src/utils.ts",
        "range": {
          "start": {"line": 5, "character": 9},
          "end": {"line": 5, "character": 18}
        }
      },
      {
        "uri": "file:///workspace/src/app.ts",
        "range": {
          "start": {"line": 42, "character": 10},
          "end": {"line": 42, "character": 19}
        }
      }
    ],
    "total": 2
  },
  "plugin": "lsp",
  "processing_time_ms": 120,
  "cached": false
}
```
**Notes:**
- Set `include_declaration: false` to exclude definition location
- Searches entire workspace, not just current file
- May include references in comments/strings depending on LSP server
- Useful for impact analysis before refactoring

---

### search_symbols

**Purpose:** Search for symbols (functions, classes, variables) across entire workspace by name pattern.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| query | string | Yes | Search query (supports partial matching) |
| workspacePath | string | No | Workspace directory to search (also accepts `workspace_path`, defaults to current directory) |

**Returns:**

Array of matching symbols with name, kind, location, and optional container name. Queries all active LSP servers.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "search_symbols",
    "arguments": {
      "query": "config"
    }
  }
}

// Response
{
  "content": [
    {
      "name": "loadConfig",
      "kind": "Function",
      "location": {
        "uri": "file:///workspace/src/config.ts",
        "range": {
          "start": {"line": 5, "character": 0},
          "end": {"line": 20, "character": 1}
        }
      },
      "containerName": "config"
    },
    {
      "name": "ConfigManager",
      "kind": "Class",
      "location": {
        "uri": "file:///workspace/src/managers.ts",
        "range": {
          "start": {"line": 10, "character": 0},
          "end": {"line": 50, "character": 1}
        }
      }
    }
  ],
  "plugin": "multi-plugin (typescript-lsp, rust-analyzer)",
  "processing_time_ms": 250,
  "cached": false
}
```
**Notes:**
- Queries ALL registered language servers (TypeScript, Rust, etc.)
- Fast for specific queries, slower for broad searches (single letters)
- Supports partial matching (query "conf" matches "loadConfig")
- Results aggregated from all active LSP servers
- Formerly named `search_workspace_symbols`

---

### find_implementations

**Purpose:** Find concrete implementations of an interface or abstract class.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | Yes | Path to file containing interface/abstract class |
| symbol_name | string | Yes | Name of interface or abstract class |
| symbol_kind | string | No | Kind hint (interface, class, etc.) |

**Returns:**

Array of implementation locations with URI and range for each implementing class.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "find_implementations",
    "arguments": {
      "file_path": "src/interfaces.ts",
      "symbol_name": "Drawable"
    }
  }
}

// Response
{
  "content": {
    "implementations": [
      {
        "uri": "file:///workspace/src/shapes/circle.ts",
        "range": {
          "start": {"line": 10, "character": 0},
          "end": {"line": 30, "character": 1}
        }
      },
      {
        "uri": "file:///workspace/src/shapes/square.ts",
        "range": {
          "start": {"line": 5, "character": 0},
          "end": {"line": 25, "character": 1}
        }
      }
    ]
  },
  "plugin": "lsp",
  "processing_time_ms": 85,
  "cached": false
}
```
**Notes:**
- Language-specific behavior: TypeScript interfaces, Rust traits, etc.
- Returns all classes that implement the specified interface
- Useful for understanding polymorphic implementations
- May return empty array if no implementations found

---

### find_type_definition

**Purpose:** Find the underlying type definition of a symbol (e.g., type of a variable).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | Yes | Path to file containing the symbol |
| symbol_name | string | Yes | Name of symbol to get type for |
| symbol_kind | string | No | Kind hint (variable, property, etc.) |

**Returns:**

Array of type definition locations with URI and range. For variables, returns the type declaration rather than variable declaration.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "find_type_definition",
    "arguments": {
      "file_path": "src/app.ts",
      "symbol_name": "user"
    }
  }
}

// Response - For variable `user: User`, returns the User type definition
{
  "content": {
    "definitions": [
      {
        "uri": "file:///workspace/src/types.ts",
        "range": {
          "start": {"line": 5, "character": 0},
          "end": {"line": 10, "character": 1}
        }
      }
    ]
  },
  "plugin": "lsp",
  "processing_time_ms": 50,
  "cached": false
}
```
**Notes:**
- Different from `find_definition` - finds type, not variable location
- For `const user: User = ...`, finds `interface User {}` definition
- Useful for navigating to type declarations in typed languages
- Returns empty if symbol has no explicit type (e.g., inferred types)

---

### get_symbol_info

**Purpose:** Get detailed hover information including documentation, types, and signatures for a symbol.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | Yes | Path to file containing the symbol |
| line | number | Yes | Line number (1-indexed) |
| character | number | Yes | Character position (0-indexed) |

**Returns:**

Hover information with markdown-formatted content including type signatures and documentation. Includes range of symbol.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "get_symbol_info",
    "arguments": {
      "file_path": "src/utils.ts",
      "line": 10,
      "character": 5
    }
  }
}

// Response
{
  "content": {
    "contents": {
      "kind": "markdown",
      "value": "```typescript\nfunction formatDate(date: Date, format?: string): string\n```\nFormats a date with optional format string.\n\n**Parameters:**\n- `date` - Date to format\n- `format` - Format string (optional, default: 'YYYY-MM-DD')\n\n**Returns:** Formatted date string"
    },
    "range": {
      "start": {"line": 10, "character": 5},
      "end": {"line": 10, "character": 15}
    }
  },
  "plugin": "lsp",
  "processing_time_ms": 35,
  "cached": false
}
```
**Notes:**
- Internally maps to LSP `get_hover` request
- Returns rich documentation from JSDoc/TSDoc comments
- Includes type information inferred by language server
- Content formatted in markdown for display
- Essential for understanding APIs without reading source

---

### get_diagnostics

**Purpose:** Get all language diagnostics (errors, warnings, hints) for a file.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | Yes | Path to file to check |

**Returns:**

Array of diagnostics with range, severity, message, code, and source. Includes total count.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "get_diagnostics",
    "arguments": {
      "file_path": "src/app.ts"
    }
  }
}

// Response
{
  "content": {
    "diagnostics": [
      {
        "range": {
          "start": {"line": 10, "character": 5},
          "end": {"line": 10, "character": 10}
        },
        "severity": "Error",
        "message": "Cannot find name 'foo'",
        "code": 2304,
        "source": "typescript"
      },
      {
        "range": {
          "start": {"line": 15, "character": 0},
          "end": {"line": 15, "character": 20}
        },
        "severity": "Warning",
        "message": "Variable 'username' is declared but never used",
        "code": 6133,
        "source": "typescript"
      }
    ],
    "total": 2
  },
  "plugin": "lsp",
  "processing_time_ms": 60,
  "cached": false
}
```
**Notes:**
- Uses LSP `textDocument/diagnostic` for real-time diagnostics
- Severity levels: Error, Warning, Information, Hint
- Diagnostics pulled from language server, not static analysis
- Essential for validating code before commits
- Error codes are language-specific (e.g., TypeScript error codes)

---

### get_call_hierarchy

**Purpose:** Get call hierarchy for a function - what calls it (incoming) or what it calls (outgoing).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| file_path | string | Yes | Path to file containing the function |
| line | number | Yes | Line number (1-indexed) |
| character | number | Yes | Character position (0-indexed) |
| type | string | No | Hierarchy type: "incoming", "outgoing", or omit for prepare |

**Returns:**

For prepare (no type): Call hierarchy item with name, kind, URI, range.
For incoming: Array of calls from other functions.
For outgoing: Array of calls to other functions.

**Example (Prepare):**

```json
// MCP request - Prepare call hierarchy
{
  "method": "tools/call",
  "params": {
    "name": "get_call_hierarchy",
    "arguments": {
      "file_path": "src/utils.ts",
      "line": 10,
      "character": 5
    }
  }
}

// Response
{
  "content": {
    "item": {
      "name": "processData",
      "kind": "Function",
      "uri": "file:///workspace/src/utils.ts",
      "range": {
        "start": {"line": 10, "character": 0},
        "end": {"line": 20, "character": 1}
      },
      "selectionRange": {
        "start": {"line": 10, "character": 9},
        "end": {"line": 10, "character": 20}
      }
    }
  },
  "plugin": "lsp",
  "processing_time_ms": 40,
  "cached": false
}
```
**Example (Incoming Calls):**

```json
// MCP request - Find what calls this function
{
  "method": "tools/call",
  "params": {
    "name": "get_call_hierarchy",
    "arguments": {
      "file_path": "src/utils.ts",
      "line": 10,
      "character": 5,
      "type": "incoming"
    }
  }
}

// Response
{
  "content": {
    "calls": [
      {
        "from": {
          "name": "handleSubmit",
          "kind": "Function",
          "uri": "file:///workspace/src/app.ts"
        },
        "fromRanges": [
          {
            "start": {"line": 50, "character": 10},
            "end": {"line": 50, "character": 21}
          }
        ]
      },
      {
        "from": {
          "name": "processForm",
          "kind": "Function",
          "uri": "file:///workspace/src/forms.ts"
        },
        "fromRanges": [
          {
            "start": {"line": 25, "character": 5},
            "end": {"line": 25, "character": 16}
          }
        ]
      }
    ]
  },
  "plugin": "lsp",
  "processing_time_ms": 95,
  "cached": false
}
```
**Example (Outgoing Calls):**

```json
// MCP request - Find what this function calls
{
  "method": "tools/call",
  "params": {
    "name": "get_call_hierarchy",
    "arguments": {
      "file_path": "src/utils.ts",
      "line": 10,
      "character": 5,
      "type": "outgoing"
    }
  }
}

// Response
{
  "content": {
    "calls": [
      {
        "to": {
          "name": "validateData",
          "kind": "Function",
          "uri": "file:///workspace/src/validation.ts"
        },
        "fromRanges": [
          {
            "start": {"line": 15, "character": 5},
            "end": {"line": 15, "character": 17}
          }
        ]
      },
      {
        "to": {
          "name": "sendEmail",
          "kind": "Function",
          "uri": "file:///workspace/src/email.ts"
        },
        "fromRanges": [
          {
            "start": {"line": 18, "character": 10},
            "end": {"line": 18, "character": 19}
          }
        ]
      }
    ]
  },
  "plugin": "lsp",
  "processing_time_ms": 105,
  "cached": false
}
```
**Notes:**
- Three-step workflow: prepare (no type), then incoming or outgoing
- Internally maps to LSP call hierarchy methods based on `type` parameter
- Type parameter determines which LSP method is called:
  - Omit or null: `prepare_call_hierarchy`
  - `"incoming"`: `get_call_hierarchy_incoming_calls`
  - `"outgoing"`: `get_call_hierarchy_outgoing_calls`
- Useful for understanding function dependencies
- Essential for refactoring impact analysis
- Language server support varies (TypeScript, Rust supported)

---

## Common Navigation Patterns

### Finding Symbol Definition and Usage

Typical workflow for exploring a symbol:

1. **Find definition**: Use `find_definition` to locate where symbol is defined
2. **Find references**: Use `find_references` to see all usage locations
3. **Get details**: Use `get_symbol_info` at definition for documentation
4. **Check diagnostics**: Use `get_diagnostics` to validate usage context

### Understanding Call Flow

Analyze function call chains:

1. **Prepare hierarchy**: Call `get_call_hierarchy` without type parameter
2. **Find callers**: Use `type: "incoming"` to see what calls this function
3. **Find callees**: Use `type: "outgoing"` to see what this function calls
4. **Repeat**: Navigate up/down call chain by repeating on discovered functions

### Workspace-Wide Symbol Search

Quick symbol lookup across entire codebase:

```json
// Search for all config-related symbols
{
  "name": "search_symbols",
  "arguments": {
    "query": "config"
  }
}

// Returns matches from all language servers (TypeScript + Rust + others)
```
### Implementation Discovery

Find all implementations of an interface:

1. **Find interface definition**: Use `find_definition` on interface usage
2. **Find implementations**: Use `find_implementations` on interface
3. **Explore each implementation**: Use `get_symbol_info` on each result
4. **Check implementation quality**: Use `get_diagnostics` on implementation files

### Type Navigation

Navigate type hierarchies:

```json
// For variable: const user: User = ...
// 1. Find variable definition
{"name": "find_definition", "arguments": {"symbol_name": "user"}}

// 2. Find type definition (goes to User interface)
{"name": "find_type_definition", "arguments": {"symbol_name": "user"}}

// 3. Find all User implementations/usages
{"name": "find_references", "arguments": {"symbol_name": "User"}}
```
### Error Resolution Workflow

Fix errors systematically:

1. **Get all errors**: `get_diagnostics` on file
2. **For each error**:
   - Use `get_symbol_info` to understand undefined symbols
   - Use `find_definition` to locate correct import sources
   - Use `find_references` to see similar correct usage patterns
3. **Verify fix**: Re-run `get_diagnostics` to confirm resolution

### Multi-Language Navigation

Navigate across language boundaries in polyglot codebases:

```json
// Search finds symbols from ALL configured language servers
{
  "name": "search_symbols",
  "arguments": {"query": "HttpClient"}
}

// Returns results from:
// - TypeScript: class HttpClient (from typescript-language-server)
// - Rust: struct HttpClient (from rust-analyzer)
```
---

## Error Handling

**Common errors:**

- **Symbol not found**: LSP server couldn't locate symbol (check spelling, imports)
- **LSP server not available**: Language server not configured or crashed (check `.typemill/config.json`)
- **Invalid position**: Line/character out of bounds (ensure 1-indexed line, 0-indexed character)
- **No implementations found**: Interface has no implementations or LSP doesn't support (check language server capabilities)

**Example error:**

```json
{
  "error": {
    "code": -32603,
    "message": "LSP request failed: Symbol not found",
    "data": {
      "tool": "find_definition",
      "file_path": "src/app.ts",
      "symbol_name": "unknownSymbol"
    }
  }
}
```
---

## LSP Server Requirements

All navigation tools require configured LSP servers in `.typemill/config.json`:

```json
{
  "servers": [
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"]
    },
    {
      "extensions": ["rs"],
      "command": ["rust-analyzer"]
    }
  ]
}
```
Run `mill setup` for automatic language server detection and configuration.

---

**See also:**
- [Refactoring Tools](refactoring.md) - Code transformations
- [System Tools](system.md) - Server health and diagnostics
