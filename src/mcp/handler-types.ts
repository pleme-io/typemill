// TypeScript interfaces for MCP handler arguments
// This file defines all the argument types used by the MCP handlers
// to replace 'args as any' casts with proper type safety

import type { CallHierarchyItem, TypeHierarchyItem } from '../types.js';

// Core handlers
export interface FindDefinitionArgs {
  file_path: string;
  symbol_name: string;
  symbol_kind?: string;
}

export interface FindReferencesArgs {
  file_path: string;
  symbol_name: string;
  symbol_kind?: string;
  include_declaration?: boolean;
}

export interface RenameSymbolArgs {
  file_path: string;
  symbol_name: string;
  symbol_kind?: string;
  new_name: string;
  dry_run?: boolean;
}

export interface RenameSymbolStrictArgs {
  file_path: string;
  line: number;
  character: number;
  new_name: string;
  dry_run?: boolean;
}

// Advanced handlers
export interface GetCodeActionsArgs {
  file_path: string;
  range?: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
}

export interface FormatDocumentArgs {
  file_path: string;
  options?: {
    tab_size?: number;
    insert_spaces?: boolean;
    trim_trailing_whitespace?: boolean;
    insert_final_newline?: boolean;
    trim_final_newlines?: boolean;
  };
}

export interface SearchWorkspaceSymbolsArgs {
  query: string;
}

export interface GetDocumentSymbolsArgs {
  file_path: string;
}

export interface GetFoldingRangesArgs {
  file_path: string;
}

export interface GetDocumentLinksArgs {
  file_path: string;
}

export interface ApplyWorkspaceEditArgs {
  changes: Record<
    string,
    Array<{
      range: {
        start: { line: number; character: number };
        end: { line: number; character: number };
      };
      newText: string;
    }>
  >;
  validate_before_apply?: boolean;
}

// Intelligence handlers
export interface GetHoverArgs {
  file_path: string;
  line: number;
  character: number;
}

export interface GetCompletionsArgs {
  file_path: string;
  line: number;
  character: number;
  trigger_character?: string;
}

export interface GetInlayHintsArgs {
  file_path: string;
  start_line: number;
  start_character: number;
  end_line: number;
  end_character: number;
}

export interface GetSemanticTokensArgs {
  file_path: string;
}

export interface GetSignatureHelpArgs {
  file_path: string;
  line: number;
  character: number;
  trigger_character?: string;
}

// Hierarchy handlers
export interface PrepareCallHierarchyArgs {
  file_path: string;
  line: number;
  character: number;
}

export interface GetCallHierarchyIncomingCallsArgs {
  item: CallHierarchyItem;
}

export interface GetCallHierarchyOutgoingCallsArgs {
  item: CallHierarchyItem;
}

export interface PrepareTypeHierarchyArgs {
  file_path: string;
  line: number;
  character: number;
}

export interface GetTypeHierarchySupertypesArgs {
  item: TypeHierarchyItem;
}

export interface GetTypeHierarchySubtypesArgs {
  item: TypeHierarchyItem;
}

export interface GetSelectionRangeArgs {
  file_path: string;
  positions: Array<{ line: number; character: number }>;
}

// Utility handlers
export interface GetDiagnosticsArgs {
  file_path: string;
}

export interface RestartServerArgs {
  extensions?: string[];
}

export interface RenameFileArgs {
  old_path: string;
  new_path: string;
  dry_run?: boolean;
}

export interface CreateFileArgs {
  file_path: string;
  content?: string;
  overwrite?: boolean;
}

export interface DeleteFileArgs {
  file_path: string;
  force?: boolean;
}

export interface HealthCheckArgs {
  include_details?: boolean;
}

// Universal Batch handler
export interface BatchExecuteArgs {
  operations: Array<{
    tool: string;
    args: unknown;
    id?: string;
  }>;
  options: {
    atomic?: boolean;
    parallel?: boolean;
    dry_run?: boolean;
    stop_on_error?: boolean;
  };
}
