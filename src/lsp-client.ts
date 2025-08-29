import { readFileSync } from 'node:fs';
import { capabilityManager } from './capability-manager.js';
import { scanDirectoryForExtensions } from './file-scanner.js';
import type { ServerState } from './lsp-types.js';
import { LSPClient as NewLSPClient } from './lsp/client.js';
import type { LSPProtocol } from './lsp/protocol.js';
import type { ServerManager } from './lsp/server-manager.js';
import { DiagnosticService } from './services/diagnostic-service.js';
import { FileService } from './services/file-service.js';
import { HierarchyService } from './services/hierarchy-service.js';
import { IntelligenceService } from './services/intelligence-service.js';
import { SymbolService } from './services/symbol-service.js';
import type {
  CodeAction,
  Config,
  Diagnostic,
  DocumentSymbol,
  FoldingRange,
  Location,
  Position,
  Range,
  SymbolInformation,
  SymbolMatch,
  TextEdit,
} from './types.js';

/**
 * LSP Client facade that maintains backward compatibility
 * while using the new service-oriented architecture
 */
export class LSPClient {
  private newClient: NewLSPClient;
  private protocol: LSPProtocol;
  private serverManager: ServerManager;
  private symbolService: SymbolService;
  private fileService: FileService;
  private diagnosticService: DiagnosticService;
  private intelligenceService: IntelligenceService;
  private hierarchyService: HierarchyService;

  constructor(configPath?: string) {
    this.newClient = new NewLSPClient(configPath);

    // Access internal components (would be properly injected in real refactor)
    this.protocol = this.newClient.protocol;
    this.serverManager = this.newClient.serverManager;

    // Initialize services with getServer wrapper
    const getServerWrapper = (filePath: string) => this.newClient.getServer(filePath);
    this.symbolService = new SymbolService(getServerWrapper, this.protocol);
    this.fileService = new FileService(getServerWrapper, this.protocol);
    this.diagnosticService = new DiagnosticService(getServerWrapper, this.protocol);
    this.intelligenceService = new IntelligenceService(getServerWrapper, this.protocol);
    this.hierarchyService = new HierarchyService(getServerWrapper, this.protocol);
  }

  // Delegate core methods to services
  async findDefinition(filePath: string, position: Position): Promise<Location[]> {
    return this.symbolService.findDefinition(filePath, position);
  }

  async findReferences(
    filePath: string,
    position: Position,
    includeDeclaration = false
  ): Promise<Location[]> {
    return this.symbolService.findReferences(filePath, position, includeDeclaration);
  }

  async renameSymbol(
    filePath: string,
    position: Position,
    newName: string
  ): Promise<{
    changes?: Record<string, Array<{ range: { start: Position; end: Position }; newText: string }>>;
  }> {
    return this.symbolService.renameSymbol(filePath, position, newName);
  }

  async getDocumentSymbols(filePath: string): Promise<DocumentSymbol[] | SymbolInformation[]> {
    return this.symbolService.getDocumentSymbols(filePath);
  }

  async searchWorkspaceSymbols(query: string): Promise<SymbolInformation[]> {
    return this.symbolService.searchWorkspaceSymbols(
      query,
      this.serverManager.activeServers,
      this.newClient.preloadServers.bind(this.newClient)
    );
  }

  async findSymbolMatches(
    filePath: string,
    symbolName: string,
    symbolKind?: string
  ): Promise<SymbolMatch[]> {
    return this.symbolService.findSymbolMatches(filePath, symbolName, symbolKind);
  }

  async formatDocument(
    filePath: string,
    options?: {
      tabSize?: number;
      insertSpaces?: boolean;
      trimTrailingWhitespace?: boolean;
      insertFinalNewline?: boolean;
      trimFinalNewlines?: boolean;
    }
  ): Promise<TextEdit[]> {
    return this.fileService.formatDocument(filePath, options);
  }

