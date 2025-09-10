/**
 * Structured logging system with AsyncLocalStorage context propagation
 *
 * Features:
 * - Automatic context inheritance via AsyncLocalStorage
 * - Environment variable configuration (LOG_LEVEL, LOG_OUTPUT, LOG_FORMAT, LOG_TAGS)
 * - JSON and human-readable formats
 * - Container-native (stdout/stderr by default)
 * - Tag-based filtering using context keys
 */

import { AsyncLocalStorage } from 'node:async_hooks';

// Log levels in order of severity
export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
}

export interface LogContext {
  // Request-level context
  request_id?: string;
  session_id?: string;
  user_id?: string;
  method?: string;
  path?: string;

  // Operation-level context
  operation?: string;
  component?: string;
  batch_id?: string;
  duration_ms?: number;

  // Error-level context
  error_code?: string;
  error_type?: string;
  status_code?: number;

  // Additional structured context
  [key: string]: string | number | boolean | undefined;
}

interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
  context: LogContext;
}

type LogOutput = 'console' | 'file' | 'both';
type LogFormat = 'json' | 'human' | 'auto';

class StructuredLogger {
  private static contextStorage = new AsyncLocalStorage<LogContext>();
  private level: LogLevel;
  private output: LogOutput;
  private format: LogFormat;
  private tagFilters: Set<string> | null = null;
  private moduleName: string;

  constructor(moduleName: string) {
    this.moduleName = moduleName;

    // Parse environment variables
    this.level = this.parseLogLevel(process.env.LOG_LEVEL || 'INFO');
    this.output = this.parseLogOutput(process.env.LOG_OUTPUT || 'console');
    this.format = this.parseLogFormat(process.env.LOG_FORMAT || 'auto');

    // Parse LOG_TAGS for filtering
    if (process.env.LOG_TAGS && process.env.LOG_TAGS !== '*') {
      this.tagFilters = new Set(process.env.LOG_TAGS.split(',').map((tag) => tag.trim()));
    }
  }

  private parseLogLevel(level: string): LogLevel {
    const levelMap: Record<string, LogLevel> = {
      DEBUG: LogLevel.DEBUG,
      INFO: LogLevel.INFO,
      WARN: LogLevel.WARN,
      ERROR: LogLevel.ERROR,
    };
    return levelMap[level.toUpperCase()] ?? LogLevel.INFO;
  }

  private parseLogOutput(output: string): LogOutput {
    const validOutputs: LogOutput[] = ['console', 'file', 'both'];
    return validOutputs.includes(output as LogOutput) ? (output as LogOutput) : 'console';
  }

  private parseLogFormat(format: string): LogFormat {
    if (format === 'auto') {
      // Auto-detect: JSON in production, human-readable otherwise
      return process.env.NODE_ENV === 'production' || process.env.ENV === 'production'
        ? 'json'
        : 'human';
    }
    const validFormats: LogFormat[] = ['json', 'human'];
    return validFormats.includes(format as LogFormat) ? (format as LogFormat) : 'human';
  }

  private shouldLog(level: LogLevel, context: LogContext): boolean {
    // Check log level
    if (level < this.level) {
      return false;
    }

    // Check tag filters
    if (this.tagFilters) {
      const contextMatches =
        this.tagFilters.size === 0 ||
        Object.entries(context).some(
          ([key, value]) =>
            this.tagFilters?.has(`${key}:${value}`) || this.tagFilters?.has(`${key}:*`)
        );

      if (!contextMatches) {
        return false;
      }
    }

    return true;
  }

  private formatMessage(entry: LogEntry): string {
    if (this.format === 'json') {
      return JSON.stringify(entry);
    }

    // Human-readable format
    const contextStr =
      Object.keys(entry.context).length > 0 ? ` ${JSON.stringify(entry.context)}` : '';

    return `[${entry.timestamp}] ${entry.level.padEnd(5)} [${this.moduleName}] ${entry.message}${contextStr}`;
  }

