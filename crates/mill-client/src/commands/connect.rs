use super::{utils, Command, CommandContext, GlobalArgs};
use crate::error::{ClientError, ClientResult};
use crate::websocket::{ConnectionState, WebSocketClient};
use async_trait::async_trait;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Connect command for persistent WebSocket sessions
pub struct ConnectCommand {
    /// Server URL override
    pub url: Option<String>,
    /// Authentication token override
    pub token: Option<String>,
    /// Auto-reconnect on disconnection
    pub auto_reconnect: bool,
    /// Session timeout (auto-disconnect after inactivity)
    pub session_timeout: Option<Duration>,
}

/// Session statistics
#[derive(Debug)]
struct SessionStats {
    commands_executed: u64,
    successful_calls: u64,
    failed_calls: u64,
    total_response_time: Duration,
    session_start: Instant,
    last_activity: Instant,
}

impl Default for SessionStats {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            commands_executed: 0,
            successful_calls: 0,
            failed_calls: 0,
            total_response_time: Duration::from_secs(0),
            session_start: now,
            last_activity: now,
        }
    }
}

impl SessionStats {
    fn new() -> Self {
        Self::default()
    }

    fn record_call(&mut self, success: bool, response_time: Duration) {
        self.commands_executed += 1;
        self.total_response_time += response_time;
        self.last_activity = Instant::now();

        if success {
            self.successful_calls += 1;
        } else {
            self.failed_calls += 1;
        }
    }

    fn average_response_time(&self) -> Duration {
        if self.commands_executed > 0 {
            self.total_response_time / self.commands_executed as u32
        } else {
            Duration::ZERO
        }
    }

    fn session_duration(&self) -> Duration {
        Instant::now() - self.session_start
    }

    fn idle_time(&self) -> Duration {
        Instant::now() - self.last_activity
    }
}

impl ConnectCommand {
    pub fn new(url: Option<String>, token: Option<String>) -> Self {
        Self {
            url,
            token,
            auto_reconnect: true,
            session_timeout: None,
        }
    }

    pub fn with_auto_reconnect(mut self, auto_reconnect: bool) -> Self {
        self.auto_reconnect = auto_reconnect;
        self
    }

    pub fn with_session_timeout(mut self, timeout: Duration) -> Self {
        self.session_timeout = Some(timeout);
        self
    }

    /// Start an interactive session
    async fn start_session(&self, ctx: &CommandContext) -> ClientResult<()> {
        ctx.interactive.banner(
            "🔌 Interactive Session",
            Some("Connected to mill server. Type 'help' for commands."),
        )?;

        let mut stats = SessionStats::new();
        let mut command_history = Vec::new();
        let mut session_active = true;

        // Connect to server
        let client = ctx
            .connect_client(self.url.clone(), self.token.clone())
            .await?;

        // Show connection info
        self.show_connection_info(ctx, &client).await?;

        // Main session loop
        while session_active {
            // Check session timeout
            if let Some(timeout) = self.session_timeout {
                if stats.idle_time() > timeout {
                    ctx.display_warning("Session timed out due to inactivity");
                    break;
                }
            }

            // Check connection state
            let state = client.get_state().await;
            if !matches!(
                state,
                ConnectionState::Connected | ConnectionState::Authenticated
            ) {
                if self.auto_reconnect {
                    ctx.display_warning("Connection lost, attempting to reconnect...");
                    if let Err(e) = client.connect().await {
                        ctx.display_error(&ClientError::ConnectionError(format!(
                            "Reconnection failed: {}",
                            e
                        )));
                        if !ctx
                            .interactive
                            .confirm("Continue trying to reconnect?", Some(true))?
                        {
                            break;
                        }
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                    ctx.display_success("Reconnected successfully");
                } else {
                    ctx.display_error(&ClientError::ConnectionError("Connection lost".to_string()));
                    break;
                }
            }

            // Get user input
            print!("codeflow> ");
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                break;
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            // Handle built-in commands
            match self
                .handle_builtin_command(
                    ctx,
                    &client,
                    input,
                    &mut stats,
                    &mut command_history,
                    &mut session_active,
                )
                .await
            {
                Ok(true) => continue, // Built-in command handled
                Ok(false) => {}       // Not a built-in command, continue to tool call
                Err(e) => {
                    ctx.display_error(&e);
                    continue;
                }
            }

            // Parse and execute tool call
            if let Err(e) = self
                .execute_session_command(ctx, &client, input, &mut stats)
                .await
            {
                ctx.display_error(&e);
            }

            command_history.push(input.to_string());
        }

        // Disconnect and show session summary
        let _ = client.disconnect().await;
        self.show_session_summary(ctx, &stats)?;

        Ok(())
    }

    /// Handle built-in session commands
    async fn handle_builtin_command(
        &self,
        ctx: &CommandContext,
        client: &WebSocketClient,
        input: &str,
        stats: &mut SessionStats,
        #[allow(clippy::ptr_arg)] history: &mut Vec<String>,
        session_active: &mut bool,
    ) -> ClientResult<bool> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }

