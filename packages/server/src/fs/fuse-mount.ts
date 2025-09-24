/**
 * FUSE filesystem mount for exposing client filesystem to LSP servers
 * Provides native filesystem access through FUSE with WebSocket backend
 */

import Fuse from '@cocalc/fuse-native';
import { logger } from '../core/diagnostics/logger.js';
import type { WebSocketTransport } from '../transports/websocket.js';
import type {
  FuseErrorCallback,
  FuseGetattrCallback,
  FuseOpenCallback,
  FuseReadCallback,
  FuseReaddirCallback,
  FuseWriteCallback,
  MountOptions,
} from '../types/fuse-types.js';
import type { EnhancedClientSession, FuseOperationResponse } from '../types/session.js';
import { FuseOperations } from './fuse-operations.js';

export interface FuseMountConfig {
  mountOptions?: string[];
  debugFuse?: boolean;
  allowOther?: boolean;
  allowRoot?: boolean;
  defaultPermissions?: boolean;
}

export class FuseMount {
  private session: EnhancedClientSession;
  private operations: FuseOperations;
  private fuse?: Fuse;
  private mountPath: string;
  private mounted = false;
  private config: FuseMountConfig;

  constructor(
    session: EnhancedClientSession,
    transport: WebSocketTransport,
    mountPath: string,
    config: FuseMountConfig = {}
  ) {
    this.session = session;
    this.transport = transport;
    this.mountPath = mountPath;
    this.config = config;
    this.operations = new FuseOperations(session, transport);
  }

  /**
   * Mount the FUSE filesystem
   */
  async mount(): Promise<void> {
    if (this.mounted) {
      throw new Error(`FUSE filesystem already mounted at ${this.mountPath}`);
    }

    try {
      logger.info('Mounting FUSE filesystem', {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
        config: this.config,
      });

      // Build mount options
      const options = this.buildMountOptions();

      // Create FUSE instance with operations
      // Note: @cocalc/fuse-native expects callback-style operations
      this.fuse = new Fuse(
        this.mountPath,
        {
          readdir: (path: string, cb: FuseReaddirCallback) => {
            this.operations.readdir(path).then(
              (result) => cb(0, result),
              (error) => cb(error.errno || -1)
            );
          },
          getattr: (path: string, cb: FuseGetattrCallback) => {
            this.operations.getattr(path).then(
              (result) => cb(0, result),
              (error) => cb(error.errno || -1)
            );
          },
          open: (path: string, flags: number, cb: FuseOpenCallback) => {
            this.operations.open(path, flags).then(
              (result) => cb(0, result),
              (error) => cb(error.errno || -1)
            );
          },
          read: (
            path: string,
            fd: number,
            buffer: Buffer,
            length: number,
            position: number,
            cb: FuseReadCallback
          ) => {
            this.operations.read(path, fd, length, position).then(
              (result) => {
                result.copy(buffer, 0, 0, Math.min(result.length, length));
                cb(Math.min(result.length, length));
              },
              (error) => cb(error.errno || -1)
            );
          },
          write: (
            path: string,
            fd: number,
            buffer: Buffer,
            _length: number,
            position: number,
            cb: FuseWriteCallback
          ) => {
            this.operations.write(path, fd, buffer, position).then(
              (result) => cb(result),
              (error) => cb(error.errno || -1)
            );
          },
          release: (path: string, fd: number, cb: FuseErrorCallback) => {
            this.operations.release(path, fd).then(
              () => cb(0),
              (error) => cb(error.errno || -1)
            );
          },
        },
        options
      );

      // Mount the filesystem
      await new Promise<void>((resolve, reject) => {
        this.fuse?.mount((error) => {
          if (error) {
            reject(new Error(`Failed to mount FUSE filesystem: ${error.message}`));
          } else {
            this.mounted = true;
            resolve();
          }
        });
      });

      logger.info('FUSE filesystem mounted successfully', {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
      });
    } catch (error) {
      logger.error('Failed to mount FUSE filesystem', error as Error, {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
      });
      throw error;
    }
  }

