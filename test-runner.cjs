#!/usr/bin/env node

const { spawn } = require('node:child_process');
const { getSystemCapabilities, printSystemInfo } = require('./test-system-utils.cjs');

// Detect system capabilities using shared utility
const capabilities = getSystemCapabilities();
const isSlowSystem = capabilities.isSlowSystem;

printSystemInfo(capabilities, 'Legacy Test Runner');
console.log('');

// Test configuration based on system
const config = {
  timeout: capabilities.baseTimeout * capabilities.timeoutMultiplier,
  parallel: !isSlowSystem,
  sharedServer: true,
  warmupDelay: isSlowSystem ? 10000 : 3000,
};

// Environment variables for tests
const testEnv = {
  ...process.env,
  TEST_MODE: isSlowSystem ? 'slow' : 'fast',
  TEST_SHARED_SERVER: 'true',
  TEST_TIMEOUT: config.timeout.toString(),
  BUN_TEST_TIMEOUT: config.timeout.toString(),
};

// Run tests with appropriate configuration
async function runTests() {
  const args = [
    'test',
    'tests/integration/call-hierarchy.test.ts',
    '--timeout',
    config.timeout.toString(),
  ];

  if (!config.parallel) {
    console.log('Running tests sequentially (slow system mode)...\n');
    // Force sequential execution in Bun
    args.push('--bail', '1'); // Stop on first failure
  } else {
    console.log('Running tests in parallel (fast system mode)...\n');
  }

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

// Main execution
(async () => {
  try {
    await runTests();
    console.log('\n✅ All tests passed!');
    process.exit(0);
  } catch (error) {
    console.error('\n❌ Tests failed:', error.message);
    process.exit(1);
  }
})();
