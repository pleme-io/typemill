/**
 * Test utilities for temporary file and directory management
 */

import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/**
 * Create a temporary directory for testing
 */
export function createTempDir(prefix = 'codebuddy-test'): string {
  return mkdtempSync(join(tmpdir(), `${prefix}-`));
}

/**
 * Create a temporary file path (doesn't create the file)
 */
export function getTempFilePath(filename: string, prefix = 'codebuddy-test'): string {
  const tempDir = createTempDir(prefix);
  return join(tempDir, filename);
}

/**
 * Clean up a temporary directory
 */
export function cleanupTempDir(dirPath: string): void {
  try {
    rmSync(dirPath, { recursive: true, force: true });
  } catch (_error) {
    // Ignore cleanup errors in tests
  }
}

/**
 * Get a consistent test directory in temp
 */
export function getTestTempDir(): string {
  const dir = join(tmpdir(), 'codebuddy-tests');
  mkdirSync(dir, { recursive: true });
  return dir;
}
