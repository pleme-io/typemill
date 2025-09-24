import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

export interface ClientConfig {
  url?: string;
  token?: string;
  defaultTimeout?: number;
  profiles?: Record<string, ProfileConfig>;
  currentProfile?: string;
}

export interface ProfileConfig {
  url: string;
  token?: string;
  name?: string;
  description?: string;
}

const CONFIG_DIR = join(homedir(), '.codeflow-buddy');
const CONFIG_FILE = join(CONFIG_DIR, 'config.json');

/**
 * Load configuration from disk.
 * Returns empty object if config doesn't exist.
 */
export async function loadConfig(): Promise<ClientConfig> {
  if (!existsSync(CONFIG_FILE)) {
    return {};
  }

  try {
    const content = await readFile(CONFIG_FILE, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    console.error('Failed to load config:', error);
    return {};
  }
}

/**
 * Save configuration to disk.
 */
export async function saveConfig(config: ClientConfig): Promise<void> {
  // Ensure config directory exists
  if (!existsSync(CONFIG_DIR)) {
    await mkdir(CONFIG_DIR, { recursive: true });
  }

  await writeFile(CONFIG_FILE, JSON.stringify(config, null, 2));
}

/**
 * Get configuration with command-line overrides.
 */
export async function getConfig(overrides: Partial<ClientConfig> = {}): Promise<ClientConfig> {
  const fileConfig = await loadConfig();

  // If a profile is specified, merge it
  let profileConfig: Partial<ClientConfig> = {};
  const profileName = overrides.currentProfile || fileConfig.currentProfile;
  if (profileName && fileConfig.profiles) {
    const profile = fileConfig.profiles[profileName];
    if (profile) {
      profileConfig = {
        url: profile.url,
        token: profile.token,
      };
    }
  }

  // Merge in order: file config < profile config < command overrides
  return {
    ...fileConfig,
    ...profileConfig,
    ...overrides,
  };
}

/**
 * Add or update a profile.
 */
export async function saveProfile(name: string, profile: ProfileConfig): Promise<void> {
  const config = await loadConfig();

  if (!config.profiles) {
    config.profiles = {};
  }

  config.profiles[name] = {
    ...profile,
    name,
  };

  await saveConfig(config);
}

/**
 * Set the current active profile.
 */
export async function setCurrentProfile(name: string): Promise<void> {
  const config = await loadConfig();

  if (!config.profiles?.[name]) {
    throw new Error(`Profile '${name}' does not exist`);
  }

  config.currentProfile = name;
  await saveConfig(config);
}

/**
 * List all available profiles.
 */
export async function listProfiles(): Promise<Record<string, ProfileConfig>> {
  const config = await loadConfig();
  return config.profiles || {};
}

/**
 * Delete a profile.
 */
export async function deleteProfile(name: string): Promise<void> {
  const config = await loadConfig();

  if (config.profiles) {
    delete config.profiles[name];

    // If this was the current profile, clear it
    if (config.currentProfile === name) {
      delete config.currentProfile;
    }

    await saveConfig(config);
  }
}
