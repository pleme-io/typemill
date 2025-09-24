/**
 * FUSE Integration Test Suite
 * Tests the Phase 4 FUSE filesystem isolation features
 */

import { existsSync } from 'node:fs';
import { mkdir, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
// Import native FUSE implementation only
import { FuseMount } from '../../src/fs/fuse-mount.js';
import { FuseOperations } from '../../src/fs/fuse-operations.js';
import { WorkspaceManager } from '../../src/server/workspace-manager.js';
import type { EnhancedClientSession } from '../../src/types/enhanced-session.js';

// Mock WebSocket transport for testing
class MockWebSocketTransport {
  private fuseOperations: any;

  setFuseOperations(ops: any) {
    this.fuseOperations = ops;
  }

  async sendRequest(_session: any, method: string, params: any): Promise<any> {
    // For FUSE operations, we need to simulate async response
    if (method.startsWith('fuse/')) {
      // Simulate async response via handleFuseResponse
      setTimeout(() => {
        const response = this.getResponseForMethod(method, params);
        if (this.fuseOperations?.handleFuseResponse) {
          this.fuseOperations.handleFuseResponse({
            correlationId: params.correlationId,
            success: true,
            data: response,
          });
        }
      }, 10);
      return; // Don't return directly, response comes via handleFuseResponse
    }

    // For non-FUSE operations, return directly
    return this.getResponseForMethod(method, params);
  }

  private getResponseForMethod(method: string, params: any): any {
    // Simulate different FUSE operations
    switch (method) {
      case 'fuse/readdir':
        return ['test-file.txt', 'another-file.js'];

      case 'fuse/stat':
        return {
          mode: 33188, // Regular file
          size: 1024,
          mtime: new Date(),
          atime: new Date(),
          ctime: new Date(),
          uid: 1000,
          gid: 1000,
          dev: 1,
          ino: 12345,
          nlink: 1,
          rdev: 0,
          blksize: 4096,
          blocks: 8,
        };

      case 'fuse/read':
        return Buffer.from('Hello, FUSE world!');

      case 'fuse/write':
        return Buffer.byteLength(params.data);

      case 'fuse/open':
      case 'fuse/release':
        return {};

      default:
        throw new Error(`Unsupported FUSE operation: ${method}`);
    }
  }
}

describe('FUSE Integration Tests (Native FUSE Only)', () => {
  let workspaceManager: WorkspaceManager;
  let testBaseDir: string;
  let mockTransport: MockWebSocketTransport;

  beforeAll(async () => {
    // Create test directories
    testBaseDir = join(tmpdir(), `codeflow-test-${Date.now()}`);
    await mkdir(testBaseDir, { recursive: true });

    mockTransport = new MockWebSocketTransport();
  });

  afterAll(async () => {
    // Cleanup test directories
    if (existsSync(testBaseDir)) {
      await rmdir(testBaseDir, { recursive: true });
    }
  });

  beforeEach(() => {
    workspaceManager = new WorkspaceManager({
      baseWorkspaceDir: join(testBaseDir, 'workspaces'),
      fuseMountPrefix: join(testBaseDir, 'mounts'),
      maxWorkspaces: 10,
      workspaceTimeoutMs: 5000, // 5 seconds for testing
      enableCleanupTimer: false, // Disable for testing
    });
  });

  afterEach(async () => {
    if (workspaceManager) {
      await workspaceManager.shutdown();
    }
  });

  describe('WorkspaceManager', () => {
    test('should create isolated workspace for session', async () => {
      const mockSession = {
        id: 'test-session-1',
        projectId: 'test-project',
      };

      const workspace = await workspaceManager.createWorkspace(mockSession);

      // Store paths before expect calls to avoid corruption
      const workspaceDir = workspace.workspaceDir;
      const fuseMount = workspace.fuseMount;
      const workspaceId = workspace.workspaceId;
      const globalProjectId = workspace.globalProjectId;

      // Test object structure without matchers that corrupt the object
      expect(workspace.sessionId).toBe('test-session-1');
      expect(workspaceId).toMatch(/^[a-f0-9-]{36}$/);
      expect(globalProjectId).toMatch(/^test-project-[a-f0-9-]+$/);
      expect(workspaceDir).toContain('workspaces');
      expect(fuseMount).toContain('mounts');
      expect(workspace.createdAt).toBeInstanceOf(Date);
      expect(workspace.lastAccessed).toBeInstanceOf(Date);

      // Verify directories were created
      expect(existsSync(workspaceDir)).toBe(true);
      expect(existsSync(fuseMount)).toBe(true);
    });

    test('should retrieve workspace for session', async () => {
      const mockSession = {
        id: 'test-session-2',
        projectId: 'test-project',
      };

      const createdWorkspace = await workspaceManager.createWorkspace(mockSession);
      const retrievedWorkspace = workspaceManager.getWorkspace('test-session-2');

      expect(retrievedWorkspace).toEqual(createdWorkspace);
    });

    test('should handle multiple concurrent sessions', async () => {
      const sessions = [
        { id: 'session-1', projectId: 'project-a' },
        { id: 'session-2', projectId: 'project-b' },
        { id: 'session-3', projectId: 'project-c' },
      ];

      const workspaces = await Promise.all(
        sessions.map((session) => workspaceManager.createWorkspace(session))
      );

      // All workspaces should be unique
      const workspaceIds = workspaces.map((w) => w.workspaceId);
      const uniqueIds = new Set(workspaceIds);
      expect(uniqueIds.size).toBe(3);

      // All should have different directories
      const workspaceDirs = workspaces.map((w) => w.workspaceDir);
      const uniqueDirs = new Set(workspaceDirs);
      expect(uniqueDirs.size).toBe(3);
    });

    test('should cleanup workspace on session disconnect', async () => {
      const mockSession = {
        id: 'test-session-cleanup',
        projectId: 'test-project',
      };

      const workspace = await workspaceManager.createWorkspace(mockSession);
      const { workspaceDir, fuseMount } = workspace;

      // Verify directories exist
      expect(existsSync(workspaceDir)).toBe(true);
      expect(existsSync(fuseMount)).toBe(true);

      // Cleanup workspace
      await workspaceManager.cleanupWorkspace('test-session-cleanup');

      // Verify directories are removed
      expect(existsSync(workspaceDir)).toBe(false);
      expect(existsSync(fuseMount)).toBe(false);

      // Verify workspace is no longer tracked
      expect(workspaceManager.getWorkspace('test-session-cleanup')).toBeUndefined();
    });

    test('should respect workspace limits', async () => {
      // Create manager with limit of 2
      const limitedManager = new WorkspaceManager({
        baseWorkspaceDir: join(testBaseDir, 'limited-workspaces'),
        fuseMountPrefix: join(testBaseDir, 'limited-mounts'),
        maxWorkspaces: 2,
        workspaceTimeoutMs: 5000,
        enableCleanupTimer: false,
      });

      try {
        // Create 3 sessions (exceeds limit)
        const _workspace1 = await limitedManager.createWorkspace({
          id: 'sess1',
          projectId: 'proj1',
        });
        const _workspace2 = await limitedManager.createWorkspace({
          id: 'sess2',
          projectId: 'proj2',
        });
        const _workspace3 = await limitedManager.createWorkspace({
          id: 'sess3',
          projectId: 'proj3',
        });

        // Should still only have 2 workspaces (oldest should be cleaned up)
        const stats = limitedManager.getStats();
        expect(stats.totalWorkspaces).toBe(2);

        // Session 1 should have been cleaned up
        expect(limitedManager.getWorkspace('sess1')).toBeUndefined();
        expect(limitedManager.getWorkspace('sess2')).toBeDefined();
        expect(limitedManager.getWorkspace('sess3')).toBeDefined();
      } finally {
        await limitedManager.shutdown();
      }
    });

    test('should provide workspace statistics', async () => {
      const sessions = [
        { id: 'stats-1', projectId: 'project-stats' },
        { id: 'stats-2', projectId: 'project-stats' },
      ];

      await Promise.all(sessions.map((s) => workspaceManager.createWorkspace(s)));

      const stats = workspaceManager.getStats();
      expect(stats).toMatchObject({
        totalWorkspaces: 2,
        activeSessions: 2,
        oldestWorkspaceAge: expect.any(Number),
        newestWorkspaceAge: expect.any(Number),
      });
    });
  });

  describe('FuseOperations', () => {
    let fuseOps: FuseOperations;
    let mockEnhancedSession: EnhancedClientSession;

    beforeEach(() => {
      mockEnhancedSession = {
        id: 'fuse-test-session',
        projectId: 'fuse-test-project',
        projectRoot: '/test/root',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'global-fuse-test-123',
        workspaceId: 'workspace-fuse-test',
        fuseMount: '/tmp/fuse-mount-test',
        workspaceDir: '/tmp/workspace-test',
        permissions: ['file:read', 'file:write', 'lsp:query', 'lsp:symbol', 'session:manage'],
      } as any;

      fuseOps = new FuseOperations(mockEnhancedSession, mockTransport as any);
      mockTransport.setFuseOperations(fuseOps);
    });

    test('should handle readdir operation', async () => {
      const entries = await fuseOps.readdir('/test/path');
      expect(entries).toEqual(['test-file.txt', 'another-file.js']);
    });

    test('should handle getattr operation', async () => {
      const stats = await fuseOps.getattr('/test/file.txt');

      expect(stats).toMatchObject({
        mode: 33188,
        size: 1024,
        mtime: expect.any(Date),
        atime: expect.any(Date),
        ctime: expect.any(Date),
        uid: 1000,
        gid: 1000,
      });
    });

    test('should handle file operations (open/read/write/release)', async () => {
      // Open file
      const fd = await fuseOps.open('/test/file.txt', 0);
      expect(typeof fd).toBe('number');
      expect(fd).toBeGreaterThan(0);

      // Read file
      const content = await fuseOps.read('/test/file.txt', fd, 1024, 0);
      expect(Buffer.isBuffer(content)).toBe(true);
      expect(content.toString()).toBe('Hello, FUSE world!');

      // Write file
      const testData = Buffer.from('Test write data');
      const bytesWritten = await fuseOps.write('/test/file.txt', fd, testData, 0);
      expect(bytesWritten).toBe(testData.length);

      // Release file
      await expect(fuseOps.release('/test/file.txt', fd)).resolves.toBeUndefined();
    });

    test('should handle read-only filesystem operations', async () => {
      // These operations should throw "Read-only filesystem" error
      await expect(fuseOps.mkdir('/test/newdir', 0o755)).rejects.toThrow('Read-only filesystem');
      await expect(fuseOps.rmdir('/test/dir')).rejects.toThrow('Read-only filesystem');
      await expect(fuseOps.unlink('/test/file.txt')).rejects.toThrow('Read-only filesystem');
      await expect(fuseOps.rename('/test/old.txt', '/test/new.txt')).rejects.toThrow(
        'Read-only filesystem'
      );
    });

    test('should handle operation timeouts', async () => {
      // Create a FUSE operations instance with mock that doesn't respond
      const slowTransport = {
        sendRequest: jest
          .fn()
          .mockImplementation(
            () =>
              new Promise((_, reject) =>
                setTimeout(() => reject(new Error('Operation timed out')), 100)
              )
          ),
        setFuseOperations: jest.fn(),
      };

      const slowFuseOps = new FuseOperations(mockEnhancedSession, slowTransport as any);
      slowTransport.setFuseOperations(slowFuseOps);

      // This should timeout quickly for testing
      await expect(slowFuseOps.readdir('/test')).rejects.toThrow('Operation timed out');
    });

    test('should cleanup pending operations', () => {
      // Cleanup should not throw
      expect(() => fuseOps.cleanup()).not.toThrow();
    });
  });

  describe('FuseMount Integration', () => {
    test('should create FuseMount instance with proper configuration', async () => {
      const mockEnhancedSession: EnhancedClientSession = {
        id: 'mount-test-session',
        projectId: 'mount-test-project',
        projectRoot: '/test/root',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'global-mount-test-123',
        workspaceId: 'workspace-mount-test',
        fuseMount: '/tmp/fuse-mount-test',
        workspaceDir: '/tmp/workspace-test',
        permissions: ['file:read', 'file:write', 'lsp:query', 'lsp:symbol', 'session:manage'],
      } as any;

      const mountPath = join(testBaseDir, 'test-mount');
      const fuseMount = new FuseMount(mockEnhancedSession, mockTransport as any, mountPath, {
        debugFuse: false,
        allowOther: false,
        defaultPermissions: true,
      });

      expect(fuseMount).toBeDefined();
      expect(fuseMount.getMountPath()).toBe(mountPath);
      expect(fuseMount.isMounted()).toBe(false);

      // Note: Actual FUSE mounting requires elevated privileges
      // These tests verify the interface without actual mounting
    });

    test('should provide mount statistics', () => {
      const mockEnhancedSession: EnhancedClientSession = {
        id: 'stats-test-session',
        projectId: 'stats-test-project',
        projectRoot: '/test/root',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'global-stats-test-123',
        workspaceId: 'workspace-stats-test',
        fuseMount: '/tmp/fuse-mount-stats',
        workspaceDir: '/tmp/workspace-stats',
      };

      const mountPath = join(testBaseDir, 'stats-mount');
      const fuseMount = new FuseMount(mockEnhancedSession, mockTransport as any, mountPath);

      const stats = fuseMount.getStats();
      expect(stats).toMatchObject({
        mounted: false,
        mountPath,
        sessionId: 'stats-test-session',
      });

      // For native FUSE, verify the standard statistics
      expect(stats.pendingOperations).toBe(0);
      expect(stats.openFiles).toBe(0);
    });
  });

  describe('Enhanced Session Integration', () => {
    test('should support enhanced session workflow', async () => {
      const mockSession = {
        id: 'enhanced-session-test',
        projectId: 'enhanced-project',
        initialized: true,
        permissions: ['file:read', 'file:write', 'lsp:query', 'lsp:symbol', 'session:manage'],
      } as any;

      // Create workspace
      const workspace = await workspaceManager.createWorkspace(mockSession);

      // Create enhanced session
      const enhancedSession: EnhancedClientSession = {
        id: mockSession.id,
        projectId: mockSession.projectId,
        projectRoot: '/test/root',
        socket: {} as any,
        initialized: true,
        globalProjectId: workspace.globalProjectId,
        workspaceId: workspace.workspaceId,
        fuseMount: workspace.fuseMount,
        workspaceDir: workspace.workspaceDir,
        permissions: mockSession.permissions,
      } as any;

      // Create FUSE operations for this session
      const fuseOps = new FuseOperations(enhancedSession, mockTransport as any);
      mockTransport.setFuseOperations(fuseOps);

      // Test basic operations work
      const entries = await fuseOps.readdir('/test');
      expect(entries).toEqual(['test-file.txt', 'another-file.js']);

      // Cleanup
      fuseOps.cleanup();
      await workspaceManager.cleanupWorkspace(mockSession.id);
    });

    test('should handle multiple sessions with isolation', async () => {
      const sessions = [
        {
          id: 'isolated-1',
          projectId: 'project-a',
          initialized: true,
          permissions: ['file:read', 'file:write'],
        },
        {
          id: 'isolated-2',
          projectId: 'project-b',
          initialized: true,
          permissions: ['file:read', 'file:write'],
        },
      ] as any[];

      const workspaces = await Promise.all(
        sessions.map((s) => workspaceManager.createWorkspace(s))
      );

      // Create FUSE operations for each session
      const fuseOperations = workspaces.map((workspace, i) => {
        const enhancedSession: EnhancedClientSession = {
          id: sessions[i]!.id,
          projectId: sessions[i]!.projectId,
          projectRoot: '/test/root',
          socket: {} as any,
          initialized: true,
          globalProjectId: workspace.globalProjectId,
          workspaceId: workspace.workspaceId,
          fuseMount: workspace.fuseMount,
          workspaceDir: workspace.workspaceDir,
          permissions: sessions[i]!.permissions,
        } as any;

        // Create separate transport for each session to avoid conflicts
        const sessionTransport = new MockWebSocketTransport();
        const ops = new FuseOperations(enhancedSession, sessionTransport as any);
        sessionTransport.setFuseOperations(ops);
        return ops;
      });

      // Test that both can operate independently
      const results = await Promise.all([
        fuseOperations[0]!.readdir('/test1'),
        fuseOperations[1]!.readdir('/test2'),
      ]);

      expect(results[0]).toEqual(['test-file.txt', 'another-file.js']);
      expect(results[1]).toEqual(['test-file.txt', 'another-file.js']);

      // Cleanup
      fuseOperations.forEach((ops) => ops.cleanup());
      await Promise.all(sessions.map((s) => workspaceManager.cleanupWorkspace(s.id)));
    });
  });

  describe('Error Handling', () => {
    test('should handle workspace creation errors gracefully', async () => {
      const invalidManager = new WorkspaceManager({
        baseWorkspaceDir: '/invalid/readonly/path',
        fuseMountPrefix: '/invalid/readonly/mounts',
        maxWorkspaces: 1,
        workspaceTimeoutMs: 1000,
        enableCleanupTimer: false,
      });

      const mockSession = {
        id: 'error-test-session',
        projectId: 'error-test-project',
      };

      try {
        await expect(invalidManager.createWorkspace(mockSession)).rejects.toThrow();
      } finally {
        await invalidManager.shutdown();
      }
    });

    test('should handle FUSE operation errors', async () => {
      const errorTransport = {
        sendRequest: jest.fn().mockRejectedValue(new Error('Network error')),
        setFuseOperations: jest.fn(),
      };

      const mockEnhancedSession: EnhancedClientSession = {
        id: 'error-fuse-session',
        projectId: 'error-fuse-project',
        projectRoot: '/test/root',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'global-error-test-123',
        workspaceId: 'workspace-error-test',
        fuseMount: '/tmp/fuse-mount-error',
        workspaceDir: '/tmp/workspace-error',
        permissions: ['file:read', 'file:write'],
      } as any;

      const errorFuseOps = new FuseOperations(mockEnhancedSession, errorTransport as any);
      errorTransport.setFuseOperations(errorFuseOps);

      await expect(errorFuseOps.readdir('/test')).rejects.toThrow('Network error');
    });

    test('should handle missing workspace gracefully', () => {
      expect(workspaceManager.getWorkspace('non-existent-session')).toBeUndefined();
    });
  });
});
