// src/mcp/definitions/advanced-tools.ts
var advancedToolDefinitions = [
  {
    name: "get_code_actions",
    description: "Get available code actions (quick fixes, refactors, organize imports) for a file or specific range. Can apply auto-fixes like removing unused imports, adding missing imports, and organizing imports.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        },
        range: {
          type: "object",
          description: "Optional range to get code actions for. If not provided, gets actions for entire file.",
          properties: {
            start: {
              type: "object",
              properties: {
                line: { type: "number", description: "Start line (0-indexed)" },
                character: { type: "number", description: "Start character (0-indexed)" }
              },
              required: ["line", "character"]
            },
            end: {
              type: "object",
              properties: {
                line: { type: "number", description: "End line (0-indexed)" },
                character: { type: "number", description: "End character (0-indexed)" }
              },
              required: ["line", "character"]
            }
          },
          required: ["start", "end"]
        }
      },
      required: ["file_path"]
    }
  },
  {
    name: "format_document",
    description: "Format a document using the language server's formatter. Applies consistent code style and formatting rules.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file to format"
        },
        options: {
          type: "object",
          description: "Formatting options",
          properties: {
            tab_size: { type: "number", description: "Size of tabs (default: 2)" },
            insert_spaces: {
              type: "boolean",
              description: "Use spaces instead of tabs (default: true)"
            },
            trim_trailing_whitespace: { type: "boolean", description: "Trim trailing whitespace" },
            insert_final_newline: { type: "boolean", description: "Insert final newline" },
            trim_final_newlines: { type: "boolean", description: "Trim final newlines" }
          }
        }
      },
      required: ["file_path"]
    }
  },
  {
    name: "search_workspace_symbols",
    description: "Search for symbols (functions, classes, variables, etc.) across the entire workspace. Useful for finding symbols by name across multiple files.",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "Search query for symbol names (supports partial matching)"
        }
      },
      required: ["query"]
    }
  },
  {
    name: "get_document_symbols",
    description: "Get all symbols (functions, classes, variables, etc.) defined in a specific file. Returns a hierarchical structure of symbols with their locations and types.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        }
      },
      required: ["file_path"]
    }
  },
  {
    name: "get_folding_ranges",
    description: "Get folding ranges for code structure understanding. Shows logical code blocks that can be folded/collapsed (functions, classes, comments, imports). Helps AI agents understand code organization and nesting levels.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        }
      },
      required: ["file_path"]
    }
  },
  {
    name: "get_document_links",
    description: "Get clickable links in a document (URLs, file references, imports, documentation links). Helps AI agents understand project relationships and external dependencies. Different language servers provide different types of links.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        }
      },
      required: ["file_path"]
    }
  },
  {
    name: "apply_workspace_edit",
    description: "Apply a workspace edit (multi-file text changes) atomically. This is the most powerful editing tool for AI agents, allowing safe modification of multiple files in a single atomic operation with rollback capability. Essential for large refactoring operations.",
    inputSchema: {
      type: "object",
      properties: {
        changes: {
          type: "object",
          description: "Map of file URIs/paths to arrays of text edits",
          additionalProperties: {
            type: "array",
            items: {
              type: "object",
              properties: {
                range: {
                  type: "object",
                  properties: {
                    start: {
                      type: "object",
                      properties: {
                        line: { type: "number", description: "Start line (0-indexed)" },
                        character: { type: "number", description: "Start character (0-indexed)" }
                      },
                      required: ["line", "character"]
                    },
                    end: {
                      type: "object",
                      properties: {
                        line: { type: "number", description: "End line (0-indexed)" },
                        character: { type: "number", description: "End character (0-indexed)" }
                      },
                      required: ["line", "character"]
                    }
                  },
                  required: ["start", "end"]
                },
                newText: {
                  type: "string",
                  description: "The new text to replace the range"
                }
              },
              required: ["range", "newText"]
            }
          }
        },
        validate_before_apply: {
          type: "boolean",
          description: "Whether to validate edit positions before applying (default: true)",
          default: true
        }
      },
      required: ["changes"]
    }
  }
];

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

