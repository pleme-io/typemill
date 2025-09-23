#!/usr/bin/env node

import { type ChildProcess, spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { isProcessRunning } from '../../utils/platform/process.js';

// Re-export for backward compatibility
export { isProcessRunning };

const TIMEOUT_MS = 2000;

/**
 * Get the full path for a command, checking common installation locations
 */
export function getCommandPath(cmd: string): string {
  // Get platform-specific paths
  const paths = getLSPServerPaths(cmd);

  // Check each path for existence
  for (const path of paths) {
    if (existsSync(path)) {
      return path;
    }
  }

  // Return original command (will use PATH)
  return cmd;
}

/**
 * Test if a command is available and working
 */
export async function testCommand(command: string[]): Promise<boolean> {
  if (!command.length) return false;

  const [cmd, ...args] = command;

  if (!cmd) return false;

  // Special handling for npx commands
  if (cmd === 'npx') {
    // First check if npm is available
    if (!(await testCommand(['npm', '--version']))) {
      return false;
    }

    // For npx commands, we assume they work if npm is available
    // since typescript-language-server is bundled
    return true;
  }

  // Get the full path for the command
  const fullCmd = getCommandPath(cmd);

  return new Promise((resolve) => {
    // Extract basename for getting test args
    const basename = fullCmd.split('/').pop() || cmd;
    const testArgs = getTestArgs(basename);
    const proc = spawn(fullCmd, testArgs, {
      stdio: 'ignore',
      shell: false,
      env: {
        ...process.env,
        PATH: '/opt/homebrew/bin:/usr/local/bin:' + (process.env.PATH || ''),
      },
    }) as ChildProcess;

    let resolved = false;

    proc.on('error', () => {
      if (!resolved) {
        resolved = true;
        resolve(false);
      }
    });

    proc.on('exit', (code: number | null) => {
      if (!resolved) {
        resolved = true;
        resolve(code === 0);
      }
    });

    // Timeout
    setTimeout(() => {
      if (!resolved) {
        resolved = true;
        proc.kill();
        resolve(false);
      }
    }, TIMEOUT_MS);
  });
}

/**
 * Get appropriate test arguments for a command
 */
function getTestArgs(command: string): string[] {
  // Commands that use 'version' without dashes
  const versionNoDash = new Set(['gopls']);

  // Commands that use '--version'
  const versionCommands = new Set([
    'pylsp',
    'rust-analyzer',
    'clangd',
    'jdtls',
    'solargraph',
    'intelephense',
    'npm',
  ]);

  const helpCommands = new Set(['docker-langserver']);

  if (versionNoDash.has(command)) {
    return ['version'];
  }
  if (versionCommands.has(command)) {
    return ['--version'];
  }
  if (helpCommands.has(command)) {
    return ['--help'];
  }
  return ['--version']; // Default
}
