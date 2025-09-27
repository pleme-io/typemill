/**
 * Cross-platform process management utilities
 * Phase 3: Process operations - replaces platform-specific process handling
 */

import treeKill from 'tree-kill';
import { getPlatformInfo } from './platform-detector.js';

/**
 * Process termination options
 */
export interface ProcessTerminationOptions {
  force?: boolean;
  timeout?: number; // milliseconds to wait before force killing
}

/**
 * Process manager interface
 */
export interface ProcessManager {
  /**
   * Check if a process is running
   */
  isRunning(pid: number): boolean;

  /**
   * Terminate a process gracefully, with force option
   */
  terminate(pid: number, options?: ProcessTerminationOptions): Promise<void>;

  /**
   * Kill a process and all its children
   */
  killTree(pid: number, signal?: string): Promise<void>;
}

/**
 * Cross-platform process manager implementation
 */
class ProcessManagerImpl implements ProcessManager {
  /**
   * Check if a process is running by PID
   */
  isRunning(pid: number): boolean {
    try {
      // Sending signal 0 doesn't kill the process, just checks if it exists
      // This works cross-platform
      process.kill(pid, 0);
      return true;
    } catch (_error) {
      return false;
    }
  }

  /**
   * Terminate a process with cross-platform handling
   */
  async terminate(pid: number, options: ProcessTerminationOptions = {}): Promise<void> {
    const { force = false, timeout = 5000 } = options;

    if (!this.isRunning(pid)) {
      return; // Process already terminated
    }

    return new Promise<void>((resolve, reject) => {
      const signal = force ? 'SIGKILL' : 'SIGTERM';

      treeKill(pid, signal, (error) => {
        if (error) {
          // If graceful termination failed and we haven't tried force yet
          if (!force && error.message.includes('No such process')) {
            resolve(); // Process already dead
            return;
          }

          if (!force) {
            // Try force kill as fallback
            setTimeout(() => {
              this.terminate(pid, { force: true, timeout }).then(resolve).catch(reject);
            }, timeout);
          } else {
            reject(error);
          }
        } else {
          resolve();
        }
      });
    });
  }

  /**
   * Kill a process tree with specified signal
   */
  async killTree(pid: number, signal: string = 'SIGTERM'): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      treeKill(pid, signal, (error) => {
        if (error) {
          reject(error);
        } else {
          resolve();
        }
      });
    });
  }
}

// Export singleton instance
export const processManager: ProcessManager = new ProcessManagerImpl();

// Convenience exports
export const { isRunning, terminate, killTree } = processManager;
