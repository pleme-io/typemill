use super::{utils, Command, CommandContext, GlobalArgs};
use crate::error::{ClientError, ClientResult};
use crate::websocket::MCPResponse;
use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::io::{self, Read};

/// Call command for direct MCP tool invocation
pub struct CallCommand {
    /// Tool name to call
    pub tool: String,
    /// JSON parameters (optional)
    pub params: Option<String>,
    /// Server URL override
    pub url: Option<String>,
    /// Authentication token override
    pub token: Option<String>,
    /// Output format (json, pretty, raw)
    pub format: OutputFormat,
    /// Read parameters from file
    pub params_file: Option<String>,
    /// Read parameters from stdin
    pub params_stdin: bool,
}

/// Output format options
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum OutputFormat {
    /// Pretty-printed with colors and formatting
    #[default]
    Pretty,
    /// Raw JSON output
    Json,
    /// Minimal output (result only)
    Raw,
}


impl CallCommand {
    pub fn new(tool: String, params: Option<String>) -> Self {
        Self {
            tool,
            params,
            url: None,
            token: None,
            format: OutputFormat::default(),
            params_file: None,
            params_stdin: false,
        }
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_params_file(mut self, file: String) -> Self {
        self.params_file = Some(file);
        self
    }

    pub fn with_params_stdin(mut self) -> Self {
        self.params_stdin = true;
        self
    }

    /// Execute the tool call
    async fn execute_tool_call(&self, ctx: &CommandContext) -> ClientResult<()> {
        // Validate tool name
        utils::validate_tool_name(&self.tool)?;

        // Get parameters from various sources
        let params = self.resolve_parameters(ctx).await?;

        if ctx.global_args.debug {
            ctx.display_info(&format!("Calling tool '{}' with parameters:", self.tool));
            if let Some(ref p) = params {
                println!("{}", ctx.formatter.json(p)?);
            } else {
                println!("  (no parameters)");
            }
            println!();
        }

        // Connect to server
        let client = ctx
            .connect_client(self.url.clone(), self.token.clone())
            .await?;

        // Execute the tool call
        ctx.display_info(&format!("Calling tool '{}'...", self.tool));

        let response = match client.call_tool(&self.tool, params).await {
            Ok(response) => {
                ctx.display_success("Tool call completed");
                response
            }
            Err(e) => {
                ctx.display_error(&e);
                return Err(e);
            }
        };

        // Disconnect
        let _ = client.disconnect().await;

        // Format and display output
        self.display_response(ctx, &response)?;

        Ok(())
    }

    /// Resolve parameters from various sources
    async fn resolve_parameters(&self, ctx: &CommandContext) -> ClientResult<Option<Value>> {
        // Priority order: stdin > file > direct params
        if self.params_stdin {
            self.read_params_from_stdin(ctx).await
        } else if let Some(ref file) = self.params_file {
            self.read_params_from_file(ctx, file).await
        } else {
            utils::parse_json_params(self.params.as_deref())
        }
    }

    /// Read parameters from stdin
    async fn read_params_from_stdin(&self, ctx: &CommandContext) -> ClientResult<Option<Value>> {
        ctx.display_info("Reading parameters from stdin...");

        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| ClientError::IoError(format!("Failed to read from stdin: {}", e)))?;

        if buffer.trim().is_empty() {
            Ok(None)
        } else {
            utils::parse_json_params(Some(&buffer))
        }
    }

    /// Read parameters from file
    async fn read_params_from_file(
        &self,
        ctx: &CommandContext,
        file: &str,
    ) -> ClientResult<Option<Value>> {
        ctx.display_info(&format!(
            "Reading parameters from file: {}",
            ctx.formatter.path(file)
        ));

        let content = fs::read_to_string(file)
            .map_err(|e| ClientError::IoError(format!("Failed to read file '{}': {}", file, e)))?;

        if content.trim().is_empty() {
            Ok(None)
        } else {
            utils::parse_json_params(Some(&content))
        }
    }

