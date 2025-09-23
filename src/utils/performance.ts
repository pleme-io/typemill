/**
 * Performance measurement utilities for MCP tools
 * Provides consistent timing and performance monitoring across all handlers
 */

import { logger } from '../core/diagnostics/logger.js';

/**
 * Performance measurement result
 */
export interface PerformanceResult<T = unknown> {
  result: T;
  duration: number;
  success: boolean;
}

/**
 * Performance measurement options
 */
export interface PerformanceOptions {
  /** Log the timing result */
  logResult?: boolean;
  /** Minimum duration (ms) to log (helps filter out noise) */
  logThreshold?: number;
  /** Additional context for logging */
  context?: Record<string, unknown>;
  /** Whether to log success/failure status */
  logStatus?: boolean;
}

/**
 * Measure the performance of an async operation
 * @param toolName Name of the MCP tool being measured
 * @param operation The async operation to measure
 * @param options Performance measurement options
 * @returns Performance result with timing information
 */
export async function measurePerformance<T>(
  toolName: string,
  operation: () => Promise<T>,
  options: PerformanceOptions = {}
): Promise<PerformanceResult<T>> {
  const {
    logResult = true,
    logThreshold = 0,
    context = {},
    logStatus = true,
  } = options;

  const startTime = performance.now();
  let success = false;
  let result: T;

  try {
    result = await operation();
    success = true;

    const duration = performance.now() - startTime;

    // Log if enabled and above threshold
    if (logResult && duration >= logThreshold) {
      const logContext = {
        tool: toolName,
        duration: Math.round(duration * 100) / 100, // Round to 2 decimal places
        success,
        ...context,
      };

      if (logStatus) {
        logger.info(`Tool ${toolName} completed`, logContext);
      } else {
        logger.debug(`Tool ${toolName} timing`, logContext);
      }
    }

    return {
      result,
      duration: Math.round(duration * 100) / 100,
      success,
    };
  } catch (error) {
    const duration = performance.now() - startTime;

    // Log error with timing
    if (logResult) {
      const logContext = {
        tool: toolName,
        duration: Math.round(duration * 100) / 100,
        success: false,
        ...context,
      };

      logger.error(`Tool ${toolName} failed`, error as Error, logContext);
    }

    // Re-throw the error with timing information
    const errorWithTiming = error as Error & { duration?: number };
    errorWithTiming.duration = Math.round(duration * 100) / 100;
    throw errorWithTiming;
  }
}

/**
 * Measure the performance of a sync operation
 * @param toolName Name of the operation being measured
 * @param operation The sync operation to measure
 * @param options Performance measurement options
 * @returns Performance result with timing information
 */
export function measureSyncPerformance<T>(
  toolName: string,
  operation: () => T,
  options: PerformanceOptions = {}
): PerformanceResult<T> {
  const {
    logResult = true,
    logThreshold = 0,
    context = {},
    logStatus = true,
  } = options;

  const startTime = performance.now();
  let success = false;
  let result: T;

  try {
    result = operation();
    success = true;

    const duration = performance.now() - startTime;

    // Log if enabled and above threshold
    if (logResult && duration >= logThreshold) {
      const logContext = {
        tool: toolName,
        duration: Math.round(duration * 100) / 100,
        success,
        ...context,
      };

      if (logStatus) {
        logger.info(`Operation ${toolName} completed`, logContext);
      } else {
        logger.debug(`Operation ${toolName} timing`, logContext);
      }
    }

    return {
      result,
      duration: Math.round(duration * 100) / 100,
      success,
    };
  } catch (error) {
    const duration = performance.now() - startTime;

    // Log error with timing
    if (logResult) {
      const logContext = {
        tool: toolName,
        duration: Math.round(duration * 100) / 100,
        success: false,
        ...context,
      };

      logger.error(`Operation ${toolName} failed`, error as Error, logContext);
    }

    // Re-throw the error with timing information
    const errorWithTiming = error as Error & { duration?: number };
    errorWithTiming.duration = Math.round(duration * 100) / 100;
    throw errorWithTiming;
  }
}

/**
 * Higher-order function to wrap MCP tool handlers with performance measurement
 * @param toolName Name of the MCP tool
 * @param handler The handler function to wrap
 * @param options Performance measurement options
 * @returns Wrapped handler with performance measurement
 */
export function withPerformanceMeasurement<TArgs, TResult>(
  toolName: string,
  handler: (args: TArgs) => Promise<TResult>,
  options: PerformanceOptions = {}
): (args: TArgs) => Promise<TResult> {
  return async (args: TArgs): Promise<TResult> => {
    const result = await measurePerformance(
      toolName,
      () => handler(args),
      {
        ...options,
        context: {
          ...options.context,
          args: typeof args === 'object' && args !== null ? Object.keys(args) : undefined,
        },
      }
    );
    return result.result;
  };
}

