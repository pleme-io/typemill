import type { LSPClient } from '../lsp-client.js';
import type { WorkspaceMethodsContext } from '../lsp-types.js';
import type { SymbolInformation } from '../types.js';

export async function searchWorkspaceSymbols(
  context: WorkspaceMethodsContext,
  query: string
): Promise<SymbolInformation[]> {
  // Ensure servers are preloaded before searching
  if (context.servers.size === 0) {
    process.stderr.write(
      '[DEBUG searchWorkspaceSymbols] No servers running, preloading servers first\n'
    );
    await context.preloadServers(false); // Preload without verbose logging
  }

  // For workspace symbol search to work, TypeScript server needs project context
  // Open a TypeScript file to establish project context if no files are open yet
  let hasOpenFiles = false;
  for (const serverState of context.servers.values()) {
    if (serverState.openFiles.size > 0) {
      hasOpenFiles = true;
      break;
    }
  }

  if (!hasOpenFiles) {
    try {
      // Try to open a TypeScript file in the workspace to establish project context
      const { scanDirectoryForExtensions, loadGitignore } = await import('../file-scanner.js');
      const gitignore = await loadGitignore(process.cwd());
      const extensions = await scanDirectoryForExtensions(process.cwd(), 2, gitignore, false);

      if (extensions.has('ts')) {
        // Find a .ts file to open for project context
        const fs = await import('node:fs/promises');
        const path = await import('node:path');

        async function findTsFile(dir: string): Promise<string | null> {
          try {
            const entries = await fs.readdir(dir, { withFileTypes: true });
            for (const entry of entries) {
              if (entry.isFile() && entry.name.endsWith('.ts')) {
                return path.join(dir, entry.name);
              }
              if (entry.isDirectory() && !entry.name.startsWith('.')) {
                const found = await findTsFile(path.join(dir, entry.name));
                if (found) return found;
              }
            }
          } catch {}
          return null;
        }

        const tsFile = await findTsFile(process.cwd());
        if (tsFile) {
          process.stderr.write(
            `[DEBUG searchWorkspaceSymbols] Opening ${tsFile} to establish project context\n`
          );
          const serverState = await context.getServer(tsFile);
          await context.ensureFileOpen(serverState, tsFile);
        }
      }
    } catch (error) {
      process.stderr.write(
        `[DEBUG searchWorkspaceSymbols] Failed to establish project context: ${error}\n`
      );
    }
  }

  // For workspace/symbol, we need to try all running servers
  const results: SymbolInformation[] = [];

  process.stderr.write(
    `[DEBUG searchWorkspaceSymbols] Searching for "${query}" across ${context.servers.size} servers\n`
  );

  for (const [serverKey, serverState] of context.servers.entries()) {
    process.stderr.write(
      `[DEBUG searchWorkspaceSymbols] Checking server: ${serverKey}, initialized: ${serverState.initialized}\n`
    );

    if (!serverState.initialized) continue;

    try {
      process.stderr.write(
        `[DEBUG searchWorkspaceSymbols] Sending workspace/symbol request for "${query}"\n`
      );

      const result = await context.sendRequest(serverState.process, 'workspace/symbol', {
        query: query,
      });

      process.stderr.write(
        `[DEBUG searchWorkspaceSymbols] Workspace symbol result: ${JSON.stringify(result)}\n`
      );

      if (Array.isArray(result)) {
        results.push(...result);
        process.stderr.write(
          `[DEBUG searchWorkspaceSymbols] Added ${result.length} symbols from server\n`
        );
      } else if (result !== null && result !== undefined) {
        process.stderr.write(`[DEBUG searchWorkspaceSymbols] Non-array result: ${typeof result}\n`);
      }
    } catch (error) {
      // Some servers might not support workspace/symbol, continue with others
      process.stderr.write(`[DEBUG searchWorkspaceSymbols] Server error: ${error}\n`);
    }
  }

  process.stderr.write(`[DEBUG searchWorkspaceSymbols] Total results found: ${results.length}\n`);
  return results;
}
