pub mod call;
pub mod connect;
pub mod doctor;
#[cfg(feature = "mcp-proxy")]
pub mod mcp;
pub mod setup;
pub mod status;

use crate::client_config::ClientConfig;
use crate::error::{ClientError, ClientResult};
use crate::formatting::Formatter;
use crate::interactive::Interactive;
use crate::websocket::WebSocketClient;
use async_trait::async_trait;
use std::time::Duration;

/// Global arguments passed to all commands
#[derive(Debug, Clone, Default)]
pub struct GlobalArgs {
    /// Enable debug logging
    pub debug: bool,
    /// Custom config file path
    pub config_path: Option<String>,
    /// Request timeout in milliseconds
    pub timeout: Option<u64>,
    /// Disable colors in output
    pub no_color: bool,
    /// Disable emojis in output
    pub no_emoji: bool,
}

/// Common trait for all CLI commands
#[async_trait]
pub trait Command: Send + Sync {
    /// Execute the command with global arguments
    async fn execute(&self, global_args: &GlobalArgs) -> ClientResult<()>;

    /// Get the command name (for logging/debugging)
    fn name(&self) -> &'static str;

    /// Get a description of what the command does
    fn description(&self) -> &'static str;
}

/// Shared utilities for commands
pub struct CommandContext {
    pub config: ClientConfig,
    pub formatter: Formatter,
    pub interactive: Interactive,
    pub global_args: GlobalArgs,
}

impl CommandContext {
    /// Create a new command context
    pub async fn new(global_args: GlobalArgs) -> ClientResult<Self> {
        // Load configuration
        let config = if let Some(ref config_path) = global_args.config_path {
            ClientConfig::load_from_path(config_path).await?
        } else {
            ClientConfig::load().await?
        };

        // Create formatter with settings from global args
        let formatter = Formatter::with_settings(!global_args.no_color, !global_args.no_emoji);

        // Create interactive helper
        let interactive = Interactive::with_formatter(formatter.clone());

        Ok(Self {
            config,
            formatter,
            interactive,
            global_args,
        })
    }

    /// Create a WebSocket client with current configuration and overrides
    pub fn create_client(
        &self,
        url_override: Option<String>,
        token_override: Option<String>,
    ) -> ClientResult<WebSocketClient> {
        let mut config = self.config.clone();

        // Apply overrides
        if let Some(url) = url_override {
            config.set_url(url);
        }
        if let Some(token) = token_override {
            config.set_token(token);
        }

        // Apply global timeout override
        if let Some(timeout) = self.global_args.timeout {
            config.set_timeout_ms(timeout);
        }

        // Validate that we have a URL
        if config.url.is_none() {
            return Err(ClientError::ConfigError(
                "No server URL configured. Run 'mill setup' or provide --url".to_string(),
            ));
        }

        Ok(WebSocketClient::new(config))
    }

    /// Connect to the server and return a ready client
    pub async fn connect_client(
        &self,
        url_override: Option<String>,
        token_override: Option<String>,
    ) -> ClientResult<WebSocketClient> {
        let client = self.create_client(url_override, token_override)?;

        self.formatter.progress_message("Connecting to server...");

        match client.connect().await {
            Ok(()) => {
                self.formatter.success_message("Connected successfully");
                Ok(client)
            }
            Err(e) => {
                self.formatter
                    .error_message(&format!("Failed to connect: {}", e));
                Err(e)
            }
        }
    }

    /// Test connection without maintaining it
    pub async fn test_connection(
        &self,
        url_override: Option<String>,
        token_override: Option<String>,
    ) -> ClientResult<Duration> {
        let client = self.create_client(url_override, token_override)?;

        // Connect
        client.connect().await?;

        // Test with ping
        let ping_time = client.ping().await?;

        // Disconnect
        client.disconnect().await?;

        Ok(ping_time)
    }

    /// Save configuration
    pub async fn save_config(&mut self) -> ClientResult<()> {
        self.config.save().await?;
        self.formatter.success_message("Configuration saved");
        Ok(())
    }

    /// Update configuration with new values
    pub fn update_config(
        &mut self,
        url: Option<String>,
        token: Option<String>,
        timeout: Option<u64>,
    ) {
        if let Some(url) = url {
            self.config.set_url(url);
        }
        if let Some(token) = token {
            self.config.set_token(token);
        }
        if let Some(timeout) = timeout {
            self.config.set_timeout_ms(timeout);
        }
    }

    /// Get configuration summary for display
    pub fn config_summary(&self) -> String {
        let url = self.config.url.as_deref().unwrap_or("<not configured>");
        let token_status = if self.config.token.is_some() {
            "✓ configured"
        } else {
            "✗ not configured"
        };
        let timeout = self.config.get_timeout_ms();

        format!(
            "{}\n{}\n{}",
            self.formatter
                .key_value("Server URL", &self.formatter.url(url)),
            self.formatter.key_value("Auth Token", token_status),
            self.formatter
                .key_value("Timeout", &format!("{}ms", timeout))
        )
    }

