use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::process::Command;

/// Helper for setting up and validating LSP servers in tests
pub struct LspSetupHelper;

impl LspSetupHelper {
    /// Check if required LSP servers are available on the system
    pub fn check_lsp_servers_available() -> Result<(), String> {
        // Check TypeScript language server
        if !Self::is_command_available("typescript-language-server") {
            return Err("TypeScript LSP test requires 'typescript-language-server' to be installed.\n\
                Install with: npm install -g typescript-language-server typescript\n\
                Or use system package manager.".to_string());
        }

        // Check Python language server
        if !Self::is_command_available("pylsp") {
            return Err("Python LSP test requires 'pylsp' (Python LSP Server) to be installed.\n\
                Install with: pip install python-lsp-server[all]\n\
                Or use conda/system package manager.".to_string());
        }

        Ok(())
    }

    /// Check if a command is available on the system
    fn is_command_available(command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Create a .codebuddy/config.json file for LSP configuration in test workspace
    pub fn setup_lsp_config(workspace: &TestWorkspace) {
        workspace.create_directory(".codebuddy");

        let config = json!({
            "servers": [
                {
                    "extensions": ["ts", "tsx", "js", "jsx"],
                    "command": ["typescript-language-server", "--stdio"],
                    "rootDir": null,
                    "restartInterval": 5
                },
                {
                    "extensions": ["py"],
                    "command": ["pylsp"],
                    "rootDir": null,
                    "restartInterval": 5
                }
            ]
        });

        workspace.create_file(
            ".codebuddy/config.json",
            &serde_json::to_string_pretty(&config).unwrap(),
        );
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

        // Give LSP time to process the file
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        // Test get_document_symbols to verify LSP is working
        let response = client
            .call_tool(
                "get_document_symbols",
                json!({
                    "file_path": test_file.to_string_lossy()
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
            Some(symbols_array) => {
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
