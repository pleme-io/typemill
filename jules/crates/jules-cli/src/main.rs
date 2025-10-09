use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod output;

/// A CLI for interacting with the Jules API.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, global = true, default_value = "table")]
    format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage sources
    Sources {
        #[command(subcommand)]
        command: commands::sources::SourcesCommand,
    },
    /// Manage sessions
    Sessions {
        #[command(subcommand)]
        command: commands::sessions::SessionsCommand,
    },
    /// Manage activities
    Activities {
        #[command(subcommand)]
        command: commands::activities::ActivitiesCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // The API key should be loaded from a config file or environment variable.
    // For simplicity, we'll expect it to be in the environment.
    let api_key =
        std::env::var("JULES_API_KEY").expect("JULES_API_KEY must be set");

    let config = jules_api::Config::new(api_key);
    let client = jules_api::JulesClient::new(config);

    match &cli.command {
        Commands::Sources { command } => {
            commands::sources::handle_sources_command(command, &client, &cli.format).await?
        }
        Commands::Sessions { command } => {
            commands::sessions::handle_sessions_command(command, &client, &cli.format).await?
        }
        Commands::Activities { command } => {
            commands::activities::handle_activities_command(command, &client, &cli.format).await?
        }
    }

    Ok(())
}