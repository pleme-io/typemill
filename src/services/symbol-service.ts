import { readFileSync } from 'node:fs';
import { capabilityManager } from '../capability-manager.js';
import * as CoreMethods from '../lsp-methods/core-methods.js';
import * as DocumentMethods from '../lsp-methods/document-methods.js';
import * as WorkspaceMethods from '../lsp-methods/workspace-methods.js';
import type { DocumentMethodsContext, ServerState } from '../lsp-types.js';
import type { LSPProtocol } from '../lsp/protocol.js';
import type {
  DocumentSymbol,
  Location,
  Position,
  SymbolInformation,
  SymbolMatch,
} from '../types.js';
import { SymbolKind } from '../types.js';

/**
 * Service for symbol-related LSP operations
 * Handles definition, references, renaming, and symbol search
 */
export class SymbolService {
  constructor(
    private getServer: (filePath: string) => Promise<ServerState>,
    private protocol: LSPProtocol
  ) {}

  /**
   * Find definition of symbol at position
   */
  async findDefinition(filePath: string, position: Position): Promise<Location[]> {
    const context: CoreMethods.CoreMethodsContext = {
      getServer: this.getServer,
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (process, method, params, timeout) =>
        this.protocol.sendRequest(process, method, params, timeout),
    };
    return CoreMethods.findDefinition(context, filePath, position);
  }

  /**
   * Find all references to symbol at position
   */
  async findReferences(
    filePath: string,
    position: Position,
    includeDeclaration = false
  ): Promise<Location[]> {
    const context: CoreMethods.CoreMethodsContext = {
      getServer: this.getServer,
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (process, method, params, timeout) =>
        this.protocol.sendRequest(process, method, params, timeout),
    };
    return CoreMethods.findReferences(context, filePath, position, includeDeclaration);
  }

  /**
   * Rename symbol at position
   */
  async renameSymbol(
    filePath: string,
    position: Position,
    newName: string
  ): Promise<{
    changes?: Record<string, Array<{ range: { start: Position; end: Position }; newText: string }>>;
  }> {
    const context: CoreMethods.CoreMethodsContext = {
      getServer: this.getServer,
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (process, method, params, timeout) =>
        this.protocol.sendRequest(process, method, params, timeout),
    };
    return CoreMethods.renameSymbol(context, filePath, position, newName);
  }

  /**
   * Search for symbols in workspace
   */
  async searchWorkspaceSymbols(query: string): Promise<SymbolInformation[]> {
    const context: WorkspaceMethods.WorkspaceMethodsContext = {
      getServer: this.getServer,
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (process, method, params, timeout) =>
        this.protocol.sendRequest(process, method, params, timeout),
      sendNotification: (process, method, params) =>
        this.protocol.sendNotification(process, method, params),
      preloadServers: async () => {},
      servers: new Map(),
    };
    return WorkspaceMethods.searchWorkspaceSymbols(context, query);
  }

  /**
   * Get document symbols
   */
  async getDocumentSymbols(filePath: string): Promise<DocumentSymbol[] | SymbolInformation[]> {
    const context: DocumentMethodsContext = {
      getServer: this.getServer,
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (process, method, params, timeout) =>
        this.protocol.sendRequest(process, method, params, timeout),
      sendNotification: (process, method, params) =>
        this.protocol.sendNotification(process, method, params),
      capabilityManager, // Properly injected
    };
    return DocumentMethods.getDocumentSymbols(context, filePath);
  }

  /**
   * Find symbol matches by name and kind
   */
  async findSymbolMatches(
    filePath: string,
    symbolName: string,
    symbolKind?: string
  ): Promise<SymbolMatch[]> {
    try {
      const symbols = await this.getDocumentSymbols(filePath);
      const matches: SymbolMatch[] = [];

      if (this.isDocumentSymbolArray(symbols)) {
        // Handle DocumentSymbol[] format
        const flatSymbols = this.flattenDocumentSymbols(symbols);
        for (const symbol of flatSymbols) {
          if (symbol.name === symbolName) {
            if (!symbolKind || this.symbolKindToString(symbol.kind) === symbolKind.toLowerCase()) {
              matches.push({
                name: symbol.name,
                kind: symbol.kind,
                position: symbol.selectionRange.start,
                range: symbol.range,
                detail: symbol.detail,
              });
            }
          }
        }
      } else {
        // Handle SymbolInformation[] format
        for (const symbol of symbols) {
          if (symbol.name === symbolName) {
            if (!symbolKind || this.symbolKindToString(symbol.kind) === symbolKind.toLowerCase()) {
              const position = await this.findSymbolPositionInFile(filePath, symbol);
              matches.push({
                name: symbol.name,
                kind: symbol.kind,
                position,
                range: symbol.location.range,
                detail: undefined,
              });
            }
          }
        }
      }

      return matches;
    } catch (error) {
      process.stderr.write(`[ERROR findSymbolMatches] ${error}\n`);
      return [];
    }
  }

