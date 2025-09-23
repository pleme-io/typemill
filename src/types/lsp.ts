/**
 * LSP (Language Server Protocol) type definitions
 * Extracted from src/types.ts during Phase 1 refactoring
 */

// Core LSP position and range types
export interface Position {
  line: number;
  character: number;
}

export interface Range {
  start: Position;
  end: Position;
}

export interface Location {
  uri: string;
  range: {
    start: Position;
    end: Position;
  };
}

export interface LSPLocation {
  uri: string;
  range: {
    start: Position;
    end: Position;
  };
}

export interface DefinitionResult {
  locations: Location[];
}

export interface ReferenceResult {
  locations: Location[];
}

export interface LSPError {
  code: number;
  message: string;
  data?: unknown;
}

export enum SymbolKind {
  File = 1,
  Module = 2,
  Namespace = 3,
  Package = 4,
  Class = 5,
  Method = 6,
  Property = 7,
  Field = 8,
  Constructor = 9,
  Enum = 10,
  Interface = 11,
  Function = 12,
  Variable = 13,
  Constant = 14,
  String = 15,
  Number = 16,
  Boolean = 17,
  Array = 18,
  Object = 19,
  Key = 20,
  Null = 21,
  EnumMember = 22,
  Struct = 23,
  Event = 24,
  Operator = 25,
  TypeParameter = 26,
}

export enum SymbolTag {
  Deprecated = 1,
}

export interface DocumentSymbol {
  name: string;
  detail?: string;
  kind: SymbolKind;
  tags?: SymbolTag[];
  deprecated?: boolean;
  range: {
    start: Position;
    end: Position;
  };
  selectionRange: {
    start: Position;
    end: Position;
  };
  children?: DocumentSymbol[];
}

export interface SymbolInformation {
  name: string;
  kind: SymbolKind;
  tags?: SymbolTag[];
  deprecated?: boolean;
  location: {
    uri: string;
    range: {
      start: Position;
      end: Position;
    };
  };
  containerName?: string;
}

export interface SymbolMatch {
  name: string;
  kind: SymbolKind;
  position: Position;
  range: {
    start: Position;
    end: Position;
  };
  detail?: string;
}

export interface SymbolSearchParams {
  file_path: string;
  symbol_name: string;
  symbol_kind: string;
}

export enum DiagnosticSeverity {
  Error = 1,
  Warning = 2,
  Information = 3,
  Hint = 4,
}

export interface DiagnosticRelatedInformation {
  location: Location;
  message: string;
}

export interface CodeDescription {
  href: string;
}

export enum DiagnosticTag {
  Unnecessary = 1,
  Deprecated = 2,
}

export interface Diagnostic {
  range: {
    start: Position;
    end: Position;
  };
  severity?: DiagnosticSeverity;
  code?: number | string;
  codeDescription?: CodeDescription;
  source?: string;
  message: string;
  tags?: DiagnosticTag[];
  relatedInformation?: DiagnosticRelatedInformation[];
  data?: unknown;
}

export interface DocumentDiagnosticReport {
  kind: 'full' | 'unchanged';
  resultId?: string;
  items?: Diagnostic[];
}

// Hierarchy types
export interface CallHierarchyItem {
  name: string;
  kind: SymbolKind;
  tags?: SymbolTag[];
  detail?: string;
  uri: string;
  range: {
    start: Position;
    end: Position;
  };
  selectionRange: {
    start: Position;
    end: Position;
  };
  data?: unknown;
}

export interface CallHierarchyIncomingCall {
  from: CallHierarchyItem;
  fromRanges: {
    start: Position;
    end: Position;
  }[];
}

export interface CallHierarchyOutgoingCall {
  to: CallHierarchyItem;
  fromRanges: {
    start: Position;
    end: Position;
  }[];
}

export interface TypeHierarchyItem {
  name: string;
  kind: SymbolKind;
  tags?: SymbolTag[];
  detail?: string;
  uri: string;
  range: {
    start: Position;
    end: Position;
  };
  selectionRange: {
    start: Position;
    end: Position;
  };
  data?: unknown;
}

export interface SelectionRange {
  range: {
    start: Position;
    end: Position;
  };
  parent?: SelectionRange;
}

// File editing and code action types
export interface FoldingRange {
  startLine: number;
  startCharacter?: number;
  endLine: number;
  endCharacter?: number;
  kind?: FoldingRangeKind;
  collapsedText?: string;
}

export enum FoldingRangeKind {
  Comment = 'comment',
  Imports = 'imports',
  Region = 'region',
}

export interface DocumentLink {
  range: {
    start: Position;
    end: Position;
  };
  target?: string;
  tooltip?: string;
  data?: unknown;
}

export interface CodeAction {
  title: string;
  kind?: string;
  diagnostics?: Diagnostic[];
  isPreferred?: boolean;
  disabled?: {
    reason: string;
  };
  edit?: WorkspaceEdit;
  command?: Command;
  data?: unknown;
}

export interface WorkspaceEdit {
  changes?: { [uri: string]: TextEdit[] };
  documentChanges?: (TextDocumentEdit | CreateFile | RenameFile | DeleteFile)[];
  changeAnnotations?: { [id: string]: ChangeAnnotation };
}

export interface TextDocumentEdit {
  textDocument: VersionedTextDocumentIdentifier;
  edits: TextEdit[];
}

export interface VersionedTextDocumentIdentifier {
  uri: string;
  version: number | null;
}

export interface CreateFile {
  kind: 'create';
  uri: string;
  options?: CreateFileOptions;
  annotationId?: string;
}

export interface CreateFileOptions {
  overwrite?: boolean;
  ignoreIfExists?: boolean;
}

export interface RenameFile {
  kind: 'rename';
  oldUri: string;
  newUri: string;
  options?: RenameFileOptions;
  annotationId?: string;
}

export interface RenameFileOptions {
  overwrite?: boolean;
  ignoreIfExists?: boolean;
}

export interface DeleteFile {
  kind: 'delete';
  uri: string;
  options?: DeleteFileOptions;
  annotationId?: string;
}

export interface DeleteFileOptions {
  recursive?: boolean;
  ignoreIfNotExists?: boolean;
}

export interface ChangeAnnotation {
  label: string;
  needsConfirmation?: boolean;
  description?: string;
}

export interface Command {
  title: string;
  command: string;
  arguments?: unknown[];
}

export interface TextEdit {
  range: {
    start: Position;
    end: Position;
  };
  newText: string;
}