#!/usr/bin/env bun

/**
 * Pre-test validation and setup
 * Ensures everything needed for tests is available
 * Auto-fixes what it can, prompts for what it can't
 */

import { execSync } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';

// Colors for output
const colors = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
  bold: '\x1b[1m',
} as const;

type ColorName = keyof typeof colors;

function log(message: string, color: ColorName = 'reset'): void {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function checkCommand(command: string): boolean {
  try {
    execSync(`${command} --version`, { stdio: 'pipe', timeout: 5000 });
    return true;
  } catch {
    return false;
  }
}

function isNewer(file1: string, file2: string): boolean {
  if (!existsSync(file1) || !existsSync(file2)) return false;
  return statSync(file1).mtime > statSync(file2).mtime;
}

async function checkBuild(): Promise<boolean> {
  const distFile = 'dist/index.js';
  const sourceFile = 'index.ts';

  // Check if build exists and is up to date
  if (!existsSync(distFile)) {
    log('⚠️  Build not found, building project...', 'yellow');
    try {
      execSync('bun run build', { stdio: 'inherit' });
      log('✅ Project built successfully', 'green');
      return true;
    } catch (_error) {
      log('❌ Build failed', 'red');
      log('   Run: bun run build', 'cyan');
      return false;
    }
  }

  // Check if source is newer than dist
  if (isNewer(sourceFile, distFile)) {
    log('⚠️  Source files are newer than build, rebuilding...', 'yellow');
    try {
      execSync('bun run build', { stdio: 'inherit' });
      log('✅ Project rebuilt successfully', 'green');
      return true;
    } catch (_error) {
      log('❌ Rebuild failed', 'red');
      log('   Run: bun run build', 'cyan');
      return false;
    }
  }

  log('✅ Build is up to date', 'green');
  return true;
}

async function checkDependencies(): Promise<boolean> {
  // Check if node_modules exists
  if (!existsSync('node_modules')) {
    log('❌ Dependencies not installed', 'red');
    log('   Run: bun install', 'cyan');
    return false;
  }

  // Check for essential TypeScript language server
  const tsServerPath = 'node_modules/.bin/typescript-language-server';
  if (!existsSync(tsServerPath)) {
    log('⚠️  TypeScript language server not found, installing...', 'yellow');
    try {
      execSync('bun install', { stdio: 'inherit' });
      log('✅ Dependencies updated', 'green');
      return true;
    } catch (_error) {
      log('❌ Failed to install dependencies', 'red');
      log('   Run: bun install', 'cyan');
      return false;
    }
  }

  log('✅ Dependencies are ready', 'green');
  return true;
}

interface LanguageServer {
  name: string;
  command: string;
  required?: boolean;
  install?: string;
}

async function checkOptionalLanguageServers(): Promise<boolean> {
  const servers: LanguageServer[] = [
    { name: 'TypeScript', command: 'npx typescript-language-server --version', required: true },
    { name: 'Python LSP', command: 'pylsp --version', install: 'pip install python-lsp-server' },
    {
      name: 'Rust Analyzer',
      command: 'rust-analyzer --version',
      install: 'rustup component add rust-analyzer',
    },
    {
      name: 'Go LSP',
      command: 'gopls version',
      install: 'go install golang.org/x/tools/gopls@latest',
    },
    {
      name: 'Clangd',
      command: 'clangd --version',
      install: 'apt install clangd (or brew install llvm)',
    },
  ];

  const available: string[] = [];
  const missing: LanguageServer[] = [];

  for (const server of servers) {
    if (checkCommand(server.command.split(' ')[0])) {
      available.push(server.name);
    } else {
      if (server.required) {
        log(`❌ ${server.name} not available (required for tests)`, 'red');
        return false;
      }
      missing.push(server);
    }
  }

  if (available.length > 0) {
    log(`✅ Language servers available: ${available.join(', ')}`, 'green');
  }

  if (missing.length > 0) {
    log('', 'reset');
    log('📋 Optional language servers not installed:', 'cyan');
    for (const server of missing) {
      if (server.install) {
        log(`   ${server.name}: ${server.install}`, 'yellow');
      }
    }
    log('   (Tests will work with TypeScript only)', 'cyan');
  }

  return true;
}

async function main(): Promise<void> {
  log('', 'reset');
  log('🔍 Pre-test validation...', 'blue');
  log('', 'reset');

  const checks = [
    { name: 'Dependencies', fn: checkDependencies },
    { name: 'Build', fn: checkBuild },
    { name: 'Language Servers', fn: checkOptionalLanguageServers },
  ];

  let allPassed = true;

  for (const check of checks) {
    try {
      const passed = await check.fn();
      if (!passed) {
        allPassed = false;
      }
    } catch (error) {
      log(`❌ ${check.name} check failed: ${(error as Error).message}`, 'red');
      allPassed = false;
    }
  }

  log('', 'reset');
  if (allPassed) {
    log('🎉 All checks passed! Tests are ready to run.', 'green');
    log('', 'reset');
    process.exit(0);
  } else {
    log('⚠️  Some issues found. Fix them and try again.', 'yellow');
    log('', 'reset');
    process.exit(1);
  }
}

// Run the checks
main().catch((error) => {
  log(`💥 Pre-test check failed: ${(error as Error).message}`, 'red');
  process.exit(1);
});
