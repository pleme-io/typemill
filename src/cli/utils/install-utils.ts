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
          console.log('    Install Go first: https://golang.org/dl/');
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
 * Detect if pip needs --break-system-packages flag
 */
export function getPipCommand(baseCommand: string[]): string[] {
  // If it's a pip command, we might need to add --break-system-packages
  if (baseCommand[0] === 'pip' || baseCommand[0] === 'pip3') {
    // Check if we're in a system-managed environment
    // For now, always add the flag for safety
    return [...baseCommand, '--break-system-packages'];
  }
  return baseCommand;
}
