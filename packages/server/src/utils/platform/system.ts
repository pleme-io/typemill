import { homedir } from 'node:os';
import { join } from 'node:path';
import globalModules from 'global-modules';
import { pathManager } from './path-manager.js';

/**
 * Get platform-specific LSP server paths
 * @returns Array of paths where LSP servers might be installed
 */
export function getLSPServerPaths(): string[] {
  const paths: string[] = [];
  const home = pathManager.getHomeDir();

  // Use global-modules package to get the correct global modules path
  paths.push(globalModules);

  // Additional standard paths
  paths.push(
    // User-specific npm global directory
    pathManager.expandPath('~/.npm-global/lib/node_modules'),

    // User node_modules
    pathManager.join(home, 'node_modules'),
    pathManager.join(home, '.local', 'lib', 'node_modules'),

    // Current project
    pathManager.join(process.cwd(), 'node_modules')
  );

  // Filter out duplicates and empty paths
  return [...new Set(paths.filter(Boolean))];
}
