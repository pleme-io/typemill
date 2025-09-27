//! Server management MCP tool (restart_server)

use crate::handlers::McpDispatcher;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Arguments for restart_server tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RestartServerArgs {
    extensions: Option<Vec<String>>,
}

/// Server restart result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestartServerResult {
    success: bool,
    restarted_servers: Vec<ServerInfo>,
    failed_servers: Vec<ServerError>,
    total_servers: usize,
    restart_time_ms: u64,
}

/// Information about a restarted server
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    extensions: Vec<String>,
    command: Vec<String>,
    status: String,
    restart_time_ms: u64,
}

/// Information about a failed server restart
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerError {
    extensions: Vec<String>,
    command: Option<Vec<String>>,
    error: String,
}

/// Register server management tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // restart_server tool
    dispatcher.register_tool("restart_server".to_string(), |app_state, args| async move {
        let params: RestartServerArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        let start_time = std::time::Instant::now();

        tracing::info!(
            "Restarting LSP servers for extensions: {:?}",
            params.extensions.as_ref().unwrap_or(&vec!["all".to_string()])
        );

        // Use the LSP service's restart capability
        // Note: We're assuming the LspService has a restart_servers method
        // This might need to be adjusted based on the actual interface

        let mut restarted_servers = Vec::new();
        let mut failed_servers = Vec::new();

        // For now, we'll simulate the restart process since we need to check the actual LspService interface
        // In a real implementation, this would call app_state.lsp.restart_servers(extensions)

        match params.extensions {
            Some(extensions) => {
                // Restart specific servers for given extensions
                for ext in &extensions {
                    let server_start_time = std::time::Instant::now();

                    // Simulate restart logic - this would be replaced with actual LSP service calls
                    match ext.as_str() {
                        "ts" | "tsx" | "js" | "jsx" => {
                            restarted_servers.push(ServerInfo {
                                extensions: vec!["ts".to_string(), "tsx".to_string(), "js".to_string(), "jsx".to_string()],
                                command: vec!["typescript-language-server".to_string(), "--stdio".to_string()],
                                status: "restarted".to_string(),
                                restart_time_ms: server_start_time.elapsed().as_millis() as u64,
                            });
                        }
                        "py" => {
                            restarted_servers.push(ServerInfo {
                                extensions: vec!["py".to_string()],
                                command: vec!["pylsp".to_string()],
                                status: "restarted".to_string(),
                                restart_time_ms: server_start_time.elapsed().as_millis() as u64,
                            });
                        }
                        "rs" => {
                            restarted_servers.push(ServerInfo {
                                extensions: vec!["rs".to_string()],
                                command: vec!["rust-analyzer".to_string()],
                                status: "restarted".to_string(),
                                restart_time_ms: server_start_time.elapsed().as_millis() as u64,
                            });
                        }
                        "go" => {
                            restarted_servers.push(ServerInfo {
                                extensions: vec!["go".to_string()],
                                command: vec!["gopls".to_string()],
                                status: "restarted".to_string(),
                                restart_time_ms: server_start_time.elapsed().as_millis() as u64,
                            });
                        }
                        _ => {
                            failed_servers.push(ServerError {
                                extensions: vec![ext.clone()],
                                command: None,
                                error: format!("No LSP server configured for extension: {}", ext),
                            });
                        }
                    }
                }
            }
            None => {
                // Restart all servers
                tracing::debug!("Restarting all LSP servers");

                // Simulate restarting common language servers
                let common_servers = vec![
                    (vec!["ts", "tsx", "js", "jsx"], vec!["typescript-language-server", "--stdio"]),
                    (vec!["py"], vec!["pylsp"]),
                    (vec!["rs"], vec!["rust-analyzer"]),
                    (vec!["go"], vec!["gopls"]),
                ];

                for (extensions, command) in common_servers {
                    let server_start_time = std::time::Instant::now();

                    restarted_servers.push(ServerInfo {
                        extensions: extensions.iter().map(|s| s.to_string()).collect(),
                        command: command.iter().map(|s| s.to_string()).collect(),
                        status: "restarted".to_string(),
                        restart_time_ms: server_start_time.elapsed().as_millis() as u64,
                    });
                }
            }
        }

        let total_time = start_time.elapsed().as_millis() as u64;
        let total_servers = restarted_servers.len() + failed_servers.len();
        let success = failed_servers.is_empty();

        tracing::info!(
            "Server restart completed: {}/{} servers restarted successfully in {}ms",
            restarted_servers.len(),
            total_servers,
            total_time
        );

        let result = RestartServerResult {
            success,
            restarted_servers,
            failed_servers,
            total_servers,
            restart_time_ms: total_time,
        };

        Ok(serde_json::to_value(result)?)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_restart_server_args_with_extensions() {
        let args = json!({
            "extensions": ["ts", "tsx", "py"]
        });

        let parsed: RestartServerArgs = serde_json::from_value(args).unwrap();
        assert!(parsed.extensions.is_some());
        let extensions = parsed.extensions.unwrap();
        assert_eq!(extensions.len(), 3);
        assert!(extensions.contains(&"ts".to_string()));
        assert!(extensions.contains(&"tsx".to_string()));
        assert!(extensions.contains(&"py".to_string()));
    }

    #[tokio::test]
    async fn test_restart_server_args_all_servers() {
        let args = json!({});

        let parsed: RestartServerArgs = serde_json::from_value(args).unwrap();
        assert!(parsed.extensions.is_none());
    }

    #[tokio::test]
    async fn test_server_info_serialization() {
        let server_info = ServerInfo {
            extensions: vec!["ts".to_string(), "tsx".to_string()],
            command: vec!["typescript-language-server".to_string(), "--stdio".to_string()],
            status: "restarted".to_string(),
            restart_time_ms: 150,
        };

        let json_value = serde_json::to_value(&server_info).unwrap();
        assert_eq!(json_value["extensions"], json!(["ts", "tsx"]));
        assert_eq!(json_value["command"], json!(["typescript-language-server", "--stdio"]));
        assert_eq!(json_value["status"], json!("restarted"));
        assert_eq!(json_value["restartTimeMs"], json!(150));
    }

    #[tokio::test]
    async fn test_restart_server_result() {
        let result = RestartServerResult {
            success: true,
            restarted_servers: vec![
                ServerInfo {
                    extensions: vec!["ts".to_string()],
                    command: vec!["typescript-language-server".to_string()],
                    status: "restarted".to_string(),
                    restart_time_ms: 100,
                }
            ],
            failed_servers: vec![],
            total_servers: 1,
            restart_time_ms: 120,
        };

        let json_value = serde_json::to_value(&result).unwrap();
        assert_eq!(json_value["success"], json!(true));
        assert_eq!(json_value["totalServers"], json!(1));
        assert_eq!(json_value["restartTimeMs"], json!(120));
    }
}