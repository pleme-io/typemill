import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

/**
 * Resolve the package version from the repository's package.json at runtime.
 * Falls back to '0.0.0' if unavailable.
 */
export function getPackageVersion(): string {
  try {
    // When bundled, import.meta.url points to dist/index.js; package.json is one level up.
    const pkgUrl = new URL('../package.json', import.meta.url);
    const pkgPath = fileURLToPath(pkgUrl);
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8')) as { version?: string };
    return pkg.version ?? '0.0.0';
  } catch {
    return '0.0.0';
  }
}
