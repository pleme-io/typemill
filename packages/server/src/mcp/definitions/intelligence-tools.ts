// MCP Tool Definitions for LLM Agent Intelligence Features

export const intelligenceToolDefinitions = [
  {
    name: 'get_hover',
    description:
      'Get hover information (documentation, types, signatures) for a symbol at a specific position. Provides rich context about project-specific APIs and functions.',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file',
        },
        line: {
          type: 'number',
          description: 'The line number (1-indexed)',
        },
        character: {
          type: 'number',
          description: 'The character position in the line (0-indexed)',
        },
      },
      required: ['file_path', 'line', 'character'],
    },
  },
  {
    name: 'get_completions',
    description:
      'Get intelligent code completions for a specific position. Returns project-aware suggestions including imports, methods, properties, and context-specific completions.',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file',
        },
        line: {
          type: 'number',
          description: 'The line number (1-indexed)',
        },
        character: {
          type: 'number',
          description: 'The character position in the line (0-indexed)',
        },
        trigger_character: {
          type: 'string',
          description:
            'Optional trigger character (e.g., ".", ":", ">") that caused the completion request',
        },
      },
      required: ['file_path', 'line', 'character'],
    },
  },
  {
    name: 'get_signature_help',
    description:
      'Get function signature help at a specific position. Shows function signatures, parameter information, and documentation for the function being called. Critical for AI agents when generating function calls with correct parameters.',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file',
        },
        line: {
          type: 'number',
          description: 'The line number (1-indexed)',
        },
        character: {
          type: 'number',
          description: 'The character position in the line (0-indexed)',
        },
        trigger_character: {
          type: 'string',
          description: 'Optional trigger character that invoked signature help (e.g., "(", ",")',
        },
      },
      required: ['file_path', 'line', 'character'],
    },
  },
];
