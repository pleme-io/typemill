/**
 * Structured logging system for CodeFlow Buddy Phase 2
 * Provides consistent logging across all components
 */

export interface LogContext {
  component?: string;
  sessionId?: string;
  projectId?: string;
  method?: string;
  duration?: number;
  [key: string]: unknown;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
  context: LogContext;
}

export class StructuredLogger {
  private static instance: StructuredLogger;
  private logLevel: number = 1; // INFO level

  private constructor() {
    // Set log level from environment
    const envLevel = process.env.LOG_LEVEL?.toUpperCase();
    switch (envLevel) {
      case 'DEBUG':
        this.logLevel = 0;
        break;
      case 'INFO':
        this.logLevel = 1;
        break;
      case 'WARN':
        this.logLevel = 2;
        break;
      case 'ERROR':
        this.logLevel = 3;
        break;
      default:
        this.logLevel = 1;
    }
  }

  static getInstance(): StructuredLogger {
    if (!StructuredLogger.instance) {
      StructuredLogger.instance = new StructuredLogger();
    }
    return StructuredLogger.instance;
  }

  private shouldLog(level: number): boolean {
    return level >= this.logLevel;
  }

  private formatLogEntry(level: number, message: string, context: LogContext = {}): LogEntry {
    const levels = ['DEBUG', 'INFO', 'WARN', 'ERROR'];
    return {
      timestamp: new Date().toISOString(),
      level: levels[level] || 'INFO',
      message,
      context,
    };
  }

  private writeLog(entry: LogEntry): void {
    const output = JSON.stringify(entry);

    // In production, you might want to send this to a log aggregation service
    // For now, we'll use console output with different methods for different levels
    switch (entry.level) {
      case 'ERROR':
        console.error(output);
        break;
      case 'WARN':
        console.warn(output);
        break;
      case 'DEBUG':
        console.debug(output);
        break;
      default:
        console.log(output);
    }
  }

  debug(message: string, context: LogContext = {}): void {
    if (this.shouldLog(0)) {
      this.writeLog(this.formatLogEntry(0, message, context));
    }
  }

  info(message: string, context: LogContext = {}): void {
    if (this.shouldLog(1)) {
      this.writeLog(this.formatLogEntry(1, message, context));
    }
  }

  warn(message: string, context: LogContext = {}): void {
    if (this.shouldLog(2)) {
      this.writeLog(this.formatLogEntry(2, message, context));
    }
  }

  error(message: string, error?: Error, context: LogContext = {}): void {
    if (this.shouldLog(3)) {
      const errorContext = {
        ...context,
        ...(error && {
          error: {
            name: error.name,
            message: error.message,
            stack: error.stack,
          },
        }),
      };
      this.writeLog(this.formatLogEntry(3, message, errorContext));
    }
  }

  // Convenience method for timing operations
  async withTiming<T>(
    operation: string,
    fn: () => Promise<T>,
    context: LogContext = {}
  ): Promise<T> {
    const startTime = Date.now();
    const operationContext = { ...context, operation };

    this.debug(`Starting ${operation}`, operationContext);

    try {
      const result = await fn();
      const duration = Date.now() - startTime;

      this.info(`Completed ${operation}`, {
        ...operationContext,
        duration,
        success: true,
      });

      return result;
    } catch (error) {
      const duration = Date.now() - startTime;

      this.error(`Failed ${operation}`, error as Error, {
        ...operationContext,
        duration,
        success: false,
      });

      throw error;
    }
  }

  // Method for logging connection events
  logConnection(event: 'connect' | 'disconnect' | 'reconnect', context: LogContext): void {
    this.info(`Client ${event}`, {
      ...context,
      event_type: 'connection',
    });
  }

  // Method for logging MCP tool usage
  logMCPTool(tool: string, duration: number, success: boolean, context: LogContext): void {
    this.info(`MCP tool ${tool} ${success ? 'succeeded' : 'failed'}`, {
      ...context,
      event_type: 'mcp_tool',
      tool,
      duration,
      success,
    });
  }

  // Method for logging LSP server events
  logLSPServer(
    event: 'start' | 'stop' | 'crash' | 'restart',
    serverKey: string,
    context: LogContext = {}
  ): void {
    const level = event === 'crash' ? 3 : 1; // ERROR : INFO
    const message = `LSP server ${event}: ${serverKey}`;

    if (level === 3) {
      this.error(message, undefined, { ...context, event_type: 'lsp_server', serverKey, event });
    } else {
      this.info(message, { ...context, event_type: 'lsp_server', serverKey, event });
    }
  }
}

// Export singleton instance
export const logger = StructuredLogger.getInstance();
