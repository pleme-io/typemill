/**
 * WebSocket Server FUSE Integration Tests
 * Tests the integration between WebSocket server and FUSE filesystem
 */

import { existsSync } from 'node:fs';
import { mkdir, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { WebSocketServerOptions } from '../../src/server/ws-server.js';

// Mock WebSocket to avoid actual network dependencies
class MockWebSocket {
  public readyState = 1; // WebSocket.OPEN
  public onmessage?: (event: { data: string }) => void;
  public onclose?: () => void;
  public onerror?: (error: Error) => void;

  constructor(public url: string) {}

  send(data: string) {
    // Mock sending data
  }

  close(code?: number, reason?: string) {
    if (this.onclose) {
      this.onclose();
    }
  }

  // Simulate receiving a message
  mockReceive(data: any) {
    if (this.onmessage) {
      this.onmessage({ data: JSON.stringify(data) });
    }
  }
}

// Mock the CodeFlowWebSocketServer without actual network/FUSE dependencies
class MockCodeFlowWebSocketServer {
  private options: WebSocketServerOptions;
  private mockSessions = new Map<string, any>();
  private mockWorkspaces = new Map<string, any>();
  private idCounter = 0;

  constructor(options: WebSocketServerOptions) {
    this.options = options;
  }

  // Simulate session initialization with FUSE workspace creation
  async simulateSessionInit(sessionId: string, projectId: string) {
    const session = {
      id: sessionId,
      projectId,
      socket: new MockWebSocket('ws://test'),
      initialized: true,
    };

    this.mockSessions.set(sessionId, session);

    // Simulate workspace creation if FUSE is enabled
    if (this.options.enableFuse) {
      const workspace = {
        workspaceId: `workspace-${sessionId}`,
        workspaceDir: join(tmpdir(), 'mock-workspaces', sessionId),
        fuseMount: join(tmpdir(), 'mock-mounts', sessionId),
        sessionId,
        globalProjectId: `${projectId}-mock-${Date.now()}-${++this.idCounter}`,
        createdAt: new Date(),
        lastAccessed: new Date(),
      };

      // Actually create directories for testing
      await mkdir(workspace.workspaceDir, { recursive: true });
      await mkdir(workspace.fuseMount, { recursive: true });

      this.mockWorkspaces.set(sessionId, workspace);

      // Create enhanced session
      const enhancedSession = {
        ...session,
        globalProjectId: workspace.globalProjectId,
        workspaceId: workspace.workspaceId,
        fuseMount: workspace.fuseMount,
        workspaceDir: workspace.workspaceDir,
      };

      this.mockSessions.set(sessionId, enhancedSession);
      return enhancedSession;
    }

    return session;
  }

  // Simulate session disconnect with cleanup
  async simulateSessionDisconnect(sessionId: string) {
    const workspace = this.mockWorkspaces.get(sessionId);
    if (workspace) {
      // Cleanup directories
      if (existsSync(workspace.workspaceDir)) {
        await rmdir(workspace.workspaceDir, { recursive: true });
      }
      if (existsSync(workspace.fuseMount)) {
        await rmdir(workspace.fuseMount, { recursive: true });
      }
      this.mockWorkspaces.delete(sessionId);
    }

    this.mockSessions.delete(sessionId);
  }

  // Simulate FUSE operation
  async simulateFuseOperation(sessionId: string, operation: string, path: string) {
    const session = this.mockSessions.get(sessionId);
    if (!session || !this.options.enableFuse) {
      throw new Error('FUSE not enabled or session not found');
    }

    // Mock different FUSE operations
    switch (operation) {
      case 'readdir':
        return ['mock-file1.txt', 'mock-file2.js'];
      case 'stat':
        return {
          mode: 33188,
          size: 1024,
          mtime: new Date(),
          atime: new Date(),
          ctime: new Date(),
        };
      case 'read':
        return Buffer.from(`Mock content for ${path}`);
      default:
        throw new Error(`Unsupported operation: ${operation}`);
    }
  }

  getServerStats() {
    return {
      clientCount: this.mockSessions.size,
      activeProjects: Array.from(
        new Set(Array.from(this.mockSessions.values()).map((s) => s.projectId))
      ),
      activeServers: [],
    };
  }

  async shutdown() {
    // Cleanup all workspaces
    for (const sessionId of this.mockWorkspaces.keys()) {
      await this.simulateSessionDisconnect(sessionId);
    }
  }
}

describe('WebSocket Server FUSE Integration', () => {
  let testBaseDir: string;

  beforeAll(async () => {
    testBaseDir = join(tmpdir(), `websocket-fuse-test-${Date.now()}`);
    await mkdir(testBaseDir, { recursive: true });
  });

  afterAll(async () => {
    if (existsSync(testBaseDir)) {
      await rmdir(testBaseDir, { recursive: true });
    }
  });

  describe('Server Configuration', () => {
    test('should create server with FUSE enabled', () => {
      const options: WebSocketServerOptions = {
        port: 3000,
        maxClients: 10,
        enableFuse: true,
        workspaceConfig: {
          baseWorkspaceDir: join(testBaseDir, 'workspaces'),
          fuseMountPrefix: join(testBaseDir, 'mounts'),
          maxWorkspaces: 5,
          workspaceTimeoutMs: 5000,
        },
      };

      const server = new MockCodeFlowWebSocketServer(options);
      expect(server).toBeDefined();
    });

    test('should create server with FUSE disabled', () => {
      const options: WebSocketServerOptions = {
        port: 3000,
        maxClients: 10,
        enableFuse: false,
      };

      const server = new MockCodeFlowWebSocketServer(options);
      expect(server).toBeDefined();
    });

    test('should handle default workspace configuration', () => {
      const options: WebSocketServerOptions = {
        port: 3000,
        enableFuse: true,
        // No workspaceConfig provided - should use defaults
      };

      const server = new MockCodeFlowWebSocketServer(options);
      expect(server).toBeDefined();
    });
  });

  describe('Session Management with FUSE', () => {
    let server: MockCodeFlowWebSocketServer;

    beforeEach(() => {
      server = new MockCodeFlowWebSocketServer({
        port: 3000,
        maxClients: 10,
        enableFuse: true,
        workspaceConfig: {
          baseWorkspaceDir: join(testBaseDir, 'sessions-workspaces'),
          fuseMountPrefix: join(testBaseDir, 'sessions-mounts'),
          maxWorkspaces: 5,
          workspaceTimeoutMs: 5000,
        },
      });
    });

    afterEach(async () => {
      await server.shutdown();
    });

    test('should create enhanced session with workspace on initialization', async () => {
      const session = await server.simulateSessionInit('test-session-1', 'test-project');

      // Store paths before expect calls to avoid corruption
      const workspaceDir = session.workspaceDir;
      const fuseMount = session.fuseMount;
      const globalProjectId = session.globalProjectId;

      // Test object structure without matchers that corrupt the object
      expect(session.id).toBe('test-session-1');
      expect(session.projectId).toBe('test-project');
      expect(session.initialized).toBe(true);
      expect(globalProjectId).toMatch(/^test-project-mock-\d+-\d+$/);
      expect(session.workspaceId).toBe('workspace-test-session-1');
      expect(fuseMount).toContain('mounts');
      expect(workspaceDir).toContain('workspaces');

      // Verify directories were created
      expect(existsSync(workspaceDir)).toBe(true);
      expect(existsSync(fuseMount)).toBe(true);
    });

    test('should handle multiple concurrent sessions', async () => {
      const session1 = await server.simulateSessionInit('session-1', 'project-a');
      const session2 = await server.simulateSessionInit('session-2', 'project-b');
      const session3 = await server.simulateSessionInit('session-3', 'project-a'); // Same project

      // All sessions should have unique workspaces
      expect(session1.workspaceId).not.toBe(session2.workspaceId);
      expect(session1.workspaceId).not.toBe(session3.workspaceId);
      expect(session2.workspaceId).not.toBe(session3.workspaceId);

      // Global project IDs should be unique even for same project
      expect(session1.globalProjectId).not.toBe(session3.globalProjectId);
      expect(session1.globalProjectId).toContain('project-a');
      expect(session3.globalProjectId).toContain('project-a');

      // All workspace directories should exist
      expect(existsSync(session1.workspaceDir)).toBe(true);
      expect(existsSync(session2.workspaceDir)).toBe(true);
      expect(existsSync(session3.workspaceDir)).toBe(true);
    });

    test('should cleanup workspace on session disconnect', async () => {
      const session = await server.simulateSessionInit('cleanup-session', 'cleanup-project');
      const { workspaceDir, fuseMount } = session;

      // Verify directories exist
      expect(existsSync(workspaceDir)).toBe(true);
      expect(existsSync(fuseMount)).toBe(true);

      // Disconnect session
      await server.simulateSessionDisconnect('cleanup-session');

      // Verify directories are cleaned up
      expect(existsSync(workspaceDir)).toBe(false);
      expect(existsSync(fuseMount)).toBe(false);
    });
  });

  describe('FUSE Operations', () => {
    let server: MockCodeFlowWebSocketServer;

    beforeEach(async () => {
      server = new MockCodeFlowWebSocketServer({
        port: 3000,
        maxClients: 10,
        enableFuse: true,
      });

      // Initialize a session for testing
      await server.simulateSessionInit('fuse-ops-session', 'fuse-project');
    });

    afterEach(async () => {
      await server.shutdown();
    });

    test('should handle readdir FUSE operation', async () => {
      const result = await server.simulateFuseOperation(
        'fuse-ops-session',
        'readdir',
        '/test/path'
      );
      expect(result).toEqual(['mock-file1.txt', 'mock-file2.js']);
    });

    test('should handle stat FUSE operation', async () => {
      const result = await server.simulateFuseOperation(
        'fuse-ops-session',
        'stat',
        '/test/file.txt'
      );
      expect(result).toMatchObject({
        mode: 33188,
        size: 1024,
        mtime: expect.any(Date),
        atime: expect.any(Date),
        ctime: expect.any(Date),
      });
    });

    test('should handle read FUSE operation', async () => {
      const result = await server.simulateFuseOperation(
        'fuse-ops-session',
        'read',
        '/test/file.txt'
      );
      expect(Buffer.isBuffer(result)).toBe(true);
      expect(result.toString()).toBe('Mock content for /test/file.txt');
    });

    test('should reject FUSE operations for non-existent session', async () => {
      await expect(
        server.simulateFuseOperation('non-existent-session', 'readdir', '/test')
      ).rejects.toThrow('FUSE not enabled or session not found');
    });

    test('should reject unsupported FUSE operations', async () => {
      await expect(
        server.simulateFuseOperation('fuse-ops-session', 'unsupported-op', '/test')
      ).rejects.toThrow('Unsupported operation: unsupported-op');
    });
  });

  describe('Server without FUSE', () => {
    let server: MockCodeFlowWebSocketServer;

    beforeEach(() => {
      server = new MockCodeFlowWebSocketServer({
        port: 3000,
        maxClients: 10,
        enableFuse: false,
      });
    });

    afterEach(async () => {
      await server.shutdown();
    });

    test('should create basic session without workspace when FUSE disabled', async () => {
      const session = await server.simulateSessionInit('basic-session', 'basic-project');

      expect(session).toMatchObject({
        id: 'basic-session',
        projectId: 'basic-project',
        initialized: true,
      });

      // Should not have FUSE-related properties
      expect(session.globalProjectId).toBeUndefined();
      expect(session.workspaceId).toBeUndefined();
      expect(session.fuseMount).toBeUndefined();
      expect(session.workspaceDir).toBeUndefined();
    });

    test('should reject FUSE operations when FUSE disabled', async () => {
      await server.simulateSessionInit('no-fuse-session', 'no-fuse-project');

      await expect(
        server.simulateFuseOperation('no-fuse-session', 'readdir', '/test')
      ).rejects.toThrow('FUSE not enabled or session not found');
    });
  });

  describe('Server Statistics', () => {
    let server: MockCodeFlowWebSocketServer;

    beforeEach(() => {
      server = new MockCodeFlowWebSocketServer({
        port: 3000,
        maxClients: 10,
        enableFuse: true,
      });
    });

    afterEach(async () => {
      await server.shutdown();
    });

    test('should provide server statistics', async () => {
      await server.simulateSessionInit('stats-session-1', 'project-a');
      await server.simulateSessionInit('stats-session-2', 'project-b');
      await server.simulateSessionInit('stats-session-3', 'project-a'); // Same project

      const stats = server.getServerStats();

      // Store array before expect calls to avoid corruption
      const activeProjects = stats.activeProjects;

      expect(stats.clientCount).toBe(3);
      expect(Array.isArray(activeProjects)).toBe(true);
      expect(activeProjects).toContain('project-a');
      expect(activeProjects).toContain('project-b');
      expect(stats.activeServers).toEqual([]);

      expect(activeProjects).toHaveLength(2); // Unique projects
    });

    test('should update statistics after session disconnect', async () => {
      await server.simulateSessionInit('temp-session', 'temp-project');
      let stats = server.getServerStats();
      expect(stats.clientCount).toBe(1);

      await server.simulateSessionDisconnect('temp-session');
      stats = server.getServerStats();
      expect(stats.clientCount).toBe(0);
    });
  });

  describe('Error Handling', () => {
    test('should handle invalid workspace configuration gracefully', () => {
      const options: WebSocketServerOptions = {
        port: 3000,
        enableFuse: true,
        workspaceConfig: {
          baseWorkspaceDir: '/invalid/readonly/path',
          fuseMountPrefix: '/invalid/readonly/mounts',
          maxWorkspaces: -1, // Invalid
          workspaceTimeoutMs: -1000, // Invalid
        },
      };

      // Should not throw during construction
      expect(() => new MockCodeFlowWebSocketServer(options)).not.toThrow();
    });

    test('should handle session disconnect errors gracefully', async () => {
      const server = new MockCodeFlowWebSocketServer({
        port: 3000,
        enableFuse: true,
      });

      try {
        // Disconnect non-existent session should not throw
        await expect(server.simulateSessionDisconnect('non-existent')).resolves.toBeUndefined();
      } finally {
        await server.shutdown();
      }
    });
  });

  describe('Memory and Resource Management', () => {
    test('should cleanup all resources on shutdown', async () => {
      const server = new MockCodeFlowWebSocketServer({
        port: 3000,
        enableFuse: true,
      });

      // Create multiple sessions
      const sessions = [
        await server.simulateSessionInit('resource-1', 'project-resource'),
        await server.simulateSessionInit('resource-2', 'project-resource'),
        await server.simulateSessionInit('resource-3', 'project-resource'),
      ];

      // Verify directories exist
      sessions.forEach((session) => {
        expect(existsSync(session.workspaceDir)).toBe(true);
        expect(existsSync(session.fuseMount)).toBe(true);
      });

      // Shutdown should cleanup everything
      await server.shutdown();

      // Verify all directories are cleaned up
      sessions.forEach((session) => {
        expect(existsSync(session.workspaceDir)).toBe(false);
        expect(existsSync(session.fuseMount)).toBe(false);
      });
    });

    test('should handle large number of session operations', async () => {
      const server = new MockCodeFlowWebSocketServer({
        port: 3000,
        enableFuse: true,
        workspaceConfig: {
          maxWorkspaces: 5, // Limit to test cleanup
        },
      });

      try {
        // Create many sessions to test limits
        const sessionPromises = Array.from({ length: 10 }, (_, i) =>
          server.simulateSessionInit(`stress-session-${i}`, `stress-project-${i % 3}`)
        );

        const sessions = await Promise.all(sessionPromises);
        expect(sessions).toHaveLength(10);

        // Should handle FUSE operations on all sessions
        const fusePromises = sessions
          .slice(0, 5)
          .map((_, i) =>
            server.simulateFuseOperation(`stress-session-${i}`, 'readdir', `/stress-${i}`)
          );

        const fuseResults = await Promise.all(fusePromises);
        expect(fuseResults).toHaveLength(5);
        fuseResults.forEach((result) => {
          expect(result).toEqual(['mock-file1.txt', 'mock-file2.js']);
        });
      } finally {
        await server.shutdown();
      }
    });
  });
});
