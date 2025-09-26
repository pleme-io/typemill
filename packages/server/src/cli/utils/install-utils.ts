#!/usr/bin/env node

import { type ChildProcess, spawn } from 'node:child_process';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { executableManager } from '../../utils/platform/executable-manager.js';
import { getPlatformInfo } from '../../utils/platform/platform-detector.js';

/**
 * Run an installation command and return success status
 */
export async function runInstallCommand(
  command: string[],
  serverName: string,
  onOutput?: (data: string) => void
): Promise<boolean> {
  const [cmd, ...args] = command;

  if (!cmd) {
    return false;
  }

  // For pipx commands, try pipx first, then fallback to pip strategies
  if (cmd === 'pipx') {
    // First try pipx
    const pipxSuccess = await tryInstallCommand([cmd, ...args], serverName, onOutput);
    if (pipxSuccess) {
      return true;
    }

    // If pipx fails, fallback to pip strategies
    console.log('    pipx failed, trying pip fallback...');
    const packageName = args[args.length - 1]; // Last arg should be package name
    if (!packageName) {
      return false;
    }
    return await tryPipFallback(packageName, serverName, onOutput);
  }

  // For pip commands, try multiple strategies
  if ((cmd === 'pip' || cmd === 'pip3') && args.includes('--user')) {
    // First try with --user (safer)
    const userSuccess = await tryInstallCommand([cmd, ...args], serverName, onOutput);
    if (userSuccess) {
      return true;
    }

    // If --user fails due to externally-managed-environment, try --break-system-packages
    const sysArgs = args.filter((arg) => arg !== '--user').concat(['--break-system-packages']);
    console.log('    Retrying with --break-system-packages...');
    return await tryInstallCommand([cmd, ...sysArgs], serverName, onOutput);
  }

  // For all other commands, run normally
  return await tryInstallCommand(command, serverName, onOutput);
}

/**
 * Try pip fallback strategies when pipx fails
 */
async function tryPipFallback(
  packageName: string,
  serverName: string,
  onOutput?: (data: string) => void
): Promise<boolean> {
  // Try different pip commands in order of preference
  const pipCommands = ['pip3', 'pip'];

  for (const pipCmd of pipCommands) {
    try {
      // Check if this pip command exists using cross-platform executable manager
      const pipExists = await executableManager.exists(pipCmd);
      if (!pipExists) {
        continue;
      }

      // Try --user first (safer)
      console.log(`    Trying ${pipCmd} --user...`);
      const userSuccess = await tryInstallCommand(
        [pipCmd, 'install', packageName, '--user'],
        serverName,
        onOutput
      );
      if (userSuccess) {
        return true;
      }

      // If --user fails, try --break-system-packages on Linux
      const platform = getPlatformInfo();
      if (platform.isLinux) {
        console.log(`    Trying ${pipCmd} with --break-system-packages...`);
        const sysSuccess = await tryInstallCommand(
          [pipCmd, 'install', packageName, '--break-system-packages'],
          serverName,
          onOutput
        );
        if (sysSuccess) {
          return true;
        }
      }
    } catch {}
  }

  return false;
}

/**
 * Try running a single install command
 */
async function tryInstallCommand(
  command: string[],
  _serverName: string,
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

    let _output = '';
    let error = '';

    proc.stdout?.on('data', (data: Buffer) => {
      const text = data.toString();
      _output += text;
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
          const platform = getPlatformInfo();
          if (platform.isMacOS) {
            console.log('    Install Go first: brew install go');
          } else {
            console.log('    Install Go first: https://golang.org/dl/');
          }
        } else if (cmd === 'rustup') {
          console.log('    Install Rust first: https://rustup.rs/');
        } else if (cmd === 'pipx') {
          const platform = getPlatformInfo();
          if (platform.isMacOS) {
            console.log('    Install pipx first: brew install pipx');
          } else {
            console.log('    Install pipx first: pip install --user pipx');
          }
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
export async function getPipCommand(baseCommand: string[]): Promise<string[]> {
  if (baseCommand[0] === 'pip' || baseCommand[0] === 'pip3') {
    // Try to find the best available pip command
    const pipCommand = await findBestPipCommand();
    const result = [pipCommand, ...baseCommand.slice(1)];

    // Only add flags for pip/pip3, not pipx
    if (pipCommand === 'pip' || pipCommand === 'pip3') {
      // Try --user first (safer), fallback to --break-system-packages if needed
      result.push('--user');
    }
    // pipx doesn't need any additional flags - it handles isolation automatically

    return result;
  }
  return baseCommand;
}

/**
 * Find the best available pip command (pipx first, then pip3, then pip)
 */
async function findBestPipCommand(): Promise<string> {
  // Prefer pipx for isolated installs, especially on macOS with externally-managed Python
  const commands = ['pipx', 'pip3', 'pip'];

  for (const cmd of commands) {
    try {
      // Cross-platform command existence check using executable manager
      const exists = await executableManager.exists(cmd);
      if (exists) {
        return cmd;
      }
    } catch {
      // Command not found, try next
    }
  }

  // Fallback to pip if nothing found (will show proper error)
  return 'pip';
}
