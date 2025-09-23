import { homedir } from 'node:os';
import { join } from 'node:path';
import process from 'node:process';

/**
 * Get platform-specific LSP server paths
 * @returns Array of paths where LSP servers might be installed
 */
export function getLSPServerPaths(): string[] {
  const paths: string[] = [];
  const home = homedir();
  const plat = process.platform;

  // Global node_modules locations
  if (plat === 'win32') {
    paths.push(
      join(process.env.APPDATA || '', 'npm', 'node_modules'),
      join(process.env.LOCALAPPDATA || '', 'npm', 'node_modules'),
      'C:\\Program Files\\nodejs\\node_modules',
      'C:\\Program Files (x86)\\nodejs\\node_modules',
    );
  } else if (plat === 'darwin') {
    paths.push(
      '/usr/local/lib/node_modules',
      '/opt/homebrew/lib/node_modules',
      join(home, '.npm-global', 'lib', 'node_modules'),
    );
  } else {
    // Linux
    paths.push('/usr/local/lib/node_modules', '/usr/lib/node_modules', join(home, '.npm-global', 'lib', 'node_modules'));
  }

  // User-specific locations
  paths.push(join(home, 'node_modules'), join(home, '.local', 'lib', 'node_modules'));

  // Current project
  paths.push(join(process.cwd(), 'node_modules'));

  // Filter out non-existent paths (we'll do this at usage time to avoid fs dependency here)
  return paths;
}