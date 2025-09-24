import { afterEach, beforeEach, describe, expect, it } from 'bun:test';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { StructuredLogger } from '../src/core/diagnostics/structured-logger';
import { PredictiveLoaderService } from '../src/services/predictive-loader';

describe('PredictiveLoaderService', () => {
  let testDir: string;
  let service: PredictiveLoaderService;
  let openedFiles: string[] = [];
  let mockLogger: StructuredLogger;

  beforeEach(() => {
    // Create a temp directory for test files
    testDir = join(tmpdir(), `predictive-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });

    // Reset opened files tracker
    openedFiles = [];

    // Create mock logger
    mockLogger = {
      debug: (..._args: any[]) => {},
      info: (..._args: any[]) => {},
      warn: (..._args: any[]) => {},
      error: (..._args: any[]) => {},
    } as any;

    // Create service with mock context
    service = new PredictiveLoaderService({
      logger: mockLogger,
      openFile: async (filePath: string) => {
        openedFiles.push(filePath);
      },
      config: {
        server: {
          enablePredictiveLoading: true,
        },
      },
    });
  });

  it('should parse ES6 imports from TypeScript file', async () => {
    const mainFile = join(testDir, 'main.ts');
    const utilsFile = join(testDir, 'utils.ts');
    const helpersFile = join(testDir, 'helpers.ts');

    // Create test files
    writeFileSync(
      mainFile,
      `
      import { helper } from './utils';
      import * as helpers from './helpers';
      import defaultExport from './utils';
      
      const result = helper();
    `
    );

    writeFileSync(
      utilsFile,
      `
      export function helper() {
        return 'hello';
      }
      export default function() { return 'default'; }
    `
    );

    writeFileSync(
      helpersFile,
      `
      export const help = () => 'help';
    `
    );

    // Trigger predictive loading
    await service.preloadImports(mainFile);

    // Check that imported files were opened
    expect(openedFiles).toContain(utilsFile);
    expect(openedFiles).toContain(helpersFile);
  });

  it('should parse CommonJS requires', async () => {
    const mainFile = join(testDir, 'main.js');
    const moduleFile = join(testDir, 'module.js');

    writeFileSync(
      mainFile,
      `
      const module = require('./module');
      const { func } = require('./module');
      
      module.func();
    `
    );

    writeFileSync(
      moduleFile,
      `
      exports.func = function() {
        return 'commonjs';
      };
    `
    );

    await service.preloadImports(mainFile);

    expect(openedFiles).toContain(moduleFile);
  });

  it('should handle dynamic imports', async () => {
    const mainFile = join(testDir, 'main.ts');
    const lazyFile = join(testDir, 'lazy.ts');

    writeFileSync(
      mainFile,
      `
      async function loadLazy() {
        const module = await import('./lazy');
        return module.default;
      }
    `
    );

    writeFileSync(
      lazyFile,
      `
      export default function lazy() {
        return 'lazy loaded';
      }
    `
    );

    await service.preloadImports(mainFile);

    expect(openedFiles).toContain(lazyFile);
  });

  it('should skip node_modules and external packages', async () => {
    const mainFile = join(testDir, 'main.ts');

    writeFileSync(
      mainFile,
      `
      import express from 'express';
      import { readFile } from 'fs/promises';
      import axios from 'axios';
      
      const app = express();
    `
    );

    await service.preloadImports(mainFile);

    // Should not try to open node_modules files
    expect(openedFiles.length).toBe(0);
  });

  it('should resolve index files', async () => {
    const mainFile = join(testDir, 'main.ts');
    const libDir = join(testDir, 'lib');
    const indexFile = join(libDir, 'index.ts');

    mkdirSync(libDir, { recursive: true });

    writeFileSync(
      mainFile,
      `
      import { lib } from './lib';
      
      lib();
    `
    );

    writeFileSync(
      indexFile,
      `
      export function lib() {
        return 'from index';
      }
    `
    );

    await service.preloadImports(mainFile);

    expect(openedFiles).toContain(indexFile);
  });

  it('should handle relative parent imports', async () => {
    const subDir = join(testDir, 'src');
    mkdirSync(subDir);

    const mainFile = join(subDir, 'main.ts');
    const configFile = join(testDir, 'config.ts');

    writeFileSync(
      mainFile,
      `
      import { config } from '../config';
      
      console.log(config);
    `
    );

    writeFileSync(
      configFile,
      `
      export const config = {
        api: 'http://localhost'
      };
    `
    );

    await service.preloadImports(mainFile);

    expect(openedFiles).toContain(configFile);
  });

  it('should cache preloaded files and not reload them', async () => {
    const mainFile = join(testDir, 'main.ts');
    const sharedFile = join(testDir, 'shared.ts');
    const otherFile = join(testDir, 'other.ts');

    writeFileSync(
      mainFile,
      `
      import { shared } from './shared';
    `
    );

    writeFileSync(
      otherFile,
      `
      import { shared } from './shared';
    `
    );

    writeFileSync(
      sharedFile,
      `
      export const shared = 'shared value';
    `
    );

    // Load from main file
    await service.preloadImports(mainFile);
    expect(openedFiles).toContain(sharedFile);
    const _firstCount = openedFiles.length;

    // Load from other file - shared should be cached
    await service.preloadImports(otherFile);

    // Should only have opened otherFile, not sharedFile again
    expect(openedFiles.filter((f) => f === sharedFile).length).toBe(1);
  });

  it('should provide statistics', () => {
    const stats = service.getStats();
    expect(stats).toHaveProperty('preloadedCount');
    expect(stats).toHaveProperty('queueSize');
    expect(stats.preloadedCount).toBeGreaterThanOrEqual(0);
    expect(stats.queueSize).toBeGreaterThanOrEqual(0);
  });

  it('should clear cache', async () => {
    const mainFile = join(testDir, 'main.ts');
    const utilFile = join(testDir, 'util.ts');

    writeFileSync(mainFile, `import { util } from './util';`);
    writeFileSync(utilFile, `export const util = 'util';`);

    await service.preloadImports(mainFile);
    let stats = service.getStats();
    expect(stats.preloadedCount).toBeGreaterThan(0);

    service.clearCache();
    stats = service.getStats();
    expect(stats.preloadedCount).toBe(0);
    expect(stats.queueSize).toBe(0);
  });

  // Cleanup
  afterEach(() => {
    try {
      rmSync(testDir, { recursive: true, force: true });
    } catch {}
  });
});
