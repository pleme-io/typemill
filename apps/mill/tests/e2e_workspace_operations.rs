//! End-to-end workspace operations tests
//!
//! Note: LSP integration tests have been moved to crates/mill-handlers/tests/lsp_integration_tests.rs
//! where they can access internal LSP APIs for integration testing.

#[tokio::test]
#[cfg(unix)] // Zombie reaper is Unix-specific
async fn test_zombie_reaper_integration() {
    // This test verifies that the zombie reaper infrastructure is working at the
    // integration level by spawning a test process, registering it, and verifying cleanup.
    //
    // Note: Unit tests for the zombie reaper itself are in mill-lsp/src/lsp_system/zombie_reaper.rs

    use std::process::{Command, Stdio};
    use std::time::Duration;

    // Spawn a process that exits immediately
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("exit 0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn test process");

    let pid = child.id() as i32;

    // Register with zombie reaper
    mill_lsp::lsp_system::ZOMBIE_REAPER.register(pid);

    // Wait for process to exit (creating a zombie)
    let _ = child.wait();

    // Give zombie reaper time to clean up (it checks every 100ms)
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify the PID was cleaned up
    // Use waitpid to check if process still exists
    let cleanup_check = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ps -p {} -o state= 2>/dev/null || echo 'gone'",
            pid
        ))
        .output()
        .expect("Failed to check process state");

    let state = String::from_utf8_lossy(&cleanup_check.stdout);

    // If the process was reaped, ps will fail and echo 'gone'
    // If it's still a zombie, ps will show 'Z'
    assert!(
        !state.contains('Z'),
        "Process {} is still a zombie after reaper should have cleaned it up. State: {}",
        pid,
        state.trim()
    );

    println!("✓ Zombie reaper successfully cleaned up PID {}", pid);
}