  /**
   * Unmount the FUSE filesystem
   */
  async unmount(): Promise<void> {
    if (!this.mounted || !this.fuse) {
      logger.warn('FUSE filesystem not mounted, skipping unmount', {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
      });
      return;
    }

    try {
      logger.info('Unmounting FUSE filesystem', {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
      });

      // Cleanup pending operations first
      this.operations.cleanup();

      // Unmount the filesystem
      await new Promise<void>((resolve, _reject) => {
        this.fuse?.unmount((error) => {
          if (error) {
            logger.error('Error during FUSE unmount', error, {
              component: 'FuseMount',
              sessionId: this.session.id,
              mountPath: this.mountPath,
            });
            // Don't reject - we want to continue cleanup
          }
          resolve();
        });
      });

      this.mounted = false;
      this.fuse = undefined;

      logger.info('FUSE filesystem unmounted successfully', {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
      });
    } catch (error) {
      logger.error('Failed to unmount FUSE filesystem', error as Error, {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
      });
      throw error;
    }
  }

  /**
   * Check if filesystem is mounted
   */
  isMounted(): boolean {
    return this.mounted;
  }

  /**
   * Get mount path
   */
  getMountPath(): string {
    return this.mountPath;
  }

  /**
   * Handle FUSE response from client
   */
  handleFuseResponse(response: FuseOperationResponse): void {
    this.operations.handleFuseResponse(response);
  }

  /**
   * Build mount options from config
   */
  private buildMountOptions(): MountOptions {
    const options: MountOptions = {
      debug: this.config.debugFuse || false,
    };

    // Add FUSE-specific options
    if (this.config.allowOther) {
      options.allow_other = true;
    }

    if (this.config.allowRoot) {
      options.allow_root = true;
    }

    if (this.config.defaultPermissions) {
      options.default_permissions = true;
    }

    // Add custom mount options
    if (this.config.mountOptions) {
      for (const option of this.config.mountOptions) {
        const [key, value] = option.split('=');
        if (key) {
          if (value !== undefined) {
            options[key] = value;
          } else {
            options[key] = true;
          }
        }
      }
    }

    return options;
  }

  /**
   * Get filesystem statistics for monitoring
   */
  getStats(): {
    mounted: boolean;
    mountPath: string;
    sessionId: string;
    pendingOperations: number;
    openFiles: number;
  } {
    return {
      mounted: this.mounted,
      mountPath: this.mountPath,
      sessionId: this.session.id,
      pendingOperations: this.operations.getPendingOperationsCount(),
      openFiles: this.operations.getOpenFilesCount(),
    };
  }

  /**
   * Force cleanup - used during emergency shutdown
   */
  async forceCleanup(): Promise<void> {
    try {
      logger.warn('Force cleaning up FUSE mount', {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
      });

      // Cleanup operations first
      this.operations.cleanup();

      // Force unmount if still mounted
      if (this.mounted && this.fuse) {
        // Use system unmount as fallback (using execFileSync to prevent injection)
        const { execFileSync } = await import('node:child_process');
        try {
          execFileSync('fusermount', ['-u', this.mountPath], { timeout: 5000 });
        } catch (_error) {
          // Try lazy unmount
          try {
            execFileSync('fusermount', ['-uz', this.mountPath], { timeout: 5000 });
          } catch (lazyError) {
            logger.error('Failed to force unmount FUSE filesystem', lazyError as Error, {
              component: 'FuseMount',
              sessionId: this.session.id,
              mountPath: this.mountPath,
            });
          }
        }
      }

      this.mounted = false;
      this.fuse = undefined;
    } catch (error) {
      logger.error('Error during force cleanup', error as Error, {
        component: 'FuseMount',
        sessionId: this.session.id,
        mountPath: this.mountPath,
      });
    }
  }
}
