import { spawn } from 'node:child_process';
import { watch } from 'node:fs';

interface WatcherOptions {
  pathsToWatch: string[];
  shutdownServer: () => Promise<void>;
  debounceMs?: number;
}

let watcherActive = false;
let debounceTimer: NodeJS.Timeout | null = null;

export function startWatcher(options: WatcherOptions): void {
  if (watcherActive) {
    console.log('Auto-restart watcher is already active');
    return;
  }

  const { pathsToWatch, shutdownServer, debounceMs = 500 } = options;
  watcherActive = true;

  console.log('🔄 Auto-restart enabled. Watching:', pathsToWatch.join(', '));

  for (const path of pathsToWatch) {
    try {
      const watcher = watch(path, { recursive: true }, (eventType, filename) => {
        if (!filename) return;

        // Ignore certain file types and directories that shouldn't trigger restarts
        if (shouldIgnoreFile(filename)) {
          return;
        }

        console.log(`📁 File change detected: ${filename} (${eventType})`);

        // Clear existing debounce timer
        if (debounceTimer) {
          clearTimeout(debounceTimer);
        }

        // Set new debounce timer
        debounceTimer = setTimeout(() => {
          triggerRestart(shutdownServer);
        }, debounceMs);
      });

      // Handle watcher errors
      watcher.on('error', (error) => {
        console.warn(`⚠️  Watcher error for ${path}:`, error.message);
      });
    } catch (error) {
      console.warn(`⚠️  Failed to watch ${path}:`, error instanceof Error ? error.message : error);
    }
  }
}

function shouldIgnoreFile(filename: string): boolean {
  const ignoredPatterns = [
    // Build outputs
    /^dist\//,
    /\.js\.map$/,

    // Temporary files
    /~$/,
    /\.tmp$/,
    /\.temp$/,

    // Editor files
    /\.swp$/,
    /\.swo$/,
    /\.DS_Store$/,

    // Logs
    /\.log$/,

    // Test coverage
    /^coverage\//,

    // Node modules (shouldn't be watched anyway, but just in case)
    /^node_modules\//,

    // Git files
    /^\.git\//,
  ];

  return ignoredPatterns.some((pattern) => pattern.test(filename));
}

async function triggerRestart(shutdownServer: () => Promise<void>): Promise<void> {
  if (!watcherActive) return;

  watcherActive = false; // Prevent multiple restarts

  console.log('🔄 Change detected. Restarting server...');

  try {
    // Spawn new process with same arguments
    const newProcess = spawn(process.argv[0] || 'node', process.argv.slice(1), {
      detached: true,
      stdio: 'inherit',
      cwd: process.cwd(),
      env: process.env,
    });

    // Allow the new process to run independently
    newProcess.unref();

    console.log('✅ New server process started');

    // Gracefully shutdown current server
    await shutdownServer();

    console.log('👋 Previous server shutdown complete');

    // Exit current process
    process.exit(0);
  } catch (error) {
    console.error('❌ Failed to restart server:', error instanceof Error ? error.message : error);
    watcherActive = true; // Re-enable watcher on failure
  }
}

export function stopWatcher(): void {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
  watcherActive = false;
  console.log('🛑 Auto-restart watcher stopped');
}