  private writeLog(entry: LogEntry): void {
    const message = this.formatMessage(entry);
    const isError = entry.level === 'ERROR' || entry.level === 'WARN';

    if (this.output === 'console' || this.output === 'both') {
      // Container-native: INFO/DEBUG → stdout, WARN/ERROR → stderr
      if (isError) {
        console.error(message);
      } else {
        console.log(message);
      }
    }

    if (this.output === 'file' || this.output === 'both') {
      // File logging would be implemented here
      // For now, skip in container environments
      if (!process.env.CONTAINER && !process.env.NODE_ENV?.includes('production')) {
        try {
          const { appendFileSync } = require('node:fs');
          const { join } = require('node:path');
          const logPath = join(process.cwd(), '.codebuddy', 'structured.log');
          appendFileSync(logPath, `${message}\n`);
        } catch {
          // Fail silently for file logging
        }
      }
    }
  }

  private log(level: LogLevel, message: string, additionalContext?: Partial<LogContext>): void {
    // Get current context from AsyncLocalStorage
    const currentContext = StructuredLogger.contextStorage.getStore() || {};

    // Merge with additional context, ensuring component is set
    const context: LogContext = {
      component: this.moduleName,
      ...currentContext,
      ...additionalContext,
    };

    if (!this.shouldLog(level, context)) {
      return;
    }

    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level: LogLevel[level],
      message,
      context,
    };

    this.writeLog(entry);
  }

  // Public logging methods
  debug(message: string, context?: Partial<LogContext>): void {
    this.log(LogLevel.DEBUG, message, context);
  }

  info(message: string, context?: Partial<LogContext>): void {
    this.log(LogLevel.INFO, message, context);
  }

  warn(message: string, context?: Partial<LogContext>): void {
    this.log(LogLevel.WARN, message, context);
  }

  error(message: string, error?: unknown, context?: Partial<LogContext>): void {
    let enrichedContext = { ...context };

    // Auto-extract error details
    if (error) {
      if (error instanceof Error) {
        enrichedContext = {
          ...enrichedContext,
          error_type: error.constructor.name,
          error_message: error.message,
          stack_trace: error.stack,
        };
      } else {
        enrichedContext = {
          ...enrichedContext,
          error_details: String(error),
        };
      }
    }

    this.log(LogLevel.ERROR, message, enrichedContext);
  }

  // Context management methods
  static withContext<T>(context: Partial<LogContext>, fn: () => T): T {
    const currentContext = StructuredLogger.contextStorage.getStore() || {};
    const newContext = { ...currentContext, ...context };
    return StructuredLogger.contextStorage.run(newContext, fn);
  }

  static async withContextAsync<T>(context: Partial<LogContext>, fn: () => Promise<T>): Promise<T> {
    const currentContext = StructuredLogger.contextStorage.getStore() || {};
    const newContext = { ...currentContext, ...context };
    return StructuredLogger.contextStorage.run(newContext, fn);
  }

  static getContext(): LogContext {
    return StructuredLogger.contextStorage.getStore() || {};
  }

  static setContext(context: Partial<LogContext>): void {
    const currentContext = StructuredLogger.contextStorage.getStore() || {};
    const newContext = { ...currentContext, ...context };
    StructuredLogger.contextStorage.enterWith(newContext);
  }

  // Child logger for component-specific context
  child(additionalContext: Partial<LogContext>): StructuredLogger {
    const childLogger = new StructuredLogger(this.moduleName);
    childLogger.level = this.level;
    childLogger.output = this.output;
    childLogger.format = this.format;
    childLogger.tagFilters = this.tagFilters;

    // The child will automatically inherit context via AsyncLocalStorage
    return childLogger;
  }

  // Test-friendly methods
  static suppressLogsInTests(): void {
    if (process.env.NODE_ENV === 'test' || process.env.JEST_WORKER_ID) {
      // Override console methods to suppress output
      console.log = () => {};
      console.error = () => {};
    }
  }
}

// Factory function for creating module-scoped loggers
export function getLogger(moduleName: string): StructuredLogger {
  return new StructuredLogger(moduleName);
}

// Middleware helper for request context injection
export function createRequestContext(
  requestId?: string,
  method?: string,
  path?: string
): Partial<LogContext> {
  return {
    request_id: requestId || `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    method,
    path,
    operation: 'request',
  };
}

// Export the class for advanced use cases
export { StructuredLogger };
