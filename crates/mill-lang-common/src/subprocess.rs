//! Subprocess-based AST parsing utilities
//!
//! This module provides common functionality for spawning external AST parsing tools
//! (Python, Node.js, Java, Go) and collecting their JSON output.
//!
//! # Usage
//!
//! ```rust,ignore
//! use mill_lang_common::subprocess::{SubprocessAstTool, run_ast_tool};
//!
//! const PYTHON_TOOL: &str = include_str!("../resources/ast_tool.py");
//!
//! let tool = SubprocessAstTool::new("python3")
//!     .with_embedded_str(PYTHON_TOOL)
//!     .with_temp_filename("ast_tool.py")
//!     .with_args(vec!["list-functions".to_string()]);
//!
//! let functions: Vec<String> = run_ast_tool(tool, source)?;
//! ```

use mill_foundation::errors::MillError;
use serde::de::DeserializeOwned;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::Builder;
use tokio::io::AsyncWriteExt;
use tracing::{debug, warn};

type PluginResult<T> = Result<T, MillError>;

/// Configuration for a subprocess-based AST parsing tool
pub struct SubprocessAstTool {
    /// Runtime executable (e.g., "python3", "node", "java", "go")
    pub runtime: String,

    /// Embedded source code or binary
    pub embedded_source: Vec<u8>,

    /// Temporary filename (e.g., "ast_tool.py", "ast_tool.js")
    pub temp_filename: String,

    /// Additional command-line arguments
    pub args: Vec<String>,

    /// Prefix for temporary directory
    pub temp_prefix: String,
}

impl SubprocessAstTool {
    /// Create a new subprocess tool with the given runtime
    pub fn new(runtime: impl Into<String>) -> Self {
        let runtime = runtime.into();
        let temp_prefix = format!("mill-{}-ast", runtime);

        Self {
            runtime,
            embedded_source: Vec::new(),
            temp_filename: "ast_tool".to_string(),
            args: Vec::new(),
            temp_prefix,
        }
    }

    /// Set the embedded source from a string
    pub fn with_embedded_str(mut self, source: &str) -> Self {
        self.embedded_source = source.as_bytes().to_vec();
        self
    }

    /// Set the embedded source from bytes
    pub fn with_embedded_bytes(mut self, source: &'static [u8]) -> Self {
        self.embedded_source = source.to_vec();
        self
    }

    /// Set the temporary filename
    pub fn with_temp_filename(mut self, filename: impl Into<String>) -> Self {
        self.temp_filename = filename.into();
        self
    }

    /// Add command-line arguments (extends existing args)
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args.extend(args);
        self
    }

    /// Add a single command-line argument
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set the temporary directory prefix
    pub fn with_temp_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.temp_prefix = prefix.into();
        self
    }
}

/// Execute an AST tool subprocess and return raw stdout bytes
///
/// Internal helper function that handles the common subprocess execution logic.
fn execute_subprocess(tool: SubprocessAstTool, source: &str) -> PluginResult<Vec<u8>> {
    debug!(
        runtime = %tool.runtime,
        filename = %tool.temp_filename,
        "Running subprocess AST tool"
    );

    // Create temporary directory
    let tmp_dir = Builder::new()
        .prefix(&tool.temp_prefix)
        .tempdir()
        .map_err(|e| MillError::internal(format!("Failed to create temp dir: {}", e)))?;

    // Write embedded tool to temporary file
    let tool_path = tmp_dir.path().join(&tool.temp_filename);
    std::fs::write(&tool_path, &tool.embedded_source).map_err(|e| {
        MillError::internal(format!(
            "Failed to write {} to temp file: {}",
            tool.temp_filename, e
        ))
    })?;

    // Build command arguments
    let mut cmd_args = Vec::new();

    // Special handling for Java (requires -jar flag)
    if tool.runtime == "java" && tool.temp_filename.ends_with(".jar") {
        cmd_args.push("-jar".to_string());
    }

    // Special handling for Go (requires "run" subcommand)
    if tool.runtime == "go" {
        cmd_args.push("run".to_string());
    }

    cmd_args.push(tool_path.to_string_lossy().to_string());
    cmd_args.extend(tool.args);

    debug!(
        runtime = %tool.runtime,
        args = ?cmd_args,
        "Spawning subprocess"
    );

    // Spawn subprocess
    let mut child = Command::new(&tool.runtime)
        .args(&cmd_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            MillError::parse(format!(
                "Failed to spawn {} subprocess. Is {} installed and in PATH? Error: {}",
                tool.runtime, tool.runtime, e
            ))
        })?;

    // Write source code to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(source.as_bytes()).map_err(|e| {
            MillError::parse(format!(
                "Failed to write to {} subprocess stdin: {}",
                tool.runtime, e
            ))
        })?;
    }

    // Wait for subprocess to complete
    let output = child.wait_with_output().map_err(|e| {
        MillError::parse(format!(
            "Failed to wait for {} subprocess: {}",
            tool.runtime, e
        ))
    })?;

    // Check exit status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            runtime = %tool.runtime,
            stderr = %stderr,
            "Subprocess failed"
        );
        return Err(MillError::parse(format!(
            "{} AST tool failed: {}",
            tool.runtime, stderr
        )));
    }

    Ok(output.stdout)
}

