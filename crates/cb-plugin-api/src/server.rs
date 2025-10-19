//! Provides scaffolding to run a `LanguagePlugin` implementation as a
//! standalone, out-of-process RPC server that communicates over stdio.

use crate::{LanguagePlugin, PluginResult};
use codebuddy_foundation::protocol::plugin_protocol::{ PluginRequest , PluginResponse };
use serde_json::Value;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

/// A server that wraps a `LanguagePlugin` to handle JSON-RPC requests.
pub struct PluginServer<P: LanguagePlugin> {
    plugin: P,
}

impl<P: LanguagePlugin + 'static> PluginServer<P> {
    /// Creates a new `PluginServer` with the given plugin implementation.
    pub fn new(plugin: P) -> Self {
        Self { plugin }
    }

    /// Runs the main event loop of the server.
    ///
    /// This function reads requests from stdin, line by line, handles them,
    /// and writes the JSON-serialized responses to stdout.
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdin = BufReader::new(io::stdin());
        let mut stdout = io::stdout();
        let mut line = String::new();

        info!(
            "External plugin server started for '{}'",
            self.plugin.metadata().name
        );

        loop {
            line.clear();
            match stdin.read_line(&mut line).await {
                Ok(0) => {
                    info!("Plugin stdin closed, shutting down.");
                    break; // EOF
                }
                Ok(_) => {
                    let request: PluginRequest = match serde_json::from_str(&line) {
                        Ok(req) => req,
                        Err(e) => {
                            let err_response =
                                PluginResponse::error(0, -32700, &format!("Parse error: {}", e));
                            let response_json = serde_json::to_string(&err_response)? + "\n";
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.flush().await?;
                            continue;
                        }
                    };

                    let response = self.handle_request(request).await;
                    let response_json = serde_json::to_string(&response)? + "\n";
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.flush().await?;
                }
                Err(e) => {
                    error!("Failed to read from stdin: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Handles a single incoming request and produces a response.
    async fn handle_request(&self, req: PluginRequest) -> PluginResponse {
        let result = match self.dispatch(req.method.as_str(), req.params).await {
            Ok(value) => value,
            Err(e) => {
                // `PluginError` has a `From` impl for `ApiError`, which can be converted
                // to the JSON-RPC error format.
                let api_error: codebuddy_foundation::error::ApiError = e.into();
                // JSON-RPC error code: -32603 = Internal error
                return PluginResponse::error(req.id, -32603, &api_error.message);
            }
        };

        PluginResponse {
            id: req.id,
            result: Some(result),
            error: None,
        }
    }

    /// Dispatches a method call to the appropriate `LanguagePlugin` function.
    async fn dispatch(&self, method: &str, params: Value) -> PluginResult<Value> {
        match method {
            "parse" => {
                let source: String = serde_json::from_value(params)?;
                let parsed = self.plugin.parse(&source).await?;
                Ok(serde_json::to_value(parsed)?)
            }
            "analyze_manifest" => {
                let path: std::path::PathBuf = serde_json::from_value(params)?;
                let manifest = self.plugin.analyze_manifest(&path).await?;
                Ok(serde_json::to_value(manifest)?)
            }
            // Add other LanguagePlugin methods here...
            _ => Err(crate::PluginError::not_supported(format!(
                "Method '{}' is not implemented",
                method
            ))),
        }
    }
}

// Implement `From` for `serde_json::Error` to `PluginError` for convenience.
impl From<serde_json::Error> for crate::PluginError {
    fn from(err: serde_json::Error) -> Self {
        crate::PluginError::invalid_input(format!("JSON deserialization error: {}", err))
    }
}