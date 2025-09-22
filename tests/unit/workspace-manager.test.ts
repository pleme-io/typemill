/**
 * Workspace Manager Unit Tests
 * Tests workspace creation, management, and cleanup functionality
 */

import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { mkdir, rmdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { WorkspaceManager } from '../../src/server/workspace-manager.js';
import type { WorkspaceInfo } from '../../src/types/enhanced-session.js';

describe('WorkspaceManager Unit Tests', () => {
  let testBaseDir: string;
  let workspaceManager: WorkspaceManager;

  beforeAll(async () => {
    testBaseDir = join(tmpdir(), `workspace-manager-test-${Date.now()}`);
    await mkdir(testBaseDir, { recursive: true });
  });

  afterAll(async () => {
    if (existsSync(testBaseDir)) {
      await rmdir(testBaseDir, { recursive: true });
    }
  });

  beforeEach(() => {
    workspaceManager = new WorkspaceManager({
      baseWorkspaceDir: join(testBaseDir, 'workspaces'),
      fuseMountPrefix: join(testBaseDir, 'mounts'),
      maxWorkspaces: 5,
      workspaceTimeoutMs: 2000, // 2 seconds for faster testing
      enableCleanupTimer: false // Disable automatic cleanup for testing
    });
  });

  afterEach(async () => {
    if (workspaceManager) {
      await workspaceManager.shutdown();
    }
  });

  describe('Workspace Creation', () => {
    test('should create workspace with unique identifiers', async () => {
      const session = { id: 'test-session-1', projectId: 'test-project' };
      const workspace = await workspaceManager.createWorkspace(session);

      expect(workspace).toMatchObject({
        sessionId: 'test-session-1',
        workspaceId: expect.stringMatching(/^[a-f0-9-]{36}$/), // UUID format
        globalProjectId: expect.stringMatching(/^test-project-[a-f0-9-]{36}$/),
        workspaceDir: expect.stringContaining('workspaces'),
        fuseMount: expect.stringContaining('mounts'),
        createdAt: expect.any(Date),
        lastAccessed: expect.any(Date)
      });

      // Verify directories were created (check immediately after creation)
      expect(existsSync(workspace.workspaceDir)).toBe(true);
      expect(existsSync(workspace.fuseMount)).toBe(true);

      // Clean up this specific workspace for this test
      await workspaceManager.cleanupWorkspace(session.id);

      // Verify cleanup worked
      expect(existsSync(workspace.workspaceDir)).toBe(false);
      expect(existsSync(workspace.fuseMount)).toBe(false);
    });

    test('should create unique workspaces for different sessions', async () => {
      const session1 = { id: 'session-1', projectId: 'project-a' };
      const session2 = { id: 'session-2', projectId: 'project-b' };

      const workspace1 = await workspaceManager.createWorkspace(session1);
      const workspace2 = await workspaceManager.createWorkspace(session2);

      // All identifiers should be unique
      expect(workspace1.workspaceId).not.toBe(workspace2.workspaceId);
      expect(workspace1.globalProjectId).not.toBe(workspace2.globalProjectId);
      expect(workspace1.workspaceDir).not.toBe(workspace2.workspaceDir);
      expect(workspace1.fuseMount).not.toBe(workspace2.fuseMount);
    });

    test('should handle same project ID with different sessions', async () => {
      const session1 = { id: 'session-1', projectId: 'same-project' };
      const session2 = { id: 'session-2', projectId: 'same-project' };

      const workspace1 = await workspaceManager.createWorkspace(session1);
      const workspace2 = await workspaceManager.createWorkspace(session2);

      // Should still create isolated workspaces even with same project ID
      expect(workspace1.workspaceId).not.toBe(workspace2.workspaceId);
      expect(workspace1.globalProjectId).not.toBe(workspace2.globalProjectId);
      expect(workspace1.globalProjectId).toContain('same-project');
      expect(workspace2.globalProjectId).toContain('same-project');
    });
  });

  describe('Workspace Retrieval', () => {
    test('should retrieve workspace by session ID', async () => {
      const session = { id: 'retrieval-test', projectId: 'test-project' };
      const created = await workspaceManager.createWorkspace(session);

      const retrieved = workspaceManager.getWorkspace('retrieval-test');
      expect(retrieved).toEqual(created);
    });

    test('should retrieve workspace by workspace ID', async () => {
      const session = { id: 'retrieval-test-2', projectId: 'test-project' };
      const created = await workspaceManager.createWorkspace(session);

      const retrieved = workspaceManager.getWorkspaceById(created.workspaceId);
      expect(retrieved).toEqual(created);
    });

    test('should return undefined for non-existent session', () => {
      const result = workspaceManager.getWorkspace('non-existent-session');
      expect(result).toBeUndefined();
    });

    test('should return undefined for non-existent workspace ID', () => {
      const result = workspaceManager.getWorkspaceById('non-existent-workspace');
      expect(result).toBeUndefined();
    });

    test('should update last accessed time on retrieval', async () => {
      const session = { id: 'access-time-test', projectId: 'test-project' };
      const created = await workspaceManager.createWorkspace(session);
      const initialAccessTime = created.lastAccessed;

      // Wait a bit to ensure time difference
      await new Promise(resolve => setTimeout(resolve, 10));

      const retrieved = workspaceManager.getWorkspace('access-time-test');
      expect(retrieved!.lastAccessed.getTime()).toBeGreaterThan(initialAccessTime.getTime());
    });
  });

  describe('Workspace Cleanup', () => {
    test('should cleanup workspace and remove directories', async () => {
      const session = { id: 'cleanup-test', projectId: 'test-project' };
      const workspace = await workspaceManager.createWorkspace(session);

      // Verify directories exist
      expect(existsSync(workspace.workspaceDir)).toBe(true);
      expect(existsSync(workspace.fuseMount)).toBe(true);

      // Cleanup
      await workspaceManager.cleanupWorkspace('cleanup-test');

      // Verify directories are removed
      expect(existsSync(workspace.workspaceDir)).toBe(false);
      expect(existsSync(workspace.fuseMount)).toBe(false);

      // Verify workspace is no longer tracked
      expect(workspaceManager.getWorkspace('cleanup-test')).toBeUndefined();
    });

    test('should handle cleanup of non-existent workspace gracefully', async () => {
      // Should not throw
      await expect(workspaceManager.cleanupWorkspace('non-existent')).resolves.toBeUndefined();
    });

    test('should force cleanup workspace by ID', async () => {
      const session = { id: 'force-cleanup-test', projectId: 'test-project' };
      const workspace = await workspaceManager.createWorkspace(session);

      const success = await workspaceManager.forceCleanupWorkspace(workspace.workspaceId);
      expect(success).toBe(true);

      // Verify workspace is cleaned up
      expect(workspaceManager.getWorkspace('force-cleanup-test')).toBeUndefined();
      expect(existsSync(workspace.workspaceDir)).toBe(false);
      expect(existsSync(workspace.fuseMount)).toBe(false);
    });

    test('should return false for force cleanup of non-existent workspace', async () => {
      const success = await workspaceManager.forceCleanupWorkspace('non-existent-workspace-id');
      expect(success).toBe(false);
    });
  });

  describe('Workspace Limits', () => {
    test('should enforce maximum workspace limit', async () => {
      // Create manager with limit of 2
      const limitedManager = new WorkspaceManager({
        baseWorkspaceDir: join(testBaseDir, 'limited'),
        fuseMountPrefix: join(testBaseDir, 'limited-mounts'),
        maxWorkspaces: 2,
        workspaceTimeoutMs: 1000,
        enableCleanupTimer: false
      });

      try {
        // Create 3 workspaces (exceeds limit)
        await limitedManager.createWorkspace({ id: 'limit-1', projectId: 'proj' });
        await limitedManager.createWorkspace({ id: 'limit-2', projectId: 'proj' });
        await limitedManager.createWorkspace({ id: 'limit-3', projectId: 'proj' });

        const stats = limitedManager.getStats();
        expect(stats.totalWorkspaces).toBe(2);

        // Oldest workspace should be cleaned up
        expect(limitedManager.getWorkspace('limit-1')).toBeUndefined();
        expect(limitedManager.getWorkspace('limit-2')).toBeDefined();
        expect(limitedManager.getWorkspace('limit-3')).toBeDefined();
      } finally {
        await limitedManager.shutdown();
      }
    });

    test('should cleanup oldest workspace when limit exceeded', async () => {
      const limitedManager = new WorkspaceManager({
        baseWorkspaceDir: join(testBaseDir, 'oldest-cleanup'),
        fuseMountPrefix: join(testBaseDir, 'oldest-cleanup-mounts'),
        maxWorkspaces: 2,
        workspaceTimeoutMs: 1000,
        enableCleanupTimer: false
      });

      try {
        const workspace1 = await limitedManager.createWorkspace({ id: 'oldest-1', projectId: 'proj' });

        // Wait to ensure different creation times
        await new Promise(resolve => setTimeout(resolve, 10));

        const workspace2 = await limitedManager.createWorkspace({ id: 'oldest-2', projectId: 'proj' });

        // Wait again
        await new Promise(resolve => setTimeout(resolve, 10));

        const workspace3 = await limitedManager.createWorkspace({ id: 'oldest-3', projectId: 'proj' });

        // Workspace 1 should be cleaned up (oldest)
        expect(limitedManager.getWorkspace('oldest-1')).toBeUndefined();
        expect(limitedManager.getWorkspace('oldest-2')).toBeDefined();
        expect(limitedManager.getWorkspace('oldest-3')).toBeDefined();

        // Verify directory was actually removed
        expect(existsSync(workspace1.workspaceDir)).toBe(false);
      } finally {
        await limitedManager.shutdown();
      }
    });
  });

  describe('Statistics and Monitoring', () => {
    test('should provide accurate statistics', async () => {
      const sessions = [
        { id: 'stats-1', projectId: 'project-a' },
        { id: 'stats-2', projectId: 'project-b' },
        { id: 'stats-3', projectId: 'project-c' }
      ];

      const startTime = Date.now();
      await Promise.all(sessions.map(s => workspaceManager.createWorkspace(s)));
      const endTime = Date.now();

      const stats = workspaceManager.getStats();

      expect(stats.totalWorkspaces).toBe(3);
      expect(stats.activeSessions).toBe(3);
      expect(stats.oldestWorkspaceAge).toBeGreaterThanOrEqual(0);
      expect(stats.newestWorkspaceAge).toBeGreaterThanOrEqual(0);
      expect(stats.oldestWorkspaceAge).toBeGreaterThanOrEqual(stats.newestWorkspaceAge);

      // Ages should be reasonable (within test execution time + some buffer)
      const maxExpectedAge = endTime - startTime + 1000; // Add 1 second buffer
      expect(stats.oldestWorkspaceAge).toBeLessThan(maxExpectedAge);
    });

    test('should list all active workspaces', async () => {
      const sessions = [
        { id: 'list-1', projectId: 'project-x' },
        { id: 'list-2', projectId: 'project-y' }
      ];

      const created = await Promise.all(sessions.map(s => workspaceManager.createWorkspace(s)));
      const listed = workspaceManager.listWorkspaces();

      expect(listed).toHaveLength(2);
      expect(listed).toEqual(expect.arrayContaining(created));
    });

    test('should return empty statistics when no workspaces exist', () => {
      const stats = workspaceManager.getStats();

      expect(stats).toEqual({
        totalWorkspaces: 0,
        activeSessions: 0,
        oldestWorkspaceAge: 0,
        newestWorkspaceAge: 0
      });
    });
  });

  describe('Configuration Validation', () => {
    test('should use default configuration when not provided', () => {
      const defaultManager = new WorkspaceManager();

      const stats = defaultManager.getStats();
      expect(stats.totalWorkspaces).toBe(0);

      // Should not throw
      expect(() => defaultManager.shutdown()).not.toThrow();
    });

    test('should accept partial configuration', () => {
      const partialManager = new WorkspaceManager({
        maxWorkspaces: 3,
        workspaceTimeoutMs: 5000
      });

      const stats = partialManager.getStats();
      expect(stats.totalWorkspaces).toBe(0);

      partialManager.shutdown();
    });
  });

  describe('Error Handling', () => {
    test('should handle directory creation failures gracefully', async () => {
      const failingManager = new WorkspaceManager({
        baseWorkspaceDir: '/invalid/readonly/path',
        fuseMountPrefix: '/invalid/readonly/mounts',
        maxWorkspaces: 1,
        workspaceTimeoutMs: 1000,
        enableCleanupTimer: false
      });

      const session = { id: 'error-test', projectId: 'error-project' };

      try {
        await expect(failingManager.createWorkspace(session)).rejects.toThrow();
      } finally {
        await failingManager.shutdown();
      }
    });

    test('should handle cleanup errors without throwing', async () => {
      const session = { id: 'cleanup-error-test', projectId: 'test-project' };
      const workspace = await workspaceManager.createWorkspace(session);

      // Manually remove directories to simulate cleanup error
      await rmdir(workspace.workspaceDir, { recursive: true });
      await rmdir(workspace.fuseMount, { recursive: true });

      // Cleanup should not throw even when directories don't exist
      await expect(workspaceManager.cleanupWorkspace('cleanup-error-test')).resolves.toBeUndefined();
    });
  });

  describe('Shutdown Behavior', () => {
    test('should cleanup all workspaces on shutdown', async () => {
      const sessions = [
        { id: 'shutdown-1', projectId: 'project-shutdown' },
        { id: 'shutdown-2', projectId: 'project-shutdown' }
      ];

      const workspaces = await Promise.all(sessions.map(s => workspaceManager.createWorkspace(s)));

      // Verify workspaces exist
      expect(workspaceManager.getStats().totalWorkspaces).toBe(2);
      workspaces.forEach(w => {
        expect(existsSync(w.workspaceDir)).toBe(true);
        expect(existsSync(w.fuseMount)).toBe(true);
      });

      // Shutdown
      await workspaceManager.shutdown();

      // Verify all directories are cleaned up
      workspaces.forEach(w => {
        expect(existsSync(w.workspaceDir)).toBe(false);
        expect(existsSync(w.fuseMount)).toBe(false);
      });
    });

    test('should be safe to call shutdown multiple times', async () => {
      await workspaceManager.shutdown();
      await expect(workspaceManager.shutdown()).resolves.toBeUndefined();
    });
  });
});