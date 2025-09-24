/**
 * Real FUSE Native Integration Tests
 * Tests actual FUSE mount operations with native filesystem access
 * NO MOCKS - Tests real FUSE functionality end-to-end
 * Requires FUSE to be installed and accessible
 */

import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { access, mkdir, readdir, readFile, rmdir, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { FuseMount } from '../../src/fs/fuse-mount.js';
import { WorkspaceManager } from '../../src/server/workspace-manager.js';
import type { EnhancedClientSession } from '../../src/types/enhanced-session.js';

// Helper to check FUSE availability
const checkFuseAvailability = () => {
  try {
    // Check for fusermount (Linux) or umount (macOS)
    try {
      execSync('which fusermount', { stdio: 'ignore' });
      return { available: true, platform: 'linux' };
    } catch {
      try {
        execSync('which umount', { stdio: 'ignore' });
        return { available: true, platform: 'darwin' };
      } catch {
        return { available: false, platform: 'unknown' };
      }
    }
  } catch {
    return { available: false, platform: 'unknown' };
  }
};

// Check if running in privileged environment
const isPrivileged = () => {
  try {
    return process.getuid() === 0 || existsSync('/dev/fuse');
  } catch {
    return false;
  }
};

const fuseInfo = checkFuseAvailability();
const privileged = isPrivileged();

const skipCondition = !fuseInfo.available || !privileged;
const testSuite = skipCondition ? describe.skip : describe;

// Real WebSocket transport that handles actual file operations
class RealWebSocketTransport {
  constructor(private workspaceDir: string) {}

  async sendRequest(_session: EnhancedClientSession, method: string, params: any): Promise<any> {
    const relativePath = params.path?.startsWith('/') ? params.path.slice(1) : params.path;
    const fullPath = join(this.workspaceDir, relativePath || '');

    try {
      switch (method) {
        case 'fuse/readdir': {
          if (!existsSync(fullPath)) {
            throw new Error('Directory not found');
          }
          const entries = await readdir(fullPath);
          return entries.filter((name) => !name.startsWith('.'));
        }

        case 'fuse/stat': {
          if (!existsSync(fullPath)) {
            throw new Error('File not found');
          }
          const stats = await stat(fullPath);
          return {
            mode: stats.mode,
            size: stats.size,
            mtime: stats.mtime,
            atime: stats.atime,
            ctime: stats.ctime,
            uid: stats.uid,
            gid: stats.gid,
            dev: stats.dev,
            ino: stats.ino,
            nlink: stats.nlink,
            rdev: stats.rdev,
            blksize: stats.blksize,
            blocks: stats.blocks,
          };
        }

        case 'fuse/read': {
          if (!existsSync(fullPath)) {
            throw new Error('File not found');
          }
          const content = await readFile(fullPath);
          const { offset = 0, size = content.length } = params;
          return content.slice(offset, offset + size);
        }

        case 'fuse/write': {
          const { data, offset: writeOffset = 0 } = params;
          let fileContent = Buffer.alloc(0);

          if (existsSync(fullPath)) {
            fileContent = await readFile(fullPath);
          }

          // Extend buffer if needed
          const requiredLength = writeOffset + data.length;
          if (fileContent.length < requiredLength) {
            const newBuffer = Buffer.alloc(requiredLength);
            fileContent.copy(newBuffer);
            fileContent = newBuffer;
          }

          // Write data at offset
          if (Buffer.isBuffer(data)) {
            data.copy(fileContent, writeOffset);
          } else {
            Buffer.from(data).copy(fileContent, writeOffset);
          }

          await writeFile(fullPath, fileContent);
          return data.length;
        }

        case 'fuse/open':
          if (!existsSync(fullPath)) {
            throw new Error('File not found');
          }
          return Math.floor(Math.random() * 1000) + 1; // Return file descriptor

        case 'fuse/release':
          return {}; // No-op for now

        case 'fuse/create':
          await writeFile(fullPath, '');
          return Math.floor(Math.random() * 1000) + 1;

        default:
          throw new Error(`Unsupported FUSE operation: ${method}`);
      }
    } catch (error) {
      throw new Error(
        `FUSE operation failed: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
}

testSuite('Real FUSE Native Integration Tests', () => {
  let testBaseDir: string;
  let workspaceManager: WorkspaceManager;

  beforeAll(async () => {
    testBaseDir = join(tmpdir(), `fuse-native-test-${Date.now()}`);
    await mkdir(testBaseDir, { recursive: true });

    workspaceManager = new WorkspaceManager({
      baseWorkspaceDir: join(testBaseDir, 'workspaces'),
      fuseMountPrefix: join(testBaseDir, 'mounts'),
      maxWorkspaces: 10,
      workspaceTimeoutMs: 30000,
      enableCleanupTimer: false,
    });
  });

  afterAll(async () => {
    if (workspaceManager) {
      await workspaceManager.shutdown();
    }

    // Force unmount any remaining mounts
    try {
      const mountsDir = join(testBaseDir, 'mounts');
      if (existsSync(mountsDir)) {
        const mounts = await readdir(mountsDir);
        for (const mount of mounts) {
          const mountPath = join(mountsDir, mount);
          try {
            if (fuseInfo.platform === 'linux') {
              execSync(`fusermount -u "${mountPath}"`, { stdio: 'ignore' });
            } else {
              execSync(`umount "${mountPath}"`, { stdio: 'ignore' });
            }
          } catch {
            // Ignore unmount errors
          }
        }
      }
    } catch {
      // Ignore cleanup errors
    }

    if (existsSync(testBaseDir)) {
      await rmdir(testBaseDir, { recursive: true });
    }
  });

  describe('Real FUSE Mount and File Operations', () => {
    test('should mount FUSE filesystem and perform real file operations', async () => {
      // Create workspace for session
      const mockSession = {
        id: 'real-fuse-session-1',
        projectId: 'real-test-project',
      };

      const workspace = await workspaceManager.createWorkspace(mockSession);

      // Create some test files in the workspace
      const testFile = join(workspace.workspaceDir, 'test.txt');
      const testData = 'Hello from real FUSE!';
      await writeFile(testFile, testData);

      const subDir = join(workspace.workspaceDir, 'subdir');
      await mkdir(subDir, { recursive: true });
      await writeFile(join(subDir, 'nested.json'), '{"message": "nested file"}');

      // Create enhanced session
      const enhancedSession: EnhancedClientSession = {
        id: mockSession.id,
        projectId: mockSession.projectId,
        projectRoot: workspace.workspaceDir,
        socket: {} as any,
        initialized: true,
        globalProjectId: workspace.globalProjectId,
        workspaceId: workspace.workspaceId,
        fuseMount: workspace.fuseMount,
        workspaceDir: workspace.workspaceDir,
        permissions: ['file:read', 'file:write', 'lsp:query'],
      } as any;

      // Create real transport that accesses the workspace directory
      const transport = new RealWebSocketTransport(workspace.workspaceDir);

      // Create and mount FUSE
      const fuseMount = new FuseMount(enhancedSession, transport as any, workspace.fuseMount, {
        debugFuse: true,
        allowOther: false,
        defaultPermissions: true,
      });

      await fuseMount.mount();
      expect(fuseMount.isMounted()).toBe(true);

      try {
        // Test reading files through FUSE mount
        const mountedFile = join(workspace.fuseMount, 'test.txt');

        // Wait a moment for FUSE to be ready
        await new Promise((resolve) => setTimeout(resolve, 1000));

        // Access file through FUSE mount point
        await access(mountedFile);
        const content = await readFile(mountedFile, 'utf-8');
        expect(content).toBe(testData);

        // Test reading directory through FUSE
        const entries = await readdir(workspace.fuseMount);
        expect(entries).toContain('test.txt');
        expect(entries).toContain('subdir');

        // Test reading nested file
        const nestedFile = join(workspace.fuseMount, 'subdir', 'nested.json');
        const nestedContent = await readFile(nestedFile, 'utf-8');
        expect(JSON.parse(nestedContent)).toEqual({ message: 'nested file' });

        // Test writing through FUSE mount
        const newFile = join(workspace.fuseMount, 'new-file.txt');
        const newContent = 'Created through FUSE mount!';
        await writeFile(newFile, newContent);

        // Verify file was written to actual workspace
        const actualFile = join(workspace.workspaceDir, 'new-file.txt');
        expect(existsSync(actualFile)).toBe(true);
        const actualContent = await readFile(actualFile, 'utf-8');
        expect(actualContent).toBe(newContent);
      } finally {
        await fuseMount.unmount();
      }

      // Cleanup
      await workspaceManager.cleanupWorkspace(mockSession.id);
    }, 30000);

    test('should handle session isolation through FUSE', async () => {
      // Create two different sessions
      const session1 = { id: 'isolation-session-1', projectId: 'isolation-project-1' };
      const session2 = { id: 'isolation-session-2', projectId: 'isolation-project-2' };

      const workspace1 = await workspaceManager.createWorkspace(session1);
      const workspace2 = await workspaceManager.createWorkspace(session2);

      // Create different files in each workspace
      await writeFile(join(workspace1.workspaceDir, 'session1.txt'), 'Session 1 data');
      await writeFile(join(workspace2.workspaceDir, 'session2.txt'), 'Session 2 data');

      // Create transports for each workspace
      const transport1 = new RealWebSocketTransport(workspace1.workspaceDir);
      const transport2 = new RealWebSocketTransport(workspace2.workspaceDir);

      // Create enhanced sessions
      const enhanced1: EnhancedClientSession = {
        id: session1.id,
        projectId: session1.projectId,
        projectRoot: workspace1.workspaceDir,
        socket: {} as any,
        initialized: true,
        globalProjectId: workspace1.globalProjectId,
        workspaceId: workspace1.workspaceId,
        fuseMount: workspace1.fuseMount,
        workspaceDir: workspace1.workspaceDir,
        permissions: ['file:read', 'file:write'],
      } as any;

      const enhanced2: EnhancedClientSession = {
        id: session2.id,
        projectId: session2.projectId,
        projectRoot: workspace2.workspaceDir,
        socket: {} as any,
        initialized: true,
        globalProjectId: workspace2.globalProjectId,
        workspaceId: workspace2.workspaceId,
        fuseMount: workspace2.fuseMount,
        workspaceDir: workspace2.workspaceDir,
        permissions: ['file:read', 'file:write'],
      } as any;

      // Mount both FUSE filesystems
      const mount1 = new FuseMount(enhanced1, transport1 as any, workspace1.fuseMount);
      const mount2 = new FuseMount(enhanced2, transport2 as any, workspace2.fuseMount);

      await mount1.mount();
      await mount2.mount();

      try {
        // Wait for mounts to be ready
        await new Promise((resolve) => setTimeout(resolve, 1000));

        // Session 1 should only see its file
        const entries1 = await readdir(workspace1.fuseMount);
        expect(entries1).toContain('session1.txt');
        expect(entries1).not.toContain('session2.txt');

        const content1 = await readFile(join(workspace1.fuseMount, 'session1.txt'), 'utf-8');
        expect(content1).toBe('Session 1 data');

        // Session 2 should only see its file
        const entries2 = await readdir(workspace2.fuseMount);
        expect(entries2).toContain('session2.txt');
        expect(entries2).not.toContain('session1.txt');

        const content2 = await readFile(join(workspace2.fuseMount, 'session2.txt'), 'utf-8');
        expect(content2).toBe('Session 2 data');

        // Cross-session access should not work
        const session1File = join(workspace1.fuseMount, 'session2.txt');
        const session2File = join(workspace2.fuseMount, 'session1.txt');

        expect(existsSync(session1File)).toBe(false);
        expect(existsSync(session2File)).toBe(false);
      } finally {
        await mount1.unmount();
        await mount2.unmount();
      }

      // Cleanup
      await workspaceManager.cleanupWorkspace(session1.id);
      await workspaceManager.cleanupWorkspace(session2.id);
    }, 45000);

    test('should handle FUSE mount errors gracefully', async () => {
      const invalidSession: EnhancedClientSession = {
        id: 'error-session',
        projectId: 'error-project',
        projectRoot: '/nonexistent',
        socket: {} as any,
        initialized: true,
        globalProjectId: 'error-global',
        workspaceId: 'error-workspace',
        fuseMount: '/invalid/mount/point',
        workspaceDir: '/nonexistent/workspace',
        permissions: ['file:read'],
      } as any;

      const transport = new RealWebSocketTransport('/nonexistent');
      const fuseMount = new FuseMount(invalidSession, transport as any, '/invalid/mount/point');

      // Should fail to mount on invalid path
      await expect(fuseMount.mount()).rejects.toThrow();
      expect(fuseMount.isMounted()).toBe(false);
    });

    test('should handle concurrent FUSE operations', async () => {
      const mockSession = {
        id: 'concurrent-session',
        projectId: 'concurrent-project',
      };

      const workspace = await workspaceManager.createWorkspace(mockSession);

      // Create multiple test files
      for (let i = 0; i < 5; i++) {
        await writeFile(join(workspace.workspaceDir, `file${i}.txt`), `Content ${i}`);
      }

      const enhancedSession: EnhancedClientSession = {
        id: mockSession.id,
        projectId: mockSession.projectId,
        projectRoot: workspace.workspaceDir,
        socket: {} as any,
        initialized: true,
        globalProjectId: workspace.globalProjectId,
        workspaceId: workspace.workspaceId,
        fuseMount: workspace.fuseMount,
        workspaceDir: workspace.workspaceDir,
        permissions: ['file:read', 'file:write'],
      } as any;

      const transport = new RealWebSocketTransport(workspace.workspaceDir);
      const fuseMount = new FuseMount(enhancedSession, transport as any, workspace.fuseMount);

      await fuseMount.mount();

      try {
        await new Promise((resolve) => setTimeout(resolve, 1000));

        // Perform concurrent read operations
        const readPromises = Array.from({ length: 5 }, (_, i) =>
          readFile(join(workspace.fuseMount, `file${i}.txt`), 'utf-8')
        );

        const contents = await Promise.all(readPromises);

        for (let i = 0; i < 5; i++) {
          expect(contents[i]).toBe(`Content ${i}`);
        }

        // Perform concurrent write operations
        const writePromises = Array.from({ length: 3 }, (_, i) =>
          writeFile(join(workspace.fuseMount, `new${i}.txt`), `New content ${i}`)
        );

        await Promise.all(writePromises);

        // Verify all files were written
        for (let i = 0; i < 3; i++) {
          const actualFile = join(workspace.workspaceDir, `new${i}.txt`);
          expect(existsSync(actualFile)).toBe(true);
          const content = await readFile(actualFile, 'utf-8');
          expect(content).toBe(`New content ${i}`);
        }
      } finally {
        await fuseMount.unmount();
      }

      await workspaceManager.cleanupWorkspace(mockSession.id);
    }, 30000);
  });

  describe('FUSE Performance and Edge Cases', () => {
    test('should handle large files through FUSE', async () => {
      const mockSession = {
        id: 'large-file-session',
        projectId: 'large-file-project',
      };

      const workspace = await workspaceManager.createWorkspace(mockSession);

      // Create a large file (1MB)
      const largeContent = 'A'.repeat(1024 * 1024);
      const largeFile = join(workspace.workspaceDir, 'large.txt');
      await writeFile(largeFile, largeContent);

      const enhancedSession: EnhancedClientSession = {
        id: mockSession.id,
        projectId: mockSession.projectId,
        projectRoot: workspace.workspaceDir,
        socket: {} as any,
        initialized: true,
        globalProjectId: workspace.globalProjectId,
        workspaceId: workspace.workspaceId,
        fuseMount: workspace.fuseMount,
        workspaceDir: workspace.workspaceDir,
        permissions: ['file:read', 'file:write'],
      } as any;

      const transport = new RealWebSocketTransport(workspace.workspaceDir);
      const fuseMount = new FuseMount(enhancedSession, transport as any, workspace.fuseMount);

      await fuseMount.mount();

      try {
        await new Promise((resolve) => setTimeout(resolve, 1000));

        const mountedLargeFile = join(workspace.fuseMount, 'large.txt');

        // Read large file through FUSE
        const readContent = await readFile(mountedLargeFile, 'utf-8');
        expect(readContent.length).toBe(largeContent.length);
        expect(readContent.slice(0, 100)).toBe('A'.repeat(100));

        // Write another large file through FUSE
        const newLargeContent = 'B'.repeat(512 * 1024);
        const newLargeFile = join(workspace.fuseMount, 'new-large.txt');
        await writeFile(newLargeFile, newLargeContent);

        // Verify in actual workspace
        const actualNewFile = join(workspace.workspaceDir, 'new-large.txt');
        expect(existsSync(actualNewFile)).toBe(true);
        const actualContent = await readFile(actualNewFile, 'utf-8');
        expect(actualContent.length).toBe(newLargeContent.length);
      } finally {
        await fuseMount.unmount();
      }

      await workspaceManager.cleanupWorkspace(mockSession.id);
    }, 45000);

    test('should handle FUSE cleanup after abnormal termination', async () => {
      const mockSession = {
        id: 'cleanup-session',
        projectId: 'cleanup-project',
      };

      const workspace = await workspaceManager.createWorkspace(mockSession);
      const enhancedSession: EnhancedClientSession = {
        id: mockSession.id,
        projectId: mockSession.projectId,
        projectRoot: workspace.workspaceDir,
        socket: {} as any,
        initialized: true,
        globalProjectId: workspace.globalProjectId,
        workspaceId: workspace.workspaceId,
        fuseMount: workspace.fuseMount,
        workspaceDir: workspace.workspaceDir,
        permissions: ['file:read'],
      } as any;

      const transport = new RealWebSocketTransport(workspace.workspaceDir);
      const fuseMount = new FuseMount(enhancedSession, transport as any, workspace.fuseMount);

      await fuseMount.mount();
      expect(fuseMount.isMounted()).toBe(true);

      // Force cleanup without normal unmount
      await fuseMount.forceCleanup();
      expect(fuseMount.isMounted()).toBe(false);

      await workspaceManager.cleanupWorkspace(mockSession.id);
    });
  });
});

// Report test environment
describe('FUSE Environment Check', () => {
  test('should report FUSE availability and environment', () => {
    console.log('\n📋 FUSE Test Environment Report:');
    console.log(`  Platform: ${fuseInfo.platform}`);
    console.log(`  FUSE Available: ${fuseInfo.available}`);
    console.log(`  Privileged: ${privileged}`);
    console.log(`  Tests Enabled: ${!skipCondition}`);

    if (skipCondition) {
      console.warn(
        '\n⚠️  Real FUSE tests skipped due to missing requirements:\n' +
          '   Requirements:\n' +
          '   - FUSE must be installed (fusermount or umount available)\n' +
          '   - Privileged access (/dev/fuse exists or root user)\n' +
          '   \n' +
          '   Setup instructions:\n' +
          '   - Linux: sudo apt-get install fuse && sudo usermod -a -G fuse $USER\n' +
          '   - macOS: Install macFUSE from https://osxfuse.github.io/\n' +
          '   - Docker: docker run --privileged --device /dev/fuse ...\n'
      );
    }

    expect(typeof fuseInfo.available).toBe('boolean');
    expect(typeof privileged).toBe('boolean');
  });
});
