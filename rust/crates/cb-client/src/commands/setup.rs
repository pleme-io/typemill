use super::{Command, CommandContext, GlobalArgs};
use crate::error::{ClientError, ClientResult};
use async_trait::async_trait;

/// Setup command for interactive configuration wizard
pub struct SetupCommand;

impl SetupCommand {
    pub fn new() -> Self {
        Self
    }

    /// Run the setup wizard
    async fn run_wizard(&self, ctx: &mut CommandContext) -> ClientResult<()> {
        ctx.interactive.banner(
            "🚀 Codebuddy Client Setup",
            Some("Let's configure your client to connect to the codebuddy server"),
        )?;

        // Current configuration summary if exists
        if ctx.is_configured() {
            ctx.interactive.info("Current configuration:");
            println!("{}", ctx.config_summary());
            println!();

            if !ctx
                .interactive
                .confirm("Would you like to update your configuration?", Some(true))?
            {
                ctx.display_info("Setup cancelled");
                return Ok(());
            }
        }

        // Step 1: Get server URL
        let url = self.setup_server_url(ctx).await?;

        // Step 2: Get authentication token (optional)
        let token = self.setup_auth_token(ctx).await?;

        // Step 3: Get timeout settings
        let timeout = self.setup_timeout(ctx).await?;

        // Step 4: Test connection
        let test_successful = self.test_connection(ctx, &url, &token, timeout).await?;

        if !test_successful
            && !ctx.interactive.confirm(
                "Connection test failed. Save configuration anyway?",
                Some(false),
            )? {
                ctx.display_info("Setup cancelled");
                return Ok(());
            }

        // Step 5: Save configuration
        ctx.update_config(Some(url), token, Some(timeout));

        // Show final summary
        ctx.interactive.banner("📝 Configuration Summary", None)?;
        println!("{}", ctx.config_summary());
        println!();

        if ctx.interactive.confirm_config(&ctx.config_summary())? {
            ctx.save_config().await?;

            ctx.interactive.banner(
                "✅ Setup Complete",
                Some("Your codebuddy client is now configured!"),
            )?;

            self.show_next_steps(ctx)?;
        } else {
            ctx.display_info("Configuration not saved");
        }

        Ok(())
    }

