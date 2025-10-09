use crate::output::formatter;
use anyhow::Result;
use clap::{Args, Subcommand};
use jules_api::{types::CreateSessionRequest, JulesClient};

#[derive(Subcommand)]
pub enum SessionsCommand {
    /// Create a new session
    Create(CreateSessionArgs),
    /// List all sessions
    List(ListSessionsArgs),
    /// Get a specific session by ID
    Get(GetSessionArgs),
    /// Delete a session by ID
    Delete(DeleteSessionArgs),
}

#[derive(Args)]
pub struct CreateSessionArgs {
    /// The ID of the source to create a session from
    #[arg(required = true)]
    pub source_id: String,
}

#[derive(Args)]
pub struct ListSessionsArgs {
    #[arg(short, long)]
    page_size: Option<u32>,
    #[arg(short, long)]
    page_token: Option<String>,
}

#[derive(Args)]
pub struct GetSessionArgs {
    /// The ID of the session to retrieve
    #[arg(required = true)]
    pub session_id: String,
}

#[derive(Args)]
pub struct DeleteSessionArgs {
    /// The ID of the session to delete
    #[arg(required = true)]
    pub session_id: String,
}

pub async fn handle_sessions_command(
    command: &SessionsCommand,
    client: &JulesClient,
    format: &str,
) -> Result<()> {
    match command {
        SessionsCommand::Create(args) => {
            let request = CreateSessionRequest {
                source_id: args.source_id.clone(),
            };
            let session = client.create_session(request).await?;
            formatter::print_session(&session, format)?;
        }
        SessionsCommand::List(args) => {
            let response = client
                .list_sessions(args.page_size, args.page_token.as_deref())
                .await?;
            formatter::print_sessions_response(&response, format)?;
        }
        SessionsCommand::Get(args) => {
            let session = client.get_session(&args.session_id).await?;
            formatter::print_session(&session, format)?;
        }
        SessionsCommand::Delete(args) => {
            client.delete_session(&args.session_id).await?;
            println!("Session {} deleted.", args.session_id);
        }
    }
    Ok(())
}