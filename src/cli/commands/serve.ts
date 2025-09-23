import { CodeFlowWebSocketServer, type TLSOptions } from '../../server/ws-server.js';
import { logger } from '../../core/diagnostics/logger.js';

export interface ServeOptions {
  port?: number;
  maxClients?: number;
  requireAuth?: boolean;
  jwtSecret?: string;
  tlsKey?: string;
  tlsCert?: string;
  tlsCa?: string;
  enableFuse?: boolean;
  allowedOrigins?: string[];
  allowedCorsOrigins?: string[];
  logLevel?: 'debug' | 'info' | 'warn' | 'error';
  workspaceConfig?: {
    baseWorkspaceDir?: string;
    fuseMountPrefix?: string;
    maxWorkspaces?: number;
    workspaceTimeoutMs?: number;
  };
}

export async function serveCommand(options: ServeOptions = {}): Promise<void> {
  // Configure logging level if provided
  const logLevel = options.logLevel || process.env.LOG_LEVEL;
  if (logLevel) {
    process.env.LOG_LEVEL = logLevel.toUpperCase();
    // Re-instantiate logger to pick up new level (if needed)
  }

  const port = options.port || 3000;
  const maxClients = options.maxClients || 10;
  const requireAuth = options.requireAuth || false;
  const jwtSecret = options.jwtSecret;
  const enableFuse = options.enableFuse || process.env.ENABLE_FUSE === 'true';
  const allowedOrigins =
    options.allowedOrigins ||
    (process.env.ALLOWED_ORIGINS ? process.env.ALLOWED_ORIGINS.split(',') : undefined);
  const allowedCorsOrigins = options.allowedCorsOrigins || allowedOrigins;

  // Configure TLS if both key and cert are provided
  let tls: TLSOptions | undefined;
  if (options.tlsKey && options.tlsCert) {
    tls = {
      keyPath: options.tlsKey,
      certPath: options.tlsCert,
      caPath: options.tlsCa,
    };
  }

  const protocol = tls ? 'WSS (Secure WebSocket)' : 'WS (WebSocket)';

  logger.info('Starting CodeFlow WebSocket server', {
    component: 'server',
    protocol,
    port,
    max_clients: maxClients,
    authentication: requireAuth ? 'Enabled' : 'Disabled',
    tls: tls ? 'Enabled' : 'Disabled',
    fuse_isolation: enableFuse ? 'Enabled' : 'Disabled',
    log_level: logLevel || 'default',
  });

  if (tls) {
    logger.info('TLS configuration loaded', {
      component: 'server',
      tls_key: tls.keyPath,
      tls_certificate: tls.certPath,
      ca_certificate: tls.caPath || 'none',
      client_cert_validation: !!tls.caPath,
    });
  }

  if (requireAuth && !jwtSecret) {
    logger.warn('Authentication enabled but no JWT secret provided', {
      component: 'server',
      message: 'Using auto-generated secret',
    });
  }

  const server = new CodeFlowWebSocketServer({
    port,
    maxClients,
    requireAuth,
    jwtSecret,
    tls,
    enableFuse,
    allowedOrigins,
    allowedCorsOrigins,
    workspaceConfig: options.workspaceConfig,
  });

  // Handle graceful shutdown
  const shutdown = async () => {
    logger.info('Shutting down server', { component: 'server' });
    await server.shutdown();
    process.exit(0);
  };

  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);

  // Keep the process alive
  process.on('exit', () => {
    logger.info('Server process exiting', { component: 'server' });
  });

  // Log server stats periodically
  setInterval(() => {
    const stats = server.getServerStats();
    logger.debug('Server statistics', {
      component: 'server',
      client_count: stats.clientCount,
      active_projects: stats.activeProjects.length,
      active_servers: stats.activeServers.length,
      stats_type: 'periodic',
    });
  }, 30000); // Every 30 seconds
}
