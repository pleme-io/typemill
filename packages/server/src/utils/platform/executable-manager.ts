/**
 * Cross-platform executable finding and management
 * Phase 4: Real implementation using 'which' package
 */

import which from 'which';
import { execSync } from 'node:child_process';
import { getPlatformInfo } from './platform-detector.js';

/**
 * Information about a found executable
 */
export interface ExecutableInfo {
  path: string;
  exists: boolean;
  version?: string;
}

/**
 * Executable manager interface
 */
export interface ExecutableManager {
  /**
   * Find an executable in the system PATH
   */
  find(executable: string): Promise<ExecutableInfo>;

  /**
   * Check if an executable exists and is accessible
   */
  exists(executable: string): Promise<boolean>;

  /**
   * Get version information for an executable
   */
  getVersion(executable: string): Promise<string | null>;

  /**
   * Get installation suggestions for a missing executable
   */
  getInstallationSuggestions(executable: string): string[];
}

/**
 * Real implementation using 'which' package for cross-platform executable finding
 */
class ExecutableManagerImpl implements ExecutableManager {
  private cache = new Map<string, ExecutableInfo>();

  async find(executable: string): Promise<ExecutableInfo> {
    // Check cache first
    if (this.cache.has(executable)) {
      return this.cache.get(executable)!;
    }

    try {
      const path = await which(executable);
      const info: ExecutableInfo = {
        path,
        exists: true,
        version: await this.getVersion(executable),
      };

      // Cache successful results
      this.cache.set(executable, info);
      return info;
    } catch (error) {
      const info: ExecutableInfo = {
        path: '',
        exists: false,
      };

      // Cache negative results for a short time to avoid repeated failures
      this.cache.set(executable, info);
      setTimeout(() => this.cache.delete(executable), 30000); // 30s cache

      return info;
    }
  }

  async exists(executable: string): Promise<boolean> {
    const info = await this.find(executable);
    return info.exists;
  }

  async getVersion(executable: string): Promise<string | null> {
    try {
      // Try common version flags
      const versionCommands = [`${executable} --version`, `${executable} -v`, `${executable} -V`];

      for (const cmd of versionCommands) {
        try {
          const output = execSync(cmd, {
            encoding: 'utf8',
            timeout: 5000,
            stdio: ['ignore', 'pipe', 'ignore'],
          });

          // Extract version number from output
          const versionMatch = output.match(/\d+\.\d+(\.\d+)?/);
          if (versionMatch) {
            return versionMatch[0];
          }
        } catch {
          // Try next command
          continue;
        }
      }

      return null;
    } catch (error) {
      return null;
    }
  }

  getInstallationSuggestions(executable: string): string[] {
    const platform = getPlatformInfo();

    // Basic platform-specific suggestions (to be enhanced later)
    if (platform.isWindows) {
      return [`choco install ${executable}`, `winget install ${executable}`];
    } else if (platform.isMacOS) {
      return [`brew install ${executable}`];
    } else if (platform.isLinux) {
      return [`sudo apt-get install ${executable}`, `sudo yum install ${executable}`];
    }

    return [`Install ${executable} using your system's package manager`];
  }
}

// Export singleton instance
export const executableManager: ExecutableManager = new ExecutableManagerImpl();

// Convenience exports
export const { find, exists, getVersion, getInstallationSuggestions } = executableManager;