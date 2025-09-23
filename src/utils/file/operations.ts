/**
 * File operation utilities
 */

import { readFile, writeFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { mkdir } from 'node:fs/promises';

/**
 * Read file content as string
 * @param filePath Path to the file
 * @returns File content as string
 */
export async function readFileContent(filePath: string): Promise<string> {
  return await readFile(filePath, 'utf-8');
}

/**
 * Write content to a file
 * @param filePath Path to the file
 * @param content Content to write
 */
export async function writeFileContent(filePath: string, content: string): Promise<void> {
  const dir = dirname(filePath);
  await mkdir(dir, { recursive: true });
  await writeFile(filePath, content, 'utf-8');
}