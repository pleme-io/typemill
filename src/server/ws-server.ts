import { readFileSync } from 'node:fs';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { createServer as createHttpsServer, type Server as HttpsServer } from 'node:https';
import type WebSocket from 'ws';
import { WebSocketServer } from 'ws';
import { type AuthRequest, type AuthResponse, JWTAuthenticator } from '../auth/jwt-auth.js';
import { logger } from '../core/diagnostics/logger.js';
import { FuseMount } from '../fs/fuse-mount.js';
import { StreamingFileAccess } from '../fs/stream.js';
import { LSPClient } from '../lsp/lsp-client.js';
import { toolRegistry } from '../mcp/tool-registry.js';
import type { ClientSession, MCPMessage } from '../transports/websocket.js';
import { WebSocketTransport } from '../transports/websocket.js';
import type {
  EnhancedClientSession,
  FuseOperationResponse,
  WorkspaceInfo,
} from '../types/session.js';
import { getPackageVersion } from '../utils/version.js';
import { LSPServerPool } from './lsp-pool.js';
import { SessionManager } from './session.js';
import { WorkspaceManager } from './workspace-manager.js';

// Import handlers to ensure they register themselves
import '../mcp/handlers/core-handlers.js';
import '../mcp/handlers/batch-handlers.js';
import '../mcp/handlers/advanced-handlers.js';
import '../mcp/handlers/hierarchy-handlers.js';
import '../mcp/handlers/intelligence-handlers.js';
import '../mcp/handlers/utility-handlers.js';

export interface TLSOptions {
  keyPath: string;
  certPath: string;
  caPath?: string; // Certificate Authority path for client certificate validation
}

export interface WebSocketServerOptions {
  port: number;
  maxClients?: number;
  requireAuth?: boolean;
  jwtSecret?: string;
  tls?: TLSOptions;
  enableFuse?: boolean;
  allowedOrigins?: string[]; // WebSocket origin validation
  allowedCorsOrigins?: string[]; // HTTP CORS origins
  workspaceConfig?: {
    baseWorkspaceDir?: string;
    fuseMountPrefix?: string;
    maxWorkspaces?: number;
    workspaceTimeoutMs?: number;
  };
}

export class CodeFlowWebSocketServer {
  private wss: WebSocketServer;
  private httpServer: ReturnType<typeof createServer> | HttpsServer;
  private transport: WebSocketTransport;
  private sessionManager: SessionManager;
  private lspServerPool: LSPServerPool;
  private streamingFS: StreamingFileAccess;
  private lspClient: LSPClient;
  private authenticator?: JWTAuthenticator;
  private isSecure: boolean;
  private clientCount = 0;
  private startTime = Date.now();

  // FUSE and workspace management
  private workspaceManager?: WorkspaceManager;
  private fuseMounts = new Map<string, FuseMount>(); // sessionId -> FuseMount

  constructor(private options: WebSocketServerOptions) {
    this.sessionManager = new SessionManager();
    this.isSecure = !!options.tls;

    // Initialize FUSE workspace manager if enabled
    if (options.enableFuse) {
      this.workspaceManager = new WorkspaceManager(options.workspaceConfig);
      logger.info('FUSE workspace manager enabled', {
        component: 'CodeFlowWebSocketServer',
        config: options.workspaceConfig,
      });
    }

    // Initialize authentication if required
    if (options.requireAuth) {
      const authConfig = JWTAuthenticator.createDefaultConfig();
      if (options.jwtSecret) {
        authConfig.secretKey = options.jwtSecret;
      }
      this.authenticator = new JWTAuthenticator(authConfig);
    }

    // Initialize LSP client (will load configuration automatically)
    this.lspClient = new LSPClient();

    this.lspServerPool = new LSPServerPool(this.lspClient);

    this.transport = new WebSocketTransport(
      this.handleMCPMessage.bind(this),
      this.handleSessionReconnect.bind(this),
      this.handleSessionDisconnect.bind(this),
      this.authenticator ? this.validateToken.bind(this) : undefined,
      this.workspaceManager ? this.handleWorkspaceCreate.bind(this) : undefined,
      this.handleFuseResponse.bind(this)
    );
    this.streamingFS = new StreamingFileAccess(this.transport);

    // Create HTTP/HTTPS server based on TLS configuration
    this.httpServer = this.createServer();

    this.wss = new WebSocketServer({
      server: this.httpServer,
      clientTracking: true,
      verifyClient: this.verifyWebSocketOrigin.bind(this),
    });

    this.setupServer();
  }

