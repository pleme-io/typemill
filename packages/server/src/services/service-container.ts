/**
 * Service Container
 * Provides a clean dependency injection pattern to replace the 6-parameter antipattern
 * All services are lazily initialized and type-safe
 */

import type { LSPClient } from '../../../@codeflow/features/src/lsp/lsp-client.js';
import type { DiagnosticService } from '../../../@codeflow/features/src/services/lsp/diagnostic-service.js';
import type { SymbolService } from '../../../@codeflow/features/src/services/lsp/symbol-service.js';
import type { Logger } from '../core/diagnostics/structured-logger.js';
import type { TransactionManager } from '../core/transaction/TransactionManager.js';
import type { FileService } from './file-service.js';
import type { HierarchyService } from './intelligence/hierarchy-service.js';
import type { IntelligenceService } from './intelligence/intelligence-service.js';
import type { PredictiveLoaderService } from './predictive-loader.js';
import type { ServiceContext } from './service-context.js';

/**
 * Container holding all service instances
 * Provides type-safe access to services without parameter explosion
 */
export class ServiceContainer {
  constructor(
    private readonly _symbolService: SymbolService,
    private readonly _fileService: FileService,
    private readonly _diagnosticService: DiagnosticService,
    private readonly _intelligenceService: IntelligenceService,
    private readonly _hierarchyService: HierarchyService,
    private readonly _lspClient: LSPClient,
    private readonly _serviceContext: ServiceContext,
    private readonly _predictiveLoader?: PredictiveLoaderService,
    private readonly _transactionManager?: TransactionManager,
    private readonly _logger?: Logger
  ) {}

  // Getters for type-safe service access
  get symbolService(): SymbolService {
    return this._symbolService;
  }

  get fileService(): FileService {
    return this._fileService;
  }

  get diagnosticService(): DiagnosticService {
    return this._diagnosticService;
  }

  get intelligenceService(): IntelligenceService {
    return this._intelligenceService;
  }

  get hierarchyService(): HierarchyService {
    return this._hierarchyService;
  }

  get lspClient(): LSPClient {
    return this._lspClient;
  }

  get serviceContext(): ServiceContext {
    return this._serviceContext;
  }

  get predictiveLoader(): PredictiveLoaderService | undefined {
    return this._predictiveLoader;
  }

  get transactionManager(): TransactionManager | undefined {
    return this._transactionManager;
  }

  get logger(): Logger | undefined {
    return this._logger;
  }

  /**
   * Factory method to create a ServiceContainer from individual services
   * This is the primary way to instantiate the container
   */
  static create(params: {
    symbolService: SymbolService;
    fileService: FileService;
    diagnosticService: DiagnosticService;
    intelligenceService: IntelligenceService;
    hierarchyService: HierarchyService;
    lspClient: LSPClient;
    serviceContext: ServiceContext;
    predictiveLoader?: PredictiveLoaderService;
    transactionManager?: TransactionManager;
    logger?: Logger;
  }): ServiceContainer {
    return new ServiceContainer(
      params.symbolService,
      params.fileService,
      params.diagnosticService,
      params.intelligenceService,
      params.hierarchyService,
      params.lspClient,
      params.serviceContext,
      params.predictiveLoader,
      params.transactionManager,
      params.logger
    );
  }

  /**
   * Get a service by type string (for dynamic access)
   * Used primarily by the batch executor
   */
  getService(serviceType: string): unknown {
    switch (serviceType) {
      case 'symbol':
        return this.symbolService;
      case 'file':
        return this.fileService;
      case 'diagnostic':
        return this.diagnosticService;
      case 'intelligence':
        return this.intelligenceService;
      case 'hierarchy':
        return this.hierarchyService;
      case 'lsp':
        return this.lspClient;
      case 'serviceContext':
        return this.serviceContext;
      case 'container':
        return this;
      default:
        return undefined;
    }
  }

  /**
   * Check if a service is available
   */
  hasService(serviceType: string): boolean {
    return this.getService(serviceType) !== undefined;
  }

  /**
   * Clone the container with updated services
   * Useful for testing or creating modified contexts
   */
  withServices(
    updates: Partial<{
      symbolService: SymbolService;
      fileService: FileService;
      diagnosticService: DiagnosticService;
      intelligenceService: IntelligenceService;
      hierarchyService: HierarchyService;
      lspClient: LSPClient;
      serviceContext: ServiceContext;
      predictiveLoader: PredictiveLoaderService;
      transactionManager: TransactionManager;
      logger: Logger;
    }>
  ): ServiceContainer {
    return new ServiceContainer(
      updates.symbolService ?? this._symbolService,
      updates.fileService ?? this._fileService,
      updates.diagnosticService ?? this._diagnosticService,
      updates.intelligenceService ?? this._intelligenceService,
      updates.hierarchyService ?? this._hierarchyService,
      updates.lspClient ?? this._lspClient,
      updates.serviceContext ?? this._serviceContext,
      updates.predictiveLoader ?? this._predictiveLoader,
      updates.transactionManager ?? this._transactionManager,
      updates.logger ?? this._logger
    );
  }
}

/**
 * Type guard to check if a value is a ServiceContainer
 */
export function isServiceContainer(value: unknown): value is ServiceContainer {
  return value instanceof ServiceContainer;
}
