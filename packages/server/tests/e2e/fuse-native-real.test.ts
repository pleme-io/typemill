/**
 * Real FUSE Native Integration Tests
 * Tests actual FUSE mount operations with native filesystem access
 * NO MOCKS - Tests real FUSE functionality end-to-end
 * Requires FUSE to be installed and accessible
 */

import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { access, mkdir, readdir, readFile, rmdir, stat, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { FuseMount } from '../../src/fs/fuse-mount.js';
import { WorkspaceManager } from '../../src/server/workspace-manager.js';
import type { EnhancedClientSession } from '../../src/types/enhanced-session.js';
import { waitForCondition } from '../helpers/polling-helpers.js';

// Helper to check FUSE availability
const checkFuseAvailability = () => {
  try {
    execSync('which fusermount || which umount', { stdio: 'ignore' });
    return { available: true };
  } catch {
    return { available: false };
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

const skipMessage = `
  ⚠️  Real FUSE tests skipped.
     Requirements:
     - FUSE must be installed (fusermount or umount available)
     - Privileged access (/dev/fuse exists or run as root)

     Setup Instructions:
     - Linux: sudo apt-get install fuse && sudo usermod -a -G fuse $USER
     - macOS: Install macFUSE from https://osxfuse.github.io/
     - Docker: Run container with --privileged --device /dev/fuse
`;

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
          if (!existsSync(fullPath)) throw new Error('Directory not found');
          const entries = await readdir(fullPath);
          return entries.filter((name) => !name.startsWith('.'));
        }
        case 'fuse/stat': {
          if (!existsSync(fullPath)) throw new Error('File not found');
          const stats = await stat(fullPath);
          return {
            ...stats,
            mode: stats.mode,
            size: stats.size,
            mtime: stats.mtime,
            atime: stats.atime,
            ctime: stats.ctime,
            uid: stats.uid,
            gid: stats.gid,
          };
        }
        case 'fuse/read': {
          if (!existsSync(fullPath)) throw new Error('File not found');
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
          const requiredLength = writeOffset + data.length;
          if (fileContent.length < requiredLength) {
            const newBuffer = Buffer.alloc(requiredLength);
            fileContent.copy(newBuffer);
            fileContent = newBuffer;
          }
          const bufferData = Buffer.isBuffer(data) ? data : Buffer.from(data);
          bufferData.copy(fileContent, writeOffset);
          await writeFile(fullPath, fileContent);
          return data.length;
        }
        case 'fuse/unlink': {
          if (!existsSync(fullPath)) throw new Error('File not found');
          await unlink(fullPath);
          return {};
        }
        case 'fuse/open':
          if (!existsSync(fullPath)) await writeFile(fullPath, '');
          return Math.floor(Math.random() * 1000) + 1;
        case 'fuse/release':
          return {};
        case 'fuse/create':
          await writeFile(fullPath, '');
          return Math.floor(Math.random() * 1000) + 1;
        default:
          throw new Error(`Unsupported FUSE operation: ${method}`);
      }
    } catch (error) {
      throw new Error(`FUSE op failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}

testSuite('Real FUSE Native Integration Tests', () => {
  let testBaseDir: string;
  let workspaceManager: WorkspaceManager;

  beforeAll(async () => {
    if (skipCondition) {
      console.warn(skipMessage);
      return;
    }
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
    if (existsSync(testBaseDir)) {
      await rmdir(testBaseDir, { recursive: true });
    }
  });

  // Main test case
  test('should mount, read, write, and delete files through a real FUSE mount', async () => {
    const mockSession = { id: 'real-fuse-session-1', projectId: 'real-test-project' };
    const workspace = await workspaceManager.createWorkspace(mockSession);
    const transport = new RealWebSocketTransport(workspace.workspaceDir);
    const enhancedSession: EnhancedClientSession = {
      ...mockSession,
      projectRoot: workspace.workspaceDir,
      socket: {} as any,
      initialized: true,
      globalProjectId: workspace.globalProjectId,
      workspaceId: workspace.workspaceId,
      fuseMount: workspace.fuseMount,
      workspaceDir: workspace.workspaceDir,
      permissions: ['file:read', 'file:write'],
    } as any;
    const fuseMount = new FuseMount(enhancedSession, transport as any, workspace.fuseMount, {
      debugFuse: false,
      allowOther: false,
      defaultPermissions: true,
    });

    try {
      // Setup initial file
      const initialFile = join(workspace.workspaceDir, 'initial.txt');
      const initialData = 'Hello from the real world!';
      await writeFile(initialFile, initialData);

      await fuseMount.mount();
      expect(fuseMount.isMounted()).toBe(true);

      // Wait for mount to be stable
      await waitForCondition(() => fuseMount.isMounted(), { timeout: 1000, interval: 100 });

      // 1. Read initial file through FUSE
      const mountedInitialFile = join(workspace.fuseMount, 'initial.txt');
      const readContent = await readFile(mountedInitialFile, 'utf-8');
      expect(readContent).toBe(initialData);

      // 2. Write a new file through FUSE
      const newFile = join(workspace.fuseMount, 'new-file.txt');
      const newData = 'This file was written through FUSE.';
      await writeFile(newFile, newData);

      // Verify in the underlying workspace
      const actualNewFile = join(workspace.workspaceDir, 'new-file.txt');
      expect(existsSync(actualNewFile)).toBe(true);
      const actualNewContent = await readFile(actualNewFile, 'utf-8');
      expect(actualNewContent).toBe(newData);

      // 3. Delete the new file through FUSE
      await unlink(newFile);

      // Verify deletion in the underlying workspace
      expect(existsSync(actualNewFile)).toBe(false);
    } finally {
      if (fuseMount.isMounted()) {
        await fuseMount.unmount();
      }
      await workspaceManager.cleanupWorkspace(mockSession.id);
    }
  }, 45000);
});