// src/mcp/definitions/hierarchy-tools.ts
var hierarchyToolDefinitions = [
  {
    name: "prepare_call_hierarchy",
    description: "Prepare call hierarchy for a symbol. Gets the call hierarchy item that can be used to explore incoming and outgoing calls.",
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
          description: "The character position in the line (0-indexed)"
        }
      },
      required: ["file_path", "line", "character"]
    }
  },
  {
    name: "get_call_hierarchy_incoming_calls",
    description: "Get all incoming calls to a function/method. Shows where this function is called from throughout the codebase.",
    inputSchema: {
      type: "object",
      properties: {
        item: {
          type: "object",
          description: "The call hierarchy item (from prepare_call_hierarchy)",
          properties: {
            name: { type: "string" },
            kind: { type: "number" },
            uri: { type: "string" },
            range: {
              type: "object",
              properties: {
                start: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                },
                end: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                }
              },
              required: ["start", "end"]
            },
            selectionRange: {
              type: "object",
              properties: {
                start: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                },
                end: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                }
              },
              required: ["start", "end"]
            }
          },
          required: ["name", "kind", "uri", "range", "selectionRange"]
        }
      },
      required: ["item"]
    }
  },
  {
    name: "get_call_hierarchy_outgoing_calls",
    description: "Get all outgoing calls from a function/method. Shows what functions this function calls.",
    inputSchema: {
      type: "object",
      properties: {
        item: {
          type: "object",
          description: "The call hierarchy item (from prepare_call_hierarchy)",
          properties: {
            name: { type: "string" },
            kind: { type: "number" },
            uri: { type: "string" },
            range: {
              type: "object",
              properties: {
                start: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                },
                end: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                }
              },
              required: ["start", "end"]
            },
            selectionRange: {
              type: "object",
              properties: {
                start: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                },
                end: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                }
              },
              required: ["start", "end"]
            }
          },
          required: ["name", "kind", "uri", "range", "selectionRange"]
        }
      },
      required: ["item"]
    }
  },
  {
    name: "prepare_type_hierarchy",
    description: "Prepare type hierarchy for a symbol. Gets the type hierarchy item for exploring inheritance relationships.",
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
          description: "The character position in the line (0-indexed)"
        }
      },
      required: ["file_path", "line", "character"]
    }
  },
  {
    name: "get_type_hierarchy_supertypes",
    description: "Get supertypes (parent classes/interfaces) in the type hierarchy.",
    inputSchema: {
      type: "object",
      properties: {
        item: {
          type: "object",
          description: "The type hierarchy item (from prepare_type_hierarchy)",
          properties: {
            name: { type: "string" },
            kind: { type: "number" },
            uri: { type: "string" },
            range: {
              type: "object",
              properties: {
                start: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                },
                end: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                }
              },
              required: ["start", "end"]
            },
            selectionRange: {
              type: "object",
              properties: {
                start: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                },
                end: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                }
              },
              required: ["start", "end"]
            }
          },
          required: ["name", "kind", "uri", "range", "selectionRange"]
        }
      },
      required: ["item"]
    }
  },
  {
    name: "get_type_hierarchy_subtypes",
    description: "Get subtypes (child classes/implementations) in the type hierarchy.",
    inputSchema: {
      type: "object",
      properties: {
        item: {
          type: "object",
          description: "The type hierarchy item (from prepare_type_hierarchy)",
          properties: {
            name: { type: "string" },
            kind: { type: "number" },
            uri: { type: "string" },
            range: {
              type: "object",
              properties: {
                start: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                },
                end: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                }
              },
              required: ["start", "end"]
            },
            selectionRange: {
              type: "object",
              properties: {
                start: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                },
                end: {
                  type: "object",
                  properties: {
                    line: { type: "number" },
                    character: { type: "number" }
                  },
                  required: ["line", "character"]
                }
              },
              required: ["start", "end"]
            }
          },
          required: ["name", "kind", "uri", "range", "selectionRange"]
        }
      },
      required: ["item"]
    }
  },
  {
    name: "get_selection_range",
    description: "Get smart selection ranges for positions in a file. Helps with expanding selections to meaningful code blocks.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        },
        positions: {
          type: "array",
          description: "Array of positions to get selection ranges for",
          items: {
            type: "object",
            properties: {
              line: {
                type: "number",
                description: "Line number (1-indexed)"
              },
              character: {
                type: "number",
                description: "Character position (0-indexed)"
              }
            },
            required: ["line", "character"]
          }
        }
      },
      required: ["file_path", "positions"]
    }
  }
];

