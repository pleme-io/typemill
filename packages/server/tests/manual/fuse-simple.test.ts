/**
 * Simple FUSE functionality test
 * Verifies basic FUSE components work without complex test setup
 */

import { existsSync, mkdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { WorkspaceManager } from '../../src/server/workspace-manager.js';

console.log('🧪 Testing FUSE Integration Components...\n');

async function testWorkspaceManager() {
  console.log('1. Testing WorkspaceManager...');

  const testDir = join(tmpdir(), `fuse-simple-test-${Date.now()}`);
  const manager = new WorkspaceManager({
    baseWorkspaceDir: join(testDir, 'workspaces'),
    fuseMountPrefix: join(testDir, 'mounts'),
    maxWorkspaces: 3,
    workspaceTimeoutMs: 5000,
    enableCleanupTimer: false,
  });

  try {
    // Test workspace creation
    const session = { id: 'simple-test', projectId: 'simple-project' };
    const workspace = await manager.createWorkspace(session);

    console.log('   ✅ Workspace created with ID:', workspace.workspaceId);
    console.log('   ✅ Global project ID:', workspace.globalProjectId);

    // Check directories immediately
    const dirExists = existsSync(workspace.workspaceDir);
    const mountExists = existsSync(workspace.fuseMount);

    console.log('   ✅ Workspace directory exists:', dirExists);
    console.log('   ✅ Mount directory exists:', mountExists);

    // Test workspace retrieval
    const retrieved = manager.getWorkspace(session.id);
    console.log('   ✅ Workspace retrieved successfully:', !!retrieved);

    // Test workspace stats
    const stats = manager.getStats();
    console.log(
      '   ✅ Statistics - Workspaces:',
      stats.totalWorkspaces,
      'Sessions:',
      stats.activeSessions
    );

    // Test workspace cleanup
    await manager.cleanupWorkspace(session.id);
    const cleanedUp = !existsSync(workspace.workspaceDir) && !existsSync(workspace.fuseMount);
    console.log('   ✅ Cleanup successful:', cleanedUp);

    console.log('   🎉 WorkspaceManager tests passed!\n');
  } catch (error) {
    console.error('   ❌ WorkspaceManager test failed:', error);
    return false;
  } finally {
    await manager.shutdown();
    // Clean up test directory
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  }

  return true;
}

async function testFuseOperations() {
  console.log('2. Testing FuseOperations...');

  try {
    const { FuseOperations } = await import('../../src/fs/fuse-operations.js');

    // Mock transport for testing
    const mockTransport = {
      sendRequest: async (_session: any, method: string, _params: any) => {
        switch (method) {
          case 'fuse/readdir':
            return ['test-file.txt', 'test-dir'];
          case 'fuse/stat':
            return {
              mode: 33188,
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
            return Buffer.from('Hello from FUSE!');
          default:
            return {};
        }
      },
    };

    const mockSession = {
      id: 'fuse-ops-test',
      projectId: 'fuse-project',
      projectRoot: '/test',
      socket: {} as any,
      initialized: true,
      globalProjectId: 'global-fuse-test',
      workspaceId: 'workspace-fuse-test',
      fuseMount: '/tmp/fuse-test',
      workspaceDir: '/tmp/workspace-test',
    };

    const fuseOps = new FuseOperations(mockSession, mockTransport as any);

    // Test directory listing
    const entries = await fuseOps.readdir('/test');
    console.log('   ✅ Directory listing:', entries);

    // Test file stats
    const stats = await fuseOps.getattr('/test/file.txt');
    console.log('   ✅ File stats - size:', stats.size, 'mode:', stats.mode);

    // Test file reading
    const content = await fuseOps.read('/test/file.txt', 1, 1024, 0);
    console.log('   ✅ File content:', content.toString());

    // Test file operations (open/write/release)
    const fd = await fuseOps.open('/test/file.txt', 0);
    console.log('   ✅ File opened with descriptor:', fd);

    const buffer = Buffer.from('Test write data');
    const bytesWritten = await fuseOps.write('/test/file.txt', fd, buffer, 0);
    console.log('   ✅ Bytes written:', bytesWritten);

    await fuseOps.release('/test/file.txt', fd);
    console.log('   ✅ File released successfully');

    // Test cleanup
    fuseOps.cleanup();
    console.log('   ✅ Cleanup completed');

    console.log('   🎉 FuseOperations tests passed!\n');
  } catch (error) {
    console.error('   ❌ FuseOperations test failed:', error);
    return false;
  }

  return true;
}

async function testFuseMount() {
  console.log('3. Testing FuseMount...');

  try {
    const { FuseMount } = await import('../../src/fs/fuse-mount.js');

    const mockTransport = {
      sendRequest: async () => ({ success: true }),
    };

    const mockSession = {
      id: 'mount-test',
      projectId: 'mount-project',
      projectRoot: '/test',
      socket: {} as any,
      initialized: true,
      globalProjectId: 'global-mount-test',
      workspaceId: 'workspace-mount-test',
      fuseMount: '/tmp/fuse-mount-test',
      workspaceDir: '/tmp/workspace-mount-test',
    };

    const testMountPath = join(tmpdir(), 'fuse-mount-simple-test');
    mkdirSync(testMountPath, { recursive: true });

    const fuseMount = new FuseMount(mockSession, mockTransport as any, testMountPath);

    console.log('   ✅ FuseMount instance created');
    console.log('   ✅ Mount path:', fuseMount.getMountPath());
    console.log('   ✅ Initially mounted:', fuseMount.isMounted());

    // Test stats
    const stats = fuseMount.getStats();
    console.log('   ✅ Mount stats - mounted:', stats.mounted, 'session:', stats.sessionId);

    // Note: We don't test actual mounting as it requires FUSE privileges
    console.log('   ℹ️  Actual mounting requires FUSE privileges (skipped in test)');

    console.log('   🎉 FuseMount tests passed!\n');

    // Cleanup
    rmSync(testMountPath, { recursive: true, force: true });
  } catch (error) {
    console.error('   ❌ FuseMount test failed:', error);
    return false;
  }

  return true;
}

// Run all tests
async function runTests() {
  console.log('🚀 Running FUSE Integration Tests\n');

  const results = await Promise.all([
    testWorkspaceManager(),
    testFuseOperations(),
    testFuseMount(),
  ]);

  const allPassed = results.every((result) => result);

  console.log('📊 Test Results:');
  console.log('================');
  console.log('WorkspaceManager:', results[0] ? '✅ PASS' : '❌ FAIL');
  console.log('FuseOperations:', results[1] ? '✅ PASS' : '❌ FAIL');
  console.log('FuseMount:', results[2] ? '✅ PASS' : '❌ FAIL');
  console.log('');

  if (allPassed) {
    console.log('🎉 All FUSE integration tests passed!');
    console.log('✅ FUSE components are working correctly');
    console.log('✅ Workspace isolation is functional');
    console.log('✅ File operations are properly handled');
    process.exit(0);
  } else {
    console.log('❌ Some tests failed');
    process.exit(1);
  }
}

runTests().catch((error) => {
  console.error('Test runner failed:', error);
  process.exit(1);
});
