import { readFileSync } from 'node:fs';
import { capabilityManager } from '../capability-manager.js';
import type { ServerState } from '../lsp-types.js';
import type { LSPProtocol } from '../lsp/protocol.js';
import { pathToUri } from '../path-utils.js';
import type {
  DocumentSymbol,
  LSPLocation,
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
    process.stderr.write(
      `[DEBUG findDefinition] Requesting definition for ${filePath} at ${position.line}:${position.character}\n`
    );

    const serverState = await this.getServer(filePath);

    // Wait for the server to be fully initialized
    await serverState.initializationPromise;

    // Ensure the file is opened and synced with the LSP server
    await this.ensureFileOpen(serverState, filePath);

    process.stderr.write('[DEBUG findDefinition] Sending textDocument/definition request\n');
    const result = await this.protocol.sendRequest(serverState.process, 'textDocument/definition', {
      textDocument: { uri: pathToUri(filePath) },
      position,
    });

    process.stderr.write(
      `[DEBUG findDefinition] Result type: ${typeof result}, isArray: ${Array.isArray(result)}\n`
    );

    if (Array.isArray(result)) {
      process.stderr.write(`[DEBUG findDefinition] Array result with ${result.length} locations\n`);
      if (result.length > 0) {
        process.stderr.write(
          `[DEBUG findDefinition] First location: ${JSON.stringify(result[0], null, 2)}\n`
        );
      }
      return result.map((loc: LSPLocation) => ({
        uri: loc.uri,
        range: loc.range,
      }));
    }
    if (result && typeof result === 'object' && 'uri' in result) {
      process.stderr.write(
        `[DEBUG findDefinition] Single location result: ${JSON.stringify(result, null, 2)}\n`
      );
      const location = result as LSPLocation;
      return [
        {
          uri: location.uri,
          range: location.range,
        },
      ];
    }

    process.stderr.write(
      '[DEBUG findDefinition] No definition found or unexpected result format\n'
    );
    return [];
  }

  /**
   * Find all references to symbol at position
   */
  async findReferences(
    filePath: string,
    position: Position,
    includeDeclaration = false
  ): Promise<Location[]> {
    const serverState = await this.getServer(filePath);

    // Wait for the server to be fully initialized
    await serverState.initializationPromise;

    // Ensure the file is opened and synced with the LSP server
    await this.ensureFileOpen(serverState, filePath);

    process.stderr.write(
      `[DEBUG] findReferences for ${filePath} at ${position.line}:${position.character}, includeDeclaration: ${includeDeclaration}\n`
    );

    const result = await this.protocol.sendRequest(serverState.process, 'textDocument/references', {
      textDocument: { uri: pathToUri(filePath) },
      position,
      context: { includeDeclaration },
    });

    process.stderr.write(
      `[DEBUG] findReferences result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : 'N/A'}\n`
    );

    if (result && Array.isArray(result) && result.length > 0) {
      process.stderr.write(`[DEBUG] First reference: ${JSON.stringify(result[0], null, 2)}\n`);
    } else if (result === null || result === undefined) {
      process.stderr.write('[DEBUG] findReferences returned null/undefined\n');
    } else {
      process.stderr.write(
        `[DEBUG] findReferences returned unexpected result: ${JSON.stringify(result)}\n`
      );
    }

    if (Array.isArray(result)) {
      return result.map((loc: LSPLocation) => ({
        uri: loc.uri,
        range: loc.range,
      }));
    }

    return [];
  }

  /**
   * Rename symbol at position
   */
  async renameSymbol(
    filePath: string,
    position: Position,
    newName: string,
    dryRun = false
  ): Promise<{
    changes?: Record<string, Array<{ range: { start: Position; end: Position }; newText: string }>>;
  }> {
    process.stderr.write(
      `[DEBUG renameSymbol] Requesting rename for ${filePath} at ${position.line}:${position.character} to "${newName}", dryRun: ${dryRun}\n`
    );

    // CRITICAL FIX: For dry_run operations, do NOT send textDocument/rename to LSP server
    // The TypeScript Language Server auto-applies rename changes to files, ignoring our dry_run intent
    if (dryRun) {
      process.stderr.write(
        '[DEBUG renameSymbol] Skipping LSP rename request for dry_run operation\n'
      );
      return {
        changes: {
          [`file://${filePath}`]: [
            {
              range: { start: position, end: position },
              newText: '[DRY_RUN_PLACEHOLDER]',
            },
          ],
        },
      };
    }

    const serverState = await this.getServer(filePath);

    // Wait for the server to be fully initialized
    await serverState.initializationPromise;

    // Ensure the file is opened and synced with the LSP server
    await this.ensureFileOpen(serverState, filePath);

    process.stderr.write('[DEBUG renameSymbol] Sending textDocument/rename request\n');
    const result = await this.protocol.sendRequest(serverState.process, 'textDocument/rename', {
      textDocument: { uri: pathToUri(filePath) },
      position,
      newName,
    });

    process.stderr.write(
      `[DEBUG renameSymbol] Result type: ${typeof result}, hasChanges: ${result && typeof result === 'object' && 'changes' in result}, hasDocumentChanges: ${result && typeof result === 'object' && 'documentChanges' in result}\n`
    );

    if (result && typeof result === 'object') {
      // Handle the 'changes' format (older LSP servers)
      if ('changes' in result) {
        const workspaceEdit = result as {
          changes: Record<
            string,
            Array<{ range: { start: Position; end: Position }; newText: string }>
          >;
        };

        const changeCount = Object.keys(workspaceEdit.changes || {}).length;
        process.stderr.write(
          `[DEBUG renameSymbol] WorkspaceEdit has changes for ${changeCount} files\n`
        );

        return workspaceEdit;
      }

      // Handle the 'documentChanges' format (modern LSP servers like gopls)
      if ('documentChanges' in result) {
        const workspaceEdit = result as {
          documentChanges?: Array<{
            textDocument: { uri: string; version?: number };
            edits: Array<{ range: { start: Position; end: Position }; newText: string }>;
          }>;
        };

        process.stderr.write(
          `[DEBUG renameSymbol] WorkspaceEdit has documentChanges with ${workspaceEdit.documentChanges?.length || 0} entries\n`
        );

        // Convert documentChanges to changes format for compatibility
        const changes: Record<
          string,
          Array<{ range: { start: Position; end: Position }; newText: string }>
        > = {};

        if (workspaceEdit.documentChanges) {
          for (const change of workspaceEdit.documentChanges) {
            // Handle TextDocumentEdit (the only type needed for symbol renames)
            if (change.textDocument && change.edits) {
              const uri = change.textDocument.uri;
              if (!changes[uri]) {
                changes[uri] = [];
              }
              changes[uri].push(...change.edits);
              process.stderr.write(
                `[DEBUG renameSymbol] Added ${change.edits.length} edits for ${uri}\n`
              );
            }
          }
        }

        return { changes };
      }
    }

    process.stderr.write('[DEBUG renameSymbol] No rename changes available\n');
    return {};
  }

  /**
   * Search for symbols in workspace
   */
  async searchWorkspaceSymbols(
    query: string,
    servers: Map<string, ServerState>,
    preloadServers: (verbose?: boolean) => Promise<void>
  ): Promise<SymbolInformation[]> {
    // Ensure servers are preloaded before searching
    if (servers.size === 0) {
      process.stderr.write(
        '[DEBUG searchWorkspaceSymbols] No servers running, preloading servers first\n'
      );
      await preloadServers(false); // Preload without verbose logging
    }

    // For workspace symbol search to work, TypeScript server needs project context
    // Open a TypeScript file to establish project context if no files are open yet
    let hasOpenFiles = false;
    for (const serverState of servers.values()) {
      if (serverState.openFiles.size > 0) {
        hasOpenFiles = true;
        break;
      }
    }

    if (!hasOpenFiles) {
      try {
        // Try to open a TypeScript file in the workspace to establish project context
        const { scanDirectoryForExtensions, loadGitignore } = await import('../file-scanner.js');
        const gitignore = await loadGitignore(process.cwd());
        const extensions = await scanDirectoryForExtensions(process.cwd(), 2, gitignore, false);

        if (extensions.has('ts')) {
          // Find a .ts file to open for project context
          const fs = await import('node:fs/promises');
          const path = await import('node:path');

          async function findTsFile(dir: string): Promise<string | null> {
            try {
              const entries = await fs.readdir(dir, { withFileTypes: true });
              for (const entry of entries) {
                if (entry.isFile() && entry.name.endsWith('.ts')) {
                  return path.join(dir, entry.name);
                }
                if (entry.isDirectory() && !entry.name.startsWith('.')) {
                  const found = await findTsFile(path.join(dir, entry.name));
                  if (found) return found;
                }
              }
            } catch {}
            return null;
          }

          const tsFile = await findTsFile(process.cwd());
          if (tsFile) {
            process.stderr.write(
              `[DEBUG searchWorkspaceSymbols] Opening ${tsFile} to establish project context\n`
            );
            const serverState = await this.getServer(tsFile);
            await this.ensureFileOpen(serverState, tsFile);
          }
        }
      } catch (error) {
        process.stderr.write(
          `[DEBUG searchWorkspaceSymbols] Failed to establish project context: ${error}\n`
        );
      }
    }

    // For workspace/symbol, we need to try all running servers
    const results: SymbolInformation[] = [];

    process.stderr.write(
      `[DEBUG searchWorkspaceSymbols] Searching for "${query}" across ${servers.size} servers\n`
    );

    for (const [serverKey, serverState] of servers.entries()) {
      process.stderr.write(
        `[DEBUG searchWorkspaceSymbols] Checking server: ${serverKey}, initialized: ${serverState.initialized}\n`
      );

      if (!serverState.initialized) continue;

      try {
        process.stderr.write(
          `[DEBUG searchWorkspaceSymbols] Sending workspace/symbol request for "${query}"\n`
        );

        const result = await this.protocol.sendRequest(serverState.process, 'workspace/symbol', {
          query: query,
        });

        process.stderr.write(
          `[DEBUG searchWorkspaceSymbols] Workspace symbol result: ${JSON.stringify(result)}\n`
        );

        if (Array.isArray(result)) {
          results.push(...result);
          process.stderr.write(
            `[DEBUG searchWorkspaceSymbols] Added ${result.length} symbols from server\n`
          );
        } else if (result !== null && result !== undefined) {
          process.stderr.write(
            `[DEBUG searchWorkspaceSymbols] Non-array result: ${typeof result}\n`
          );
        }
      } catch (error) {
        // Some servers might not support workspace/symbol, continue with others
        process.stderr.write(`[DEBUG searchWorkspaceSymbols] Server error: ${error}\n`);
      }
    }

    process.stderr.write(`[DEBUG searchWorkspaceSymbols] Total results found: ${results.length}\n`);
    return results;
  }

  /**
   * Get document symbols
   */
  async getDocumentSymbols(filePath: string): Promise<DocumentSymbol[] | SymbolInformation[]> {
    const serverState = await this.getServer(filePath);

    // Wait for the server to be fully initialized
    await serverState.initializationPromise;

    // Ensure the file is opened and synced with the LSP server
    await this.ensureFileOpen(serverState, filePath);

    process.stderr.write(`[DEBUG] Requesting documentSymbol for: ${filePath}\n`);

    const result = await this.protocol.sendRequest(
      serverState.process,
      'textDocument/documentSymbol',
      {
        textDocument: { uri: pathToUri(filePath) },
      }
    );

    process.stderr.write(
      `[DEBUG] documentSymbol result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : 'N/A'}\n`
    );

    if (result && Array.isArray(result) && result.length > 0) {
      process.stderr.write(`[DEBUG] First symbol: ${JSON.stringify(result[0], null, 2)}\n`);
    } else if (result === null || result === undefined) {
      process.stderr.write('[DEBUG] documentSymbol returned null/undefined\n');
    } else {
      process.stderr.write(
        `[DEBUG] documentSymbol returned unexpected result: ${JSON.stringify(result)}\n`
      );
    }

    if (Array.isArray(result)) {
      return result as DocumentSymbol[] | SymbolInformation[];
    }

    return [];
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

  // Utility methods (inlined from DocumentMethods)
  flattenDocumentSymbols(symbols: DocumentSymbol[]): DocumentSymbol[] {
    const flattened: DocumentSymbol[] = [];

    for (const symbol of symbols) {
      flattened.push(symbol);
      if (symbol.children) {
        flattened.push(...this.flattenDocumentSymbols(symbol.children));
      }
    }

    return flattened;
  }

  isDocumentSymbolArray(
    symbols: DocumentSymbol[] | SymbolInformation[]
  ): symbols is DocumentSymbol[] {
    if (symbols.length === 0) return true;
    const firstSymbol = symbols[0];
    if (!firstSymbol) return true;
    // DocumentSymbol has 'range' and 'selectionRange', SymbolInformation has 'location'
    return 'range' in firstSymbol && 'selectionRange' in firstSymbol;
  }

  symbolKindToString(kind: SymbolKind): string {
    const kindMap: Record<SymbolKind, string> = {
      [SymbolKind.File]: 'file',
      [SymbolKind.Module]: 'module',
      [SymbolKind.Namespace]: 'namespace',
      [SymbolKind.Package]: 'package',
      [SymbolKind.Class]: 'class',
      [SymbolKind.Method]: 'method',
      [SymbolKind.Property]: 'property',
      [SymbolKind.Field]: 'field',
      [SymbolKind.Constructor]: 'constructor',
      [SymbolKind.Enum]: 'enum',
      [SymbolKind.Interface]: 'interface',
      [SymbolKind.Function]: 'function',
      [SymbolKind.Variable]: 'variable',
      [SymbolKind.Constant]: 'constant',
      [SymbolKind.String]: 'string',
      [SymbolKind.Number]: 'number',
      [SymbolKind.Boolean]: 'boolean',
      [SymbolKind.Array]: 'array',
      [SymbolKind.Object]: 'object',
      [SymbolKind.Key]: 'key',
      [SymbolKind.Null]: 'null',
      [SymbolKind.EnumMember]: 'enum_member',
      [SymbolKind.Struct]: 'struct',
      [SymbolKind.Event]: 'event',
      [SymbolKind.Operator]: 'operator',
      [SymbolKind.TypeParameter]: 'type_parameter',
    };
    return kindMap[kind] || 'unknown';
  }

  getValidSymbolKinds(): string[] {
    return [
      'file',
      'module',
      'namespace',
      'package',
      'class',
      'method',
      'property',
      'field',
      'constructor',
      'enum',
      'interface',
      'function',
      'variable',
      'constant',
      'string',
      'number',
      'boolean',
      'array',
      'object',
      'key',
      'null',
      'enum_member',
      'struct',
      'event',
      'operator',
      'type_parameter',
    ];
  }

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