  async getCodeActions(
    filePath: string,
    range?: Range,
    context?: { diagnostics?: Diagnostic[] }
  ): Promise<CodeAction[]> {
    return this.fileService.getCodeActions(
      filePath,
      range || { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } },
      context || { diagnostics: [] }
    );
  }

  async getFoldingRanges(filePath: string): Promise<FoldingRange[]> {
    return this.fileService.getFoldingRanges(filePath);
  }

  async getDocumentLinks(filePath: string): Promise<import('./types.js').DocumentLink[]> {
    return this.fileService.getDocumentLinks(filePath);
  }

  async getDiagnostics(filePath: string): Promise<Diagnostic[]> {
    return this.diagnosticService.getDiagnostics(filePath);
  }

  async syncFileContent(filePath: string): Promise<void> {
    return this.fileService.syncFileContent(filePath);
  }

  // Intelligence methods
  async getHover(filePath: string, position: Position): Promise<import('./types.js').Hover | null> {
    return this.intelligenceService.getHover(filePath, position);
  }

  async getCompletions(
    filePath: string,
    position: Position,
    triggerCharacter?: string
  ): Promise<import('./types.js').CompletionItem[]> {
    return this.intelligenceService.getCompletions(filePath, position, triggerCharacter);
  }

  async getSignatureHelp(
    filePath: string,
    position: Position,
    triggerCharacter?: string
  ): Promise<import('./types.js').SignatureHelp | null> {
    return this.intelligenceService.getSignatureHelp(filePath, position, triggerCharacter);
  }

  async getInlayHints(filePath: string, range: Range): Promise<import('./types.js').InlayHint[]> {
    return this.intelligenceService.getInlayHints(filePath, range);
  }

  async getSemanticTokens(filePath: string): Promise<import('./types.js').SemanticTokens | null> {
    return this.intelligenceService.getSemanticTokens(filePath);
  }

  // Hierarchy methods
  async prepareCallHierarchy(
    filePath: string,
    position: Position
  ): Promise<import('./types.js').CallHierarchyItem[]> {
    return this.hierarchyService.prepareCallHierarchy(filePath, position);
  }

  async getCallHierarchyIncomingCalls(
    item: import('./types.js').CallHierarchyItem
  ): Promise<import('./types.js').CallHierarchyIncomingCall[]> {
    return this.hierarchyService.getCallHierarchyIncomingCalls(item);
  }

  async getCallHierarchyOutgoingCalls(
    item: import('./types.js').CallHierarchyItem
  ): Promise<import('./types.js').CallHierarchyOutgoingCall[]> {
    return this.hierarchyService.getCallHierarchyOutgoingCalls(item);
  }

  async prepareTypeHierarchy(
    filePath: string,
    position: Position
  ): Promise<import('./types.js').TypeHierarchyItem[]> {
    return this.hierarchyService.prepareTypeHierarchy(filePath, position);
  }

  async getTypeHierarchySupertypes(
    item: import('./types.js').TypeHierarchyItem
  ): Promise<import('./types.js').TypeHierarchyItem[]> {
    return this.hierarchyService.getTypeHierarchySupertypes(item);
  }

  async getTypeHierarchySubtypes(
    item: import('./types.js').TypeHierarchyItem
  ): Promise<import('./types.js').TypeHierarchyItem[]> {
    return this.hierarchyService.getTypeHierarchySubtypes(item);
  }

  async getSelectionRange(
    filePath: string,
    positions: Position[]
  ): Promise<import('./types.js').SelectionRange[]> {
    return this.hierarchyService.getSelectionRange(filePath, positions);
  }

  // Direct delegation to new client
  async getServer(filePath: string): Promise<ServerState> {
    return this.newClient.getServer(filePath);
  }

  // Internal method for services to get server with config access
  private async getServerForService(filePath: string): Promise<ServerState> {
    return this.newClient.getServer(filePath);
  }

  async sendRequest(
    process: import('node:child_process').ChildProcess,
    method: string,
    params: unknown,
    timeout?: number
  ): Promise<unknown> {
    return this.protocol.sendRequest(process, method, params, timeout);
  }

  sendNotification(
    process: import('node:child_process').ChildProcess,
    method: string,
    params: unknown
  ): void {
    this.protocol.sendNotification(process, method, params);
  }

  async restartServer(extensions?: string[]): Promise<string[]> {
    return this.newClient.restartServer(extensions);
  }

  // Compatibility aliases
  async findSymbolsByName(
    filePath: string,
    symbolName: string,
    symbolKind?: string
  ): Promise<{ matches: SymbolMatch[]; warning?: string }> {
    const matches = await this.findSymbolMatches(filePath, symbolName, symbolKind);
    return { matches };
  }

  async restartServers(
    extensions?: string[]
  ): Promise<{ success: boolean; restarted: string[]; failed: string[]; message: string }> {
    try {
      const restarted = await this.restartServer(extensions);
      const message = `Successfully restarted ${restarted.length} LSP server(s)`;
      return { success: true, restarted, failed: [], message };
    } catch (error) {
      const message = `Failed to restart servers: ${error instanceof Error ? error.message : String(error)}`;
      return { success: false, restarted: [], failed: [message], message };
    }
  }

  async preloadServers(): Promise<void> {
    return this.newClient.preloadServers();
  }

  // Utility methods from services
  get flattenDocumentSymbols() {
    return this.symbolService.flattenDocumentSymbols;
  }

  get isDocumentSymbolArray() {
    return this.symbolService.isDocumentSymbolArray;
  }

  get symbolKindToString() {
    return this.symbolService.symbolKindToString;
  }

  get getValidSymbolKinds() {
    return this.symbolService.getValidSymbolKinds;
  }

  // Capability methods
  hasCapability(filePath: string, capabilityPath: string): Promise<boolean> {
    return this.getServer(filePath)
      .then((serverState) => {
        return capabilityManager.hasCapability(serverState, capabilityPath);
      })
      .catch(() => false);
  }

  async getCapabilityInfo(filePath: string): Promise<string> {
    try {
      const serverState = await this.getServer(filePath);
      return capabilityManager.getCapabilityInfo(serverState);
    } catch (error) {
      return `Error getting server: ${error instanceof Error ? error.message : String(error)}`;
    }
  }

  async validateCapabilities(
    filePath: string,
    requiredCapabilities: string[]
  ): Promise<{
    supported: boolean;
    missing: string[];
    serverDescription: string;
  }> {
    try {
      const serverState = await this.getServer(filePath);
      const validation = capabilityManager.validateRequiredCapabilities(
        serverState,
        requiredCapabilities
      );
      return {
        ...validation,
        serverDescription: capabilityManager.getServerDescription(serverState),
      };
    } catch (error) {
      return {
        supported: false,
        missing: requiredCapabilities,
        serverDescription: 'Unknown Server',
      };
    }
  }

  // File synchronization
  private async ensureFileOpen(serverState: ServerState, filePath: string): Promise<void> {
    return this.fileService.ensureFileOpen(serverState, filePath);
  }

  dispose(): void {
    this.newClient.dispose();
  }
}