    /// Setup server URL with validation and smart defaults
    async fn setup_server_url(&self, ctx: &CommandContext) -> ClientResult<String> {
        ctx.interactive
            .progress_message("Step 1: Server Configuration");

        // Smart defaults based on common development setups
        let current_url = ctx.config.url.as_deref();
        let _default_url = current_url.unwrap_or("ws://localhost:3000");

        println!();
        ctx.interactive
            .info("Enter the WebSocket URL of your codebuddy server");
        ctx.interactive.info("Common formats:");
        println!("  • Local development: ws://localhost:3000");
        println!("  • Remote server: ws://your-server.com:3000");
        println!("  • Secure connection: wss://your-server.com:3000");
        println!();

        loop {
            match ctx.interactive.get_server_url(current_url) {
                Ok(url) => {
                    // Additional validation
                    if !url.starts_with("ws://") && !url.starts_with("wss://") {
                        ctx.interactive
                            .error_message("URL must start with ws:// or wss://");
                        if !ctx.interactive.retry_on_error("Invalid URL format", true)? {
                            return Err(ClientError::ConfigError("Invalid URL format".to_string()));
                        }
                        continue;
                    }

                    ctx.interactive
                        .success_message(&format!("Server URL: {}", ctx.formatter.url(&url)));
                    return Ok(url);
                }
                Err(e) => {
                    if !ctx
                        .interactive
                        .retry_on_error(&format!("Error: {}", e), true)?
                    {
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Setup authentication token (optional)
    async fn setup_auth_token(&self, ctx: &CommandContext) -> ClientResult<Option<String>> {
        println!();
        ctx.interactive
            .progress_message("Step 2: Authentication (Optional)");

        println!();
        ctx.interactive
            .info("Authentication token provides secure access to the server");
        ctx.interactive
            .info("Leave empty if your server doesn't require authentication");
        println!();

        if ctx.interactive.confirm(
            "Do you want to configure an authentication token?",
            Some(false),
        )? {
            loop {
                match ctx
                    .interactive
                    .optional_input("Authentication token", ctx.config.get_token())
                {
                    Ok(Some(token)) => {
                        // Basic token validation
                        if token.trim().len() < 8 {
                            if !ctx
                                .interactive
                                .retry_on_error("Token should be at least 8 characters", true)?
                            {
                                return Ok(None);
                            }
                            continue;
                        }

                        ctx.interactive
                            .success_message("Authentication token configured");
                        return Ok(Some(token));
                    }
                    Ok(None) => {
                        ctx.interactive
                            .warning_message("No authentication token configured");
                        return Ok(None);
                    }
                    Err(e) => {
                        if !ctx
                            .interactive
                            .retry_on_error(&format!("Error: {}", e), true)?
                        {
                            return Err(e);
                        }
                    }
                }
            }
        } else {
            ctx.interactive.info("Skipping authentication token setup");
            Ok(None)
        }
    }

    /// Setup timeout configuration
    async fn setup_timeout(&self, ctx: &CommandContext) -> ClientResult<u64> {
        println!();
        ctx.interactive
            .progress_message("Step 3: Timeout Configuration");

        println!();
        ctx.interactive
            .info("Request timeout determines how long to wait for server responses");
        ctx.interactive.info("Recommended values:");
        println!("  • Fast local network: 10-30 seconds (10000-30000ms)");
        println!("  • Remote/slow network: 30-60 seconds (30000-60000ms)");
        println!();

        let current_timeout = Some(ctx.config.get_timeout_ms());
        ctx.interactive.get_timeout(current_timeout)
    }

    /// Test connection to the server
    async fn test_connection(
        &self,
        ctx: &CommandContext,
        url: &str,
        token: &Option<String>,
        _timeout: u64,
    ) -> ClientResult<bool> {
        println!();
        ctx.interactive.progress_message("Step 4: Connection Test");

        println!();
        ctx.interactive.info("Testing connection to server...");

        let progress = ctx.formatter.progress_bar(None, "Connecting...");

        match ctx
            .test_connection(Some(url.to_string()), token.clone())
            .await
        {
            Ok(ping_time) => {
                progress.finish_with_message("Connection successful!");

                ctx.interactive.success_message(&format!(
                    "✅ Connected successfully (ping: {})",
                    ctx.formatter.duration(ping_time)
                ));

                // Try to get server capabilities
                println!();
                ctx.interactive.info("Checking server capabilities...");

                if let Ok(client) = ctx.create_client(Some(url.to_string()), token.clone()) {
                    if client.connect().await.is_ok() {
                        match client.get_capabilities().await {
                            Ok(capabilities) => {
                                ctx.interactive
                                    .success_message("Server capabilities retrieved");
                                println!();
                                ctx.interactive.info("Server capabilities:");
                                println!("{}", super::utils::format_capabilities(&capabilities));
                            }
                            Err(e) => {
                                ctx.interactive.warning_message(&format!(
                                    "Could not retrieve server capabilities: {}",
                                    e
                                ));
                            }
                        }
                        let _ = client.disconnect().await;
                    }
                }

                Ok(true)
            }
            Err(e) => {
                progress.finish_with_message("Connection failed");

                ctx.interactive
                    .error_message(&format!("❌ Connection failed: {}", e));

                // Provide helpful troubleshooting tips
                self.show_troubleshooting_tips(ctx, url, &e);

                Ok(false)
            }
        }
    }

    /// Show troubleshooting tips based on the error
    fn show_troubleshooting_tips(&self, ctx: &CommandContext, url: &str, error: &ClientError) {
        println!();
        ctx.interactive.warning_message("Troubleshooting tips:");

        match error {
            ClientError::ConnectionError(msg) if msg.contains("Connection refused") => {
                println!("  • Make sure the codebuddy server is running");
                println!("  • Check that the port is correct (default: 3000)");
                println!("  • Verify the server is accepting WebSocket connections");
            }
            ClientError::ConnectionError(msg) if msg.contains("timeout") => {
                println!("  • The server might be slow to respond");
                println!("  • Check your network connection");
                println!("  • Try increasing the timeout value");
            }
            ClientError::AuthError(_) => {
                println!("  • Verify the authentication token is correct");
                println!("  • Check if the server requires authentication");
                println!("  • Ensure the token hasn't expired");
            }
            _ => {
                println!("  • Check the server URL format (ws:// or wss://)");
                println!("  • Verify the server is running and accessible");
                println!("  • Check firewall and network settings");
            }
        }

        if url.starts_with("ws://") {
            println!("  • If using a remote server, consider using wss:// for secure connections");
        }

        println!();
    }

    /// Show next steps after successful setup
    fn show_next_steps(&self, ctx: &CommandContext) -> ClientResult<()> {
        println!();
        ctx.interactive.info("Next steps:");
        println!("  1. Test your connection: codebuddy status");
        println!("  2. Start an interactive session: codebuddy connect");
        println!("  3. Call a specific tool: codebuddy call <tool_name> [params]");
        println!();

        ctx.interactive
            .info("For help with any command, use --help flag");
        println!("  Example: codebuddy call --help");
        println!();

        Ok(())
    }

    /// Quick setup mode (non-interactive)
    async fn quick_setup(
        &self,
        ctx: &mut CommandContext,
        url: String,
        token: Option<String>,
    ) -> ClientResult<()> {
        ctx.display_info("Running quick setup...");

        // Validate URL
        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            return Err(ClientError::ConfigError(
                "URL must start with ws:// or wss://".to_string(),
            ));
        }

        // Update configuration
        ctx.update_config(Some(url.clone()), token.clone(), None);

        // Test connection
        ctx.display_info("Testing connection...");
        match ctx.test_connection(Some(url), token).await {
            Ok(ping_time) => {
                ctx.display_success(&format!(
                    "Connected successfully (ping: {})",
                    ctx.formatter.duration(ping_time)
                ));
            }
            Err(e) => {
                ctx.display_warning(&format!("Connection test failed: {}", e));
            }
        }

        // Save configuration
        ctx.save_config().await?;
        ctx.display_success("Configuration saved");

        Ok(())
    }
}

impl Default for SetupCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for SetupCommand {
    async fn execute(&self, global_args: &GlobalArgs) -> ClientResult<()> {
        let mut ctx = CommandContext::new(global_args.clone()).await?;

        // Check for quick setup mode via environment variables
        if let (Ok(url), token) = (
            std::env::var("CODEBUDDY_URL"),
            std::env::var("CODEBUDDY_TOKEN").ok(),
        ) {
            if !ctx.interactive.confirm(
                &format!("Use environment variables for setup? (URL: {})", url),
                Some(true),
            )? {
                return self.run_wizard(&mut ctx).await;
            }

            return self.quick_setup(&mut ctx, url, token).await;
        }

        // Run interactive wizard
        self.run_wizard(&mut ctx).await
    }

    fn name(&self) -> &'static str {
        "setup"
    }

    fn description(&self) -> &'static str {
        "Interactive configuration wizard for the codebuddy client"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_command_creation() {
        let cmd = SetupCommand::new();
        assert_eq!(cmd.name(), "setup");
        assert_eq!(
            cmd.description(),
            "Interactive configuration wizard for the codebuddy client"
        );
    }

    #[test]
    fn test_setup_command_default() {
        let cmd = SetupCommand::default();
        assert_eq!(cmd.name(), "setup");
    }
}
