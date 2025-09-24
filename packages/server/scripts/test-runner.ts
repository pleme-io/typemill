#!/usr/bin/env bun

/**
 * Unified test runner with multiple modes and flag support
 */

import { execSync, spawn } from 'node:child_process';
import { getSystemCapabilities, printSlowSystemInfo, printSystemInfo } from './test-utils.ts';

// Parse command line arguments
const args = process.argv.slice(2);
const isDebug = args.includes('--debug');
const isVerbose = args.includes('--verbose') || args.includes('-v');
const isFast = args.includes('--fast');
const isMinimal = args.includes('--minimal');

// Remove our custom flags from args to pass to bun test
const bunArgs = args.filter(
  (arg) => !['--debug', '--verbose', '-v', '--fast', '--minimal'].includes(arg)
);

// Get system capabilities
const capabilities = getSystemCapabilities();
const isSlowSystem = capabilities.isSlowSystem;

// Determine mode
let mode: 'fast' | 'minimal' | 'auto';
if (isMinimal) {
  mode = 'minimal';
} else if (isFast) {
  mode = 'fast';
} else {
  mode = 'auto'; // Use system detection
}

// Configuration based on mode
const getConfig = () => {
  const baseConfig = {
    timeout: capabilities.baseTimeout * capabilities.timeoutMultiplier,
    parallel: !isSlowSystem,
    logLevel: isDebug ? 'DEBUG' : isVerbose ? 'INFO' : 'WARN',
  };

  switch (mode) {
    case 'minimal':
      return {
        ...baseConfig,
        timeout: 600000, // 10 minutes max
        parallel: false, // Always sequential
        skipPreload: true,
        minimalConfig: true,
        logLevel: isDebug ? 'DEBUG' : 'ERROR', // Ultra-quiet unless debug
      };

    case 'fast':
      return {
        ...baseConfig,
        prewarming: !isSlowSystem,
        skipLSPPreload: isSlowSystem,
      };

    default: // auto
      return {
        ...baseConfig,
        prewarming: !isSlowSystem,
        skipLSPPreload: isSlowSystem,
      };
  }
};

const config = getConfig();

// Set up environment
const testEnv = {
  ...process.env,
  TEST_MODE: isSlowSystem ? 'slow' : 'fast',
  TEST_SHARED_SERVER: 'true',
  TEST_PREWARMING: config.prewarming?.toString() || 'false',
  SKIP_LSP_PRELOAD: config.skipPreload?.toString() || config.skipLSPPreload?.toString() || 'false',
  TEST_MINIMAL_CONFIG: config.minimalConfig?.toString() || 'false',
  TEST_TIMEOUT: config.timeout.toString(),
  BUN_TEST_TIMEOUT: config.timeout.toString(),
  // Logging configuration
  LOG_LEVEL: config.logLevel,
  DEBUG: isDebug ? '*' : '',
  CODEBUDDY_DEBUG: isDebug ? '1' : '',
  // Memory optimization
  NODE_OPTIONS: isSlowSystem ? '--max-old-space-size=2048' : '--max-old-space-size=4096',
};

// Get test files from command line or use defaults
const defaultTests = [
  'tests/unit/restart-server.test.ts',
  'tests/core/comprehensive.test.ts',
  'tests/e2e/lsp-client.test.ts',
];

const testsToRun = bunArgs.length > 0 ? bunArgs : defaultTests;

async function runPreTestCheck(): Promise<void> {
  try {
    console.log('Running pre-test validation...');
    execSync('bun scripts/pre-test-check.ts', { stdio: 'inherit' });
    console.log('');
  } catch (_error) {
    console.error('❌ Pre-test validation failed. Please fix the issues above.');
    process.exit(1);
  }
}

async function runTests(): Promise<void> {
  // Run pre-test validation first
  await runPreTestCheck();

  // Print system information
  const modeString = mode === 'auto' ? (isSlowSystem ? 'auto (slow)' : 'auto (fast)') : mode;
  printSystemInfo(capabilities, `Test Runner (${modeString} mode)`);

  if (mode === 'minimal') {
    console.log('🐌 MINIMAL MODE ENABLED:');
    console.log('   - LSP preload: DISABLED');
    console.log('   - Config: Minimal (TypeScript only)');
    console.log('   - Timeout: 10 minutes per test');
    console.log('   - Memory: 2GB limit');
    console.log('   - Execution: Sequential only');
    console.log('   - Logging: ERROR level only\n');
  } else {
    console.log(`🔥 Starting tests with ${config.prewarming ? 'prewarming' : 'minimal'} mode...\n`);
    printSlowSystemInfo(capabilities);
  }

  // Prepare bun test arguments
  const testArgs = ['test', ...testsToRun, '--timeout', config.timeout.toString()];

  if (!config.parallel) {
    console.log('Running tests sequentially (slow system mode)...');
    testArgs.push('--bail', '1');
  } else {
    console.log('Running tests in parallel (fast system mode)...');
  }

  console.log(`Command: bun ${testArgs.join(' ')}\n`);

  // Spawn the test process
  const proc = spawn('bun', testArgs, {
    env: testEnv,
    stdio: 'inherit',
  });

  return new Promise((resolve, reject) => {
    proc.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`Tests failed with code ${code}`));
      }
    });
  });
}

// Main execution with timing
(async () => {
  const startTime = Date.now();

  try {
    await runTests();
    const elapsed = Date.now() - startTime;
    console.log(`\n✅ All tests passed in ${(elapsed / 1000).toFixed(1)}s!`);
    process.exit(0);
  } catch (error) {
    const elapsed = Date.now() - startTime;
    console.error(
      `\n❌ Tests failed after ${(elapsed / 1000).toFixed(1)}s:`,
      (error as Error).message
    );
    process.exit(1);
  }
})();
