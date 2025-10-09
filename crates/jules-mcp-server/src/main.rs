use jules_mcp_server::{Config, JulesMcpServer};
use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;

    let filter = EnvFilter::try_new(&config.log_level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr) // Log to stderr to keep stdout clean for MCP messages
        .json()
        .init();

    let server = JulesMcpServer::new(config)?;
    server.run().await?;
    Ok(())
}