  // Utility methods from DocumentMethods
  flattenDocumentSymbols = DocumentMethods.flattenDocumentSymbols;
  isDocumentSymbolArray = DocumentMethods.isDocumentSymbolArray;
  symbolKindToString = DocumentMethods.symbolKindToString;
  getValidSymbolKinds = DocumentMethods.getValidSymbolKinds;

  /**
   * Find precise position of symbol in file
   */
  private async findSymbolPositionInFile(
    filePath: string,
    symbol: SymbolInformation
  ): Promise<Position> {
    try {
      const fileContent = readFileSync(filePath, 'utf-8');
      const lines = fileContent.split('\n');

      const range = symbol.location.range;
      const startLine = range.start.line;
      const endLine = range.end.line;

      // Search within the symbol's range for the symbol name
      for (let lineNum = startLine; lineNum <= endLine && lineNum < lines.length; lineNum++) {
        const line = lines[lineNum];
        if (!line) continue;

        let searchStart = 0;
        if (lineNum === startLine) {
          searchStart = range.start.character;
        }

        let searchEnd = line.length;
        if (lineNum === endLine) {
          searchEnd = range.end.character;
        }

        const searchText = line.substring(searchStart, searchEnd);
        const symbolIndex = searchText.indexOf(symbol.name);

        if (symbolIndex !== -1) {
          const actualCharacter = searchStart + symbolIndex;
          return {
            line: lineNum,
            character: actualCharacter,
          };
        }
      }

      return range.start;
    } catch (error) {
      return symbol.location.range.start;
    }
  }

  private stringToSymbolKind(kindStr: string): SymbolKind | null {
    const kindMap: Record<string, SymbolKind> = {
      file: SymbolKind.File,
      module: SymbolKind.Module,
      namespace: SymbolKind.Namespace,
      package: SymbolKind.Package,
      class: SymbolKind.Class,
      method: SymbolKind.Method,
      property: SymbolKind.Property,
      field: SymbolKind.Field,
      constructor: SymbolKind.Constructor,
      enum: SymbolKind.Enum,
      interface: SymbolKind.Interface,
      function: SymbolKind.Function,
      variable: SymbolKind.Variable,
      constant: SymbolKind.Constant,
      string: SymbolKind.String,
      number: SymbolKind.Number,
      boolean: SymbolKind.Boolean,
      array: SymbolKind.Array,
    };
    return kindMap[kindStr.toLowerCase()] || null;
  }

  /**
   * Ensure file is open in LSP server
   */
  private async ensureFileOpen(serverState: ServerState, filePath: string): Promise<void> {
    if (serverState.openFiles.has(filePath)) {
      return;
    }

    const fileContent = readFileSync(filePath, 'utf-8');

    this.protocol.sendNotification(serverState.process, 'textDocument/didOpen', {
      textDocument: {
        uri: `file://${filePath}`,
        languageId: this.getLanguageId(filePath),
        version: 1,
        text: fileContent,
      },
    });

    serverState.openFiles.add(filePath);
  }

  private getLanguageId(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase();
    const languageMap: Record<string, string> = {
      ts: 'typescript',
      tsx: 'typescriptreact',
      js: 'javascript',
      jsx: 'javascriptreact',
      py: 'python',
      go: 'go',
      rs: 'rust',
      java: 'java',
      cpp: 'cpp',
      c: 'c',
      h: 'c',
      hpp: 'cpp',
    };
    return languageMap[ext || ''] || 'plaintext';
  }
}
