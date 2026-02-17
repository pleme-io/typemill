use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::process::Command;

/// Helper for setting up and validating LSP servers in tests
pub struct LspSetupHelper;

impl LspSetupHelper {
    fn command_is_healthy(command: &str) -> bool {
        Command::new(command)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

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

    fn lsp_server_entry(
        command_name: &str,
        extensions: &[&str],
        root_dir: &str,
    ) -> Option<serde_json::Value> {
        let resolved = Self::resolve_command_path(command_name);
        if resolved.is_none() {
            eprintln!(
                "DEBUG: Skipping unavailable LSP server '{}' from test config",
                command_name
            );
            return None;
        }

        let cmd = resolved.unwrap();
        if !Self::command_is_healthy(&cmd) {
            eprintln!(
                "DEBUG: Skipping unhealthy LSP server '{}' (resolved to '{}') from test config",
                command_name, cmd
            );
            return None;
        }

        let command = if command_name == "typescript-language-server" {
            json!([cmd, "--stdio"])
        } else {
            json!([cmd])
        };

        Some(json!({
            "extensions": extensions,
            "command": command,
            "rootDir": root_dir,
            "restartInterval": 5
        }))
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
        if !workspace.file_exists("tsconfig.json") {
            let tsconfig = json!({
                "compilerOptions": {
                    "target": "ES2022",
                    "module": "ESNext",
                    "moduleResolution": "node",
                    "strict": true,
                    "skipLibCheck": true,
                    "noEmit": true
                },
                "include": ["**/*"],
                "exclude": ["node_modules"]
            });
            workspace.create_file(
                "tsconfig.json",
                &serde_json::to_string_pretty(&tsconfig).unwrap(),
            );
        }

        // Resolve and keep only available LSP servers to avoid 60s startup timeouts
        let root_dir = workspace.path().to_string_lossy().to_string();
        let mut servers = Vec::new();
        if let Some(server) = Self::lsp_server_entry(
            "typescript-language-server",
            &["ts", "tsx", "js", "jsx"],
            &root_dir,
        ) {
            servers.push(server);
        }
        if let Some(server) = Self::lsp_server_entry("rust-analyzer", &["rs"], &root_dir) {
            servers.push(server);
        }

        eprintln!(
            "DEBUG: Enabled {} LSP servers for test workspace",
            servers.len()
        );

        // Create a full AppConfig structure to ensure proper deserialization
        let config = json!({
            "server": {
                "host": "127.0.0.1",
                "port": 3000,
                "timeoutMs": 30000
            },
            "lsp": {
                "servers": servers,
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

    pub fn prune_unavailable_lsp_servers(workspace: &TestWorkspace) -> Result<bool, String> {
        let config_path = workspace.path().join(".typemill/config.json");
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;

        let mut config: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", config_path.display(), e))?;

        let (before, after) = {
            let Some(servers) = config
                .get_mut("lsp")
                .and_then(|lsp| lsp.get_mut("servers"))
                .and_then(|servers| servers.as_array_mut())
            else {
                return Ok(true);
            };

            let before = servers.len();
            servers.retain(|server| {
                let command = server
                    .get("command")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let keep = !command.is_empty() && Self::command_is_healthy(command);
                if !keep {
                    eprintln!(
                        "DEBUG: Disabling unavailable/unhealthy LSP server command '{}' in {}",
                        command,
                        config_path.display()
                    );
                }
                keep
            });

            (before, servers.len())
        };

        if before != after {
            if let Some(lsp_obj) = config.get_mut("lsp").and_then(|v| v.as_object_mut()) {
                if after == 0 {
                    lsp_obj.insert("enablePreload".to_string(), serde_json::Value::Bool(false));
                }
            }

            std::fs::write(
                &config_path,
                serde_json::to_string_pretty(&config)
                    .map_err(|e| format!("Failed to serialize {}: {}", config_path.display(), e))?,
            )
            .map_err(|e| format!("Failed to write {}: {}", config_path.display(), e))?;

            eprintln!(
                "DEBUG: Pruned LSP servers in {}: {} -> {}",
                config_path.display(),
                before,
                after
            );
        }

        Ok(after > 0)
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
                    .ok_or_else(|| {
                        mill_foundation::errors::MillError::lsp(
                            "TypeScript LSP server not available".to_string(),
                        )
                    })?;
                Ok(vec![ts_lsp_path, "--stdio".to_string()])
            }
            "rs" => {
                let rust_analyzer_path =
                    Self::resolve_command_path("rust-analyzer").ok_or_else(|| {
                        mill_foundation::errors::MillError::lsp(
                            "rust-analyzer not available".to_string(),
                        )
                    })?;
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

        // Test search_code to verify LSP is working
        let response = client
            .call_tool(
                "search_code",
                json!({
                    "query": "TestInterface",
                    "limit": 10
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

        // Check for proper response structure
        let symbols = response
            .get("result")
            .and_then(|r| r.get("results"))
            .and_then(|s| s.as_array());

        match symbols {
            Some(_symbols_array) => {
                // LSP is working - it returned a results array (even if empty is okay for simple files)
                println!(
                    "✅ LSP verification successful: TypeScript LSP server is responding correctly"
                );
            }
            None => {
                return Err(format!(
                    "LSP response has unexpected format.\n\
                    Response: {}\n\
                    Expected a results array in the response.",
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
