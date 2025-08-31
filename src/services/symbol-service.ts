import { readFileSync } from 'node:fs';
import { capabilityManager } from '../capability-manager.js';
import type { ServerState } from '../lsp-types.js';
import { pathToUri, uriToPath } from '../path-utils.js';
import type {
  DocumentSymbol,
  LSPLocation,
  Location,
  Position,
  SymbolInformation,
  SymbolMatch,
} from '../types.js';
import { SymbolKind } from '../types.js';
import type { ServiceContext } from './service-context.js';

/**
 * Service for symbol-related LSP operations
 * Handles definition, references, renaming, and symbol search
 */
export class SymbolService {
  constructor(private context: ServiceContext) {}

  /**
   * Find definition of symbol at position
   */
  async findDefinition(filePath: string, position: Position): Promise<Location[]> {
    process.stderr.write(
      `[DEBUG findDefinition] Requesting definition for ${filePath} at ${position.line}:${position.character}\n`
    );

    const serverState = await this.context.prepareFile(filePath);

    process.stderr.write('[DEBUG findDefinition] Sending textDocument/definition request\n');
    const result = await this.context.protocol.sendRequest(
      serverState.process,
      'textDocument/definition',
      {
        textDocument: { uri: pathToUri(filePath) },
        position,
      }
    );

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
    const serverState = await this.context.prepareFile(filePath);

    process.stderr.write(
      `[DEBUG] findReferences for ${filePath} at ${position.line}:${position.character}, includeDeclaration: ${includeDeclaration}\n`
    );

    const result = await this.context.protocol.sendRequest(
      serverState.process,
      'textDocument/references',
      {
        textDocument: { uri: pathToUri(filePath) },
        position,
        context: { includeDeclaration },
      }
    );

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

    const serverState = await this.context.prepareFile(filePath);

    // CRITICAL FIX: For multi-file rename to work, we need to:
    // 1. First open all potential project files (same extension) in the directory tree
    // 2. Then find references (which now works across all opened files)
    // 3. Finally perform the rename

    // Step 1: Open all TypeScript/JavaScript files in the project to enable cross-file operations
    const projectFiles = new Set<string>();
    const fileExt = filePath.match(/\.(tsx?|jsx?|mjs|cjs)$/)?.[1];
    if (fileExt) {
      process.stderr.write(
        '[DEBUG renameSymbol] Opening project files to enable cross-file rename...\n'
      );

      // Find project root (go up until we find package.json or .git)
      const { dirname, join } = await import('node:path');
      const { existsSync, readdirSync, statSync } = await import('node:fs');

      let projectRoot = dirname(filePath);
      while (projectRoot !== '/' && projectRoot !== '.') {
        if (
          existsSync(join(projectRoot, 'package.json')) ||
          existsSync(join(projectRoot, '.git'))
        ) {
          break;
        }
        const parent = dirname(projectRoot);
        if (parent === projectRoot) break;
        projectRoot = parent;
      }

      // Recursively find all files with same extension in project
      const findProjectFiles = (dir: string, depth = 0): void => {
        if (depth > 5) return; // Limit depth to avoid scanning too deep

        try {
          const entries = readdirSync(dir);
          for (const entry of entries) {
            const fullPath = join(dir, entry);

            // Skip node_modules, dist, build, etc.
            if (
              entry === 'node_modules' ||
              entry === 'dist' ||
              entry === 'build' ||
              entry === '.git'
            ) {
              continue;
            }

            const stats = statSync(fullPath);
            if (stats.isDirectory()) {
              findProjectFiles(fullPath, depth + 1);
            } else if (stats.isFile() && fullPath.match(/\.(tsx?|jsx?|mjs|cjs)$/)) {
              projectFiles.add(fullPath);
            }
          }
        } catch (error) {
          // Ignore errors reading directories
        }
      };

      findProjectFiles(projectRoot);

      // Open all project files (up to a reasonable limit)
      const filesToOpen = Array.from(projectFiles).slice(0, 50); // Limit to 50 files
      process.stderr.write(`[DEBUG renameSymbol] Opening ${filesToOpen.length} project files...\n`);

      for (const projectFile of filesToOpen) {
        try {
          const fileServerState = await this.context.prepareFile(projectFile);
        } catch (error) {
          // Ignore errors opening individual files
        }
      }
    }

    // Step 2: Now find references (this should work across all opened files)
    const referencingFiles = new Set<string>();
    try {
      process.stderr.write(
        '[DEBUG renameSymbol] Finding cross-file references for multi-file rename\n'
      );
      const references = await this.findReferences(filePath, position, true);
      for (const ref of references) {
        const refFilePath = uriToPath(ref.uri);
        referencingFiles.add(refFilePath);
      }
      process.stderr.write(
        `[DEBUG renameSymbol] Found references in ${referencingFiles.size} files\n`
      );
    } catch (error) {
      process.stderr.write(
        `[DEBUG renameSymbol] Could not find references for pre-opening: ${error}\n`
      );
      // Fallback to just the main file
      referencingFiles.add(filePath);
    }

    // Step 3: Ensure all referencing files are opened (some may already be open from step 1)
    for (const refFilePath of referencingFiles) {
      try {
        const fileServerState = await this.context.prepareFile(refFilePath);
        process.stderr.write(
          `[DEBUG renameSymbol] Ensured file is open for rename: ${refFilePath}\n`
        );
      } catch (error) {
        process.stderr.write(`[DEBUG renameSymbol] Failed to open ${refFilePath}: ${error}\n`);
      }
    }

    // Give LSP server time to process the newly opened files
    // This is critical for the server to establish proper cross-file relationships
    if (referencingFiles.size > 1) {
      process.stderr.write(
        '[DEBUG renameSymbol] Waiting for LSP server to process opened files...\n'
      );
      await new Promise((resolve) => setTimeout(resolve, 1000)); // 1 second delay
    }

    // For dry_run operations, we can now safely call the LSP server since we know which files will be affected
    if (dryRun) {
      process.stderr.write('[DEBUG renameSymbol] Performing dry-run rename to preview changes\n');
      // We still call the LSP server but will not apply the workspace edit
      // This gives us accurate preview of what would change
    }

    process.stderr.write('[DEBUG renameSymbol] Sending textDocument/rename request\n');
    const result = await this.context.protocol.sendRequest(
      serverState.process,
      'textDocument/rename',
      {
        textDocument: { uri: pathToUri(filePath) },
        position,
        newName,
      }
    );

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
    preloadServers: (verbose?: boolean) => Promise<void>,
    workspacePath?: string
  ): Promise<SymbolInformation[]> {
    process.stderr.write(
      `[DEBUG searchWorkspaceSymbols] servers.size=${servers.size}, server keys: [${Array.from(servers.keys()).join(', ')}]\n`
    );

    // Check if we have any initialized servers first
    const initializedServers = Array.from(servers.values()).filter((s) => s.initialized);
    process.stderr.write(
      `[DEBUG searchWorkspaceSymbols] initialized servers: ${initializedServers.length}/${servers.size}\n`
    );

    // Only preload if we have no servers at all (not even uninitialized ones)
    // This prevents redundant preloading when servers are already starting up
    if (servers.size === 0) {
      process.stderr.write('[DEBUG searchWorkspaceSymbols] No servers found, preloading...\n');
      await preloadServers(false);
    } else if (initializedServers.length === 0) {
      process.stderr.write(
        '[DEBUG searchWorkspaceSymbols] Servers exist but none initialized, waiting briefly...\n'
      );
      // Brief wait for existing servers to finish initializing instead of preloading again
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    // For workspace symbol search, we need at least some files open for context
    // But avoid excessive file opening that causes timeouts
    const hasAnyOpenFiles = Array.from(servers.values()).some((s) => s.openFiles.size > 0);

    if (!hasAnyOpenFiles && workspacePath) {
      try {
        process.stderr.write(
          '[DEBUG searchWorkspaceSymbols] Opening minimal files for workspace context\n'
        );

        // Just open a few key files instead of scanning the entire project
        const { existsSync, readdirSync } = await import('node:fs');
        const { join } = await import('node:path');

        if (existsSync(workspacePath)) {
          const files = readdirSync(workspacePath)
            .filter((f) => f.match(/\.(ts|js|tsx|jsx)$/))
            .slice(0, 3) // Open maximum 3 files for context
            .map((f) => join(workspacePath, f));

          for (const filePath of files) {
            try {
              await this.context.prepareFile(filePath);
            } catch (error) {
              // Ignore individual file errors
            }
          }

          process.stderr.write(
            `[DEBUG searchWorkspaceSymbols] Opened ${files.length} context files\n`
          );

          // Brief pause to let files process
          if (files.length > 0) {
            await new Promise((resolve) => setTimeout(resolve, 100));
          }
        }
      } catch (error) {
        process.stderr.write(
          `[DEBUG searchWorkspaceSymbols] Failed to establish minimal context: ${error}\n`
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

        const result = await this.context.protocol.sendRequest(
          serverState.process,
          'workspace/symbol',
          {
            query: query,
          }
        );

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
    const serverState = await this.context.prepareFile(filePath);

    process.stderr.write(`[DEBUG] Requesting documentSymbol for: ${filePath}\n`);

    const result = await this.context.protocol.sendRequest(
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

  // ensureFileOpen() and getLanguageId() methods removed - provided by ServiceContext
  // This eliminates ~45 lines of duplicated code from this service
}
