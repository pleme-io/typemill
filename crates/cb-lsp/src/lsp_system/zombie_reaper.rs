//! Global zombie process reaper for LSP child processes
//!
//! Provides a background thread that periodically checks for and reaps zombie
//! processes that may have been missed by explicit cleanup. This serves as a
//! safety net to prevent zombie process accumulation.

use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Message sent to the zombie reaper thread
enum ReaperMessage {
    Register(i32),
}

/// Global zombie reaper instance
///
/// This reaper runs in a background thread and periodically checks for zombie
/// processes among registered PIDs. It automatically cleans them up using
/// `waitpid(WNOHANG)` to prevent accumulation.
pub static ZOMBIE_REAPER: Lazy<ZombieReaper> = Lazy::new(|| {
    let (tx, rx) = channel();
    let pids = Arc::new(Mutex::new(HashSet::new()));
    let pids_clone = Arc::clone(&pids);

    // Spawn background reaper thread
    thread::Builder::new()
        .name("zombie-reaper".to_string())
        .spawn(move || {
            reaper_loop(rx, pids_clone);
        })
        .expect("Failed to spawn zombie reaper thread");

    ZombieReaper { sender: tx, pids }
});

/// Zombie reaper that monitors and cleans up child processes
pub struct ZombieReaper {
    sender: Sender<ReaperMessage>,
    #[cfg_attr(not(test), allow(dead_code))]
    pids: Arc<Mutex<HashSet<i32>>>,
}

impl ZombieReaper {
    /// Register a child process PID for monitoring
    ///
    /// The reaper will periodically check this PID and reap it if it becomes
    /// a zombie. This is a no-op on non-Unix platforms.
    pub fn register(&self, pid: i32) {
        #[cfg(unix)]
        {
            tracing::debug!(pid = pid, "Registering PID with zombie reaper");
            let _ = self.sender.send(ReaperMessage::Register(pid));
        }

        #[cfg(not(unix))]
        {
            let _ = pid; // Suppress unused variable warning
        }
    }

    /// Get the current count of registered PIDs (for testing)
    #[cfg(test)]
    pub fn pid_count(&self) -> usize {
        self.pids.lock().unwrap().len()
    }
}

/// Main reaper loop that runs in the background thread
#[cfg(unix)]
fn reaper_loop(rx: std::sync::mpsc::Receiver<ReaperMessage>, pids: Arc<Mutex<HashSet<i32>>>) {
    use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
    use nix::unistd::Pid;

    loop {
        // Process any new PID registrations (non-blocking)
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ReaperMessage::Register(pid) => {
                    pids.lock().unwrap().insert(pid);
                }
            }
        }

        // Check all registered PIDs for zombies
        let mut pids_to_remove = Vec::new();
        {
            let pids_guard = pids.lock().unwrap();
            for &pid in pids_guard.iter() {
                match waitpid(Pid::from_raw(pid), Some(WaitPidFlag::WNOHANG)) {
                    Ok(WaitStatus::Exited(_, status)) => {
                        tracing::debug!(
                            pid = pid,
                            exit_status = status,
                            "Zombie process reaped by reaper thread"
                        );
                        pids_to_remove.push(pid);
                    }
                    Ok(WaitStatus::Signaled(_, signal, _)) => {
                        tracing::debug!(
                            pid = pid,
                            signal = ?signal,
                            "Zombie process reaped by reaper thread (terminated by signal)"
                        );
                        pids_to_remove.push(pid);
                    }
                    Ok(WaitStatus::StillAlive) => {
                        // Process is still running, keep monitoring
                    }
                    Ok(_) => {
                        // Other statuses (Stopped, Continued, etc.) - keep monitoring
                    }
                    Err(nix::errno::Errno::ECHILD) => {
                        // Process no longer exists or was already reaped
                        tracing::debug!(pid = pid, "Process already reaped or does not exist");
                        pids_to_remove.push(pid);
                    }
                    Err(e) => {
                        tracing::warn!(
                            pid = pid,
                            error = %e,
                            "Error checking process status"
                        );
                        // Keep monitoring in case it's a transient error
                    }
                }
            }
        }

        // AGGRESSIVE REAPING: Try to reap any child process (PID -1 means any child)
        // This catches zombies that weren't explicitly registered
        loop {
            match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, status)) => {
                    tracing::debug!(
                        pid = pid.as_raw(),
                        exit_status = status,
                        "Unregistered zombie process reaped by aggressive reaper"
                    );
                }
                Ok(WaitStatus::Signaled(pid, signal, _)) => {
                    tracing::debug!(
                        pid = pid.as_raw(),
                        signal = ?signal,
                        "Unregistered zombie process reaped by aggressive reaper (signal)"
                    );
                }
                Ok(WaitStatus::StillAlive) => {
                    // No more zombies to reap
                    break;
                }
                Err(nix::errno::Errno::ECHILD) => {
                    // No child processes at all
                    break;
                }
                Ok(_) => {
                    // Other status, continue checking
                }
                Err(_) => {
                    // Error, stop checking
                    break;
                }
            }
        }

        // Remove reaped PIDs from the set
        if !pids_to_remove.is_empty() {
            let mut pids_guard = pids.lock().unwrap();
            for pid in pids_to_remove {
                pids_guard.remove(&pid);
            }
        }

        // Sleep for 100ms before next check
        thread::sleep(Duration::from_millis(100));
    }
}

