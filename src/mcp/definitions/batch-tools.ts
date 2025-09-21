// Universal Batch MCP tool definition for executing multiple operations
// Replaces specific batch tools with a generalized batching system

export const batchToolDefinitions = [
  {
    name: 'batch_execute',
    description:
      'Execute multiple MCP operations in a single atomic or non-atomic transaction. Supports any combination of existing tools with options for parallel execution, dry-run previews, and rollback on failure. More powerful than individual batch tools like batch_move_files.',
    inputSchema: {
      type: 'object',
      properties: {
        operations: {
          type: 'array',
          description: 'Array of MCP tool operations to execute',
          items: {
            type: 'object',
            properties: {
              tool: {
                type: 'string',
                description:
                  'Name of the MCP tool to execute (e.g., "find_definition", "rename_file", "get_diagnostics")',
                enum: [
                  // Core tools
                  'find_definition',
                  'find_references',
                  'rename_symbol',
                  'rename_symbol_strict',

                  // Advanced tools
                  'get_code_actions',
                  'format_document',
                  'search_workspace_symbols',
                  'get_document_symbols',
                  'get_folding_ranges',
                  'get_document_links',
                  'apply_workspace_edit',

                  // Intelligence tools
                  'get_hover',
                  'get_completions',
                  'get_inlay_hints',
                  'get_semantic_tokens',
                  'get_signature_help',

                  // Hierarchy tools
                  'prepare_call_hierarchy',
                  'get_call_hierarchy_incoming_calls',
                  'get_call_hierarchy_outgoing_calls',
                  'prepare_type_hierarchy',
                  'get_type_hierarchy_supertypes',
                  'get_type_hierarchy_subtypes',
                  'get_selection_range',

                  // Utility tools
                  'get_diagnostics',
                  'restart_server',
                  'rename_file',
                  'create_file',
                  'delete_file',
                  'health_check',
                ],
              },
              args: {
                type: 'object',
                description: 'Arguments for the specific tool (same as individual tool arguments)',
                additionalProperties: true,
              },
              id: {
                type: 'string',
                description:
                  'Optional identifier for correlating results (useful for parallel execution)',
                maxLength: 100,
              },
            },
            required: ['tool', 'args'],
            additionalProperties: false,
          },
          minItems: 1,
          maxItems: 50,
        },
        options: {
          type: 'object',
          description: 'Execution options for the batch operation',
          properties: {
            atomic: {
              type: 'boolean',
              description:
                'If true, all operations succeed or all fail with automatic rollback (default: false)',
              default: false,
            },
            parallel: {
              type: 'boolean',
              description:
                'If true, execute operations in parallel for better performance (default: false for sequential)',
              default: false,
            },
            dry_run: {
              type: 'boolean',
              description:
                'If true, preview what would happen without executing any operations (default: false)',
              default: false,
            },
            stop_on_error: {
              type: 'boolean',
              description: 'If true, stop execution when first error occurs (default: true)',
              default: true,
            },
          },
          additionalProperties: false,
          default: {},
        },
      },
      required: ['operations'],
      additionalProperties: false,
    },
  },
] as const;
