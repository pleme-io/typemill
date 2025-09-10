#!/usr/bin/env node

/**
 * Shared system detection utilities for test runners
 */

const { cpus } = require('node:os');

/**
 * Analyze current system capabilities and load
 */
function getSystemCapabilities() {
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

/**
 * Get human-readable reason why system is considered slow
 */
function getSlowSystemReason(capabilities) {
  if (!capabilities.isSlowSystem) return null;

  if (capabilities.cpuCount <= 4) return 'Low CPU count';
  if (capabilities.cpuUtilization > 0.7) return 'High CPU load';
  if (capabilities.totalMemory < 8 * 1024 * 1024 * 1024) return 'Low total memory';
  if (capabilities.memoryUtilization > 0.8) return 'High memory usage';
  if (capabilities.freeMemory < 2 * 1024 * 1024 * 1024) return 'Low free memory';
  return 'Multiple factors';
}

/**
 * Print system diagnostics
 */
function printSystemInfo(capabilities, title = 'System Info') {
  console.log(`🚀 ${title}`);
  console.log(
    `System: ${capabilities.cpuCount} CPUs, ${capabilities.totalMemoryGB.toFixed(1)}GB total, ${capabilities.freeMemoryGB.toFixed(1)}GB free`
  );
  console.log(
    `Load: ${(capabilities.cpuUtilization * 100).toFixed(1)}% CPU, ${(capabilities.memoryUtilization * 100).toFixed(1)}% RAM`
  );
  console.log(`Mode: ${capabilities.isSlowSystem ? 'SLOW' : 'FAST'}`);
}

/**
 * Print slow system diagnostics
 */
function printSlowSystemInfo(capabilities) {
  if (!capabilities.isSlowSystem) return;

  console.log('⚠️  SLOW SYSTEM MODE: Optimized for reliability over speed');
  console.log(`   - Reason: ${getSlowSystemReason(capabilities)}`);
  console.log('   - LSP preload: DISABLED');
  console.log('   - Timeouts: 5+ minutes');
  console.log('   - Memory: Reduced limits');
}

module.exports = {
  getSystemCapabilities,
  getSlowSystemReason,
  printSystemInfo,
  printSlowSystemInfo,
};
