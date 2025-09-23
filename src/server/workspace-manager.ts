/**
 * Workspace Manager for isolated project environments with FUSE mounts
 * Manages creation, cleanup, and lifecycle of isolated workspaces
 */

import { randomUUID } from 'node:crypto';
import { access, mkdir, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { logger } from '../core/diagnostics/logger.js';
import type { EnhancedClientSession, WorkspaceInfo } from '../types/enhanced-session.js';

export interface WorkspaceManagerConfig {
  baseWorkspaceDir: string;
  fuseMountPrefix: string;
  maxWorkspaces: number;
  workspaceTimeoutMs: number;
  enableCleanupTimer: boolean;
}

export class WorkspaceManager {
  private workspaces = new Map<string, WorkspaceInfo>();
  private sessions = new Map<string, string>(); // sessionId -> workspaceId
  private cleanupTimer?: NodeJS.Timeout;
  private config: WorkspaceManagerConfig;

  constructor(config: Partial<WorkspaceManagerConfig> = {}) {
    this.config = {
      baseWorkspaceDir: config.baseWorkspaceDir || join(tmpdir(), 'codeflow-workspaces'),
      fuseMountPrefix: config.fuseMountPrefix || join(tmpdir(), 'codeflow-mounts'),
      maxWorkspaces: config.maxWorkspaces || 50,
      workspaceTimeoutMs: config.workspaceTimeoutMs || 3600000, // 1 hour
      enableCleanupTimer: config.enableCleanupTimer ?? true,
      ...config,
    };

    if (this.config.enableCleanupTimer) {
      this.startCleanupTimer();
    }
  }

  /**
   * Create a new isolated workspace for a session
   */
  async createWorkspace(
    session: Pick<EnhancedClientSession, 'id' | 'projectId'>
  ): Promise<WorkspaceInfo> {
    // Check workspace limit
    if (this.workspaces.size >= this.config.maxWorkspaces) {
      await this.cleanupOldestWorkspace();
    }

    // Generate unique identifiers
    const workspaceId = randomUUID();
    const globalProjectId = `${session.projectId}-${randomUUID()}`;
    const workspaceDir = join(this.config.baseWorkspaceDir, workspaceId);
    const fuseMount = join(this.config.fuseMountPrefix, workspaceId);

    try {
      // Create workspace directories
      await mkdir(workspaceDir, { recursive: true });
      await mkdir(fuseMount, { recursive: true });

      const workspaceInfo: WorkspaceInfo = {
        workspaceId,
        workspaceDir,
        fuseMount,
        sessionId: session.id,
        globalProjectId,
        createdAt: new Date(),
        lastAccessed: new Date(),
      };

      // Store workspace info
      this.workspaces.set(workspaceId, workspaceInfo);
      this.sessions.set(session.id, workspaceId);

      logger.info('Created isolated workspace', {
        component: 'WorkspaceManager',
        sessionId: session.id,
        workspaceId,
        globalProjectId,
        workspaceDir,
        fuseMount,
      });

      return workspaceInfo;
    } catch (error) {
      logger.error('Failed to create workspace', error as Error, {
        component: 'WorkspaceManager',
        sessionId: session.id,
        workspaceId,
        workspaceDir,
        fuseMount,
      });
      throw error;
    }
  }

  /**
   * Get workspace info for a session
   */
  getWorkspace(sessionId: string): WorkspaceInfo | undefined {
    const workspaceId = this.sessions.get(sessionId);
    if (!workspaceId) return undefined;

    const workspace = this.workspaces.get(workspaceId);
    if (workspace) {
      // Update last accessed time
      workspace.lastAccessed = new Date();
    }

    return workspace;
  }

  /**
   * Get workspace info by workspace ID
   */
  getWorkspaceById(workspaceId: string): WorkspaceInfo | undefined {
    const workspace = this.workspaces.get(workspaceId);
    if (workspace) {
      workspace.lastAccessed = new Date();
    }
    return workspace;
  }

  /**
   * Clean up workspace for a session
   */
  async cleanupWorkspace(sessionId: string): Promise<void> {
    const workspaceId = this.sessions.get(sessionId);
    if (!workspaceId) return;

    const workspace = this.workspaces.get(workspaceId);
    if (!workspace) return;

    try {
      // Remove directories if they exist
      await this.removeDirectorySafe(workspace.workspaceDir);
      await this.removeDirectorySafe(workspace.fuseMount);

      // Remove from maps
      this.workspaces.delete(workspaceId);
      this.sessions.delete(sessionId);

      logger.info('Cleaned up workspace', {
        component: 'WorkspaceManager',
        sessionId,
        workspaceId,
        workspaceDir: workspace.workspaceDir,
        fuseMount: workspace.fuseMount,
      });
    } catch (error) {
      logger.error('Failed to cleanup workspace', error as Error, {
        component: 'WorkspaceManager',
        sessionId,
        workspaceId,
        workspace,
      });
    }
  }

  /**
   * Clean up oldest workspace to make room for new ones
   */
  private async cleanupOldestWorkspace(): Promise<void> {
    let oldestWorkspace: WorkspaceInfo | null = null;
    let oldestWorkspaceId = '';

    for (const [workspaceId, workspace] of this.workspaces) {
      if (!oldestWorkspace || workspace.lastAccessed < oldestWorkspace.lastAccessed) {
        oldestWorkspace = workspace;
        oldestWorkspaceId = workspaceId;
      }
    }

    if (oldestWorkspace) {
      logger.info('Cleaning up oldest workspace to make room', {
        component: 'WorkspaceManager',
        workspaceId: oldestWorkspaceId,
        lastAccessed: oldestWorkspace.lastAccessed,
      });
      await this.cleanupWorkspace(oldestWorkspace.sessionId);
    }
  }

  /**
   * Start periodic cleanup timer for expired workspaces
   */
  private startCleanupTimer(): void {
    const interval = Math.min(this.config.workspaceTimeoutMs / 4, 300000); // Check every 5 minutes max

    this.cleanupTimer = setInterval(() => {
      this.cleanupExpiredWorkspaces().catch((error) => {
        logger.error('Failed to cleanup expired workspaces', error as Error, {
          component: 'WorkspaceManager',
        });
      });
    }, interval);
  }

  /**
   * Clean up workspaces that have exceeded timeout
   */
  private async cleanupExpiredWorkspaces(): Promise<void> {
    const now = new Date();
    const expiredWorkspaces: string[] = [];

    for (const [workspaceId, workspace] of this.workspaces) {
      const timeSinceAccess = now.getTime() - workspace.lastAccessed.getTime();
      if (timeSinceAccess > this.config.workspaceTimeoutMs) {
        expiredWorkspaces.push(workspace.sessionId);
      }
    }

    if (expiredWorkspaces.length > 0) {
      logger.info('Cleaning up expired workspaces', {
        component: 'WorkspaceManager',
        count: expiredWorkspaces.length,
        expiredWorkspaces,
      });

      for (const sessionId of expiredWorkspaces) {
        await this.cleanupWorkspace(sessionId);
      }
    }
  }

  /**
   * Safely remove directory, ignoring errors if it doesn't exist
   */
  private async removeDirectorySafe(dirPath: string): Promise<void> {
    try {
      await access(dirPath);
      await rmdir(dirPath, { recursive: true });
    } catch (error) {
      // Ignore errors if directory doesn't exist
      if ((error as any).code !== 'ENOENT') {
        throw error;
      }
    }
  }

  /**
   * Get statistics about active workspaces
   */
  getStats(): {
    totalWorkspaces: number;
    activeSessions: number;
    oldestWorkspaceAge: number;
    newestWorkspaceAge: number;
  } {
    const now = new Date();
    let oldestAge = 0;
    let newestAge = 0;

    for (const workspace of this.workspaces.values()) {
      const age = now.getTime() - workspace.createdAt.getTime();
      if (oldestAge === 0 || age > oldestAge) oldestAge = age;
      if (newestAge === 0 || age < newestAge) newestAge = age;
    }

    return {
      totalWorkspaces: this.workspaces.size,
      activeSessions: this.sessions.size,
      oldestWorkspaceAge: oldestAge,
      newestWorkspaceAge: newestAge,
    };
  }

  /**
   * Clean up all workspaces and stop cleanup timer
   */
  async shutdown(): Promise<void> {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = undefined;
    }

    const sessionIds = Array.from(this.sessions.keys());
    logger.info('Shutting down workspace manager', {
      component: 'WorkspaceManager',
      totalWorkspaces: sessionIds.length,
    });

    for (const sessionId of sessionIds) {
      await this.cleanupWorkspace(sessionId);
    }
  }

  /**
   * List all active workspaces (for debugging/monitoring)
   */
  listWorkspaces(): WorkspaceInfo[] {
    return Array.from(this.workspaces.values());
  }

  /**
   * Force cleanup of a specific workspace by ID
   */
  async forceCleanupWorkspace(workspaceId: string): Promise<boolean> {
    const workspace = this.workspaces.get(workspaceId);
    if (!workspace) return false;

    await this.cleanupWorkspace(workspace.sessionId);
    return true;
  }
}
