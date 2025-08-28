export interface LSPServerConfig {
  extensions: string[];
  command: string[];
  rootDir?: string;
  restartInterval?: number; // in minutes, optional auto-restart interval
  initializationOptions?: unknown; // LSP initialization options
}

export interface Config {
  servers: LSPServerConfig[];
}

export interface Position {
  line: number;
  character: number;
}

export interface Location {
  uri: string;
  range: {
    start: Position;
    end: Position;
  };
}

interface DefinitionResult {
  locations: Location[];
}

interface ReferenceResult {
  locations: Location[];
}

interface SymbolSearchParams {
  file_path: string;
  symbol_name: string;
  symbol_kind: string;
}

export interface LSPError {
  code: number;
  message: string;
  data?: unknown;
}

export interface LSPLocation {
  uri: string;
  range: {
    start: Position;
    end: Position;
  };
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

enum SymbolTag {
  Deprecated = 1,
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

enum DiagnosticSeverity {
  Error = 1,
  Warning = 2,
  Information = 3,
  Hint = 4,
}

interface DiagnosticRelatedInformation {
  location: Location;
  message: string;
}

interface CodeDescription {
  href: string;
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

enum DiagnosticTag {
  Unnecessary = 1,
  Deprecated = 2,
}

export interface DocumentDiagnosticReport {
  kind: 'full' | 'unchanged';
  resultId?: string;
  items?: Diagnostic[];
}

// New types for LLM agent intelligence features

export interface Hover {
  contents: MarkupContent | MarkedString | MarkedString[];
  range?: {
    start: Position;
    end: Position;
  };
}

interface MarkupContent {
  kind: 'plaintext' | 'markdown';
  value: string;
}

interface MarkedString {
  language: string;
  value: string;
}

export interface CompletionItem {
  label: string;
  labelDetails?: CompletionItemLabelDetails;
  kind?: CompletionItemKind;
  tags?: CompletionItemTag[];
  detail?: string;
  documentation?: string | MarkupContent;
  deprecated?: boolean;
  preselect?: boolean;
  sortText?: string;
  filterText?: string;
  insertText?: string;
  insertTextFormat?: InsertTextFormat;
  insertTextMode?: InsertTextMode;
  textEdit?: TextEdit;
  additionalTextEdits?: TextEdit[];
  commitCharacters?: string[];
  command?: Command;
  data?: unknown;
}

interface CompletionItemLabelDetails {
  detail?: string;
  description?: string;
}

enum CompletionItemKind {
  Text = 1,
  Method = 2,
  Function = 3,
  Constructor = 4,
  Field = 5,
  Variable = 6,
  Class = 7,
  Interface = 8,
  Module = 9,
  Property = 10,
  Unit = 11,
  Value = 12,
  Enum = 13,
  Keyword = 14,
  Snippet = 15,
  Color = 16,
  File = 17,
  Reference = 18,
  Folder = 19,
  EnumMember = 20,
  Constant = 21,
  Struct = 22,
  Event = 23,
  Operator = 24,
  TypeParameter = 25,
}

enum CompletionItemTag {
  Deprecated = 1,
}

enum InsertTextFormat {
  PlainText = 1,
  Snippet = 2,
}

enum InsertTextMode {
  AsIs = 1,
  AdjustIndentation = 2,
}

export interface TextEdit {
  range: {
    start: Position;
    end: Position;
  };
  newText: string;
}

interface Command {
  title: string;
  command: string;
  arguments?: unknown[];
}

export interface InlayHint {
  position: Position;
  label: string | InlayHintLabelPart[];
  kind?: InlayHintKind;
  textEdits?: TextEdit[];
  tooltip?: string | MarkupContent;
  paddingLeft?: boolean;
  paddingRight?: boolean;
  data?: unknown;
}

interface InlayHintLabelPart {
  value: string;
  tooltip?: string | MarkupContent;
  location?: Location;
  command?: Command;
}

enum InlayHintKind {
  Type = 1,
  Parameter = 2,
}

export interface InlayHintParams {
  textDocument: {
    uri: string;
  };
  range: {
    start: Position;
    end: Position;
  };
}

export interface SemanticTokens {
  resultId?: string;
  data: number[];
}

export interface SemanticTokensParams {
  textDocument: {
    uri: string;
  };
}

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

export interface SignatureHelp {
  signatures: SignatureInformation[];
  activeSignature?: number;
  activeParameter?: number;
}

interface SignatureInformation {
  label: string;
  documentation?: string | MarkupContent;
  parameters?: ParameterInformation[];
  activeParameter?: number;
}

interface ParameterInformation {
  label: string | [number, number];
  documentation?: string | MarkupContent;
}

export interface FoldingRange {
  startLine: number;
  startCharacter?: number;
  endLine: number;
  endCharacter?: number;
  kind?: FoldingRangeKind;
  collapsedText?: string;
}

enum FoldingRangeKind {
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

interface TextDocumentEdit {
  textDocument: VersionedTextDocumentIdentifier;
  edits: TextEdit[];
}

interface VersionedTextDocumentIdentifier {
  uri: string;
  version: number;
}

interface CreateFile {
  kind: 'create';
  uri: string;
  options?: {
    overwrite?: boolean;
    ignoreIfExists?: boolean;
  };
  annotationId?: string;
}

interface RenameFile {
  kind: 'rename';
  oldUri: string;
  newUri: string;
  options?: {
    overwrite?: boolean;
    ignoreIfExists?: boolean;
  };
  annotationId?: string;
}

interface DeleteFile {
  kind: 'delete';
  uri: string;
  options?: {
    recursive?: boolean;
    ignoreIfNotExists?: boolean;
  };
  annotationId?: string;
}

interface ChangeAnnotation {
  label: string;
  needsConfirmation?: boolean;
  description?: string;
}