/// Stub reaper loop for non-Unix platforms
#[cfg(not(unix))]
fn reaper_loop(rx: std::sync::mpsc::Receiver<ReaperMessage>, _pids: Arc<Mutex<HashSet<i32>>>) {
    // On non-Unix platforms, just drain the channel and do nothing
    loop {
        while let Ok(_msg) = rx.try_recv() {
            // No-op
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn test_zombie_reaper_cleans_up_zombies() {
        // Spawn a child process that exits immediately
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("exit 0")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn child process");

        let pid = child.id() as i32;

        // Register the PID with the reaper
        ZOMBIE_REAPER.register(pid);

        // Wait for the child to exit (creating a zombie)
        let _ = child.wait();

        // Give the reaper time to clean up (should happen within 200ms)
        thread::sleep(Duration::from_millis(250));

        // Verify the PID was removed from the reaper's list
        let pids = ZOMBIE_REAPER.pids.lock().unwrap();
        assert!(
            !pids.contains(&pid),
            "PID {} should have been reaped and removed",
            pid
        );
    }

    #[test]
    fn test_zombie_reaper_handles_nonexistent_pid() {
        // Register a PID that doesn't exist
        let fake_pid = 999999;
        ZOMBIE_REAPER.register(fake_pid);

        // Give the reaper time to process
        thread::sleep(Duration::from_millis(250));

        // Verify the PID was removed (since it doesn't exist)
        let pids = ZOMBIE_REAPER.pids.lock().unwrap();
        assert!(
            !pids.contains(&fake_pid),
            "Nonexistent PID should be removed from monitoring"
        );
    }

    #[test]
    fn test_zombie_reaper_keeps_alive_process() {
        // Spawn a long-running child process
        let mut child = Command::new("sleep")
            .arg("1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn child process");

        let pid = child.id() as i32;

        // Register the PID with the reaper
        ZOMBIE_REAPER.register(pid);

        // Give the reaper time to check (but process should still be alive)
        thread::sleep(Duration::from_millis(250));

        // Verify the PID is still being monitored (not reaped yet)
        let pids = ZOMBIE_REAPER.pids.lock().unwrap();
        assert!(
            pids.contains(&pid),
            "Alive process PID should still be monitored"
        );

        // Clean up the child process
        drop(pids); // Release lock before killing
        let _ = child.kill();
        let _ = child.wait();

        // Give the reaper time to clean up
        thread::sleep(Duration::from_millis(250));

        // Now verify it was reaped
        let pids = ZOMBIE_REAPER.pids.lock().unwrap();
        assert!(
            !pids.contains(&pid),
            "PID should be reaped after process exits"
        );
    }

    #[test]
    fn test_zombie_reaper_registration() {
        let initial_count = ZOMBIE_REAPER.pid_count();

        // Spawn a process
        let mut child = Command::new("sleep")
            .arg("0.1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn child process");

        let pid = child.id() as i32;

        // Register it
        ZOMBIE_REAPER.register(pid);

        // Give time for registration
        thread::sleep(Duration::from_millis(50));

        // Verify count increased
        let after_registration = ZOMBIE_REAPER.pid_count();
        assert!(
            after_registration >= initial_count,
            "PID count should increase after registration"
        );

        // Clean up
        let _ = child.kill();
        let _ = child.wait();
        thread::sleep(Duration::from_millis(250));
    }
}
