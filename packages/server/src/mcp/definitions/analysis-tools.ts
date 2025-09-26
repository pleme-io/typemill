/**
 * Analysis tool definitions for MCP
 * Phase 3: Advanced code analysis features
 */

import type { Tool } from '@modelcontextprotocol/sdk/types.js';

export const ANALYSIS_TOOLS: Tool[] = [
  {
    name: 'find_dead_code',
    description: 'Find potentially dead (unused) code in the codebase using MCP tools',
    inputSchema: {
      type: 'object',
      properties: {
        files: {
          type: 'array',
          items: { type: 'string' },
          description: 'Specific files to analyze (optional, defaults to common source files)',
        },
        exclude_tests: {
          type: 'boolean',
          description: 'Whether to exclude test files from analysis (default: true)',
          default: true,
        },
        min_references: {
          type: 'number',
          description:
            'Minimum number of references required to not be considered dead (default: 1)',
          default: 1,
        },
      },
      additionalProperties: false,
    },
  },
  {
    name: 'fix_imports',
    description: 'Fix import paths in a file after it has been moved to a new location',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'Current path of the file with broken imports',
        },
        old_path: {
          type: 'string',
          description: 'Previous location of the file before it was moved',
        },
      },
      required: ['file_path', 'old_path'],
      additionalProperties: false,
    },
  },
  {
    name: 'analyze_imports',
    description: 'Analyze import relationships for a file or directory',
    inputSchema: {
      type: 'object',
      properties: {
        file_path: {
          type: 'string',
          description: 'Path to the file or directory to analyze',
        },
        include_importers: {
          type: 'boolean',
          description: 'Include files that import this file (default: true)',
          default: true,
        },
        include_imports: {
          type: 'boolean',
          description: 'Include files that this file imports (default: true)',
          default: true,
        },
      },
      required: ['file_path'],
      additionalProperties: false,
    },
  },
  {
    name: 'rename_directory',
    description: 'Rename a directory and update all import statements across the codebase',
    inputSchema: {
      type: 'object',
      properties: {
        old_path: {
          type: 'string',
          description: 'Current path of the directory',
        },
        new_path: {
          type: 'string',
          description: 'New path for the directory',
        },
        dry_run: {
          type: 'boolean',
          description: 'Preview changes without applying them (default: false)',
          default: false,
        },
      },
      required: ['old_path', 'new_path'],
      additionalProperties: false,
    },
  },
];
