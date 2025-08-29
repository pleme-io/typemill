import { execSync } from 'node:child_process';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';

/**
 * Creates a unique test directory that won't conflict with parallel tests
 */
export function createTestDir(prefix: string): string {
  const dir = `/tmp/${prefix}-${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
  if (existsSync(dir)) {
    rmSync(dir, { recursive: true, force: true });
  }
  mkdirSync(dir, { recursive: true });

  // Verify directory was created
  if (!existsSync(dir)) {
    throw new Error(`Failed to create test directory: ${dir}`);
  }

  return dir;
}

/**
 * Safely cleans up a test directory
 */
export function cleanupTestDir(dir: string): void {
  try {
    if (existsSync(dir)) {
      rmSync(dir, { recursive: true, force: true });
    }
  } catch (error) {
    // Ignore cleanup errors - another process may have already cleaned it
  }
}

/**
 * Ensures filesystem operations are flushed
 */
export function flushFilesystem(): void {
  // Force filesystem sync on Unix systems
  if (process.platform !== 'win32') {
    try {
      execSync('sync', { stdio: 'ignore' });
    } catch {
      // Ignore sync errors on systems that don't support it
    }
  }
}

/**
 * Ensures a file exists after writing
 */
export function ensureFileExists(filePath: string, maxRetries = 10): void {
  for (let i = 0; i < maxRetries; i++) {
    if (existsSync(filePath)) {
      return;
    }
    // Small delay before retry
    execSync('sleep 0.01', { stdio: 'ignore' });
  }
  throw new Error(`File not available after ${maxRetries} retries: ${filePath}`);
}

/**
 * Creates a unique test file path
 */
export function createTestFilePath(dir: string, filename: string): string {
  const timestamp = Date.now();
  const random = Math.random().toString(36).slice(2, 7);
  const parts = filename.split('.');
  const ext = parts.pop();
  const base = parts.join('.');
  return `${dir}/${base}-${timestamp}-${random}.${ext}`;
}