// src/mcp/definitions/intelligence-tools.ts
var intelligenceToolDefinitions = [
  {
    name: "get_hover",
    description: "Get hover information (documentation, types, signatures) for a symbol at a specific position. Provides rich context about project-specific APIs and functions.",
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
          description: "The character position in the line (0-indexed)"
        }
      },
      required: ["file_path", "line", "character"]
    }
  },
  {
    name: "get_completions",
    description: "Get intelligent code completions for a specific position. Returns project-aware suggestions including imports, methods, properties, and context-specific completions.",
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
          description: "The character position in the line (0-indexed)"
        },
        trigger_character: {
          type: "string",
          description: 'Optional trigger character (e.g., ".", ":", ">") that caused the completion request'
        }
      },
      required: ["file_path", "line", "character"]
    }
  },
  {
    name: "get_inlay_hints",
    description: "Get inlay hints for a range of code. Shows parameter names, type annotations, and other helpful inline information.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        },
        start_line: {
          type: "number",
          description: "The start line number (1-indexed)"
        },
        start_character: {
          type: "number",
          description: "The start character position (0-indexed)"
        },
        end_line: {
          type: "number",
          description: "The end line number (1-indexed)"
        },
        end_character: {
          type: "number",
          description: "The end character position (0-indexed)"
        }
      },
      required: ["file_path", "start_line", "start_character", "end_line", "end_character"]
    }
  },
  {
    name: "get_semantic_tokens",
    description: "Get semantic token information for syntax highlighting and code understanding. Provides detailed token types and modifiers for enhanced code analysis.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file"
        }
      },
      required: ["file_path"]
    }
  },
  {
    name: "get_signature_help",
    description: "Get function signature help at a specific position. Shows function signatures, parameter information, and documentation for the function being called. Critical for AI agents when generating function calls with correct parameters.",
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
          description: "The character position in the line (0-indexed)"
        },
        trigger_character: {
          type: "string",
          description: 'Optional trigger character that invoked signature help (e.g., "(", ",")'
        }
      },
      required: ["file_path", "line", "character"]
    }
  }
];

// src/mcp/definitions/utility-tools.ts
var utilityToolDefinitions = [
  {
    name: "get_diagnostics",
    description: "Get language diagnostics (errors, warnings, hints) for a file. Uses LSP textDocument/diagnostic to pull current diagnostics.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file to get diagnostics for"
        }
      },
      required: ["file_path"]
    }
  },
  {
    name: "restart_server",
    description: "Manually restart LSP servers. Can restart servers for specific file extensions or all running servers.",
    inputSchema: {
      type: "object",
      properties: {
        extensions: {
          type: "array",
          items: { type: "string" },
          description: 'Array of file extensions to restart servers for (e.g., ["ts", "tsx"]). If not provided, all servers will be restarted.'
        }
      }
    }
  },
  {
    name: "rename_file",
    description: "Rename or move a file and automatically update all import statements that reference it. Works with TypeScript, JavaScript, JSX, and TSX files.",
    inputSchema: {
      type: "object",
      properties: {
        old_path: {
          type: "string",
          description: "Current path to the file"
        },
        new_path: {
          type: "string",
          description: "New path for the file (can be in a different directory)"
        },
        dry_run: {
          type: "boolean",
          description: "Preview changes without applying them (default: false)",
          default: false
        }
      },
      required: ["old_path", "new_path"]
    }
  },
  {
    name: "create_file",
    description: "Create a new file with optional content and notify relevant LSP servers. Ensures proper LSP workspace synchronization for newly created files.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path where the new file should be created"
        },
        content: {
          type: "string",
          description: "Initial content for the file (default: empty string)",
          default: ""
        },
        overwrite: {
          type: "boolean",
          description: "Whether to overwrite existing file if it exists (default: false)",
          default: false
        }
      },
      required: ["file_path"]
    }
  },
  {
    name: "delete_file",
    description: "Delete a file and notify relevant LSP servers. Ensures proper LSP workspace synchronization and cleanup for deleted files.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: {
          type: "string",
          description: "The path to the file to delete"
        },
        force: {
          type: "boolean",
          description: "Force deletion even if file has uncommitted changes (default: false)",
          default: false
        }
      },
      required: ["file_path"]
    }
  }
];

// src/mcp/definitions/index.ts
var allToolDefinitions = [
  ...coreToolDefinitions,
  ...advancedToolDefinitions,
  ...utilityToolDefinitions,
  ...intelligenceToolDefinitions,
  ...hierarchyToolDefinitions
];
export {
  allToolDefinitions
};
