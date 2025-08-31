// MCP Tool Definitions for Hierarchy and Navigation Features

export const hierarchyToolDefinitions = [
  {
    name: 'prepare_call_hierarchy',
    description:
      'Prepare call hierarchy for a symbol. Gets the call hierarchy item that can be used to explore incoming and outgoing calls.',
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
    name: 'get_call_hierarchy_incoming_calls',
    description:
      'Get all incoming calls to a function/method. Shows where this function is called from throughout the codebase. Can use either a prepared call hierarchy item or file position.',
    inputSchema: {
      type: 'object',
      properties: {
        item: {
          type: 'object',
          description: 'The call hierarchy item (from prepare_call_hierarchy)',
          properties: {
            name: { type: 'string' },
            kind: { type: 'number' },
            uri: { type: 'string' },
            range: {
              type: 'object',
              properties: {
                start: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
                end: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
              },
              required: ['start', 'end'],
            },
            selectionRange: {
              type: 'object',
              properties: {
                start: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
                end: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
              },
              required: ['start', 'end'],
            },
          },
          required: ['name', 'kind', 'uri', 'range', 'selectionRange'],
        },
        file_path: {
          type: 'string',
          description: 'The path to the file (alternative to item)',
        },
        line: {
          type: 'number',
          description: 'The line number (1-indexed, alternative to item)',
        },
        character: {
          type: 'number',
          description: 'The character position in the line (0-indexed, alternative to item)',
        },
      },
      oneOf: [{ required: ['item'] }, { required: ['file_path', 'line', 'character'] }],
    },
  },
  {
    name: 'get_call_hierarchy_outgoing_calls',
    description:
      'Get all outgoing calls from a function/method. Shows what functions this function calls. Can use either a prepared call hierarchy item or file position.',
    inputSchema: {
      type: 'object',
      properties: {
        item: {
          type: 'object',
          description: 'The call hierarchy item (from prepare_call_hierarchy)',
          properties: {
            name: { type: 'string' },
            kind: { type: 'number' },
            uri: { type: 'string' },
            range: {
              type: 'object',
              properties: {
                start: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
                end: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
              },
              required: ['start', 'end'],
            },
            selectionRange: {
              type: 'object',
              properties: {
                start: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
                end: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
              },
              required: ['start', 'end'],
            },
          },
          required: ['name', 'kind', 'uri', 'range', 'selectionRange'],
        },
        file_path: {
          type: 'string',
          description: 'The path to the file (alternative to item)',
        },
        line: {
          type: 'number',
          description: 'The line number (1-indexed, alternative to item)',
        },
        character: {
          type: 'number',
          description: 'The character position in the line (0-indexed, alternative to item)',
        },
      },
      oneOf: [{ required: ['item'] }, { required: ['file_path', 'line', 'character'] }],
    },
  },
  {
    name: 'prepare_type_hierarchy',
    description:
      'Prepare type hierarchy for a symbol. Gets the type hierarchy item for exploring inheritance relationships.',
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
    name: 'get_type_hierarchy_supertypes',
    description: 'Get supertypes (parent classes/interfaces) in the type hierarchy.',
    inputSchema: {
      type: 'object',
      properties: {
        item: {
          type: 'object',
          description: 'The type hierarchy item (from prepare_type_hierarchy)',
          properties: {
            name: { type: 'string' },
            kind: { type: 'number' },
            uri: { type: 'string' },
            range: {
              type: 'object',
              properties: {
                start: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
                end: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
              },
              required: ['start', 'end'],
            },
            selectionRange: {
              type: 'object',
              properties: {
                start: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
                end: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
              },
              required: ['start', 'end'],
            },
          },
          required: ['name', 'kind', 'uri', 'range', 'selectionRange'],
        },
      },
      required: ['item'],
    },
  },
  {
    name: 'get_type_hierarchy_subtypes',
    description: 'Get subtypes (child classes/implementations) in the type hierarchy.',
    inputSchema: {
      type: 'object',
      properties: {
        item: {
          type: 'object',
          description: 'The type hierarchy item (from prepare_type_hierarchy)',
          properties: {
            name: { type: 'string' },
            kind: { type: 'number' },
            uri: { type: 'string' },
            range: {
              type: 'object',
              properties: {
                start: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
                end: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
              },
              required: ['start', 'end'],
            },
            selectionRange: {
              type: 'object',
              properties: {
                start: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
                end: {
                  type: 'object',
                  properties: {
                    line: { type: 'number' },
                    character: { type: 'number' },
                  },
                  required: ['line', 'character'],
                },
              },
              required: ['start', 'end'],
            },
          },
          required: ['name', 'kind', 'uri', 'range', 'selectionRange'],
        },
      },
      required: ['item'],
    },
  },
  {
    name: 'get_selection_range',
    description:
      'Get smart selection ranges for positions in a file. Helps with expanding selections to meaningful code blocks.',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'The path to the file',
        },
        positions: {
          type: 'array',
          description: 'Array of positions to get selection ranges for',
          items: {
            type: 'object',
            properties: {
              line: {
                type: 'number',
                description: 'Line number (1-indexed)',
              },
              character: {
                type: 'number',
                description: 'Character position (0-indexed)',
              },
            },
            required: ['line', 'character'],
          },
        },
      },
      required: ['file_path', 'positions'],
    },
  },
];