  private verifyWebSocketOrigin(info: { origin: string; req: IncomingMessage }): boolean {
    // If no origins specified, allow all (backward compatibility)
    if (!this.options.allowedOrigins || this.options.allowedOrigins.length === 0) {
      return true;
    }

    // Check if origin is in allowed list
    const origin = info.origin || (info.req.headers.origin as string);
    if (!origin) {
      // No origin header - reject for security
      logger.warn('WebSocket connection rejected - no origin header', {
        component: 'WebSocketServer',
        remoteAddress: info.req.socket.remoteAddress,
      });
      return false;
    }

    const isAllowed =
      this.options.allowedOrigins.includes(origin) || this.options.allowedOrigins.includes('*');

    if (!isAllowed) {
      logger.warn('WebSocket connection rejected - unauthorized origin', {
        component: 'WebSocketServer',
        origin,
        allowedOrigins: this.options.allowedOrigins,
      });
    }

    return isAllowed;
  }

  private createServer(): ReturnType<typeof createServer> | HttpsServer {
    if (this.options.tls) {
      return this.createHttpsServer();
    }
    return createServer(this.handleHttpRequest.bind(this));
  }

  private createHttpsServer(): HttpsServer {
    if (!this.options.tls) {
      throw new Error('TLS configuration required for HTTPS server');
    }

    try {
      const tlsOptions = {
        key: readFileSync(this.options.tls.keyPath),
        cert: readFileSync(this.options.tls.certPath),
        ...(this.options.tls.caPath && {
          ca: readFileSync(this.options.tls.caPath),
          requestCert: true,
          rejectUnauthorized: true,
        }),
      };

      logger.info('Creating HTTPS server with TLS configuration', {
        component: 'WebSocketServer',
        keyPath: this.options.tls.keyPath,
        certPath: this.options.tls.certPath,
        caPath: this.options.tls.caPath,
        clientCertValidation: !!this.options.tls.caPath,
      });

      return createHttpsServer(tlsOptions, this.handleHttpRequest.bind(this));
    } catch (error) {
      logger.error('Failed to create HTTPS server', error as Error, {
        component: 'WebSocketServer',
        tlsOptions: this.options.tls,
      });

      throw new Error(
        `Failed to create HTTPS server: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }

  private setupServer(): void {
    this.wss.on('connection', (ws: WebSocket, req) => {
      this.clientCount++;

      logger.logConnection('connect', {
        component: 'WebSocketServer',
        clientCount: this.clientCount,
        remoteAddress: req.socket.remoteAddress,
        userAgent: req.headers['user-agent'],
      });

      // Check client limit
      if (this.options.maxClients && this.clientCount > this.options.maxClients) {
        logger.warn('Server at capacity, rejecting connection', {
          component: 'WebSocketServer',
          clientCount: this.clientCount,
          maxClients: this.options.maxClients,
        });
        ws.close(1008, 'Server at capacity');
        this.clientCount--;
        return;
      }

      // Set up the WebSocket transport for this connection
      this.transport.handleConnection(ws);

      ws.on('close', () => {
        this.clientCount--;
        logger.logConnection('disconnect', {
          component: 'WebSocketServer',
          clientCount: this.clientCount,
        });
      });
    });

    this.wss.on('error', (error) => {
      logger.error('WebSocket server error', error, {
        component: 'WebSocketServer',
      });
    });

    // Start HTTP/HTTPS server
    this.httpServer.listen(this.options.port, () => {
      logger.info(`${this.isSecure ? 'WSS' : 'WS'} server started`, {
        component: 'WebSocketServer',
        port: this.options.port,
        maxClients: this.options.maxClients,
        secure: this.isSecure,
        protocol: this.isSecure ? 'wss' : 'ws',
        authentication: !!this.authenticator,
      });
    });

    this.httpServer.on('error', (error) => {
      logger.error('HTTP server error', error, {
        component: 'HTTPServer',
      });
    });
  }

  private handleHttpRequest(req: IncomingMessage, res: ServerResponse): void {
    // Set CORS headers based on configuration
    const origin = req.headers.origin as string;
    const corsOrigin = this.options.allowedCorsOrigins?.length
      ? this.options.allowedCorsOrigins.includes('*')
        ? '*'
        : origin && this.options.allowedCorsOrigins.includes(origin)
          ? origin
          : this.options.allowedCorsOrigins[0]
      : '*';

    res.setHeader('Access-Control-Allow-Origin', corsOrigin || '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
    res.setHeader('Access-Control-Allow-Credentials', 'true');

    // Handle preflight requests
    if (req.method === 'OPTIONS') {
      res.writeHead(200);
      res.end();
      return;
    }

    // Health check endpoint
    if (req.url === '/healthz') {
      this.handleHealthCheck(req, res);
      return;
    }

    // Metrics endpoint
    if (req.url === '/metrics') {
      this.handleMetrics(req, res);
      return;
    }

    // Authentication endpoint
    if (req.url === '/auth' && req.method === 'POST') {
      this.handleAuth(req, res);
      return;
    }

    // 404 for all other routes
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Not found' }));
  }

  private handleHealthCheck(req: IncomingMessage, res: ServerResponse): void {
    try {
      const sessionStats = this.sessionManager.getStats();
      const lspServers = this.lspServerPool.getActiveServers();

      const healthData = {
        status: 'healthy',
        timestamp: new Date().toISOString(),
        version: getPackageVersion(),
        uptime: Math.floor((Date.now() - this.startTime) / 1000),
        connections: {
          active: sessionStats.activeSessions,
          disconnected: sessionStats.disconnectedSessions,
          total: this.clientCount,
        },
        sessions: {
          active: sessionStats.activeSessions,
          disconnected: sessionStats.disconnectedSessions,
          projects: sessionStats.activeProjects,
          ...(sessionStats.oldestDisconnection && {
            oldestDisconnection: sessionStats.oldestDisconnection.toISOString(),
          }),
        },
        lspServers: {
          active: lspServers.length,
          crashed: lspServers.filter((s) => s.refCount === 0).length,
          projects: [...new Set(lspServers.map((s) => s.projectId))].length,
          languages: [...new Set(lspServers.map((s) => s.language))].length,
        },
        cache: {
          ...this.streamingFS.getCacheStats(),
        },
        deltaProcessor: {
          ...this.streamingFS.getDeltaStats(),
        },
        authentication: {
          enabled: !!this.authenticator,
          ...(this.authenticator && {
            issuer: this.authenticator.config.issuer,
            audience: this.authenticator.config.audience,
          }),
        },
        security: {
          tls: this.isSecure,
          protocol: this.isSecure ? 'wss' : 'ws',
          ...(this.isSecure &&
            this.options.tls && {
              clientCertValidation: !!this.options.tls.caPath,
            }),
        },
      };

      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(healthData, null, 2));

      logger.debug('Health check requested', {
        component: 'HTTPServer',
        userAgent: req.headers['user-agent'],
        remoteAddress: req.socket.remoteAddress,
      });
    } catch (error) {
      logger.error('Health check failed', error as Error, {
        component: 'HTTPServer',
      });

      res.writeHead(503, { 'Content-Type': 'application/json' });
      res.end(
        JSON.stringify({
          status: 'unhealthy',
          timestamp: new Date().toISOString(),
          error: 'Internal server error',
        })
      );
    }
  }

  private handleMetrics(req: IncomingMessage, res: ServerResponse): void {
    try {
      const sessionStats = this.sessionManager.getStats();
      const lspServers = this.lspServerPool.getActiveServers();

      // Prometheus-style metrics
      const metrics = [
        '# HELP codeflow_connections_active Number of active WebSocket connections',
        '# TYPE codeflow_connections_active gauge',
        `codeflow_connections_active ${sessionStats.activeSessions}`,
        '',
        '# HELP codeflow_connections_disconnected Number of disconnected sessions waiting for reconnection',
        '# TYPE codeflow_connections_disconnected gauge',
        `codeflow_connections_disconnected ${sessionStats.disconnectedSessions}`,
        '',
        '# HELP codeflow_projects_active Number of active projects',
        '# TYPE codeflow_projects_active gauge',
        `codeflow_projects_active ${sessionStats.activeProjects}`,
        '',
        '# HELP codeflow_lsp_servers_active Number of active LSP servers',
        '# TYPE codeflow_lsp_servers_active gauge',
        `codeflow_lsp_servers_active ${lspServers.length}`,
        '',
        '# HELP codeflow_uptime_seconds Server uptime in seconds',
        '# TYPE codeflow_uptime_seconds counter',
        `codeflow_uptime_seconds ${Math.floor((Date.now() - this.startTime) / 1000)}`,
        '',
      ].join('\n');

      res.writeHead(200, { 'Content-Type': 'text/plain' });
      res.end(metrics);
    } catch (error) {
      logger.error('Metrics request failed', error as Error, {
        component: 'HTTPServer',
      });

      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end('# Error generating metrics\n');
    }
  }

  private handleSessionReconnect(sessionId: string, socket: any): ClientSession | null {
    return this.sessionManager.reconnectSession(sessionId, socket);
  }

  private async handleSessionDisconnect(sessionId: string): Promise<void> {
    try {
      // Unmount FUSE filesystem if exists
      const fuseMount = this.fuseMounts.get(sessionId);
      if (fuseMount) {
        await fuseMount.unmount();
        this.fuseMounts.delete(sessionId);
        logger.info('FUSE filesystem unmounted for session', {
          component: 'CodeFlowWebSocketServer',
          sessionId,
        });
      }

      // Cleanup workspace if exists
      if (this.workspaceManager) {
        await this.workspaceManager.cleanupWorkspace(sessionId);
      }

      // Clean up streaming FS cache
      this.streamingFS.cleanupSession(sessionId);

      // Notify session manager
      this.sessionManager.handleDisconnection(sessionId, (expiredSession) => {
        logger.info('Session expired and cleaned up', {
          component: 'WebSocketServer',
          sessionId,
          projectId: expiredSession.projectId,
        });
      });
    } catch (error) {
      logger.error('Error during session disconnect cleanup', error as Error, {
        component: 'CodeFlowWebSocketServer',
        sessionId,
      });
    }
  }

  private handleAuth(req: IncomingMessage, res: ServerResponse): void {
    if (!this.authenticator) {
      res.writeHead(501, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Authentication not enabled' }));
      return;
    }

    let body = '';
    req.on('data', (chunk) => {
      body += chunk.toString();
    });

    req.on('end', async () => {
      try {
        const authRequest: AuthRequest = JSON.parse(body);

        if (!authRequest.projectId || !authRequest.secretKey) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Missing projectId or secretKey' }));
          return;
        }

        const authResponse = await this.authenticator?.generateToken(authRequest);

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify(authResponse));

        logger.info('Authentication successful', {
          component: 'HTTPServer',
          projectId: authRequest.projectId,
          sessionId: authRequest.sessionId,
          userAgent: req.headers['user-agent'],
          remoteAddress: req.socket.remoteAddress,
        });
      } catch (error) {
        logger.error('Authentication failed', error as Error, {
          component: 'HTTPServer',
          userAgent: req.headers['user-agent'],
          remoteAddress: req.socket.remoteAddress,
        });

        res.writeHead(401, { 'Content-Type': 'application/json' });
        res.end(
          JSON.stringify({
            error: 'Authentication failed',
            message: error instanceof Error ? error.message : 'Unknown error',
          })
        );
      }
    });

    req.on('error', (error) => {
      logger.error('Authentication request error', error, {
        component: 'HTTPServer',
      });

      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Internal server error' }));
    });
  }

  private async validateToken(token: string, projectId: string): Promise<boolean> {
    if (!this.authenticator) {
      return false;
    }

    try {
      const payload = await this.authenticator.verifyToken(token);

      // Verify project ID matches
      if (payload.projectId !== projectId) {
        logger.warn('Token project ID mismatch', {
          component: 'WebSocketServer',
          tokenProjectId: payload.projectId,
          requestedProjectId: projectId,
        });
        return false;
      }

      // Check required permissions
      const requiredPermissions = ['file:read', 'file:write', 'lsp:query'];
      for (const permission of requiredPermissions) {
        if (!this.authenticator.hasPermission(payload, permission)) {
          logger.warn('Token missing required permission', {
            component: 'WebSocketServer',
            projectId,
            missingPermission: permission,
          });
          return false;
        }
      }

      return true;
    } catch (error) {
      logger.error('Token validation failed', error as Error, {
        component: 'WebSocketServer',
        projectId,
      });
      return false;
    }
  }

  private async handleMCPMessage(session: ClientSession, message: MCPMessage): Promise<any> {
    const startTime = Date.now();

    return logger.withTiming(
      `MCP ${message.method}`,
      async () => {
        // Add session to manager if not already tracked
        if (!this.sessionManager.getSession(session.id)) {
          this.sessionManager.addSession(session);
        }

        // Handle file change notifications
        if (message.method === 'server/fileChanged') {
          // Convert client absolute path to project-relative path
          const notification = {
            ...message.params,
            path: this.streamingFS.toProjectPath(session, message.params.path),
          };
          this.streamingFS.handleFileChanged(session, notification);
          return {};
        }

        // Look up the tool handler
        const toolInfo = toolRegistry.get(message.method);
        if (!toolInfo) {
          throw new Error(`Unknown tool: ${message.method}`);
        }

        // Prepare services for the handler
        const services: any = {};

        if (toolInfo.requiresService === 'symbol' || toolInfo.requiresService === 'file') {
          // For LSP-based tools, we need the appropriate LSP server
          const clientFilePath = message.params.file_path;
          if (clientFilePath && typeof clientFilePath === 'string') {
            // Convert client absolute path to project-relative path
            const projectPath = this.streamingFS.toProjectPath(session, clientFilePath);
            const extension = this.getFileExtension(projectPath);

            // Get workspace directory for enhanced sessions with FUSE support
            const enhancedSession = this.transport.getEnhancedSession(session.id);
            const workspaceDir = enhancedSession?.workspaceDir;

            const server = await this.lspServerPool.getServer(
              session.projectId,
              extension,
              workspaceDir
            );
            services.lspClient = this.lspClient;
            services.server = server;

            // Update the message params to use project-relative path for LSP operations
            message.params = {
              ...message.params,
              file_path: projectPath,
            };
          }
        }

        if (toolInfo.requiresService === 'file') {
          // For file-based tools, provide streaming file access
          services.fileAccess = this.streamingFS;
          services.session = session;
        }

        if (toolInfo.requiresService === 'batch') {
          // For batch tools, provide the tool registry and other services
          services.toolRegistry = toolRegistry;
          services.lspClient = this.lspClient;
          services.lspServerPool = this.lspServerPool;
          services.fileAccess = this.streamingFS;
          services.session = session;
        }

        // Execute the tool handler
        const result = await toolInfo.handler(message.params, services);

        // Release server reference if we used one
        if (services.server) {
          const filePath = message.params.file_path;
          if (filePath && typeof filePath === 'string') {
            const extension = this.getFileExtension(filePath);
            this.lspServerPool.releaseServer(session.projectId, extension);
          }
        }

        return result;
      },
      {
        component: 'WebSocketServer',
        sessionId: session.id,
        projectId: session.projectId,
        method: message.method,
      }
    );
  }

  private getFileExtension(filePath: string): string {
    const parts = filePath.split('.');
    if (parts.length > 1) {
      return parts[parts.length - 1]!;
    }
    return '';
  }

  getServerStats(): {
    clientCount: number;
    activeProjects: string[];
    activeServers: Array<{ projectId: string; language: string; refCount: number; lastUsed: Date }>;
  } {
    return {
      clientCount: this.clientCount,
      activeProjects: this.sessionManager.getActiveProjects(),
      activeServers: this.lspServerPool.getActiveServers(),
    };
  }

  /**
   * Handle workspace creation for enhanced sessions with FUSE support
   */
  private async handleWorkspaceCreate(session: ClientSession): Promise<WorkspaceInfo> {
    if (!this.workspaceManager) {
      throw new Error('Workspace manager not initialized - FUSE not enabled');
    }

    try {
      const workspaceInfo = await this.workspaceManager.createWorkspace(session);

      // Create and mount FUSE filesystem for this session
      if (workspaceInfo.fuseMount) {
        const fuseMount = new FuseMount(
          session as EnhancedClientSession,
          this.transport,
          workspaceInfo.fuseMount,
          {
            debugFuse: process.env.DEBUG_FUSE === 'true',
            allowOther: false,
            defaultPermissions: true,
          }
        );

        await fuseMount.mount();
        this.fuseMounts.set(session.id, fuseMount);

        logger.info('FUSE filesystem mounted for session', {
          component: 'CodeFlowWebSocketServer',
          sessionId: session.id,
          workspaceId: workspaceInfo.workspaceId,
          fuseMount: workspaceInfo.fuseMount,
        });
      }

      return workspaceInfo;
    } catch (error) {
      logger.error('Failed to create workspace', error as Error, {
        component: 'CodeFlowWebSocketServer',
        sessionId: session.id,
        projectId: session.projectId,
      });
      throw error;
    }
  }

  /**
   * Handle FUSE operation responses from clients
   */
  private handleFuseResponse(sessionId: string, response: FuseOperationResponse): void {
    const fuseMount = this.fuseMounts.get(sessionId);
    if (fuseMount) {
      fuseMount.handleFuseResponse(response);
    } else {
      logger.warn('Received FUSE response for session without mount', {
        component: 'CodeFlowWebSocketServer',
        sessionId,
        correlationId: response.correlationId,
      });
    }
  }

  async shutdown(): Promise<void> {
    logger.info('Starting server shutdown', { component: 'WebSocketServer' });

    // Unmount all FUSE filesystems
    const unmountPromises: Promise<void>[] = [];
    for (const [sessionId, fuseMount] of this.fuseMounts) {
      unmountPromises.push(
        fuseMount.unmount().catch((error) => {
          logger.error('Error unmounting FUSE during shutdown', error as Error, {
            component: 'WebSocketServer',
            sessionId,
          });
        })
      );
    }

    await Promise.allSettled(unmountPromises);
    this.fuseMounts.clear();

    // Shutdown workspace manager
    if (this.workspaceManager) {
      await this.workspaceManager.shutdown();
    }

    // Close all client connections
    this.wss.clients.forEach((ws) => {
      ws.close(1001, 'Server shutdown');
    });

    // Stop the WebSocket server
    await new Promise<void>((resolve) => {
      this.wss.close(() => resolve());
    });

    // Stop the HTTP server
    await new Promise<void>((resolve) => {
      this.httpServer.close(() => resolve());
    });

    // Shutdown LSP server pool
    await this.lspServerPool.shutdown();

    // Shutdown session manager
    this.sessionManager.shutdown();

    logger.info('Server shutdown complete', { component: 'WebSocketServer' });
  }
}
