import { cpus } from 'node:os';

/**
 * System capabilities for test optimization
 */
export interface SystemCapabilities {
  // Raw metrics
  cpuCount: number;
  totalMemory: number;
  freeMemory: number;
  loadAverage: number;

  // Calculated metrics
  cpuUtilization: number;
  memoryUtilization: number;
  freeMemoryGB: number;
  totalMemoryGB: number;

  // Decision
  isSlowSystem: boolean;

  // Test timeouts
  timeoutMultiplier: number;
  baseTimeout: number;
}

/**
 * Analyze current system capabilities and load
 */
export function getSystemCapabilities(): SystemCapabilities {
  const cpuCount = cpus().length;
  const totalMemory = require('node:os').totalmem();
  const freeMemory = require('node:os').freemem();
  const loadAverage = require('node:os').loadavg()[0]; // 1-minute load average

  // Check available resources vs total
  const memoryUtilization = (totalMemory - freeMemory) / totalMemory;
  const cpuUtilization = loadAverage / cpuCount; // Load per CPU core

  // System is slow if:
  // - Few CPUs OR high CPU load (>0.7 load per core)
  // - Low total memory OR high memory usage (>80% used) OR low free memory (<2GB)
  const isSlowSystem =
    cpuCount <= 4 ||
    cpuUtilization > 0.7 ||
    totalMemory < 8 * 1024 * 1024 * 1024 ||
    memoryUtilization > 0.8 ||
    freeMemory < 2 * 1024 * 1024 * 1024;

  return {
    // Raw metrics
    cpuCount,
    totalMemory,
    freeMemory,
    loadAverage,

    // Calculated metrics
    cpuUtilization,
    memoryUtilization,
    freeMemoryGB: freeMemory / (1024 * 1024 * 1024),
    totalMemoryGB: totalMemory / (1024 * 1024 * 1024),

    // Decision
    isSlowSystem,

    // Test timeouts
    timeoutMultiplier: isSlowSystem ? 15 : 4, // 300s for slow, 80s for fast
    baseTimeout: isSlowSystem ? 20000 : 10000, // 20s for slow, 10s for fast
  };
}
