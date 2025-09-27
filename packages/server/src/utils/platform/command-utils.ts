/**
 * Platform-aware command execution utilities
 * Handles differences between Windows and Unix-like systems
 */

import { type SpawnOptions, spawn } from 'node:child_process';
import { platform } from 'node:os';

/**
 * Detect if we're running on Windows
 */
export function isWindows(): boolean {
  return platform() === 'win32';
}

/**
 * Detect if we're running on macOS
 */
export function isMacOS(): boolean {
  return platform() === 'darwin';
}

/**
 * Detect if we're running on Linux
 */
export function isLinux(): boolean {
  return platform() === 'linux';
}

/**
 * Get platform-appropriate spawn options
 * Windows needs shell:true for certain commands (npm, npx, etc.)
 * Unix systems work better with shell:false for security
 */
export function getPlatformSpawnOptions(command: string, baseOptions?: SpawnOptions): SpawnOptions {
  const isWin = isWindows();

  // Commands that require shell on Windows
  const windowsShellCommands = [
    'npm',
    'npx',
    'yarn',
    'pnpm',
    'bun',
    'node',
    'python',
    'pip',
    'pipx',
    'py',
    'git',
    'code',
    'cursor',
  ];

  // Check if this command needs shell on Windows
  const needsShell =
    isWin &&
    windowsShellCommands.some(
      (cmd) => command === cmd || command.endsWith(`${cmd}.cmd`) || command.endsWith(`${cmd}.exe`)
    );

  return {
    ...baseOptions,
    shell: needsShell,
    // On Windows, we need to handle paths with spaces properly
    windowsVerbatimArguments: isWin ? true : undefined,
  };
}

/**
 * Spawn a command with platform-aware settings
 * Automatically handles Windows vs Unix differences
 */
export function spawnCommand(command: string, args: string[] = [], options?: SpawnOptions) {
  const platformOptions = getPlatformSpawnOptions(command, options);
  return spawn(command, args, platformOptions);
}

/**
 * Check if a command is available on the system
 * Uses platform-appropriate methods
 */
export async function isCommandAvailable(command: string[], timeout = 2000): Promise<boolean> {
  try {
    const [cmd, ...args] = command;

    if (!cmd) return false;

    // Special handling for npx commands - check if npm is available
    if (cmd === 'npx') {
      return isCommandAvailable(['npm', '--version'], timeout);
    }

    // Try to run the command with --version or --help
    const testArgs = cmd === 'npm' ? ['--version'] : ['--version'];

    return new Promise((resolve) => {
      const proc = spawnCommand(cmd, testArgs, {
        stdio: 'ignore',
      });

      proc.on('error', () => resolve(false));
      proc.on('exit', (code) => resolve(code === 0));

      // Timeout after specified time
      setTimeout(() => {
        proc.kill();
        resolve(false);
      }, timeout);
    });
  } catch {
    return false;
  }
}

/**
 * Normalize a command for the current platform
 * On Windows, some commands need .cmd or .exe extensions
 */
export function normalizeCommand(command: string): string {
  if (!isWindows()) {
    return command;
  }

  // Commands that need .cmd extension on Windows
  const cmdExtensions = ['npm', 'npx', 'yarn', 'pnpm'];
  if (cmdExtensions.includes(command)) {
    return `${command}.cmd`;
  }

  // Commands that might need .exe extension
  const exeCommands = ['node', 'git', 'python'];
  if (exeCommands.includes(command)) {
    // Check if .exe version exists, otherwise use as-is
    // The shell will handle finding it in PATH
    return command;
  }

  return command;
}

/**
 * Execute a command and capture output
 * Returns stdout, stderr, and exit code
 */
export async function execCommand(
  command: string,
  args: string[] = [],
  options?: SpawnOptions
): Promise<{ stdout: string; stderr: string; exitCode: number | null }> {
  return new Promise((resolve) => {
    const proc = spawnCommand(command, args, options);

    let stdout = '';
    let stderr = '';

    if (proc.stdout) {
      proc.stdout.on('data', (data) => {
        stdout += data.toString();
      });
    }

    if (proc.stderr) {
      proc.stderr.on('data', (data) => {
        stderr += data.toString();
      });
    }

    proc.on('error', (error) => {
      stderr += error.message;
      resolve({ stdout, stderr, exitCode: 1 });
    });

    proc.on('exit', (code) => {
      resolve({ stdout, stderr, exitCode: code });
    });
  });
}
