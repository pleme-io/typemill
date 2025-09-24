/**
 * FUSE filesystem operations that bridge to WebSocket client
 * Implements all necessary FUSE callbacks for LSP server filesystem access
 */

import { randomUUID } from 'node:crypto';
import { resolve } from 'node:path';
import { logger } from '../core/diagnostics/logger.js';
import type { WebSocketTransport } from '../transports/websocket.js';
import type {
  AsyncOperationCallback,
  FileOperationResult,
  FuseStats,
} from '../types/fuse-types.js';
import type {
  EnhancedClientSession,
  FuseOperationRequest,
  FuseOperationResponse,
} from '../types/session.js';

// FuseStats is now imported from types/fuse-types.ts

export interface FuseOperationHandlers {
  readdir(path: string): Promise<string[]>;
  getattr(path: string): Promise<FuseStats>;
  open(path: string, flags: number): Promise<number>;
  read(path: string, fd: number, size: number, offset: number): Promise<Buffer>;
  write(path: string, fd: number, buffer: Buffer, offset: number): Promise<number>;
  release(path: string, fd: number): Promise<void>;
  truncate(path: string, size: number): Promise<void>;
  mkdir(path: string, mode: number): Promise<void>;
  rmdir(path: string): Promise<void>;
  unlink(path: string): Promise<void>;
  rename(oldPath: string, newPath: string): Promise<void>;
}

export class FuseOperations implements FuseOperationHandlers {
  private session: EnhancedClientSession;
  private transport: WebSocketTransport;
  private pendingOperations = new Map<string, AsyncOperationCallback<FileOperationResult>>();
  private readonly OPERATION_TIMEOUT_MS = 30000; // 30 seconds
  private fileDescriptors = new Map<number, string>(); // fd -> path mapping
  private nextFd = 1;

  constructor(session: EnhancedClientSession, transport: WebSocketTransport) {
    this.session = session;
    this.transport = transport;
  }

  /**
   * Validate session has required permissions for the operation
   */
  private validatePermissions(operation: string, path?: string): void {
    // Check if session is authenticated
    if (!this.session.initialized) {
      throw new Error('Session not initialized');
    }

    // Check if session has required permissions
    const permissions = this.session.permissions || [];

    // Map operations to required permissions
    const requiredPermissions: Record<string, string> = {
      readdir: 'file:read',
      getattr: 'file:read',
      open: 'file:read',
      read: 'file:read',
      write: 'file:write',
      truncate: 'file:write',
      mkdir: 'file:write',
      rmdir: 'file:write',
      unlink: 'file:write',
      rename: 'file:write',
      release: 'file:read',
    };

    const required = requiredPermissions[operation];
    if (required && !permissions.includes(required)) {
      logger.warn('Permission denied for FUSE operation', {
        component: 'FuseOperations',
        sessionId: this.session.id,
        operation,
        required,
        path,
        hasPermissions: permissions,
      });
      throw new Error(`Permission denied: ${operation} requires ${required}`);
    }

    // Validate path is within session workspace using proper path resolution
    if (path && this.session.workspaceDir) {
      // Resolve the path to an absolute path within the workspace
      const resolvedPath = resolve(this.session.workspaceDir, path);

      // Ensure the resolved path is still within the workspace directory
      if (!resolvedPath.startsWith(this.session.workspaceDir)) {
        logger.warn('Path traversal attempt blocked', {
          component: 'FuseOperations',
          sessionId: this.session.id,
          path,
          resolvedPath,
          workspaceDir: this.session.workspaceDir,
        });
        throw new Error('Path traversal not allowed - path must remain within workspace');
      }
    }
  }

  /**
   * Send a FUSE operation request to the client and wait for response
   */
  private async sendFuseOperation<T = FileOperationResult>(
    method: FuseOperationRequest['method'],
    path: string,
    options: Partial<FuseOperationRequest> = {}
  ): Promise<T> {
    const correlationId = randomUUID();

    const request: FuseOperationRequest = {
      method,
      path,
      correlationId,
      ...options,
    };

    return new Promise((resolve, reject) => {
      // Set up timeout
      const timeout = setTimeout(() => {
        this.pendingOperations.delete(correlationId);
        reject(new Error(`FUSE operation ${method} timed out for path: ${path}`));
      }, this.OPERATION_TIMEOUT_MS);

      this.pendingOperations.set(correlationId, { resolve, reject, timeout });

      // Send request to client
      this.transport.sendRequest(this.session, method, request).catch((error) => {
        clearTimeout(timeout);
        this.pendingOperations.delete(correlationId);
        reject(error);
      });
    });
  }

  /**
   * Handle response from client for a FUSE operation
   */
  handleFuseResponse(response: FuseOperationResponse): void {
    const { correlationId, success, data, error, errno } = response;
    const pending = this.pendingOperations.get(correlationId);

    if (!pending) {
      logger.warn('Received FUSE response for unknown correlation ID', {
        component: 'FuseOperations',
        correlationId,
        sessionId: this.session.id,
      });
      return;
    }

    clearTimeout(pending.timeout);
    this.pendingOperations.delete(correlationId);

    if (success) {
      pending.resolve(data);
    } else {
      const fuseError = new Error(error || 'FUSE operation failed') as Error & { errno?: number };
      fuseError.errno = errno || -2; // ENOENT by default
      pending.reject(fuseError);
    }
  }

  /**
   * Read directory contents
   */
  async readdir(path: string): Promise<string[]> {
    this.validatePermissions('readdir', path);
    try {
      logger.debug('FUSE readdir operation', {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
      });

      const entries = await this.sendFuseOperation<string[]>('fuse/readdir', path);
      return entries || [];
    } catch (error) {
      logger.error('FUSE readdir failed', error as Error, {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
      });
      throw error;
    }
  }

