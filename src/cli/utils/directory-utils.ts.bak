#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import type { CLIConfig, StateFile } from '../types.js';

const CODEBUDDY_DIR = '.codebuddy';

export function getCodebuddyDir(): string {
  return join(process.cwd(), CODEBUDDY_DIR);
}

export function getConfigPath(): string {
  return join(getCodebuddyDir(), 'config.json');
}

export function getStatePath(): string {
  return join(getCodebuddyDir(), 'state.json');
}

export function getControlDir(): string {
  return join(getCodebuddyDir(), 'control');
}

/**
 * Ensure .codebuddy directory structure exists
 */
export function ensureDirectoryStructure(): void {
  const dirs = [getCodebuddyDir(), getControlDir()];

  for (const dir of dirs) {
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }
  }
}

/**
 * Clean migration from old codebuddy.json if it exists
 * No prompting, just move and inform
 */
export function migrateOldConfig(): boolean {
  const oldConfigPath = join(process.cwd(), 'codebuddy.json');
  const newConfigPath = getConfigPath();

  if (existsSync(oldConfigPath) && !existsSync(newConfigPath)) {
    ensureDirectoryStructure();

    try {
      const configContent = readFileSync(oldConfigPath, 'utf-8');
      writeFileSync(newConfigPath, configContent);
      unlinkSync(oldConfigPath);

      console.log('Migrated config to .codebuddy/config.json');
      return true;
    } catch (error) {
      console.error(`Failed to migrate config: ${error}`);
      return false;
    }
  }

  return false;
}

/**
 * Read config file or return null if it doesn't exist
 */
export function readConfig(): CLIConfig | null {
  return readConfigInternal(true);
}

/**
 * Read config file silently (no error output)
 */
export function readConfigSilent(): CLIConfig | null {
  return readConfigInternal(false);
}

/**
 * Internal config reader with optional error reporting
 */
function readConfigInternal(showErrors: boolean): CLIConfig | null {
  const configPath = getConfigPath();

  if (!existsSync(configPath)) {
    return null;
  }

  try {
    const content = readFileSync(configPath, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    if (showErrors) {
      console.error(`Warning: Invalid config file at ${configPath}`);
      console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
      console.error('Run: codebuddy init');
    }
    return null;
  }
}

/**
 * Write config file
 */
export function writeConfig(config: CLIConfig): void {
  ensureDirectoryStructure();
  const configPath = getConfigPath();
  writeFileSync(configPath, JSON.stringify(config, null, 2));
}

/**
 * Read state file (PIDs only)
 */
export function readState(): StateFile {
  const statePath = getStatePath();

  if (!existsSync(statePath)) {
    return {};
  }

  try {
    const content = readFileSync(statePath, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    return {};
  }
}