        match parts[0] {
            "help" | "?" => {
                self.show_session_help(ctx)?;
                Ok(true)
            }
            "quit" | "exit" | "q" => {
                ctx.display_info("Ending session...");
                *session_active = false;
                Ok(true)
            }
            "status" => {
                self.show_session_status(ctx, client, stats).await?;
                Ok(true)
            }
            "ping" => {
                let start = Instant::now();
                match client.ping().await {
                    Ok(ping_time) => {
                        let total_time = start.elapsed();
                        ctx.display_success(&format!(
                            "Pong! Server: {}, Total: {}",
                            ctx.formatter.duration(ping_time),
                            ctx.formatter.duration(total_time)
                        ));
                        stats.record_call(true, total_time);
                    }
                    Err(e) => {
                        ctx.display_error(&e);
                        stats.record_call(false, start.elapsed());
                    }
                }
                Ok(true)
            }
            "capabilities" => {
                let start = Instant::now();
                match client.get_capabilities().await {
                    Ok(capabilities) => {
                        println!();
                        ctx.display_info("Server capabilities:");
                        println!("{}", utils::format_capabilities(&capabilities));
                        stats.record_call(true, start.elapsed());
                    }
                    Err(e) => {
                        ctx.display_error(&e);
                        stats.record_call(false, start.elapsed());
                    }
                }
                Ok(true)
            }
            "history" => {
                self.show_command_history(ctx, history)?;
                Ok(true)
            }
            "clear" => {
                ctx.interactive.clear_screen()?;
                Ok(true)
            }
            "stats" => {
                self.show_detailed_stats(ctx, stats)?;
                Ok(true)
            }
            _ => Ok(false), // Not a built-in command
        }
    }

    /// Execute a tool call command in the session
    async fn execute_session_command(
        &self,
        ctx: &CommandContext,
        client: &WebSocketClient,
        input: &str,
        stats: &mut SessionStats,
    ) -> ClientResult<()> {
        let start = Instant::now();

        // Parse the command - expect "tool_name [json_params]"
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let tool_name = parts[0];
        let params_str = parts.get(1).copied();

        // Validate tool name
        utils::validate_tool_name(tool_name)?;

        // Parse parameters
        let params = utils::parse_json_params(params_str)?;

        // Execute the tool call
        match client.call_tool(tool_name, params).await {
            Ok(response) => {
                let elapsed = start.elapsed();
                stats.record_call(utils::is_success_response(&response), elapsed);

                println!();
                println!("{}", ctx.formatter.mcp_response(&response)?);

                if utils::is_success_response(&response) {
                    println!();
                    ctx.display_info(&format!("Completed in {}", ctx.formatter.duration(elapsed)));
                }
            }
            Err(e) => {
                stats.record_call(false, start.elapsed());
                return Err(e);
            }
        }

        Ok(())
    }

    /// Show connection information
    async fn show_connection_info(
        &self,
        ctx: &CommandContext,
        client: &WebSocketClient,
    ) -> ClientResult<()> {
        let state = client.get_state().await;
        let connection_status = utils::format_connection_status(
            matches!(
                state,
                ConnectionState::Connected | ConnectionState::Authenticated
            ),
            matches!(state, ConnectionState::Authenticated),
        );

        println!();
        ctx.display_info("Session Information:");
        println!(
            "  {}",
            ctx.formatter.key_value("Status", &connection_status)
        );

        if let Some(url) = self.url.as_ref().or(ctx.config.url.as_ref()) {
            println!(
                "  {}",
                ctx.formatter.key_value("Server", &ctx.formatter.url(url))
            );
        }

        println!(
            "  {}",
            ctx.formatter
                .key_value("Auto-reconnect", &self.auto_reconnect.to_string())
        );

        if let Some(timeout) = self.session_timeout {
            println!(
                "  {}",
                ctx.formatter
                    .key_value("Session timeout", &format!("{}s", timeout.as_secs()))
            );
        }

        println!();

        // Test basic connectivity
        if let Ok(ping_time) = client.ping().await {
            ctx.display_success(&format!(
                "Server responding (ping: {})",
                ctx.formatter.duration(ping_time)
            ));
        }

        println!();
        Ok(())
    }

    /// Show session help
    fn show_session_help(&self, ctx: &CommandContext) -> ClientResult<()> {
        println!();
        ctx.formatter.header("📖 Session Commands");
        println!();

        let commands = vec![
            ("help, ?", "Show this help message"),
            ("quit, exit, q", "End the session"),
            ("status", "Show connection and session status"),
            ("ping", "Test server connectivity"),
            ("capabilities", "Show server capabilities"),
            ("history", "Show command history"),
            ("clear", "Clear the screen"),
            ("stats", "Show detailed session statistics"),
            ("", ""),
            ("Tool Calls:", ""),
            ("<tool_name>", "Call a tool without parameters"),
            ("<tool_name> <json>", "Call a tool with JSON parameters"),
        ];

        let headers = vec!["Command", "Description"];
        let rows: Vec<Vec<String>> = commands
            .into_iter()
            .map(|(cmd, desc)| vec![cmd.to_string(), desc.to_string()])
            .collect();

        println!("{}", ctx.formatter.table(&headers, &rows));

        println!();
        ctx.display_info("Examples:");
        println!("  ping");
        println!("  inspect_code {{\"filePath\": \"src/main.rs\", \"line\": 10, \"character\": 5}}");
        println!("  search_code {{\"query\": \"MyFunction\", \"limit\": 10}}");
        println!();

        Ok(())
    }

    /// Show current session status
    async fn show_session_status(
        &self,
        ctx: &CommandContext,
        client: &WebSocketClient,
        stats: &SessionStats,
    ) -> ClientResult<()> {
        println!();
        ctx.formatter.header("📊 Session Status");
        println!();

        let state = client.get_state().await;
        let is_connected = matches!(
            state,
            ConnectionState::Connected | ConnectionState::Authenticated
        );
        let is_authenticated = matches!(state, ConnectionState::Authenticated);

        let status_items = vec![
            (
                "Connection".to_string(),
                utils::format_connection_status(is_connected, is_authenticated),
                is_connected,
            ),
            (
                "Commands executed".to_string(),
                stats.commands_executed.to_string(),
                true,
            ),
            (
                "Success rate".to_string(),
                if stats.commands_executed > 0 {
                    format!(
                        "{:.1}%",
                        (stats.successful_calls as f64 / stats.commands_executed as f64) * 100.0
                    )
                } else {
                    "N/A".to_string()
                },
                stats.failed_calls == 0,
            ),
            (
                "Session duration".to_string(),
                ctx.formatter.duration(stats.session_duration()),
                true,
            ),
            (
                "Idle time".to_string(),
                ctx.formatter.duration(stats.idle_time()),
                true,
            ),
        ];

        println!("{}", ctx.formatter.status_summary(&status_items));

        if let Ok(ping_time) = client.ping().await {
            println!("Current ping: {}", ctx.formatter.duration(ping_time));
        }

        Ok(())
    }

    /// Show command history
    fn show_command_history(&self, ctx: &CommandContext, history: &[String]) -> ClientResult<()> {
        if history.is_empty() {
            ctx.display_info("No commands in history");
            return Ok(());
        }

        println!();
        ctx.formatter.header("📝 Command History");
        println!();

        for (i, cmd) in history.iter().enumerate() {
            println!("  {}: {}", i + 1, cmd);
        }

        println!();
        ctx.display_info(&format!("Total commands: {}", history.len()));
        Ok(())
    }

    /// Show detailed session statistics
    fn show_detailed_stats(&self, ctx: &CommandContext, stats: &SessionStats) -> ClientResult<()> {
        println!();
        ctx.formatter.header("📈 Detailed Statistics");
        println!();

        println!(
            "  {}",
            ctx.formatter.key_value(
                "Session started",
                &format!("{:?} ago", stats.session_duration())
            )
        );
        println!(
            "  {}",
            ctx.formatter
                .key_value("Last activity", &format!("{:?} ago", stats.idle_time()))
        );
        println!(
            "  {}",
            ctx.formatter
                .key_value("Total commands", &stats.commands_executed.to_string())
        );
        println!(
            "  {}",
            ctx.formatter
                .key_value("Successful calls", &stats.successful_calls.to_string())
        );
        println!(
            "  {}",
            ctx.formatter
                .key_value("Failed calls", &stats.failed_calls.to_string())
        );

        if stats.commands_executed > 0 {
            let success_rate =
                (stats.successful_calls as f64 / stats.commands_executed as f64) * 100.0;
            println!(
                "  {}",
                ctx.formatter
                    .key_value("Success rate", &format!("{:.1}%", success_rate))
            );
            println!(
                "  {}",
                ctx.formatter.key_value(
                    "Average response time",
                    &ctx.formatter.duration(stats.average_response_time())
                )
            );
            println!(
                "  {}",
                ctx.formatter.key_value(
                    "Total response time",
                    &ctx.formatter.duration(stats.total_response_time)
                )
            );
        }

        println!();
        Ok(())
    }

    /// Show session summary on exit
    fn show_session_summary(&self, ctx: &CommandContext, stats: &SessionStats) -> ClientResult<()> {
        println!();
        ctx.formatter.header("📋 Session Summary");
        println!();

        println!(
            "  {}",
            ctx.formatter.key_value(
                "Duration",
                &ctx.formatter.duration(stats.session_duration())
            )
        );
        println!(
            "  {}",
            ctx.formatter
                .key_value("Commands executed", &stats.commands_executed.to_string())
        );

        if stats.commands_executed > 0 {
            let success_rate =
                (stats.successful_calls as f64 / stats.commands_executed as f64) * 100.0;
            println!(
                "  {}",
                ctx.formatter
                    .key_value("Success rate", &format!("{:.1}%", success_rate))
            );
            println!(
                "  {}",
                ctx.formatter.key_value(
                    "Average response time",
                    &ctx.formatter.duration(stats.average_response_time())
                )
            );
        }

        println!();

        if stats.commands_executed > 0 {
            ctx.display_success("Thank you for using mill!");
        } else {
            ctx.display_info("No commands were executed in this session");
        }

        Ok(())
    }
}

