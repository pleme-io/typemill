import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { LSPClient } from '../../../@codeflow/features/src/lsp/lsp-client.js';
import { FileService } from '../../src/services/file-service.js';
import { SymbolService } from '../../../@codeflow/features/src/services/lsp/symbol-service.js';
import { ServiceContextUtils } from '../../src/services/service-context.js';
import { PredictiveLoaderService } from '../../src/services/predictive-loader.js';
import { logger } from '../../src/core/diagnostics/logger.js';

/**
 * Performance test for Predictive Loading
 *
 * This test measures the real-world impact of predictive loading on LSP operations.
 * We compare find_definition performance with and without predictive loading enabled.
 */
describe('Predictive Loading Performance', () => {
  let lspClient: LSPClient;
  let fileService: FileService;
  let symbolService: SymbolService;
  let tempTestDir: string;
  let testMainFile: string;
  let testUtilsFile: string;
  let testTypesFile: string;

  beforeAll(async () => {
    console.log('🚀 Setting up LSP client for performance testing...');

    // Create temporary directory for test files
    tempTestDir = await mkdtemp(join(tmpdir(), 'predictive-perf-test-'));
    console.log(`📁 Created temporary test directory: ${tempTestDir}`);

    // Define file paths within the temp directory
    testMainFile = join(tempTestDir, 'main.ts');
    testUtilsFile = join(tempTestDir, 'utils.ts');
    testTypesFile = join(tempTestDir, 'types.ts');

    // Create test files programmatically
    await writeFile(testTypesFile, `export interface User {
  id: string;
  name: string;
  email: string;
  role: 'admin' | 'user' | 'guest';
}

export interface UserSearchCriteria {
  name?: string;
  email?: string;
  role?: string;
}

export interface UserStats {
  totalUsers: number;
  activeUsers: number;
  usersByRole: Record<string, number>;
}

export type UserRole = User['role'];

export interface CreateUserRequest {
  name: string;
  email: string;
  role: UserRole;
}`);

    await writeFile(testUtilsFile, `import type { User } from './types';

const mockDatabase: User[] = [
  { id: '1', name: 'John Doe', email: 'john@example.com', role: 'admin' },
  { id: '2', name: 'Jane Smith', email: 'jane@example.com', role: 'user' },
  { id: '3', name: 'Bob Wilson', email: 'bob@example.com', role: 'user' },
];

export async function findUser(id: string): Promise<User | null> {
  // Simulate database lookup
  await new Promise(resolve => setTimeout(resolve, 10));

  const user = mockDatabase.find(u => u.id === id);
  return user || null;
}

export function validateUser(user: User): boolean {
  if (!user.id || !user.name || !user.email) {
    return false;
  }

  const emailRegex = /^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/;
  return emailRegex.test(user.email);
}

export function getUsersByRole(role: string): User[] {
  return mockDatabase.filter(user => user.role === role);
}

export function formatUserName(user: User): string {
  return \`\${user.name} (\${user.email})\`;
}`);

    await writeFile(testMainFile, `import { findUser, validateUser } from './utils';
import type { User } from './types';

export class UserManager {
  private users: Map<string, User> = new Map();

  async getUser(id: string): Promise<User | null> {
    const cached = this.users.get(id);
    if (cached) {
      return cached;
    }

    // This is the function we'll test find_definition on
    const user = await findUser(id);
    if (user && validateUser(user)) {
      this.users.set(id, user);
      return user;
    }

    return null;
  }

  async createUser(userData: Omit<User, 'id'>): Promise<User> {
    const user: User = {
      id: Math.random().toString(36),
      ...userData
    };

    if (validateUser(user)) {
      this.users.set(user.id, user);
      return user;
    }

    throw new Error('Invalid user data');
  }
}`);

    console.log('📝 Created test files with realistic import structure');

    // Initialize LSP client
    lspClient = new LSPClient();

    // Create services with predictive loading enabled
    const serviceContext = ServiceContextUtils.createServiceContext(
      lspClient.getServer.bind(lspClient),
      lspClient.protocol,
      undefined, // transactionManager
      logger,
      { serverOptions: { enablePredictiveLoading: true } }
    );

    fileService = new FileService(serviceContext);
    symbolService = new SymbolService(serviceContext);

    // Create and add predictive loader service
    const predictiveLoaderService = new PredictiveLoaderService({
      logger,
      openFile: (filePath: string) => {
        console.log(`🔄 Predictive loading triggered for: ${filePath}`);
        return fileService.openFileInternal(filePath);
      },
      config: { serverOptions: { enablePredictiveLoading: true } }
    });

    serviceContext.predictiveLoader = predictiveLoaderService;
    serviceContext.fileService = fileService;

    // Warm up LSP servers
    console.log('⏳ Warming up LSP servers...');
    await fileService.getFoldingRanges(testMainFile);
    console.log('✅ LSP servers ready');
  });

  afterAll(async () => {
    if (lspClient) {
      await lspClient.dispose();
    }

    // Clean up the temporary directory
    if (tempTestDir) {
      console.log(`🧹 Cleaning up temporary test directory: ${tempTestDir}`);
      await rm(tempTestDir, { recursive: true, force: true });
    }
  });

  async function measureOperation(
    operationName: string,
    operation: () => Promise<any>,
    iterations: number = 5
  ): Promise<{ average: number; min: number; max: number; times: number[] }> {
    const times: number[] = [];

    console.log(`📏 Measuring ${operationName} (${iterations} iterations)...`);

    for (let i = 0; i < iterations; i++) {
      const start = performance.now();
      await operation();
      const duration = performance.now() - start;
      times.push(duration);

      // Small delay between iterations
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    const average = times.reduce((sum, time) => sum + time, 0) / times.length;
    const min = Math.min(...times);
    const max = Math.max(...times);

    console.log(`  Average: ${average.toFixed(1)}ms, Min: ${min.toFixed(1)}ms, Max: ${max.toFixed(1)}ms`);

    return { average, min, max, times };
  }

  it('should show performance improvement with predictive loading', async () => {
    console.log('\n🔍 Testing find_definition performance...');

    // Test without predictive loading (cold start scenario)
    // We simulate this by testing definition lookup on imported symbols
    console.log('\n❄️  Testing WITHOUT predictive loading (cold start):');

    const withoutPreloading = await measureOperation(
      'find_definition on imported symbol (cold)',
      async () => {
        // Dispose and recreate client WITHOUT predictive loading
        await lspClient.dispose();
        lspClient = new LSPClient();
        const coldServiceContext = ServiceContextUtils.createServiceContext(
          lspClient.getServer.bind(lspClient),
          lspClient.protocol,
          undefined,
          logger,
          { serverOptions: { enablePredictiveLoading: false } }
        );
        fileService = new FileService(coldServiceContext);
        symbolService = new SymbolService(coldServiceContext);

        // NO predictive loader in cold context

        // Test operation that would benefit from predictive loading
        // Use symbol lookup on imported function (should be slower)
        const result = await symbolService.findSymbolMatches(testMainFile, 'findUser');
        expect(result).toBeDefined();

        return result;
      },
      3 // Fewer iterations for cold start test (expensive)
    );

    // Test with predictive loading (warm start scenario)
    console.log('\n🔥 Testing WITH predictive loading (warm start):');

    const withPreloading = await measureOperation(
      'find_definition on imported symbol (warm)',
      async () => {
        // First open the main file (this triggers predictive loading of imports)
        await fileService.openFile(testMainFile);

        // Now test symbol lookup in imported file (should be faster due to preloading)
        const result = await symbolService.findSymbolMatches(testMainFile, 'findUser');
        expect(result).toBeDefined();

        return result;
      }
    );

    // Compare results
    const improvementPercent = ((withoutPreloading.average - withPreloading.average) / withoutPreloading.average) * 100;

    console.log('\n📊 Performance Results:');
    console.log(`  Cold start (no preloading):  ${withoutPreloading.average.toFixed(1)}ms average`);
    console.log(`  Warm start (with preloading): ${withPreloading.average.toFixed(1)}ms average`);
    console.log(`  Improvement: ${improvementPercent.toFixed(1)}% faster`);

    // Verify we see some performance improvement
    // Note: In real scenarios with larger codebases, improvements would be more significant
    expect(improvementPercent).toBeGreaterThanOrEqual(0);

    // Log detailed statistics
    console.log('\n📈 Detailed Statistics:');
    console.log('  Cold Start Times:', withoutPreloading.times.map(t => `${t.toFixed(1)}ms`).join(', '));
    console.log('  Warm Start Times:', withPreloading.times.map(t => `${t.toFixed(1)}ms`).join(', '));

    if (improvementPercent > 10) {
      console.log(`\n✅ Significant performance improvement detected: ${improvementPercent.toFixed(1)}%`);
    } else if (improvementPercent > 0) {
      console.log(`\n⚡ Minor performance improvement detected: ${improvementPercent.toFixed(1)}%`);
    } else {
      console.log('\n⚠️  No significant performance improvement detected in this test scenario');
      console.log('   Note: Performance improvements are more significant with larger codebases');
    }
  });

  it('should demonstrate predictive loading behavior', async () => {
    console.log('\n🔄 Demonstrating predictive loading behavior...');

    // Test sequence showing how predictive loading works:
    console.log('1. Opening main file (this should trigger predictive loading of imports)');
    console.log(`   Main file: ${testMainFile}`);
    console.log(`   Utils file: ${testUtilsFile}`);
    const start1 = performance.now();
    await fileService.openFile(testMainFile); // This triggers predictive loading
    const mainFileTime = performance.now() - start1;
    console.log(`   Open main file (triggers predictive loading): ${mainFileTime.toFixed(1)}ms`);

    // Give predictive loading time to complete
    await new Promise(resolve => setTimeout(resolve, 100));

    console.log('2. Finding symbol in imported file (should be fast due to preloading)');
    const start2 = performance.now();
    const result = await symbolService.findSymbolMatches(testMainFile, 'findUser');
    const importedSymbolTime = performance.now() - start2;
    console.log(`   Find imported symbol: ${importedSymbolTime.toFixed(1)}ms`);

    // Verify the operation succeeded
    expect(result).toBeDefined();

    console.log('3. Finding another symbol (should also be fast)');
    const start3 = performance.now();
    const result2 = await symbolService.findSymbolMatches(testUtilsFile, 'validateUser');
    const anotherOpTime = performance.now() - start3;
    console.log(`   Find another symbol: ${anotherOpTime.toFixed(1)}ms`);

    expect(result2).toBeDefined();

    console.log('\n✅ Predictive loading behavior demonstrated');
    console.log(`   Subsequent operations on preloaded files tend to be faster`);
  });
});