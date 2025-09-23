/**
 * Workflow Definitions - Pre-built workflows that chain multiple MCP tools
 * These workflows demonstrate the power of tool dependency management
 */

import type { WorkflowToolDefinition } from '../handler-types.js';

/**
 * Advanced Dead Code Analysis Workflow
 *
 * This workflow automates the process of finding potentially unused code by:
 * 1. Getting all symbols from specified files
 * 2. For each exported symbol, finding its references
 * 3. Identifying symbols with no external references
 * 4. Generating a comprehensive report
 *
 * This replaces the single find_dead_code tool with a more flexible,
 * multi-step approach that can be easily customized and extended.
 */
export const deadCodeAnalysisWorkflow: WorkflowToolDefinition = {
  name: 'workflow_dead_code_analysis',
  type: 'workflow',
  description: 'Comprehensive dead code analysis using multi-step symbol and reference checking',
  inputSchema: {
    type: 'object',
    properties: {
      files: {
        type: 'array',
        items: { type: 'string' },
        description: 'List of files to analyze for dead code',
      },
      exclude_tests: {
        type: 'boolean',
        description: 'Whether to exclude test files from analysis',
        default: true,
      },
      min_references: {
        type: 'number',
        description: 'Minimum number of references required to consider code as "live"',
        default: 1,
      },
    },
    required: ['files'],
  },
  steps: [
    {
      id: 'get_symbols',
      tool: 'get_document_symbols',
      description: 'Extract all symbols from the first file',
      args: {
        file_path: '{{input.files.0}}',
      },
    },
    {
      id: 'find_symbol_references',
      tool: 'find_references',
      description: 'Find references for the first exported symbol',
      args: {
        file_path: '{{input.files.0}}',
        symbol_name: '{{get_symbols.symbols.0.name}}',
        include_declaration: false,
      },
    },
  ],
};

/**
 * Refactoring Assistant Workflow
 *
 * This workflow helps with safe code refactoring by:
 * 1. Finding the definition of a symbol
 * 2. Finding all references to that symbol
 * 3. Getting hover information for context
 * 4. Preparing for safe renaming
 */
export const refactoringAssistantWorkflow: WorkflowToolDefinition = {
  name: 'workflow_refactoring_assistant',
  type: 'workflow',
  description:
    'Comprehensive analysis before refactoring a symbol, including definition, references, and context',
  inputSchema: {
    type: 'object',
    properties: {
      file_path: {
        type: 'string',
        description: 'Path to the file containing the symbol',
      },
      symbol_name: {
        type: 'string',
        description: 'Name of the symbol to analyze',
      },
      line: {
        type: 'number',
        description: 'Line number where the symbol is located',
      },
      character: {
        type: 'number',
        description: 'Character position where the symbol is located',
      },
    },
    required: ['file_path', 'symbol_name', 'line', 'character'],
  },
  steps: [
    {
      id: 'find_definition',
      tool: 'find_definition',
      description: 'Locate the definition of the symbol',
      args: {
        file_path: '{{input.file_path}}',
        symbol_name: '{{input.symbol_name}}',
      },
    },
    {
      id: 'find_all_references',
      tool: 'find_references',
      description: 'Find all references to this symbol',
      args: {
        file_path: '{{input.file_path}}',
        symbol_name: '{{input.symbol_name}}',
        include_declaration: true,
      },
    },
    {
      id: 'get_context',
      tool: 'get_hover',
      description: 'Get detailed type and documentation information',
      args: {
        file_path: '{{input.file_path}}',
        line: '{{input.line}}',
        character: '{{input.character}}',
      },
    },
    {
      id: 'get_document_symbols',
      tool: 'get_document_symbols',
      description: 'Get all symbols in the file for context',
      args: {
        file_path: '{{input.file_path}}',
      },
    },
  ],
};

/**
 * Code Quality Assessment Workflow
 *
 * This workflow provides a comprehensive code quality assessment by:
 * 1. Getting all symbols in a file
 * 2. Running diagnostics to find errors and warnings
 * 3. Analyzing code actions for potential improvements
 * 4. Getting folding ranges to understand code structure
 */