impl Default for ConnectCommand {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[async_trait]
impl Command for ConnectCommand {
    async fn execute(&self, global_args: &GlobalArgs) -> ClientResult<()> {
        let ctx = CommandContext::new(global_args.clone()).await?;

        // Check configuration
        if !ctx.is_configured() && self.url.is_none() {
            ctx.display_error(&ClientError::ConfigError(
                "No server URL configured. Run 'mill setup' or provide --url".to_string(),
            ));

            if ctx
                .interactive
                .confirm("Would you like to run setup now?", Some(true))?
            {
                let setup_cmd = super::setup::SetupCommand::new();
                setup_cmd.execute(global_args).await?;
            }

            return Ok(());
        }

        self.start_session(&ctx).await
    }

    fn name(&self) -> &'static str {
        "connect"
    }

    fn description(&self) -> &'static str {
        "Start an interactive session with the mill server"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_command_creation() {
        let cmd = ConnectCommand::new(None, None);
        assert_eq!(cmd.name(), "connect");
        assert_eq!(
            cmd.description(),
            "Start an interactive session with the mill server"
        );
        assert!(cmd.auto_reconnect);
        assert!(cmd.session_timeout.is_none());
    }

    #[test]
    fn test_connect_command_with_options() {
        let cmd = ConnectCommand::new(
            Some("ws://localhost:3000".to_string()),
            Some("test-token".to_string()),
        )
        .with_auto_reconnect(false)
        .with_session_timeout(Duration::from_secs(3600));

        assert_eq!(cmd.url, Some("ws://localhost:3000".to_string()));
        assert_eq!(cmd.token, Some("test-token".to_string()));
        assert!(!cmd.auto_reconnect);
        assert_eq!(cmd.session_timeout, Some(Duration::from_secs(3600)));
    }

    #[test]
    fn test_session_stats() {
        let mut stats = SessionStats::new();
        assert_eq!(stats.commands_executed, 0);
        assert_eq!(stats.successful_calls, 0);
        assert_eq!(stats.failed_calls, 0);

        stats.record_call(true, Duration::from_millis(100));
        assert_eq!(stats.commands_executed, 1);
        assert_eq!(stats.successful_calls, 1);
        assert_eq!(stats.failed_calls, 0);

        stats.record_call(false, Duration::from_millis(200));
        assert_eq!(stats.commands_executed, 2);
        assert_eq!(stats.successful_calls, 1);
        assert_eq!(stats.failed_calls, 1);

        assert_eq!(stats.average_response_time(), Duration::from_millis(150));
    }
}