/**
 * Performance tracking class for multiple measurements
 */
export class PerformanceTracker {
  private measurements: Array<{
    toolName: string;
    duration: number;
    success: boolean;
    timestamp: number;
    context?: Record<string, unknown>;
  }> = [];

  /**
   * Add a measurement to the tracker
   */
  addMeasurement(
    toolName: string,
    duration: number,
    success: boolean,
    context?: Record<string, unknown>
  ): void {
    this.measurements.push({
      toolName,
      duration,
      success,
      timestamp: Date.now(),
      context,
    });
  }

  /**
   * Get performance statistics
   */
  getStats(): {
    totalMeasurements: number;
    successRate: number;
    averageDuration: number;
    slowestTool: { name: string; duration: number } | null;
    fastestTool: { name: string; duration: number } | null;
    byTool: Record<string, {
      count: number;
      avgDuration: number;
      successRate: number;
      totalDuration: number;
    }>;
  } {
    if (this.measurements.length === 0) {
      return {
        totalMeasurements: 0,
        successRate: 0,
        averageDuration: 0,
        slowestTool: null,
        fastestTool: null,
        byTool: {},
      };
    }

    const totalDuration = this.measurements.reduce((sum, m) => sum + m.duration, 0);
    const successCount = this.measurements.filter(m => m.success).length;
    const sorted = [...this.measurements].sort((a, b) => b.duration - a.duration);

    // Group by tool
    const byTool: Record<string, Array<typeof this.measurements[0]>> = {};
    for (const measurement of this.measurements) {
      if (!byTool[measurement.toolName]) {
        byTool[measurement.toolName] = [];
      }
      byTool[measurement.toolName]!.push(measurement);
    }

    const toolStats: Record<string, {
      count: number;
      avgDuration: number;
      successRate: number;
      totalDuration: number;
    }> = {};

    for (const [toolName, measurements] of Object.entries(byTool)) {
      const toolTotalDuration = measurements.reduce((sum, m) => sum + m.duration, 0);
      const toolSuccessCount = measurements.filter(m => m.success).length;

      toolStats[toolName] = {
        count: measurements.length,
        avgDuration: toolTotalDuration / measurements.length,
        successRate: toolSuccessCount / measurements.length,
        totalDuration: toolTotalDuration,
      };
    }

    return {
      totalMeasurements: this.measurements.length,
      successRate: successCount / this.measurements.length,
      averageDuration: totalDuration / this.measurements.length,
      slowestTool: sorted.length > 0 ? { name: sorted[0]!.toolName, duration: sorted[0]!.duration } : null,
      fastestTool: sorted.length > 0 ? { name: sorted[sorted.length - 1]!.toolName, duration: sorted[sorted.length - 1]!.duration } : null,
      byTool: toolStats,
    };
  }

  /**
   * Clear all measurements
   */
  clear(): void {
    this.measurements = [];
  }

  /**
   * Get recent measurements (last N measurements)
   */
  getRecent(count: number): Array<typeof this.measurements[0]> {
    return this.measurements.slice(-count);
  }

  /**
   * Log performance summary
   */
  logSummary(): void {
    const stats = this.getStats();

    if (stats.totalMeasurements === 0) {
      logger.info('No performance measurements recorded');
      return;
    }

    logger.info('Performance Summary', {
      total_measurements: stats.totalMeasurements,
      success_rate: Math.round(stats.successRate * 100) / 100,
      average_duration: Math.round(stats.averageDuration * 100) / 100,
      slowest_tool: stats.slowestTool,
      fastest_tool: stats.fastestTool,
    });

    // Log per-tool stats
    for (const [toolName, toolStats] of Object.entries(stats.byTool)) {
      logger.debug(`Tool ${toolName} performance`, {
        count: toolStats.count,
        avg_duration: Math.round(toolStats.avgDuration * 100) / 100,
        success_rate: Math.round(toolStats.successRate * 100) / 100,
        total_duration: Math.round(toolStats.totalDuration * 100) / 100,
      });
    }
  }
}

/**
 * Global performance tracker instance
 */
export const globalPerformanceTracker = new PerformanceTracker();

/**
 * Utility to measure and track performance automatically
 */
export async function measureAndTrack<T>(
  toolName: string,
  operation: () => Promise<T>,
  options: PerformanceOptions = {}
): Promise<T> {
  const result = await measurePerformance(toolName, operation, options);

  globalPerformanceTracker.addMeasurement(
    toolName,
    result.duration,
    result.success,
    options.context
  );

  return result.result;
}