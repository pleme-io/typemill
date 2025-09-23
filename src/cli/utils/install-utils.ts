#!/usr/bin/env node

import { type ChildProcess, spawn } from 'node:child_process';
import { homedir } from 'node:os';
import { join } from 'node:path';

/**
 * Run an installation command and return success status
 */
export async function runInstallCommand(
  command: string[],
  serverName: string,
  onOutput?: (data: string) => void
): Promise<boolean> {
  return new Promise((resolve) => {
    const [cmd, ...args] = command;

    if (!cmd) {
      resolve(false);
      return;
    }

    // Set up environment for Go commands
    const env = { ...process.env };
    if (cmd === 'go') {
      // Ensure GOPATH is set
      if (!env.GOPATH) {
        env.GOPATH = join(homedir(), 'go');
      }
      // Add GOPATH/bin to PATH
      env.PATH = `${join(env.GOPATH, 'bin')}:${env.PATH}`;
    }

    const proc = spawn(cmd, args, {
      stdio: 'pipe',
      env,
    }) as ChildProcess;

    let output = '';
    let error = '';

    proc.stdout?.on('data', (data: Buffer) => {
      const text = data.toString();
      output += text;
      if (onOutput) {
        onOutput(text);
      }
    });

    proc.stderr?.on('data', (data: Buffer) => {
      const text = data.toString();
      error += text;
    });

    proc.on('error', (err: Error) => {
      if (err.message.includes('ENOENT')) {
        console.log(`    Error: ${cmd} command not found`);
        if (cmd === 'pip' || cmd === 'pip3') {
          console.log('    Install Python first: https://python.org/downloads/');
        } else if (cmd === 'go') {
          if (process.platform === 'darwin') {
            console.log('    Install Go first: brew install go');
          } else {
            console.log('    Install Go first: https://golang.org/dl/');
          }
        } else if (cmd === 'rustup') {
          console.log('    Install Rust first: https://rustup.rs/');
        }
      } else {
        console.log(`    Error: ${err.message}`);
      }
      resolve(false);
    });

    proc.on('close', (code: number | null) => {
      if (code === 0) {
        resolve(true);
      } else {
        if (error.trim()) {
          console.log(`    Error: ${error.trim()}`);
        }
        resolve(false);
      }
    });
  });
}

/**
 * Detect and fix pip command with proper fallbacks and flags
 */
export function getPipCommand(baseCommand: string[]): string[] {
  if (baseCommand[0] === 'pip' || baseCommand[0] === 'pip3') {
    // Try to find the best available pip command
    const pipCommand = findBestPipCommand();
    const result = [pipCommand, ...baseCommand.slice(1)];

    // Add appropriate flags based on platform and pip version
    if (pipCommand === 'pip' || pipCommand === 'pip3') {
      if (process.platform === 'darwin') {
        // macOS: prefer --user to avoid system Python issues
        result.push('--user');
      } else {
        // Linux: use --break-system-packages for externally managed environments
        result.push('--break-system-packages');
      }
    }

    return result;
  }
  return baseCommand;
}

/**
 * Find the best available pip command (pip, pip3, pipx)
 */
function findBestPipCommand(): string {
  // Prefer pip3 over pip for better Python 3 compatibility
  const commands = ['pip3', 'pip', 'pipx'];

  for (const cmd of commands) {
    try {
      // Cross-platform command existence check
      const checkCommand = process.platform === 'win32' ? `where ${cmd}` : `which ${cmd}`;
      require('child_process').execSync(checkCommand, { stdio: 'ignore' });
      return cmd;
    } catch {
      // Command not found, try next
    }
  }

  // Fallback to pip if nothing found (will show proper error)
  return 'pip';
}
