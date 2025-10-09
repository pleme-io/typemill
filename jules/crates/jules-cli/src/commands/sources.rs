use crate::output::formatter;
use anyhow::Result;
use clap::{Args, Subcommand};
use jules_api::JulesClient;

#[derive(Subcommand)]
pub enum SourcesCommand {
    /// List all available sources
    List(ListSourcesArgs),
    /// Get a specific source by ID
    Get(GetSourceArgs),
}

#[derive(Args)]
pub struct ListSourcesArgs {
    #[arg(short, long)]
    page_size: Option<u32>,
    #[arg(short, long)]
    page_token: Option<String>,
}

#[derive(Args)]
pub struct GetSourceArgs {
    /// The ID of the source to retrieve
    #[arg(required = true)]
    pub source_id: String,
}

pub async fn handle_sources_command(
    command: &SourcesCommand,
    client: &JulesClient,
    format: &str,
) -> Result<()> {
    match command {
        SourcesCommand::List(args) => {
            let response = client
                .list_sources(args.page_size, args.page_token.as_deref())
                .await?;
            formatter::print_sources_response(&response, format)?;
        }
        SourcesCommand::Get(args) => {
            let source = client.get_source(&args.source_id).await?;
            formatter::print_source(&source, format)?;
        }
    }
    Ok(())
}