export const codeQualityWorkflow: WorkflowToolDefinition = {
  name: 'workflow_code_quality',
  type: 'workflow',
  description:
    'Comprehensive code quality assessment including symbols, diagnostics, and improvement suggestions',
  inputSchema: {
    type: 'object',
    properties: {
      file_path: {
        type: 'string',
        description: 'Path to the file to analyze',
      },
    },
    required: ['file_path'],
  },
  steps: [
    {
      id: 'get_symbols',
      tool: 'get_document_symbols',
      description: 'Get all symbols to understand code structure',
      args: {
        file_path: '{{input.file_path}}',
      },
    },
    {
      id: 'get_diagnostics',
      tool: 'get_diagnostics',
      description: 'Find errors, warnings, and hints',
      args: {
        file_path: '{{input.file_path}}',
      },
    },
    {
      id: 'get_code_actions',
      tool: 'get_code_actions',
      description: 'Get available code improvements and quick fixes',
      args: {
        file_path: '{{input.file_path}}',
      },
    },
    {
      id: 'get_folding_ranges',
      tool: 'get_folding_ranges',
      description: 'Understand code structure and nesting',
      args: {
        file_path: '{{input.file_path}}',
      },
    },
  ],
};

/**
 * Symbol Intelligence Workflow
 *
 * This workflow provides deep intelligence about a specific symbol by:
 * 1. Finding its definition
 * 2. Getting hover information
 * 3. Finding all references
 * 4. Getting signature help if it's a function
 * 5. Preparing call hierarchy for navigation
 */
export const symbolIntelligenceWorkflow: WorkflowToolDefinition = {
  name: 'workflow_symbol_intelligence',
  type: 'workflow',
  description:
    'Deep analysis of a symbol including definition, references, hover info, and call hierarchy',
  inputSchema: {
    type: 'object',
    properties: {
      file_path: {
        type: 'string',
        description: 'Path to the file containing the symbol',
      },
      symbol_name: {
        type: 'string',
        description: 'Name of the symbol to analyze',
      },
      line: {
        type: 'number',
        description: 'Line number where the symbol is located',
      },
      character: {
        type: 'number',
        description: 'Character position where the symbol is located',
      },
    },
    required: ['file_path', 'symbol_name', 'line', 'character'],
  },
  steps: [
    {
      id: 'find_definition',
      tool: 'find_definition',
      description: 'Locate the symbol definition',
      args: {
        file_path: '{{input.file_path}}',
        symbol_name: '{{input.symbol_name}}',
      },
    },
    {
      id: 'get_hover_info',
      tool: 'get_hover',
      description: 'Get type information and documentation',
      args: {
        file_path: '{{input.file_path}}',
        line: '{{input.line}}',
        character: '{{input.character}}',
      },
    },
    {
      id: 'find_references',
      tool: 'find_references',
      description: 'Find all usages of this symbol',
      args: {
        file_path: '{{input.file_path}}',
        symbol_name: '{{input.symbol_name}}',
        include_declaration: true,
      },
    },
    {
      id: 'get_signature_help',
      tool: 'get_signature_help',
      description: 'Get function signature if applicable',
      args: {
        file_path: '{{input.file_path}}',
        line: '{{input.line}}',
        character: '{{input.character}}',
      },
    },
    {
      id: 'prepare_call_hierarchy',
      tool: 'prepare_call_hierarchy',
      description: 'Prepare for call hierarchy navigation',
      args: {
        file_path: '{{input.file_path}}',
        line: '{{input.line}}',
        character: '{{input.character}}',
      },
    },
  ],
};

/**
 * Export all workflow definitions
 */
export const allWorkflowDefinitions: WorkflowToolDefinition[] = [
  deadCodeAnalysisWorkflow,
  refactoringAssistantWorkflow,
  codeQualityWorkflow,
  symbolIntelligenceWorkflow,
];
