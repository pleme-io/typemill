/**
 * Cross-platform path management utilities
 * Phase 1: Foundation layer - wraps Node.js path module with additional utilities
 */

import * as os from 'node:os';
import * as path from 'node:path';
import envPaths from 'env-paths';
import { getPlatformInfo } from './platform-detector.js';

/**
 * Path management interface
 */
export interface PathManager {
  // Core path operations (Node.js path module wrappers)
  join(...paths: string[]): string;
  resolve(...paths: string[]): string;
  relative(from: string, to: string): string;
  dirname(p: string): string;
  basename(p: string, ext?: string): string;
  extname(p: string): string;
  normalize(p: string): string;

  // Cross-platform specific utilities
  normalizePosix(p: string): string;
  expandPath(p: string): string;
  isAbsolute(p: string): boolean;

  // Directory utilities
  getHomeDir(): string;
  getTempDir(): string;
  getConfigDir(appName: string): string;
  getDataDir(appName: string): string;
  getCacheDir(appName: string): string;
}

/**
 * Implementation of cross-platform path manager
 */
class PathManagerImpl implements PathManager {
  // Direct wrappers around Node.js path module
  join = path.join;
  resolve = path.resolve;
  relative = path.relative;
  dirname = path.dirname;
  basename = path.basename;
  extname = path.extname;
  normalize = path.normalize;
  isAbsolute = path.isAbsolute;

  /**
   * Normalize path to use POSIX separators (for import/export statements)
   */
  normalizePosix(p: string): string {
    return path.posix.normalize(p.replace(/\\/g, '/'));
  }

  /**
   * Expand path with environment variables and home directory
   */
  expandPath(filepath: string): string {
    const platform = getPlatformInfo();

    // Handle project-relative paths
    if (filepath.startsWith('./') || (!filepath.includes('/') && !filepath.includes('\\'))) {
      if (filepath.startsWith('./')) {
        return path.resolve(process.cwd(), filepath.slice(2));
      }
      if (filepath.startsWith('.')) {
        return path.resolve(process.cwd(), filepath);
      }
    }

    // Expand home directory
    if (filepath.startsWith('~/')) {
      return path.resolve(os.homedir(), filepath.slice(2));
    }

    // Expand environment variables on Windows
    if (platform.isWindows) {
      filepath = filepath.replace(/%([^%]+)%/g, (_, name) => {
        return process.env[name] || '';
      });
    }

    return path.resolve(filepath);
  }

  /**
   * Get user's home directory
   */
  getHomeDir(): string {
    return os.homedir();
  }

  /**
   * Get system temporary directory
   */
  getTempDir(): string {
    return os.tmpdir();
  }

  /**
   * Get application configuration directory
   */
  getConfigDir(appName: string): string {
    return envPaths(appName).config;
  }

  /**
   * Get application data directory
   */
  getDataDir(appName: string): string {
    return envPaths(appName).data;
  }

  /**
   * Get application cache directory
   */
  getCacheDir(appName: string): string {
    return envPaths(appName).cache;
  }
}

// Export singleton instance
export const pathManager: PathManager = new PathManagerImpl();

// Also export individual functions for convenience
export const {
  join,
  resolve,
  relative,
  dirname,
  basename,
  extname,
  normalize,
  isAbsolute,
  normalizePosix,
  expandPath,
  getHomeDir,
  getTempDir,
  getConfigDir,
  getDataDir,
  getCacheDir,
} = pathManager;
