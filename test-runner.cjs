#!/usr/bin/env node

/**
 * Smart test runner that handles --debug and other flags
 */

const { spawn } = require('node:child_process');

// Parse command line arguments
const args = process.argv.slice(2);
const isDebug = args.includes('--debug');
const isVerbose = args.includes('--verbose') || args.includes('-v');

// Remove our custom flags from args to pass to bun test
const bunArgs = args.filter((arg) => !['--debug', '--verbose', '-v'].includes(arg));

// Set up environment based on flags
const testEnv = {
  ...process.env,
  LOG_LEVEL: isDebug ? 'DEBUG' : isVerbose ? 'INFO' : 'WARN',
  DEBUG: isDebug ? '*' : '',
  CODEBUDDY_DEBUG: isDebug ? '1' : '',
};

// Default to fast test runner if no specific test files provided
const shouldUseFastRunner = bunArgs.length === 0 || bunArgs.every((arg) => !arg.endsWith('.ts'));

if (shouldUseFastRunner) {
  console.log(
    `Running fast test runner with ${isDebug ? 'DEBUG' : isVerbose ? 'INFO' : 'WARN'} logging...`
  );

  const proc = spawn('node', ['test-runner-fast.cjs', ...bunArgs], {
    env: testEnv,
    stdio: 'inherit',
  });

  proc.on('exit', (code) => {
    process.exit(code);
  });
} else {
  console.log(
    `Running specific tests with ${isDebug ? 'DEBUG' : isVerbose ? 'INFO' : 'WARN'} logging...`
  );

  const proc = spawn('bun', ['test', ...bunArgs], {
    env: testEnv,
    stdio: 'inherit',
  });

  proc.on('exit', (code) => {
    process.exit(code);
  });
}
