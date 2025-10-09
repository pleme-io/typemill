use crate::mcp::{McpMessage, McpProtocol};
use anyhow::Result;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

/// A transport for sending and receiving MCP messages over stdio.
pub struct StdioTransport;

impl StdioTransport {
    /// Reads a single MCP message from stdin.
    pub async fn read_message() -> Result<Option<McpMessage>> {
        let mut stdin = BufReader::new(io::stdin());
        let mut buffer = String::new();

        match stdin.read_line(&mut buffer).await {
            Ok(0) => Ok(None), // EOF
            Ok(_) => {
                let msg = McpProtocol::deserialize(&buffer)?;
                Ok(Some(msg))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Writes a single MCP message to stdout.
    pub async fn write_message(message: &McpMessage) -> Result<()> {
        let mut stdout = io::stdout();
        let buffer = McpProtocol::serialize(message)?;
        stdout.write_all(buffer.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
        Ok(())
    }
}