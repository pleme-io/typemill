// Advanced MCP tool definitions for IDE features and code analysis

export const advancedToolDefinitions = [
  {
    name: 'get_code_actions',
    description:
      'Get available code actions (quick fixes, refactors, organize imports) for a file or specific range. Can apply auto-fixes like removing unused imports, adding missing imports, and organizing imports.',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file',
        },
        range: {
          type: 'object',
          description:
            'Optional range to get code actions for. If not provided, gets actions for entire file.',
          properties: {
            start: {
              type: 'object',
              properties: {
                line: { type: 'number', description: 'Start line (0-indexed)' },
                character: { type: 'number', description: 'Start character (0-indexed)' },
              },
              required: ['line', 'character'],
            },
            end: {
              type: 'object',
              properties: {
                line: { type: 'number', description: 'End line (0-indexed)' },
                character: { type: 'number', description: 'End character (0-indexed)' },
              },
              required: ['line', 'character'],
            },
          },
          required: ['start', 'end'],
        },
      },
      required: ['file_path'],
    },
  },
  {
    name: 'format_document',
    description:
      "Format a document using the language server's formatter. Applies consistent code style and formatting rules.",
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file to format',
        },
        options: {
          type: 'object',
          description: 'Formatting options',
          properties: {
            tab_size: { type: 'number', description: 'Size of tabs (default: 2)' },
            insert_spaces: {
              type: 'boolean',
              description: 'Use spaces instead of tabs (default: true)',
            },
            trim_trailing_whitespace: { type: 'boolean', description: 'Trim trailing whitespace' },
            insert_final_newline: { type: 'boolean', description: 'Insert final newline' },
            trim_final_newlines: { type: 'boolean', description: 'Trim final newlines' },
          },
        },
      },
      required: ['file_path'],
    },
  },
  {
    name: 'search_workspace_symbols',
    description:
      'Search for symbols (functions, classes, variables, etc.) across the entire workspace. Useful for finding symbols by name across multiple files.',
    inputSchema: {
      type: 'object',
      properties: {
        query: {
          type: 'string',
          description: 'Search query for symbol names (supports partial matching)',
        },
        workspace_path: {
          type: 'string',
          description:
            'Optional workspace path to search within (defaults to current working directory)',
        },
      },
      required: ['query'],
    },
  },
  {
    name: 'get_document_symbols',
    description:
      'Get all symbols (functions, classes, variables, etc.) defined in a specific file. Returns a hierarchical structure of symbols with their locations and types.',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file',
        },
      },
      required: ['file_path'],
    },
  },
  {
    name: 'get_folding_ranges',
    description:
      'Get folding ranges for code structure understanding. Shows logical code blocks that can be folded/collapsed (functions, classes, comments, imports). Helps AI agents understand code organization and nesting levels.',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file',
        },
      },
      required: ['file_path'],
    },
  },
  {
    name: 'get_document_links',
    description:
      'Get clickable links in a document (URLs, file references, imports, documentation links). Helps AI agents understand project relationships and external dependencies. Different language servers provide different types of links.',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file',
        },
      },
      required: ['file_path'],
    },
  },
  {
    name: 'apply_workspace_edit',
    description:
      'Apply a workspace edit (multi-file text changes) atomically. This is the most powerful editing tool for AI agents, allowing safe modification of multiple files in a single atomic operation with rollback capability. Essential for large refactoring operations.',
    inputSchema: {
      type: 'object',
      properties: {
        changes: {
          type: 'object',
          description: 'Map of file URIs/paths to arrays of text edits',
          additionalProperties: {
            type: 'array',
            items: {
              type: 'object',
              properties: {
                range: {
                  type: 'object',
                  properties: {
                    start: {
                      type: 'object',
                      properties: {
                        line: { type: 'number', description: 'Start line (0-indexed)' },
                        character: { type: 'number', description: 'Start character (0-indexed)' },
                      },
                      required: ['line', 'character'],
                    },
                    end: {
                      type: 'object',
                      properties: {
                        line: { type: 'number', description: 'End line (0-indexed)' },
                        character: { type: 'number', description: 'End character (0-indexed)' },
                      },
                      required: ['line', 'character'],
                    },
                  },
                  required: ['start', 'end'],
                },
                newText: {
                  type: 'string',
                  description: 'The new text to replace the range',
                },
              },
              required: ['range', 'newText'],
            },
          },
        },
        validate_before_apply: {
          type: 'boolean',
          description: 'Whether to validate edit positions before applying (default: true)',
          default: true,
        },
      },
      required: ['changes'],
    },
  },
] as const;
