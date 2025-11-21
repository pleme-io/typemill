use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::process::Command;

/// Helper for setting up and validating LSP servers in tests
pub struct LspSetupHelper;

impl LspSetupHelper {
    /// Check if required LSP servers are available on the system
    /// Note: Language support temporarily reduced to TypeScript + Rust
    pub fn check_lsp_servers_available() -> Result<(), String> {
        // Check TypeScript language server
        if !Self::is_command_available("typescript-language-server") {
            return Err(
                "TypeScript LSP test requires 'typescript-language-server' to be installed.\n\
                Install with: npm install -g typescript-language-server typescript\n\
                Or use system package manager."
                    .to_string(),
            );
        }

        // Note: rust-analyzer is typically available if Rust toolchain is installed
        // We don't fail if it's missing since it's usually present

        Ok(())
    }

    /// Check if a command is available on the system
    pub fn is_command_available(command: &str) -> bool {
        // Use the PATH environment variable to find the command
        if let Ok(path_env) = std::env::var("PATH") {
            // Use shellexpand with context to handle missing variables gracefully
            let expanded_path =
                shellexpand::env_with_context_no_errors(&path_env, |var| std::env::var(var).ok())
                    .to_string();

            for path_dir in expanded_path.split(if cfg!(windows) { ';' } else { ':' }) {
                let full_path = std::path::Path::new(path_dir).join(command);
                if full_path.exists() && full_path.is_file() {
                    return true;
                }
            }
        }

        // Check common LSP installation locations that might not be in test process PATH
        // The cargo config sets PATH for child processes, but not the test process itself
        let home = std::env::var("HOME").ok();

        if let Some(home_dir) = &home {
            // Check standard locations
            let standard_paths = vec![
                format!("{}/.local/bin/{}", home_dir, command),
                format!("{}/.cargo/bin/{}", home_dir, command),
            ];

            for path in standard_paths {
                if std::path::Path::new(&path).is_file() {
                    return true;
                }
            }

            // Check NVM node versions (for typescript-language-server)
            let nvm_dir = format!("{}/.nvm/versions/node", home_dir);
            if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let bin_path = entry.path().join("bin").join(command);
                        if bin_path.is_file() {
                            return true;
                        }
                    }
                }
            }
        }

        // Fallback to which command
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Create a .typemill/config.json file for LSP configuration in test workspace
    /// Note: Language support temporarily reduced to TypeScript + Rust
    pub fn setup_lsp_config(workspace: &TestWorkspace) {
        workspace.create_directory(".typemill");

        // Resolve absolute paths for LSP servers to avoid PATH issues
        let ts_lsp_path = Self::resolve_command_path("typescript-language-server")
            .unwrap_or_else(|| "typescript-language-server".to_string());
        let rust_analyzer_path = Self::resolve_command_path("rust-analyzer")
            .unwrap_or_else(|| "rust-analyzer".to_string());

        // Always log LSP paths for debugging test failures
        eprintln!("DEBUG: Resolved TypeScript LSP path: {}", ts_lsp_path);
        eprintln!("DEBUG: Resolved Rust LSP path: {}", rust_analyzer_path);

        // Create a full AppConfig structure to ensure proper deserialization
        let config = json!({
            "server": {
                "host": "127.0.0.1",
                "port": 3000,
                "timeoutMs": 30000
            },
            "lsp": {
                "servers": [
                    {
                        "extensions": ["ts", "tsx", "js", "jsx"],
                        "command": [ts_lsp_path, "--stdio"],
                        "rootDir": null,
                        "restartInterval": 5
                    },
                    {
                        "extensions": ["rs"],
                        "command": [rust_analyzer_path],
                        "rootDir": null,
                        "restartInterval": 5
                    }
                ],
                "defaultTimeoutMs": 30000,
                "enablePreload": true
            },
            "logging": {
                "level": "debug",
                "format": "json"
            },
            "cache": {
                "enabled": true,
                "maxSizeBytes": 104857600,
                "ttlSeconds": 300,
                "persistent": false,
                "cacheDir": null
            }
        });

        let config_str = serde_json::to_string_pretty(&config).unwrap();

        eprintln!("DEBUG: LSP Config being written:\n{}", config_str);

        workspace.create_file(".typemill/config.json", &config_str);

        // Only log config details if RUST_LOG=debug
        if std::env::var("RUST_LOG")
            .unwrap_or_default()
            .to_lowercase()
            .contains("debug")
        {
            let config_path = workspace.path().join(".typemill/config.json");
            eprintln!("DEBUG: Creating LSP config:\n{}", config_str);
            eprintln!("DEBUG: LSP config created at: {}", config_path.display());

            // Verify the file exists and read it back
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                eprintln!("DEBUG: Config file verified, size: {} bytes", content.len());
            } else {
                eprintln!("DEBUG: WARNING: Config file could not be read!");
            }
        }
    }

    /// Resolve full path for a command
    fn resolve_command_path(command: &str) -> Option<String> {
        // Search PATH for the command
        if let Ok(path_env) = std::env::var("PATH") {
            // Use shellexpand with context to handle missing variables gracefully
            let expanded_path =
                shellexpand::env_with_context_no_errors(&path_env, |var| std::env::var(var).ok())
                    .to_string();

            for path_dir in expanded_path.split(if cfg!(windows) { ';' } else { ':' }) {
                let full_path = std::path::Path::new(path_dir).join(command);
                if full_path.exists() && full_path.is_file() {
                    return full_path.to_string_lossy().to_string().into();
                }
            }
        }

        // Check common LSP installation locations
        if let Ok(home_dir) = std::env::var("HOME") {
            // Check standard locations
            let standard_paths = vec![
                format!("{}/.local/bin/{}", home_dir, command),
                format!("{}/.cargo/bin/{}", home_dir, command),
            ];

            for path in standard_paths {
                if std::path::Path::new(&path).is_file() {
                    return Some(path);
                }
            }

            // Check NVM node versions (for typescript-language-server)
            let nvm_dir = format!("{}/.nvm/versions/node", home_dir);
            if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let bin_path = entry.path().join("bin").join(command);
                        if bin_path.is_file() {
                            return Some(bin_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Get the LSP command for a given file extension
    /// Note: Language support temporarily reduced to TypeScript + Rust
    pub fn get_lsp_command(
        extension: &str,
    ) -> Result<Vec<String>, mill_foundation::errors::MillError> {
        match extension {
            "ts" | "tsx" | "js" | "jsx" => {
                let ts_lsp_path = Self::resolve_command_path("typescript-language-server")
                    .unwrap_or_else(|| "typescript-language-server".to_string());
                Ok(vec![ts_lsp_path, "--stdio".to_string()])
            }
            "rs" => {
                let rust_analyzer_path = Self::resolve_command_path("rust-analyzer")
                    .unwrap_or_else(|| "rust-analyzer".to_string());
                Ok(vec![rust_analyzer_path])
            }
            _ => Err(mill_foundation::errors::MillError::lsp(format!(
                "No LSP server configured for extension: {} (only TypeScript and Rust supported)",
                extension
            ))),
        }
    }

    /// Verify that LSP servers are working with the test client
    pub async fn verify_lsp_functionality(
        client: &mut TestClient,
        workspace: &TestWorkspace,
    ) -> Result<(), String> {
        // Create a simple TypeScript file to test with
        let test_file = workspace.path().join("test_lsp.ts");
        std::fs::write(
            &test_file,
            r#"
export interface TestInterface {
    name: string;
    value: number;
}

export function testFunction(param: string): string {
    return param.toUpperCase();
}
"#,
        )
        .map_err(|e| format!("Failed to create test file: {}", e))?;

        // Wait for LSP to index the file (polling is faster and more reliable than fixed sleep)
        client
            .wait_for_lsp_ready(&test_file, 10000)
            .await
            .map_err(|e| format!("LSP did not become ready: {}", e))?;

        // Test get_document_symbols to verify LSP is working
        let response = client
            .call_tool(
                "get_document_symbols",
                json!({
                    "filePath": test_file.to_string_lossy()
                }),
            )
            .await
            .map_err(|e| format!("LSP call failed: {}", e))?;

        // Check if we got an error response
        if let Some(error) = response.get("error") {
            return Err(format!(
                "LSP server error: {}\n\
                This indicates the LSP server is not working properly.\n\
                Check that typescript-language-server is installed and functional.",
                error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
            ));
        }

        // Check for proper response structure (may be nested in result.content)
        let symbols = if let Some(result) = response.get("result") {
            if let Some(content) = result.get("content") {
                content.get("symbols").and_then(|s| s.as_array())
            } else {
                result.get("symbols").and_then(|s| s.as_array())
            }
        } else {
            response.get("symbols").and_then(|s| s.as_array())
        };

        match symbols {
            Some(_symbols_array) => {
                // LSP is working - it returned a symbols array (even if empty is okay for simple files)
                println!(
                    "✅ LSP verification successful: TypeScript LSP server is responding correctly"
                );
            }
            None => {
                return Err(format!(
                    "LSP response has unexpected format.\n\
                    Response: {}\n\
                    Expected a symbols array in the response.",
                    response
                ));
            }
        }

        // Clean up test file
        std::fs::remove_file(&test_file).ok();

        Ok(())
    }

    /// Full LSP setup and validation for test workspace
    pub async fn setup_and_verify_lsp(
        workspace: &TestWorkspace,
        client: &mut TestClient,
    ) -> Result<(), String> {
        // Check system requirements first
        Self::check_lsp_servers_available()?;

        // Setup LSP configuration
        Self::setup_lsp_config(workspace);

        // Verify LSP functionality
        Self::verify_lsp_functionality(client, workspace).await?;

        Ok(())
    }
}
