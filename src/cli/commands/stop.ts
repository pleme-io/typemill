import { existsSync, readFileSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';
import { isProcessRunning, terminateProcess } from '../../utils/platform/process.js';

const PID_FILE = join('.codebuddy', 'server.pid');

export async function stopCommand(): Promise<void> {
  // Check if PID file exists
  if (!existsSync(PID_FILE)) {
    console.log('No running server found (PID file not found)');
    return;
  }

  try {
    // Read the PID from the file
    const pidContent = readFileSync(PID_FILE, 'utf-8').trim();
    const pid = Number.parseInt(pidContent, 10);

    if (Number.isNaN(pid)) {
      console.error('Invalid PID in server.pid file');
      unlinkSync(PID_FILE); // Clean up invalid PID file
      return;
    }

    // Check if the process is actually running
    if (!isProcessRunning(pid)) {
      console.log('Server is not running (process not found)');
      unlinkSync(PID_FILE); // Clean up stale PID file
      return;
    }

    // Try to stop the server gracefully
    try {
      await terminateProcess(pid, false);
      console.log(`Stopping server (PID: ${pid})...`);

      // Wait a bit to check if it stopped
      let attempts = 0;
      const maxAttempts = 10;
      const checkInterval = 500; // 500ms

      const checkStopped = () => {
        return new Promise<boolean>((resolve) => {
          setTimeout(() => {
            resolve(!isProcessRunning(pid));
          }, checkInterval);
        });
      };

      while (attempts < maxAttempts) {
        if (await checkStopped()) {
          console.log('Server stopped successfully');
          unlinkSync(PID_FILE);
          return;
        }
        attempts++;
      }

      // If still running after timeout, force kill
      console.log('Server did not stop gracefully, forcing shutdown...');
      await terminateProcess(pid, true);
      unlinkSync(PID_FILE);
      console.log('Server forcefully stopped');
    } catch (error) {
      if (error instanceof Error && 'code' in error) {
        const errorCode = (error as NodeJS.ErrnoException).code;
        if (errorCode === 'EPERM' || error.message?.includes('Access is denied')) {
          console.error(
            'Permission denied: Cannot stop the server (different user or insufficient permissions)'
          );
        } else if (errorCode === 'ESRCH') {
          console.log('Server already stopped');
          unlinkSync(PID_FILE);
        } else {
          console.error('Failed to stop server:', error.message);
        }
      } else {
        console.error('Failed to stop server:', String(error));
      }
    }
  } catch (error) {
    if (error instanceof Error) {
      console.error('Error reading PID file:', error.message);
    } else {
      console.error('Error reading PID file:', String(error));
    }
  }
}
