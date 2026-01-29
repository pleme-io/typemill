use super::{Command, CommandContext, GlobalArgs};
use crate::client_config::ClientConfig;
use crate::error::{ClientError, ClientResult};
use crate::websocket::{ConnectionState, WebSocketClient};
use async_trait::async_trait;
use std::time::Duration;

/// Status command for health checking and diagnostics
pub struct StatusCommand {
    /// Server URL override
    pub url: Option<String>,
    /// Authentication token override
    pub token: Option<String>,
    /// Show detailed information
    pub verbose: bool,
}

impl StatusCommand {
    pub fn new(url: Option<String>, token: Option<String>) -> Self {
        Self {
            url,
            token,
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Run comprehensive status check
    async fn run_status_check(&self, ctx: &CommandContext) -> ClientResult<()> {
        ctx.formatter.header("🔍 TypeMill Client Status");
        println!();

        // Status tracking
        let mut overall_status = true;
        let mut status_items = Vec::new();

        // 1. Configuration Check
        self.check_configuration(ctx, &mut status_items, &mut overall_status)
            .await?;

        // 2. Connection Check
        if ctx.is_configured() || self.url.is_some() {
            self.check_connection(ctx, &mut status_items, &mut overall_status)
                .await?;
        } else {
            status_items.push((
                "Server Connection".to_string(),
                "Cannot test - no URL configured".to_string(),
                false,
            ));
            overall_status = false;
        }

        // 3. Display summary
        self.display_status_summary(ctx, &status_items, overall_status)?;

        // 4. Show recommendations if needed
        if !overall_status {
            self.show_recommendations(ctx, &status_items)?;
        }

        Ok(())
    }

    /// Check client configuration
    async fn check_configuration(
        &self,
        ctx: &CommandContext,
        status_items: &mut Vec<(String, String, bool)>,
        overall_status: &mut bool,
    ) -> ClientResult<()> {
        ctx.display_info("📋 Checking Configuration");

        // Configuration file
        let config_path = ClientConfig::default_config_path()?;
        let config_exists = config_path.exists();

        status_items.push((
            "Config File".to_string(),
            if config_exists {
                format!("Found at {}", config_path.display())
            } else {
                "Not found".to_string()
            },
            config_exists,
        ));

        // Server URL
        let url_status = if let Some(url) = self.url.as_ref().or(ctx.config.url.as_ref()) {
            ctx.formatter.url(url).to_string()
        } else {
            "Not configured".to_string()
        };
        let has_url = self.url.is_some() || ctx.config.url.is_some();

        status_items.push(("Server URL".to_string(), url_status, has_url));

        if !has_url {
            *overall_status = false;
        }

        // Authentication token
        let has_token = ctx.config.token.is_some() || self.token.is_some();
        status_items.push((
            "Auth Token".to_string(),
            if has_token {
                "Configured".to_string()
            } else {
                "Not configured".to_string()
            },
            true, // Token is optional, so always "OK"
        ));

        // Timeout configuration
        let timeout = ctx.config.get_timeout_ms();
        status_items.push(("Timeout".to_string(), format!("{}ms", timeout), true));

        if self.verbose {
            println!();
            ctx.display_info("Configuration details:");
            println!("{}", ctx.config_summary());
        }

        println!();
        Ok(())
    }

    /// Check server connection and capabilities
    async fn check_connection(
        &self,
        ctx: &CommandContext,
        status_items: &mut Vec<(String, String, bool)>,
        overall_status: &mut bool,
    ) -> ClientResult<()> {
        ctx.display_info("🌐 Checking Server Connection");

        let client = match ctx.create_client(self.url.clone(), self.token.clone()) {
            Ok(client) => client,
            Err(e) => {
                status_items.push((
                    "Client Creation".to_string(),
                    format!("Failed: {}", e),
                    false,
                ));
                *overall_status = false;
                return Ok(());
            }
        };

        // Test connection
        let connection_result = self.test_connection(&client).await;
        match connection_result {
            Ok((ping_time, state)) => {
                let state_str = super::utils::format_connection_status(
                    matches!(
                        state,
                        ConnectionState::Connected | ConnectionState::Authenticated
                    ),
                    matches!(state, ConnectionState::Authenticated),
                );

                status_items.push((
                    "Connection".to_string(),
                    format!("{} ({})", state_str, ctx.formatter.duration(ping_time)),
                    matches!(
                        state,
                        ConnectionState::Connected | ConnectionState::Authenticated
                    ),
                ));

                // Test authentication if token is available
                if ctx.config.has_token() || self.token.is_some() {
                    self.check_authentication(&client, status_items).await?;
                }

                // Check server capabilities
                self.check_server_capabilities(ctx, &client, status_items)
                    .await?;

                // Test basic tool availability
                if self.verbose {
                    self.check_tool_availability(ctx, &client, status_items)
                        .await?;
                }
            }
            Err(e) => {
                status_items.push(("Connection".to_string(), format!("Failed: {}", e), false));
                *overall_status = false;

                // Provide connection diagnostics
                self.diagnose_connection_failure(ctx, &e)?;
            }
        }

        let _ = client.disconnect().await;
        println!();
        Ok(())
    }

    /// Test connection and return ping time and state
    async fn test_connection(
        &self,
        client: &WebSocketClient,
    ) -> ClientResult<(Duration, ConnectionState)> {
        // Connect
        client.connect().await?;

        // Get connection state
        let state = client.get_state().await;

        // Ping to measure response time
        let ping_time = client.ping().await?;

        Ok((ping_time, state))
    }

    /// Check authentication status
    async fn check_authentication(
        &self,
        client: &WebSocketClient,
        status_items: &mut Vec<(String, String, bool)>,
    ) -> ClientResult<()> {
        let state = client.get_state().await;
        let is_authenticated = matches!(state, ConnectionState::Authenticated);

        status_items.push((
            "Authentication".to_string(),
            if is_authenticated {
                "Authenticated".to_string()
            } else {
                "Not authenticated".to_string()
            },
            is_authenticated,
        ));

        Ok(())
    }

    /// Check server capabilities
    async fn check_server_capabilities(
        &self,
        ctx: &CommandContext,
        client: &WebSocketClient,
        status_items: &mut Vec<(String, String, bool)>,
    ) -> ClientResult<()> {
        match client.get_capabilities().await {
            Ok(capabilities) => {
                status_items.push((
                    "Server Capabilities".to_string(),
                    "Available".to_string(),
                    true,
                ));

                if self.verbose {
                    println!();
                    ctx.display_info("Server capabilities:");
                    println!("{}", super::utils::format_capabilities(&capabilities));
                }
            }
            Err(e) => {
                status_items.push((
                    "Server Capabilities".to_string(),
                    format!("Failed: {}", e),
                    false,
                ));
            }
        }

        Ok(())
    }

    /// Check availability of common tools
    async fn check_tool_availability(
        &self,
        ctx: &CommandContext,
        client: &WebSocketClient,
        status_items: &mut Vec<(String, String, bool)>,
    ) -> ClientResult<()> {
        ctx.display_info("🔧 Testing Tool Availability");

        // Magnificent Seven - the complete public API
        let common_tools = vec![
            "inspect_code",
            "search_code",
            "rename_all",
            "relocate",
            "prune",
            "refactor",
            "workspace",
        ];

        let mut successful_tools = 0;

        for tool in &common_tools {
            match client.call_tool(tool, None).await {
                Ok(response) => {
                    if response.error.is_none()
                        || response.error.as_ref().map(|e| e.code) == Some(-32601)
                    {
                        // Tool exists (either success or "method not found" which means it's recognized)
                        successful_tools += 1;
                    }
                }
                Err(_) => {
                    // Tool might not be available or server issue
                }
            }
        }

        let all_available = successful_tools == common_tools.len();
        status_items.push((
            "Tool Availability".to_string(),
            format!(
                "{}/{} tools available",
                successful_tools,
                common_tools.len()
            ),
            all_available,
        ));

        Ok(())
    }

    /// Diagnose connection failure and provide specific guidance
    fn diagnose_connection_failure(
        &self,
        ctx: &CommandContext,
        error: &ClientError,
    ) -> ClientResult<()> {
        println!();
        ctx.display_warning("🔍 Connection Diagnostics");

        match error {
            ClientError::ConnectionError(msg) if msg.contains("Connection refused") => {
                println!("• The server is not running or not accepting connections");
                println!("• Check if mill server is started on the specified port");
                println!("• Verify the port number in your configuration");
            }
            ClientError::ConnectionError(msg) if msg.contains("timeout") => {
                println!("• The server is not responding within the timeout period");
                println!("• The server might be overloaded or network is slow");
                println!("• Try increasing the timeout value");
            }
            ClientError::ConnectionError(msg) if msg.contains("invalid URL") => {
                println!("• The server URL format is incorrect");
                println!("• Ensure URL starts with ws:// or wss://");
                println!("• Check for typos in the hostname or port");
            }
            ClientError::AuthError(_) => {
                println!("• Authentication failed with the provided token");
                println!("• Verify the token is correct and not expired");
                println!("• Check if the server requires authentication");
            }
            _ => {
                println!("• General connection error occurred");
                println!("• Check network connectivity");
                println!("• Verify server is accessible from your location");
            }
        }

        println!();
        Ok(())
    }

    /// Display status summary table
    fn display_status_summary(
        &self,
        ctx: &CommandContext,
        status_items: &[(String, String, bool)],
        overall_status: bool,
    ) -> ClientResult<()> {
        println!();
        ctx.formatter.header("📊 Status Summary");
        println!();

        // Display status table
        println!("{}", ctx.formatter.status_summary(status_items));

        // Overall status
        println!();
        if overall_status {
            ctx.display_success("✅ All systems operational");
        } else {
            ctx.display_warning("⚠️  Some issues detected");
        }

        Ok(())
    }

    /// Show recommendations for fixing issues
    fn show_recommendations(
        &self,
        ctx: &CommandContext,
        status_items: &[(String, String, bool)],
    ) -> ClientResult<()> {
        println!();
        ctx.display_info("💡 Recommendations");

        let mut has_recommendations = false;

        for (category, _, is_ok) in status_items {
            if !is_ok {
                has_recommendations = true;
                match category.as_str() {
                    "Config File" => {
                        println!("• Run 'mill setup' to create configuration");
                    }
                    "Server URL" => {
                        println!("• Configure server URL with 'mill setup'");
                        println!("• Or use --url flag: mill status --url ws://localhost:3000");
                    }
                    "Connection" => {
                        println!("• Ensure mill server is running");
                        println!("• Check server URL and network connectivity");
                        println!("• Run 'mill setup' to reconfigure");
                    }
                    "Authentication" => {
                        println!("• Verify authentication token is correct");
                        println!("• Update token with 'mill setup'");
                    }
                    "Server Capabilities" => {
                        println!("• Server might be an older version");
                        println!("• Check server logs for errors");
                    }
                    "Tool Availability" => {
                        println!("• Some tools might not be available on this server");
                        println!("• Check server configuration and language server setup");
                    }
                    _ => {}
                }
            }
        }

        if !has_recommendations {
            ctx.display_info("No specific recommendations needed");
        }

        println!();
        Ok(())
    }
}

impl Default for StatusCommand {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[async_trait]
impl Command for StatusCommand {
    async fn execute(&self, global_args: &GlobalArgs) -> ClientResult<()> {
        let ctx = CommandContext::new(global_args.clone()).await?;
        self.run_status_check(&ctx).await
    }

    fn name(&self) -> &'static str {
        "status"
    }

    fn description(&self) -> &'static str {
        "Check client status and verify connectivity to the server"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_command_creation() {
        let cmd = StatusCommand::new(None, None);
        assert_eq!(cmd.name(), "status");
        assert_eq!(
            cmd.description(),
            "Check client status and verify connectivity to the server"
        );
        assert!(!cmd.verbose);
    }

    #[test]
    fn test_status_command_with_params() {
        let cmd = StatusCommand::new(
            Some("ws://example.com:3000".to_string()),
            Some("test-token".to_string()),
        )
        .with_verbose(true);

        assert_eq!(cmd.url, Some("ws://example.com:3000".to_string()));
        assert_eq!(cmd.token, Some("test-token".to_string()));
        assert!(cmd.verbose);
    }

    #[test]
    fn test_status_command_default() {
        let cmd = StatusCommand::default();
        assert_eq!(cmd.name(), "status");
        assert!(cmd.url.is_none());
        assert!(cmd.token.is_none());
    }
}
