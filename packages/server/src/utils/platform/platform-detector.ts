/**
 * Centralized platform detection and feature discovery
 * Phase 1: Foundation layer - safe infrastructure only
 */

import * as process from 'node:process';

export type SupportedPlatform = 'win32' | 'darwin' | 'linux' | 'unknown';

/**
 * Platform information and capabilities
 */
export interface PlatformInfo {
  platform: SupportedPlatform;
  arch: string;
  version: string;
  isWindows: boolean;
  isMacOS: boolean;
  isLinux: boolean;
  isUnix: boolean;
}

/**
 * Get comprehensive platform information
 */
export function getPlatformInfo(): PlatformInfo {
  const platform =
    process.platform === 'win32' || process.platform === 'darwin' || process.platform === 'linux'
      ? process.platform
      : ('unknown' as SupportedPlatform);

  return {
    platform,
    arch: process.arch,
    version: process.version,
    isWindows: platform === 'win32',
    isMacOS: platform === 'darwin',
    isLinux: platform === 'linux',
    isUnix: platform === 'darwin' || platform === 'linux',
  };
}

/**
 * Feature detection capabilities
 */
export interface FeatureCapabilities {
  fuse: boolean;
  docker: boolean;
  wsl: boolean;
}

/**
 * Detect system capabilities (stub for now)
 * Will be implemented in later phases
 */
export function detectCapabilities(): FeatureCapabilities {
  // Stub implementation - will be enhanced in Phase 5
  return {
    fuse: false,
    docker: false,
    wsl: false,
  };
}

/**
 * Check if we're running in a supported environment
 */
export function isSupportedPlatform(): boolean {
  const info = getPlatformInfo();
  return info.platform !== 'unknown';
}