    /// Display the response in the specified format
    fn display_response(&self, ctx: &CommandContext, response: &MCPResponse) -> ClientResult<()> {
        match self.format {
            OutputFormat::Pretty => {
                println!();
                println!("{}", ctx.formatter.mcp_response(response)?);
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(response).map_err(|e| {
                    ClientError::SerializationError(format!("Failed to serialize response: {}", e))
                })?;
                println!("{}", json);
            }
            OutputFormat::Raw => {
                if let Some(ref error) = response.error {
                    eprintln!("Error {}: {}", error.code, error.message);
                    std::process::exit(1);
                } else if let Some(ref result) = response.result {
                    let output = serde_json::to_string(result).map_err(|e| {
                        ClientError::SerializationError(format!(
                            "Failed to serialize result: {}",
                            e
                        ))
                    })?;
                    println!("{}", output);
                } else {
                    println!("null");
                }
            }
        }

        Ok(())
    }

    /// Show examples of common tool calls
    fn show_examples(&self, ctx: &CommandContext) -> ClientResult<()> {
        ctx.formatter.header("🔧 Common Tool Call Examples");
        println!();

        let examples = vec![
            (
                "Find definition of a symbol",
                "find_definition",
                r#"{"file_path": "src/main.rs", "symbol_name": "main"}"#,
            ),
            (
                "Find references to a symbol",
                "find_references",
                r#"{"file_path": "src/lib.rs", "symbol_name": "MyFunction"}"#,
            ),
            (
                "Get document symbols",
                "get_document_symbols",
                r#"{"file_path": "src/types.rs"}"#,
            ),
            (
                "Format a document",
                "format_document",
                r#"{"file_path": "src/main.rs"}"#,
            ),
            (
                "Get diagnostics",
                "get_diagnostics",
                r#"{"file_path": "src/lib.rs"}"#,
            ),
            (
                "Rename a symbol",
                "rename_symbol",
                r#"{"file_path": "src/main.rs", "symbol_name": "old_name", "new_name": "new_name"}"#,
            ),
        ];

        for (description, tool, params) in examples {
            println!("{}", ctx.formatter.info(description));
            println!("  codebuddy call {} '{}'", tool, params);
            println!();
        }

        ctx.display_info("Tips:");
        println!("  • Use --format json for machine-readable output");
        println!("  • Use --format raw for result-only output");
        println!("  • Read params from file: --params-file params.json");
        println!("  • Read params from stdin: --params-stdin");
        println!("  • Override server: --url ws://localhost:3000");
        println!();

        Ok(())
    }

    /// Interactive parameter builder
    async fn interactive_params(&self, ctx: &CommandContext) -> ClientResult<Option<Value>> {
        ctx.interactive.banner(
            &format!("🔧 Interactive Parameter Builder for '{}'", self.tool),
            Some("Let's build the parameters step by step"),
        )?;

        // Tool-specific parameter helpers
        match self.tool.as_str() {
            "find_definition" | "find_references" => self.build_symbol_params(ctx).await,
            "get_document_symbols" | "format_document" | "get_diagnostics" => {
                self.build_file_params(ctx).await
            }
            "rename_symbol" => self.build_rename_params(ctx).await,
            _ => self.build_generic_params(ctx).await,
        }
    }

    /// Build parameters for symbol-based tools
    async fn build_symbol_params(&self, ctx: &CommandContext) -> ClientResult<Option<Value>> {
        let file_path = ctx.interactive.required_input("File path", None)?;
        let symbol_name = ctx.interactive.required_input("Symbol name", None)?;

        let params = serde_json::json!({
            "file_path": file_path,
            "symbol_name": symbol_name
        });

        Ok(Some(params))
    }

    /// Build parameters for file-based tools
    async fn build_file_params(&self, ctx: &CommandContext) -> ClientResult<Option<Value>> {
        let file_path = ctx.interactive.required_input("File path", None)?;

        let params = serde_json::json!({
            "file_path": file_path
        });

        Ok(Some(params))
    }

    /// Build parameters for rename tool
    async fn build_rename_params(&self, ctx: &CommandContext) -> ClientResult<Option<Value>> {
        let file_path = ctx.interactive.required_input("File path", None)?;
        let symbol_name = ctx
            .interactive
            .required_input("Current symbol name", None)?;
        let new_name = ctx.interactive.required_input("New symbol name", None)?;

        let params = serde_json::json!({
            "file_path": file_path,
            "symbol_name": symbol_name,
            "new_name": new_name
        });

        Ok(Some(params))
    }

