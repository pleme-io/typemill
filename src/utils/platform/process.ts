import { exec } from 'node:child_process';

/**
 * Check if a process is running by PID
 * @param pid Process ID to check
 * @returns true if process is running, false otherwise
 */
export function isProcessRunning(pid: number): boolean {
  try {
    // Sending signal 0 doesn't kill the process, just checks if it exists
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return false;
  }
}

/**
 * Terminate a process by PID
 * @param pid Process ID to terminate
 * @returns Promise that resolves when process is terminated
 */
export function terminateProcess(pid: number): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    // First try SIGTERM (graceful shutdown)
    try {
      const args = process.platform === 'win32' ? ['taskkill', '/PID', pid.toString(), '/F'] : ['kill', '-TERM', pid.toString()];
      const proc = exec(args.join(' '), {
        detached: true,
        stdio: 'ignore',
      });

      proc.on('error', (err) => {
        // If SIGTERM fails on Unix, try SIGKILL
        if (process.platform !== 'win32') {
          exec(`kill -9 ${pid}`, (fallbackErr) => {
            if (fallbackErr) reject(fallbackErr);
            else resolve();
          });
        } else {
          reject(err);
        }
      });

      proc.on('exit', (code) => {
        if (code === 0) {
          resolve();
        } else if (process.platform !== 'win32') {
          // Try SIGKILL as fallback
          exec(`kill -9 ${pid}`, (err) => {
            if (err) reject(err);
            else resolve();
          });
        } else {
          reject(new Error(`Failed to terminate process ${pid}`));
        }
      });
    } catch (err) {
      reject(err);
    }
  });
}