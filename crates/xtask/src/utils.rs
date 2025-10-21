use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Run a command and return success/failure
pub fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to run: {} {}", program, args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("{} failed with status: {}", program, status);
    }

    Ok(())
}

/// Run a command and capture output
pub fn run_cmd_output(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to run: {} {}", program, args.join(" ")))?;

    if !output.status.success() {
        anyhow::bail!(
            "{} failed: {}",
            program,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get the workspace root directory
#[allow(dead_code)]
pub fn workspace_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to get current directory")
}