    /// Build parameters for generic tools
    async fn build_generic_params(&self, ctx: &CommandContext) -> ClientResult<Option<Value>> {
        ctx.interactive
            .warning_message("Generic parameter builder - manual JSON input required");

        let json_input = ctx.interactive.optional_input(
            "Parameters (JSON format, leave empty for no parameters)",
            None,
        )?;

        if let Some(input) = json_input {
            utils::parse_json_params(Some(&input))
        } else {
            Ok(None)
        }
    }
}

impl Default for CallCommand {
    fn default() -> Self {
        Self::new("ping".to_string(), None)
    }
}

#[async_trait]
impl Command for CallCommand {
    async fn execute(&self, global_args: &GlobalArgs) -> ClientResult<()> {
        let ctx = CommandContext::new(global_args.clone()).await?;

        // Special case: if tool is "examples", show examples
        if self.tool == "examples" {
            return self.show_examples(&ctx);
        }

        // Special case: if tool is "interactive", run interactive parameter builder
        if self.tool == "interactive" {
            ctx.display_warning("Interactive mode requires a specific tool name");
            return Ok(());
        }

        // Check if we need to run interactive parameter builder
        if self.params.is_none() && self.params_file.is_none() && !self.params_stdin
            && ctx.interactive.confirm(
                "No parameters provided. Would you like to use the interactive parameter builder?",
                Some(false),
            )? {
                let params = self.interactive_params(&ctx).await?;
                if let Some(p) = params {
                    println!();
                    ctx.display_info("Generated parameters:");
                    println!("{}", ctx.formatter.json(&p)?);
                    println!();

                    if !ctx
                        .interactive
                        .confirm("Proceed with these parameters?", Some(true))?
                    {
                        ctx.display_info("Tool call cancelled");
                        return Ok(());
                    }

                    // Create a new command with the built parameters
                    let mut new_cmd = self.clone();
                    new_cmd.params = Some(serde_json::to_string(&p).unwrap());
                    return new_cmd.execute_tool_call(&ctx).await;
                }
            }

        self.execute_tool_call(&ctx).await
    }

    fn name(&self) -> &'static str {
        "call"
    }

    fn description(&self) -> &'static str {
        "Call a specific MCP tool with optional parameters"
    }
}

impl Clone for CallCommand {
    fn clone(&self) -> Self {
        Self {
            tool: self.tool.clone(),
            params: self.params.clone(),
            url: self.url.clone(),
            token: self.token.clone(),
            format: self.format.clone(),
            params_file: self.params_file.clone(),
            params_stdin: self.params_stdin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_command_creation() {
        let cmd = CallCommand::new("find_definition".to_string(), None);
        assert_eq!(cmd.tool, "find_definition");
        assert!(cmd.params.is_none());
        assert_eq!(cmd.format, OutputFormat::Pretty);
    }

    #[test]
    fn test_call_command_with_params() {
        let params = r#"{"file_path": "src/main.rs"}"#.to_string();
        let cmd = CallCommand::new("test_tool".to_string(), Some(params.clone()))
            .with_url("ws://localhost:3000".to_string())
            .with_token("test-token".to_string())
            .with_format(OutputFormat::Json);

        assert_eq!(cmd.tool, "test_tool");
        assert_eq!(cmd.params, Some(params));
        assert_eq!(cmd.url, Some("ws://localhost:3000".to_string()));
        assert_eq!(cmd.token, Some("test-token".to_string()));
        assert_eq!(cmd.format, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_default() {
        assert_eq!(OutputFormat::default(), OutputFormat::Pretty);
    }

    #[test]
    fn test_call_command_clone() {
        let cmd = CallCommand::new("test".to_string(), Some("{}".to_string()));
        let cloned = cmd.clone();

        assert_eq!(cmd.tool, cloned.tool);
        assert_eq!(cmd.params, cloned.params);
    }
}
