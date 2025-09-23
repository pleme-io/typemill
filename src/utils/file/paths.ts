/**
 * Path utilities
 */

import { resolve, normalize, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Resolve a path to an absolute path
 * @param filePath Path to resolve
 * @returns Absolute path
 */
export function resolvePath(...paths: string[]): string {
  return resolve(...paths);
}

/**
 * Normalize a path
 * @param filePath Path to normalize
 * @returns Normalized path
 */
export function normalizePath(filePath: string): string {
  return normalize(filePath);
}

/**
 * Convert file:// URL to path
 * @param url File URL
 * @returns File path
 */
export function urlToPath(url: string): string {
  if (url.startsWith('file://')) {
    return fileURLToPath(url);
  }
  return url;
}

/**
 * Convert path to file:// URL
 * @param filePath File path
 * @returns File URL
 */
export function pathToUrl(filePath: string): string {
  return `file://${resolve(filePath)}`;
}

/**
 * Get relative path from one path to another
 * @param from Source path
 * @param to Target path
 * @returns Relative path
 */
export function getRelativePath(from: string, to: string): string {
  return relative(from, to);
}

/**
 * Join multiple path segments
 * @param paths Path segments
 * @returns Joined path
 */
export function joinPath(...paths: string[]): string {
  return join(...paths);
}