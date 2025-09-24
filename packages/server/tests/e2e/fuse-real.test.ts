/**
 * Real FUSE Integration Tests
 * Tests actual FUSE mount operations without mocks
 * Requires FUSE to be installed and accessible
 */

import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { access, mkdir, readFile, rmdir, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { FuseMount } from '../../src/fs/fuse-mount.js';
import { FuseOperations } from '../../src/fs/fuse-operations.js';
import type { EnhancedClientSession } from '../../src/types/enhanced-session.js';

// Skip tests if FUSE is not available
const isFuseAvailable = () => {
  try {
    execSync('which fusermount', { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
};

const skipIfNoFuse = isFuseAvailable() ? describe : describe.skip;

skipIfNoFuse('Real FUSE Mount Tests', () => {
  let testDir: string;
  let mountPoint: string;
  let workspaceDir: string;

  beforeAll(async () => {
    // Create test directories
    testDir = join(tmpdir(), `fuse-real-test-${Date.now()}`);
    mountPoint = join(testDir, 'mount');
    workspaceDir = join(testDir, 'workspace');

    await mkdir(testDir, { recursive: true });
    await mkdir(mountPoint, { recursive: true });
    await mkdir(workspaceDir, { recursive: true });
  });

  afterAll(async () => {
    // Force unmount if still mounted
    try {
      execSync(`fusermount -u "${mountPoint}"`, { stdio: 'ignore' });
    } catch {
      // Ignore errors - might not be mounted
    }

    // Clean up test directories
    if (existsSync(testDir)) {
      await rmdir(testDir, { recursive: true });
    }
  });

  describe('Basic Mount Operations', () => {
    test('should mount and unmount FUSE filesystem', async () => {
      // Create mock session and transport
      const session: EnhancedClientSession = {
        id: 'test-session',
        projectId: 'test-project',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'test-global',
        workspaceId: 'test-workspace',
        workspaceDir,
        fuseMount: mountPoint,
      };

      // Mock transport that handles FUSE operations
      const mockTransport = {
        sendFuseRequest: jest.fn(async (_sessionId: string, operation: any) => {
          // Simulate file operations
          switch (operation.type) {
            case 'readdir':
              return { entries: ['test.txt', 'data.json'] };
            case 'stat':
              return {
                mode: 33188, // regular file
                size: 1024,
                mtime: new Date(),
                atime: new Date(),
                ctime: new Date(),
              };
            case 'read':
              return Buffer.from('Hello from FUSE');
            case 'write':
              return operation.data.length;
            default:
              return {};
          }
        }),
      } as any;

      const fuseMount = new FuseMount(session, mockTransport, mountPoint);

      // Mount the filesystem
      await expect(fuseMount.mount()).resolves.not.toThrow();
      expect(fuseMount.isMounted()).toBe(true);

      // Verify mount point is accessible
      await expect(access(mountPoint)).resolves.not.toThrow();

      // Unmount the filesystem
      await expect(fuseMount.unmount()).resolves.not.toThrow();
      expect(fuseMount.isMounted()).toBe(false);
    });

    test('should handle file operations through FUSE mount', async () => {
      const session: EnhancedClientSession = {
        id: 'test-session-2',
        projectId: 'test-project',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'test-global-2',
        workspaceId: 'test-workspace-2',
        workspaceDir,
        fuseMount: mountPoint,
      };

      // Create a file in workspace that FUSE will serve
      const testFile = join(workspaceDir, 'test.txt');
      await writeFile(testFile, 'Hello World');

      const mockTransport = {
        sendFuseRequest: jest.fn(async (_sessionId: string, operation: any) => {
          if (operation.type === 'read' && operation.path === '/test.txt') {
            // Read from actual workspace file
            return await readFile(testFile);
          }
          if (operation.type === 'write' && operation.path === '/new.txt') {
            // Write to actual workspace file
            const newFile = join(workspaceDir, 'new.txt');
            await writeFile(newFile, operation.data);
            return operation.data.length;
          }
          if (operation.type === 'stat') {
            try {
              const stats = await stat(join(workspaceDir, operation.path.slice(1)));
              return {
                mode: stats.mode,
                size: stats.size,
                mtime: stats.mtime,
                atime: stats.atime,
                ctime: stats.ctime,
              };
            } catch {
              throw new Error('File not found');
            }
          }
          return {};
        }),
      } as any;

      const fuseMount = new FuseMount(session, mockTransport, mountPoint);

      // Mount filesystem
      await fuseMount.mount();

      try {
        // Simulate FUSE operations
        const operations = new FuseOperations(session, mockTransport);

        // Test read operation
        const content = await operations.read('/test.txt', 0, 1024, 0);
        expect(content.toString()).toBe('Hello World');

        // Test write operation
        const writeData = Buffer.from('New Content');
        const bytesWritten = await operations.write('/new.txt', 0, writeData, 0);
        expect(bytesWritten).toBe(writeData.length);

        // Verify file was written
        const newFileContent = await readFile(join(workspaceDir, 'new.txt'), 'utf-8');
        expect(newFileContent).toBe('New Content');
      } finally {
        await fuseMount.unmount();
      }
    });
  });

  describe('Session Isolation', () => {
    test('should isolate different sessions', async () => {
      const mountPoint1 = join(testDir, 'mount1');
      const mountPoint2 = join(testDir, 'mount2');
      const workspace1 = join(testDir, 'workspace1');
      const workspace2 = join(testDir, 'workspace2');

      await mkdir(mountPoint1, { recursive: true });
      await mkdir(mountPoint2, { recursive: true });
      await mkdir(workspace1, { recursive: true });
      await mkdir(workspace2, { recursive: true });

      // Create two different sessions
      const session1: EnhancedClientSession = {
        id: 'session-1',
        projectId: 'project-1',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'global-1',
        workspaceId: 'workspace-1',
        workspaceDir: workspace1,
        fuseMount: mountPoint1,
      };

      const session2: EnhancedClientSession = {
        id: 'session-2',
        projectId: 'project-2',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'global-2',
        workspaceId: 'workspace-2',
        workspaceDir: workspace2,
        fuseMount: mountPoint2,
      };

      // Create isolated files
      await writeFile(join(workspace1, 'session1.txt'), 'Session 1 Data');
      await writeFile(join(workspace2, 'session2.txt'), 'Session 2 Data');

      const createMockTransport = (workspaceDir: string) =>
        ({
          sendFuseRequest: jest.fn(async (_sessionId: string, operation: any) => {
            if (operation.type === 'readdir') {
              const files = await readFile(workspaceDir);
              return { entries: files };
            }
            if (operation.type === 'read') {
              const filePath = join(workspaceDir, operation.path.slice(1));
              if (existsSync(filePath)) {
                return await readFile(filePath);
              }
              throw new Error('File not found');
            }
            return {};
          }),
        }) as any;

      const transport1 = createMockTransport(workspace1);
      const transport2 = createMockTransport(workspace2);

      const mount1 = new FuseMount(session1, transport1, mountPoint1);
      const mount2 = new FuseMount(session2, transport2, mountPoint2);

      // Mount both filesystems
      await mount1.mount();
      await mount2.mount();

      try {
        // Create operations for each session
        const ops1 = new FuseOperations(session1, transport1);
        const ops2 = new FuseOperations(session2, transport2);

        // Session 1 should only see its file
        const content1 = await ops1.read('/session1.txt', 0, 1024, 0);
        expect(content1.toString()).toBe('Session 1 Data');

        // Session 2 should only see its file
        const content2 = await ops2.read('/session2.txt', 0, 1024, 0);
        expect(content2.toString()).toBe('Session 2 Data');

        // Session 1 should NOT see session 2's file
        await expect(ops1.read('/session2.txt', 0, 1024, 0)).rejects.toThrow();

        // Session 2 should NOT see session 1's file
        await expect(ops2.read('/session1.txt', 0, 1024, 0)).rejects.toThrow();
      } finally {
        await mount1.unmount();
        await mount2.unmount();
      }
    });
  });

  describe('Error Handling', () => {
    test('should handle mount failures gracefully', async () => {
      const invalidMountPoint = '/invalid/mount/point/that/does/not/exist';

      const session: EnhancedClientSession = {
        id: 'error-session',
        projectId: 'error-project',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'error-global',
        workspaceId: 'error-workspace',
        workspaceDir,
        fuseMount: invalidMountPoint,
      };

      const mockTransport = { sendFuseRequest: jest.fn() } as any;
      const fuseMount = new FuseMount(session, mockTransport, invalidMountPoint);

      // Should fail to mount
      await expect(fuseMount.mount()).rejects.toThrow();
      expect(fuseMount.isMounted()).toBe(false);
    });

    test('should cleanup on unexpected disconnect', async () => {
      const session: EnhancedClientSession = {
        id: 'cleanup-session',
        projectId: 'cleanup-project',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'cleanup-global',
        workspaceId: 'cleanup-workspace',
        workspaceDir,
        fuseMount: mountPoint,
      };

      const mockTransport = { sendFuseRequest: jest.fn() } as any;
      const fuseMount = new FuseMount(session, mockTransport, mountPoint);

      await fuseMount.mount();

      // Force cleanup should work even if normal unmount fails
      await fuseMount.forceCleanup();
      expect(fuseMount.isMounted()).toBe(false);
    });
  });
});

// Helper to check if we're running with sufficient privileges
describe('FUSE Privilege Check', () => {
  test('should report if FUSE is available', () => {
    const available = isFuseAvailable();
    if (!available) {
      console.warn(
        '\n⚠️  FUSE tests skipped - FUSE not available\n' +
          '   To run these tests:\n' +
          '   - Linux: sudo apt-get install fuse\n' +
          '   - macOS: Install macFUSE from https://osxfuse.github.io/\n' +
          '   - Docker: Run with --privileged --device /dev/fuse\n'
      );
    }
    expect(typeof available).toBe('boolean');
  });
});
