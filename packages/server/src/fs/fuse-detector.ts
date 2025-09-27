/**
 * FUSE availability detector
 * Checks if FUSE is installed and properly configured for the current platform
 */

import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { logger } from '../core/diagnostics/logger.js';
import { executableManager } from '../utils/platform/executable-manager.js';
import { getPlatformInfo } from '../utils/platform/platform-detector.js';

export interface FuseStatus {
  available: boolean;
  platform: NodeJS.Platform;
  reason?: string;
  installCommand?: string;
  documentation?: string;
}

/**
 * Check if FUSE is available on the system
 */
export function checkFuseAvailability(): FuseStatus {
  const platformInfo = getPlatformInfo();
  const platform = platformInfo.platform;

  try {
    // Try to load the fuse-native module
    require('@cocalc/fuse-native');

    // Additional platform-specific checks
    switch (platform) {
      case 'linux':
        return checkLinuxFuse();

      case 'darwin':
        return checkMacOSFuse();

      case 'win32':
        return {
          available: false,
          platform,
          reason: 'FUSE is not supported on Windows',
          documentation: 'Consider using WSL2 (Windows Subsystem for Linux) for FUSE support',
        };

      default:
        return {
          available: false,
          platform,
          reason: `FUSE support for platform '${platform}' is not verified`,
        };
    }
  } catch (error) {
    // Module load failed - provide platform-specific guidance
    return getFuseInstallationGuide(platform, error);
  }
}

/**
 * Check FUSE availability on Linux
 */
function checkLinuxFuse(): FuseStatus {
  try {
    // Check if fusermount is available - keep sync for now to avoid breaking changes
    execSync('which fusermount', { stdio: 'ignore' });

    // Check if user is in fuse group (optional but recommended)
    try {
      const groups = execSync('groups').toString();
      if (!groups.includes('fuse')) {
        logger.warn('User not in fuse group - may have permission issues', {
          component: 'FuseDetector',
        });
      }
    } catch {
      // Groups check failed, not critical
    }

    return {
      available: true,
      platform: 'linux',
    };
  } catch {
    return {
      available: false,
      platform: 'linux',
      reason: 'FUSE tools not found',
      installCommand: 'sudo apt-get install fuse fuse-dev',
      documentation: 'You may also need to run: sudo usermod -aG fuse $USER',
    };
  }
}

/**
 * Check FUSE availability on macOS
 */
function checkMacOSFuse(): FuseStatus {
  try {
    // Check for macFUSE installation
    const macFusePaths = [
      '/usr/local/lib/libfuse.dylib',
      '/Library/Frameworks/macFUSE.framework',
      '/Library/Filesystems/macfuse.fs',
    ];

    const macFuseInstalled = macFusePaths.some((path) => existsSync(path));

    if (!macFuseInstalled) {
      // Check for OSXFUSE (legacy)
      const osxFusePaths = [
        '/Library/Frameworks/OSXFUSE.framework',
        '/Library/Filesystems/osxfuse.fs',
      ];

      const osxFuseInstalled = osxFusePaths.some((path) => existsSync(path));

      if (!osxFuseInstalled) {
        return {
          available: false,
          platform: 'darwin',
          reason: 'macFUSE not installed',
          installCommand: 'brew install --cask macfuse',
          documentation:
            'Visit https://osxfuse.github.io for manual installation. Note: You may need to allow the kernel extension in System Preferences > Security & Privacy.',
        };
      }
    }

    return {
      available: true,
      platform: 'darwin',
    };
  } catch (_error) {
    return {
      available: false,
      platform: 'darwin',
      reason: 'Failed to detect macFUSE installation',
      installCommand: 'brew install --cask macfuse',
      documentation: 'Visit https://osxfuse.github.io for more information',
    };
  }
}

/**
 * Get installation guide when FUSE module load fails
 */
function getFuseInstallationGuide(platform: string, error: unknown): FuseStatus {
  const errorMessage = error instanceof Error ? error.message : String(error);

  switch (platform) {
    case 'linux':
      return {
        available: false,
        platform,
        reason: 'FUSE native module not available',
        installCommand: 'sudo apt-get install fuse fuse-dev && npm rebuild @cocalc/fuse-native',
        documentation: `Error: ${errorMessage}. Ensure FUSE development headers are installed.`,
      };

    case 'darwin':
      return {
        available: false,
        platform,
        reason: 'FUSE native module not available',
        installCommand: 'brew install --cask macfuse && npm rebuild @cocalc/fuse-native',
        documentation: `Error: ${errorMessage}. Install macFUSE and rebuild native modules.`,
      };

    case 'win32':
      return {
        available: false,
        platform,
        reason: 'FUSE is not supported on Windows',
        documentation: 'Use WSL2 for FUSE support on Windows',
      };

    default:
      return {
        available: false,
        platform,
        reason: 'FUSE native module not available',
        documentation: `Error: ${errorMessage}`,
      };
  }
}

/**
 * Print FUSE status to console with helpful instructions
 */
export function printFuseStatus(status: FuseStatus): void {
  if (status.available) {
    console.log(`✅ FUSE is available on ${status.platform}`);
  } else {
    console.log(`❌ FUSE is not available on ${status.platform}`);
    if (status.reason) {
      console.log(`   Reason: ${status.reason}`);
    }
    if (status.installCommand) {
      console.log(`   To install: ${status.installCommand}`);
    }
    if (status.documentation) {
      console.log(`   Documentation: ${status.documentation}`);
    }
  }
}

/**
 * Attempt to setup FUSE for the current platform
 */
export async function setupFuseForPlatform(): Promise<boolean> {
  const status = checkFuseAvailability();

  if (status.available) {
    logger.info('FUSE already available', {
      component: 'FuseDetector',
      platform: status.platform,
    });
    return true;
  }

  logger.warn('FUSE not available', {
    component: 'FuseDetector',
    platform: status.platform,
    reason: status.reason,
  });

  // Print setup instructions
  printFuseStatus(status);

  // For automated environments, we could attempt installation
  // but for safety, we'll just provide instructions
  return false;
}
