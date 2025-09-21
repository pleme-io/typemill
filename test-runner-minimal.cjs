#!/usr/bin/env node

/**
 * Ultra-minimal test runner for slow systems
 * Skips all heavy features for maximum speed
 */

const { spawn } = require('node:child_process');
const { getSystemCapabilities, printSystemInfo } = require('./test-system-utils.cjs');

// System detection using shared utility
const capabilities = getSystemCapabilities();

printSystemInfo(capabilities, 'Ultra-Minimal Test Runner (Slow System Mode)');

// Ultra-conservative configuration
const config = {
  timeout: 600000, // 10 minutes max
  parallel: false, // Always sequential
  skipPreload: true,
  minimalConfig: true,
};

// Minimal environment
const testEnv = {
  ...process.env,
  TEST_MODE: 'slow',
  TEST_SHARED_SERVER: 'true',
  SKIP_LSP_PRELOAD: 'true',
  TEST_MINIMAL_CONFIG: 'true',
  TEST_TIMEOUT: config.timeout.toString(),
  BUN_TEST_TIMEOUT: config.timeout.toString(),
  // Ultra-quiet logging
  LOG_LEVEL: 'ERROR',
  DEBUG: '',
  CODEBUDDY_DEBUG: '',
  NODE_OPTIONS: '--max-old-space-size=1024', // Minimal memory
};

// Only run essential tests
const essentialTests = [
  'tests/core/quick.test.ts', // Basic functionality
];

// Override with command line args if provided
const testsToRun = process.argv.slice(2).length > 0 ? process.argv.slice(2) : essentialTests;

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

  console.log('🐌 SLOW SYSTEM MODE ENABLED:');
  console.log('   - LSP preload: DISABLED');
  console.log('   - Config: Minimal (TypeScript only)');
  console.log('   - Timeout: 10 minutes per test');
  console.log('   - Memory: 1GB limit');
  console.log('   - Execution: Sequential only\n');

  const args = [
    'test',
    ...testsToRun,
    '--timeout',
    config.timeout.toString(),
    '--bail',
    '1', // Stop on first failure
  ];

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

// Main execution
(async () => {
  const startTime = Date.now();

  try {
    await runTests();
    const elapsed = Date.now() - startTime;
    console.log(`\n✅ Tests completed in ${(elapsed / 1000).toFixed(1)}s!`);
    process.exit(0);
  } catch (error) {
    const elapsed = Date.now() - startTime;
    console.error(`\n❌ Tests failed after ${(elapsed / 1000).toFixed(1)}s:`, error.message);
    process.exit(1);
  }
})();
