use crate::output::formatter;
use anyhow::Result;
use clap::{Args, Subcommand};
use jules_api::JulesClient;

#[derive(Subcommand)]
pub enum ActivitiesCommand {
    /// List all activities for a session
    List(ListActivitiesArgs),
}

#[derive(Args)]
pub struct ListActivitiesArgs {
    /// The ID of the session to list activities for
    #[arg(required = true)]
    pub session_id: String,
    #[arg(short, long)]
    page_size: Option<u32>,
    #[arg(short, long)]
    page_token: Option<String>,
}

pub async fn handle_activities_command(
    command: &ActivitiesCommand,
    client: &JulesClient,
    format: &str,
) -> Result<()> {
    match command {
        ActivitiesCommand::List(args) => {
            let response = client
                .list_activities(&args.session_id, args.page_size, args.page_token.as_deref())
                .await?;
            formatter::print_activities_response(&response, format)?;
        }
    }
    Ok(())
}