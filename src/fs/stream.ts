import { PersistentFileCache } from '../core/cache.js';
import { logger } from '../core/logger.js';
import type { ClientSession } from '../transports/websocket.js';
import type {
  DeltaWriteRequest,
  DeltaWriteResponse,
  WebSocketTransport,
} from '../transports/websocket.js';
import { DeltaProcessor, type FileDelta } from './delta.js';

export interface FileReadRequest {
  path: string;
}

export interface FileReadResponse {
  content: string;
  mtime: number;
}

export interface FileChangedNotification {
  path: string;
  changeType: 'created' | 'changed' | 'deleted';
}

export class StreamingFileAccess {
  private fileCache = new PersistentFileCache();
  private deltaProcessor = new DeltaProcessor();

  constructor(private transport: WebSocketTransport) {}

  async readFile(session: ClientSession, path: string): Promise<string> {
    try {
      // Check cache first
      const cached = this.fileCache.getFile(session.id, path);
      if (cached) {
        logger.debug('File read cache hit', {
          component: 'StreamingFileAccess',
          sessionId: session.id,
          projectId: session.projectId,
          path,
          mtime: cached.mtime,
        });
        return cached.content;
      }

      // Cache miss - fetch from client
      logger.debug('File read cache miss', {
        component: 'StreamingFileAccess',
        sessionId: session.id,
        projectId: session.projectId,
        path,
      });

      const response = (await this.transport.sendRequest(session, 'client/readFile', {
        path,
      } as FileReadRequest)) as FileReadResponse;

      // Cache the result
      this.fileCache.setFile(session.id, path, response.content, response.mtime);

      logger.debug('File read completed and cached', {
        component: 'StreamingFileAccess',
        sessionId: session.id,
        projectId: session.projectId,
        path,
        contentLength: response.content.length,
        mtime: response.mtime,
      });

      return response.content;
    } catch (error) {
      logger.error('File read failed', error as Error, {
        component: 'StreamingFileAccess',
        sessionId: session.id,
        projectId: session.projectId,
        path,
      });

      throw new Error(
        `Failed to read file ${path}: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }

  async writeFile(session: ClientSession, path: string, content: string): Promise<void> {
    try {
      // Check if we can use delta update
      const cachedFile = this.fileCache.getFile(session.id, path);
      let usedDelta = false;

      if (cachedFile && this.deltaProcessor.shouldUseDelta(cachedFile.content, content)) {
        // Try delta update
        const delta = this.deltaProcessor.generateDelta(cachedFile.content, content, path);

        if (delta) {
          try {
            const deltaResponse = (await this.transport.sendRequest(session, 'client/writeDelta', {
              path,
              delta,
            } as DeltaWriteRequest)) as DeltaWriteResponse;

            if (deltaResponse.success && deltaResponse.usedDelta) {
              usedDelta = true;

              logger.info('Delta write successful', {
                component: 'StreamingFileAccess',
                sessionId: session.id,
                projectId: session.projectId,
                path,
                fullSize: content.length,
                deltaSize: delta.deltaSize,
                compressionRatio: Math.round((1 - delta.compressionRatio) * 100) / 100,
              });
            }
          } catch (deltaError) {
            logger.warn('Delta write failed, falling back to full write', {
              component: 'StreamingFileAccess',
              sessionId: session.id,
              projectId: session.projectId,
              path,
              error: deltaError instanceof Error ? deltaError.message : 'Unknown error',
            });
          }
        }
      }

      // Fallback to full file write if delta wasn't used
      if (!usedDelta) {
        await this.transport.sendRequest(session, 'client/writeFile', { path, content });
      }

      // Update cache with new content (always do this)
      const mtime = Date.now();
      this.fileCache.setFile(session.id, path, content, mtime);

      logger.debug('File write completed', {
        component: 'StreamingFileAccess',
        sessionId: session.id,
        projectId: session.projectId,
        path,
        contentLength: content.length,
        usedDelta,
      });
    } catch (error) {
      logger.error('File write failed', error as Error, {
        component: 'StreamingFileAccess',
        sessionId: session.id,
        projectId: session.projectId,
        path,
      });

      throw new Error(
        `Failed to write file ${path}: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }

  async fileExists(session: ClientSession, path: string): Promise<boolean> {
    try {
      const response = await this.transport.sendRequest(session, 'client/fileExists', { path });
      return response.exists;
    } catch (error) {
      // If the request fails, assume file doesn't exist
      return false;
    }
  }

  async listDirectory(session: ClientSession, path: string): Promise<string[]> {
    try {
      const response = await this.transport.sendRequest(session, 'client/listDirectory', { path });
      return response.files || [];
    } catch (error) {
      throw new Error(
        `Failed to list directory ${path}: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }

  async getFileStats(
    session: ClientSession,
    path: string
  ): Promise<{ size: number; mtime: Date; isDirectory: boolean }> {
    try {
      const response = await this.transport.sendRequest(session, 'client/getFileStats', { path });
      return {
        size: response.size,
        mtime: new Date(response.mtime),
        isDirectory: response.isDirectory,
      };
    } catch (error) {
      throw new Error(
        `Failed to get file stats for ${path}: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    }
  }

  // Convert absolute client path to relative path within project
  toProjectPath(session: ClientSession, clientPath: string): string {
    if (clientPath.startsWith(session.projectRoot)) {
      return clientPath.slice(session.projectRoot.length).replace(/^\/+/, '');
    }
    return clientPath;
  }

  // Convert relative project path to absolute client path
  toClientPath(session: ClientSession, projectPath: string): string {
    const cleanPath = projectPath.replace(/^\/+/, '');
    return `${session.projectRoot}/${cleanPath}`.replace(/\/+/g, '/');
  }

  // Handle file change notifications from client with intelligent invalidation
  handleFileChanged(session: ClientSession, notification: FileChangedNotification): void {
    let invalidatedCount = 0;

    switch (notification.changeType) {
      case 'changed':
      case 'deleted': {
        // Invalidate the specific file
        const wasInvalidated = this.fileCache.invalidateFile(session.id, notification.path);
        invalidatedCount = wasInvalidated ? 1 : 0;
        break;
      }

      case 'created':
        // For new files, no cache invalidation needed
        // But we might want to invalidate directory listings if we cached them
        invalidatedCount = 0;
        break;
    }

    logger.debug('File change notification processed with event-driven invalidation', {
      component: 'StreamingFileAccess',
      sessionId: session.id,
      projectId: session.projectId,
      path: notification.path,
      changeType: notification.changeType,
      cacheEntriesInvalidated: invalidatedCount,
      cacheStats: this.fileCache.getStats(),
    });
  }

  // Handle bulk file changes (e.g., git operations, build processes)
  handleBulkFileChanges(session: ClientSession, notifications: FileChangedNotification[]): void {
    let totalInvalidated = 0;

    for (const notification of notifications) {
      if (notification.changeType === 'changed' || notification.changeType === 'deleted') {
        if (this.fileCache.invalidateFile(session.id, notification.path)) {
          totalInvalidated++;
        }
      }
    }

    logger.info('Bulk file changes processed with event-driven invalidation', {
      component: 'StreamingFileAccess',
      sessionId: session.id,
      projectId: session.projectId,
      changedFiles: notifications.length,
      cacheEntriesInvalidated: totalInvalidated,
      cacheStats: this.fileCache.getStats(),
    });
  }

  // Clean up cache for a disconnected session
  cleanupSession(sessionId: string): void {
    const deletedCount = this.fileCache.invalidateSession(sessionId);

    logger.debug('Session cache cleanup completed', {
      component: 'StreamingFileAccess',
      sessionId,
      deletedEntries: deletedCount,
    });
  }

  // Get enhanced cache statistics for monitoring
  getCacheStats() {
    return this.fileCache.getStats();
  }

  // Get delta processor statistics for monitoring
  getDeltaStats() {
    return this.deltaProcessor.getStats();
  }

  // Clean up resources
  dispose(): void {
    this.fileCache.dispose();
  }
}
