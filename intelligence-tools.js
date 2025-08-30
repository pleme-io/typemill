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
export {
  intelligenceToolDefinitions
};
