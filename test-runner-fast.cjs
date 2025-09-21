#!/usr/bin/env node

/**
 * Fast test runner with LSP server prewarming
 * Reduces test startup from 30s+ to ~5s
 */

const { spawn } = require('node:child_process');
const {
  getSystemCapabilities,
  printSystemInfo,
  printSlowSystemInfo,
} = require('./test-system-utils.cjs');

// Detect system capabilities using shared utility
const capabilities = getSystemCapabilities();
const isSlowSystem = capabilities.isSlowSystem;

printSystemInfo(capabilities, 'Fast Test Runner');

// Enhanced configuration for better performance
const config = {
  timeout: capabilities.baseTimeout * capabilities.timeoutMultiplier,
  parallel: !isSlowSystem,
  sharedServer: true,
  prewarming: !isSlowSystem, // Only prewarm on fast systems
  skipLSPPreload: isSlowSystem, // Skip LSP preload on slow systems
};

// Environment variables optimized for performance
const testEnv = {
  ...process.env,
  TEST_MODE: isSlowSystem ? 'slow' : 'fast',
  TEST_SHARED_SERVER: 'true',
  TEST_PREWARMING: config.prewarming.toString(),
  SKIP_LSP_PRELOAD: config.skipLSPPreload.toString(),
  TEST_MINIMAL_CONFIG: isSlowSystem ? 'true' : 'false',
  TEST_TIMEOUT: config.timeout.toString(),
  BUN_TEST_TIMEOUT: config.timeout.toString(),
  // Reduce test verbosity
  LOG_LEVEL: 'WARN',
  DEBUG: '',
  CODEBUDDY_DEBUG: '',
  // Optimize Node.js for testing
  NODE_OPTIONS: isSlowSystem ? '--max-old-space-size=2048' : '--max-old-space-size=4096',
};

// Get test files from command line or use defaults
const testFiles = process.argv.slice(2);
const defaultTests = [
  'tests/unit/restart-server.test.ts',
  'tests/core/quick.test.ts',
  'tests/integration/lsp-client.test.ts',
];

const testsToRun = testFiles.length > 0 ? testFiles : defaultTests;

async function runTests() {
  // Run pre-test validation first
  try {
    console.log('Running pre-test validation...');
    require('node:child_process').execSync('node scripts/pre-test-check.cjs', { stdio: 'inherit' });
    console.log('');
  } catch (error) {
    console.error('❌ Pre-test validation failed. Please fix the issues above.');
    process.exit(1);
  }

  console.log(`🔥 Starting tests with ${config.prewarming ? 'prewarming' : 'minimal'} mode...\n`);

  printSlowSystemInfo(capabilities);

  const args = ['test', ...testsToRun, '--timeout', config.timeout.toString()];

  if (!config.parallel) {
    console.log('Running tests sequentially (slow system mode)...');
    args.push('--bail', '1');
  } else {
    console.log('Running tests in parallel (fast system mode)...');
  }

  console.log(`Command: bun ${args.join(' ')}\n`);

  const proc = spawn('bun', args, {
    env: testEnv,
    stdio: 'inherit',
  });

  return new Promise((resolve, reject) => {
    proc.on('exit', (code) => {
      if (code === 0) {
        resolve(code);
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
    console.error(`\n❌ Tests failed after ${(elapsed / 1000).toFixed(1)}s:`, error.message);
    process.exit(1);
  }
})();
