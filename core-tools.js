// src/mcp/definitions/core-tools.ts
var coreToolDefinitions = [
  {
    name: "find_definition",
    description: "Find the definition of a symbol by name and kind in a file. Returns definitions for all matching symbols.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        },
        symbol_name: {
          type: "string",
          description: "The name of the symbol"
        },
        symbol_kind: {
          type: "string",
          description: "The kind of symbol (function, class, variable, method, etc.)"
        }
      },
      required: ["file_path", "symbol_name"]
    }
  },
  {
    name: "find_references",
    description: "Find all references to a symbol by name and kind in a file. Returns references for all matching symbols.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        },
        symbol_name: {
          type: "string",
          description: "The name of the symbol"
        },
        symbol_kind: {
          type: "string",
          description: "The kind of symbol (function, class, variable, method, etc.)"
        },
        include_declaration: {
          type: "boolean",
          description: "Whether to include the declaration",
          default: true
        }
      },
      required: ["file_path", "symbol_name"]
    }
  },
  {
    name: "rename_symbol",
    description: "Rename a symbol by name and kind in a file. If multiple symbols match, returns candidate positions and suggests using rename_symbol_strict. By default, this will apply the rename to the files. Use dry_run to preview changes without applying them.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        },
        symbol_name: {
          type: "string",
          description: "The name of the symbol"
        },
        symbol_kind: {
          type: "string",
          description: "The kind of symbol (function, class, variable, method, etc.)"
        },
        new_name: {
          type: "string",
          description: "The new name for the symbol"
        },
        dry_run: {
          type: "boolean",
          description: "If true, only preview the changes without applying them (default: false)"
        }
      },
      required: ["file_path", "symbol_name", "new_name"]
    }
  },
  {
    name: "rename_symbol_strict",
    description: "Rename a symbol at a specific position in a file. Use this when rename_symbol returns multiple candidates. By default, this will apply the rename to the files. Use dry_run to preview changes without applying them.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        },
        line: {
          type: "number",
          description: "The line number (1-indexed)"
        },
        character: {
          type: "number",
          description: "The character position in the line (1-indexed)"
        },
        new_name: {
          type: "string",
          description: "The new name for the symbol"
        },
        dry_run: {
          type: "boolean",
          description: "If true, only preview the changes without applying them (default: false)"
        }
      },
      required: ["file_path", "line", "character", "new_name"]
    }
  }
];
export {
  coreToolDefinitions
};