  /**
   * Get file/directory attributes
   */
  async getattr(path: string): Promise<FuseStats> {
    this.validatePermissions('getattr', path);
    try {
      logger.debug('FUSE getattr operation', {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
      });

      const stats = await this.sendFuseOperation<FuseStats>('fuse/stat', path);

      // Ensure dates are Date objects
      if (stats.mtime && !(stats.mtime instanceof Date)) {
        stats.mtime = new Date(stats.mtime);
      }
      if (stats.atime && !(stats.atime instanceof Date)) {
        stats.atime = new Date(stats.atime);
      }
      if (stats.ctime && !(stats.ctime instanceof Date)) {
        stats.ctime = new Date(stats.ctime);
      }

      return stats;
    } catch (error) {
      logger.error('FUSE getattr failed', error as Error, {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
      });
      throw error;
    }
  }

  /**
   * Open file and return file descriptor
   */
  async open(path: string, flags: number): Promise<number> {
    // Check if write mode requires write permission
    const isWrite = flags & 0x0001 || flags & 0x0002; // O_WRONLY or O_RDWR
    this.validatePermissions(isWrite ? 'write' : 'open', path);
    try {
      logger.debug('FUSE open operation', {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        flags,
      });

      // Request client to prepare file for reading
      await this.sendFuseOperation('fuse/open', path, { flags });

      // Generate local file descriptor
      const fd = this.nextFd++;
      this.fileDescriptors.set(fd, path);

      return fd;
    } catch (error) {
      logger.error('FUSE open failed', error as Error, {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        flags,
      });
      throw error;
    }
  }

  /**
   * Read file content
   */
  async read(path: string, fd: number, size: number, offset: number): Promise<Buffer> {
    this.validatePermissions('read', path);
    try {
      logger.debug('FUSE read operation', {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        fd,
        size,
        offset,
      });

      const data = await this.sendFuseOperation<Buffer>('fuse/read', path, {
        flags: fd,
        data: Buffer.from([size, offset]),
      });

      return Buffer.isBuffer(data) ? data : Buffer.from(data);
    } catch (error) {
      logger.error('FUSE read failed', error as Error, {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        fd,
      });
      throw error;
    }
  }

  /**
   * Write file content
   */
  async write(path: string, fd: number, buffer: Buffer, offset: number): Promise<number> {
    this.validatePermissions('write', path);
    try {
      logger.debug('FUSE write operation', {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        fd,
        size: buffer.length,
        offset,
      });

      const bytesWritten = await this.sendFuseOperation<number>('fuse/write', path, {
        flags: fd,
        data: buffer,
      });

      return bytesWritten || buffer.length;
    } catch (error) {
      logger.error('FUSE write failed', error as Error, {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        fd,
      });
      throw error;
    }
  }

  /**
   * Release (close) file descriptor
   */
  async release(path: string, fd: number): Promise<void> {
    this.validatePermissions('release', path);
    try {
      logger.debug('FUSE release operation', {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        fd,
      });

      await this.sendFuseOperation('fuse/release', path, { flags: fd });
      this.fileDescriptors.delete(fd);
    } catch (error) {
      logger.error('FUSE release failed', error as Error, {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        fd,
      });
      // Don't throw on release failures - clean up anyway
      this.fileDescriptors.delete(fd);
    }
  }

  /**
   * Truncate file to specified size
   */
  async truncate(path: string, size: number): Promise<void> {
    this.validatePermissions('truncate', path);
    try {
      await this.sendFuseOperation('fuse/write', path, {
        data: Buffer.from([size]),
      });
    } catch (error) {
      logger.error('FUSE truncate failed', error as Error, {
        component: 'FuseOperations',
        sessionId: this.session.id,
        path,
        size,
      });
      throw error;
    }
  }

  /**
   * Create directory
   */
  async mkdir(path: string, mode: number): Promise<void> {
    this.validatePermissions('mkdir', path);
    logger.warn('FUSE mkdir operation not implemented - read-only filesystem', {
      component: 'FuseOperations',
      path,
      mode,
    });
    throw new Error('Read-only filesystem');
  }

  /**
   * Remove directory
   */
  async rmdir(path: string): Promise<void> {
    this.validatePermissions('rmdir', path);
    logger.warn('FUSE rmdir operation not implemented - read-only filesystem', {
      component: 'FuseOperations',
      path,
    });
    throw new Error('Read-only filesystem');
  }

  /**
   * Delete file
   */
  async unlink(path: string): Promise<void> {
    this.validatePermissions('unlink', path);
    logger.warn('FUSE unlink operation not implemented - read-only filesystem', {
      component: 'FuseOperations',
      path,
    });
    throw new Error('Read-only filesystem');
  }

  /**
   * Rename file
   */
  async rename(oldPath: string, newPath: string): Promise<void> {
    this.validatePermissions('rename', oldPath);
    this.validatePermissions('rename', newPath);
    logger.warn('FUSE rename operation not implemented - read-only filesystem', {
      component: 'FuseOperations',
      oldPath,
      newPath,
    });
    throw new Error('Read-only filesystem');
  }

  /**
   * Get pending operations count (for stats)
   */
  getPendingOperationsCount(): number {
    return this.pendingOperations.size;
  }

  /**
   * Get open files count (for stats)
   */
  getOpenFilesCount(): number {
    return this.fileDescriptors.size;
  }

  /**
   * Cleanup all pending operations
   */
  cleanup(): void {
    for (const [_correlationId, pending] of this.pendingOperations) {
      clearTimeout(pending.timeout);
      pending.reject(new Error('FUSE operations cleanup - session ending'));
    }
    this.pendingOperations.clear();
    this.fileDescriptors.clear();
  }
}