/// Run an AST tool subprocess and parse its JSON output
///
/// # Arguments
///
/// * `tool` - Configuration for the subprocess tool
/// * `source` - Source code to pass to the tool via stdin
///
/// # Returns
///
/// Deserialized output of type `T` from the tool's stdout
///
/// # Errors
///
/// Returns `MillError` if:
/// - Failed to create temporary directory or file
/// - Failed to spawn subprocess
/// - Subprocess returned non-zero exit code
/// - Failed to deserialize JSON output
pub fn run_ast_tool<T: DeserializeOwned>(tool: SubprocessAstTool, source: &str) -> PluginResult<T> {
    let stdout = execute_subprocess(tool, source)?;

    // Deserialize JSON output
    serde_json::from_slice(&stdout).map_err(|e| {
        let stdout_preview = String::from_utf8_lossy(&stdout);
        warn!(
            error = %e,
            stdout_preview = %stdout_preview.chars().take(200).collect::<String>(),
            "Failed to parse JSON output"
        );
        MillError::parse(format!("Failed to parse JSON from AST tool: {}", e))
    })
}

/// Run an AST tool subprocess and return raw stdout as string
///
/// Useful when the tool doesn't return JSON or when you need to parse
/// the output in a custom way.
pub fn run_ast_tool_raw(tool: SubprocessAstTool, source: &str) -> PluginResult<String> {
    let stdout = execute_subprocess(tool, source)?;
    Ok(String::from_utf8_lossy(&stdout).to_string())
}

/// Execute an AST tool subprocess asynchronously and return raw stdout bytes
///
/// Internal helper function that handles the common subprocess execution logic.
async fn execute_subprocess_async(tool: SubprocessAstTool, source: &str) -> PluginResult<Vec<u8>> {
    debug!(
        runtime = %tool.runtime,
        filename = %tool.temp_filename,
        "Running subprocess AST tool (async)"
    );

    // Create temporary directory
    // Note: This is still synchronous but usually fast. We could spawn_blocking but
    // for just mkdir it might not be worth the overhead unless heavily contented.
    let tmp_dir = Builder::new()
        .prefix(&tool.temp_prefix)
        .tempdir()
        .map_err(|e| MillError::internal(format!("Failed to create temp dir: {}", e)))?;

    // Write embedded tool to temporary file
    let tool_path = tmp_dir.path().join(&tool.temp_filename);

    // Use async file write
    tokio::fs::write(&tool_path, &tool.embedded_source)
        .await
        .map_err(|e| {
            MillError::internal(format!(
                "Failed to write {} to temp file: {}",
                tool.temp_filename, e
            ))
        })?;

    // Build command arguments
    let mut cmd_args = Vec::new();

    // Special handling for Java (requires -jar flag)
    if tool.runtime == "java" && tool.temp_filename.ends_with(".jar") {
        cmd_args.push("-jar".to_string());
    }

    // Special handling for Go (requires "run" subcommand)
    if tool.runtime == "go" {
        cmd_args.push("run".to_string());
    }

    cmd_args.push(tool_path.to_string_lossy().to_string());
    cmd_args.extend(tool.args);

    debug!(
        runtime = %tool.runtime,
        args = ?cmd_args,
        "Spawning subprocess (async)"
    );

    // Spawn subprocess
    let mut child = tokio::process::Command::new(&tool.runtime)
        .args(&cmd_args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            MillError::parse(format!(
                "Failed to spawn {} subprocess. Is {} installed and in PATH? Error: {}",
                tool.runtime, tool.runtime, e
            ))
        })?;

    // Write source code to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(source.as_bytes()).await.map_err(|e| {
            MillError::parse(format!(
                "Failed to write to {} subprocess stdin: {}",
                tool.runtime, e
            ))
        })?;
    }

    // Wait for subprocess to complete
    let output = child.wait_with_output().await.map_err(|e| {
        MillError::parse(format!(
            "Failed to wait for {} subprocess: {}",
            tool.runtime, e
        ))
    })?;

    // Check exit status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            runtime = %tool.runtime,
            stderr = %stderr,
            "Subprocess failed (async)"
        );
        return Err(MillError::parse(format!(
            "{} AST tool failed: {}",
            tool.runtime, stderr
        )));
    }

    Ok(output.stdout)
}

/// Run an AST tool subprocess asynchronously and parse its JSON output
pub async fn run_ast_tool_async<T: DeserializeOwned>(
    tool: SubprocessAstTool,
    source: &str,
) -> PluginResult<T> {
    let stdout = execute_subprocess_async(tool, source).await?;

    // Deserialize JSON output
    serde_json::from_slice(&stdout).map_err(|e| {
        let stdout_preview = String::from_utf8_lossy(&stdout);
        warn!(
            error = %e,
            stdout_preview = %stdout_preview.chars().take(200).collect::<String>(),
            "Failed to parse JSON output"
        );
        MillError::parse(format!("Failed to parse JSON from AST tool: {}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_subprocess_tool_builder() {
        let tool = SubprocessAstTool::new("python3")
            .with_embedded_str("print('hello')")
            .with_temp_filename("test.py")
            .with_arg("--verbose")
            .with_args(vec!["arg1".to_string(), "arg2".to_string()]);

        assert_eq!(tool.runtime, "python3");
        assert_eq!(tool.temp_filename, "test.py");
        assert_eq!(tool.args, vec!["--verbose", "arg1", "arg2"]);
    }
}