    /// Display error with proper formatting
    pub fn display_error(&self, error: &ClientError) {
        eprintln!("{}", self.formatter.client_error(error));
    }

    /// Display success message
    pub fn display_success(&self, message: &str) {
        println!("{}", self.formatter.success(message));
    }

    /// Display info message
    pub fn display_info(&self, message: &str) {
        println!("{}", self.formatter.info(message));
    }

    /// Display warning message
    pub fn display_warning(&self, message: &str) {
        println!("{}", self.formatter.warning(message));
    }

    /// Check if configuration is complete
    pub fn is_configured(&self) -> bool {
        self.config.is_complete()
    }

    /// Get suggestions for unconfigured settings
    pub fn configuration_suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        if self.config.url.is_none() {
            suggestions.push("Run 'mill setup' to configure server URL".to_string());
        }

        if !self.config.has_token() {
            suggestions
                .push("Consider adding an authentication token for secure access".to_string());
        }

        suggestions
    }
}

/// Utility functions for command implementations
pub mod utils {
    use super::*;
    use serde_json::Value;

    /// Parse JSON parameters from string
    pub fn parse_json_params(params_str: Option<&str>) -> ClientResult<Option<Value>> {
        match params_str {
            Some(s) if s.trim().is_empty() => Ok(None),
            Some(s) => serde_json::from_str(s).map(Some).map_err(|e| {
                ClientError::SerializationError(format!("Invalid JSON parameters: {}", e))
            }),
            None => Ok(None),
        }
    }

    /// Validate tool name format
    pub fn validate_tool_name(tool: &str) -> ClientResult<()> {
        if tool.trim().is_empty() {
            return Err(ClientError::RequestError(
                "Tool name cannot be empty".to_string(),
            ));
        }

        // Basic validation - tool names should be alphanumeric with underscores
        if !tool.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(ClientError::RequestError(
                "Tool name should only contain letters, numbers, and underscores".to_string(),
            ));
        }

        Ok(())
    }

    /// Create a timeout duration from milliseconds
    pub fn timeout_from_ms(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    /// Format connection status for display
    pub fn format_connection_status(connected: bool, authenticated: bool) -> String {
        match (connected, authenticated) {
            (true, true) => "Connected and authenticated".to_string(),
            (true, false) => "Connected (not authenticated)".to_string(),
            (false, _) => "Not connected".to_string(),
        }
    }

    /// Extract error message from MCP response
    pub fn extract_error_message(response: &crate::websocket::MCPResponse) -> Option<String> {
        response.error.as_ref().map(|e| e.message.clone())
    }

    /// Check if response indicates success
    pub fn is_success_response(response: &crate::websocket::MCPResponse) -> bool {
        response.error.is_none()
    }

    /// Format capabilities for display
    pub fn format_capabilities(capabilities: &Value) -> String {
        // Try to extract and format known capability fields
        let mut output = String::new();

        if let Some(obj) = capabilities.as_object() {
            for (key, value) in obj {
                let formatted_value = match value {
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.clone(),
                    Value::Array(arr) => format!("{} items", arr.len()),
                    Value::Object(obj) => format!("{} properties", obj.len()),
                    Value::Null => "null".to_string(),
                };

                output.push_str(&format!("  {}: {}\n", key, formatted_value));
            }
        }

        if output.is_empty() {
            "No capabilities reported".to_string()
        } else {
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_global_args_default() {
        let args = GlobalArgs::default();
        assert!(!args.debug);
        assert!(args.config_path.is_none());
        assert!(args.timeout.is_none());
        assert!(!args.no_color);
        assert!(!args.no_emoji);
    }

    #[test]
    fn test_parse_json_params() {
        // Valid JSON
        let result = utils::parse_json_params(Some(r#"{"key": "value"}"#)).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), json!({"key": "value"}));

        // Empty string
        let result = utils::parse_json_params(Some("")).unwrap();
        assert!(result.is_none());

        // None
        let result = utils::parse_json_params(None).unwrap();
        assert!(result.is_none());

        // Invalid JSON
        let result = utils::parse_json_params(Some("invalid"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tool_name() {
        // Valid names
        assert!(utils::validate_tool_name("inspect_code").is_ok());
        assert!(utils::validate_tool_name("test123").is_ok());
        assert!(utils::validate_tool_name("simple").is_ok());

        // Invalid names
        assert!(utils::validate_tool_name("").is_err());
        assert!(utils::validate_tool_name("invalid-name").is_err());
        assert!(utils::validate_tool_name("with space").is_err());
        assert!(utils::validate_tool_name("with.dot").is_err());
    }

    #[test]
    fn test_format_connection_status() {
        assert_eq!(
            utils::format_connection_status(true, true),
            "Connected and authenticated"
        );
        assert_eq!(
            utils::format_connection_status(true, false),
            "Connected (not authenticated)"
        );
        assert_eq!(
            utils::format_connection_status(false, true),
            "Not connected"
        );
        assert_eq!(
            utils::format_connection_status(false, false),
            "Not connected"
        );
    }
}
