//! Auth CLI commands

use mill_config::config::AppConfig;
use std::process;
use tracing::error;

/// Handle the generate-token command
pub async fn handle_generate_token(
    project_id: Option<String>,
    user_id: Option<String>,
    expiry_seconds: Option<u64>,
) {
    // Load configuration
    let config = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    // Check if authentication is configured
    let auth_config = match config.server.auth {
        Some(c) => c,
        None => {
            eprintln!("❌ Authentication is not configured in .typemill/config.toml");
            eprintln!("   Add [server.auth] section with jwt_secret to enable authentication");
            process::exit(1);
        }
    };

    // Use custom expiry or default from config
    let expiry = expiry_seconds.unwrap_or(auth_config.jwt_expiry_seconds);

    // Generate token
    match mill_auth::generate_token(
        &auth_config.jwt_secret,
        expiry,
        &auth_config.jwt_issuer,
        &auth_config.jwt_audience,
        project_id.clone(),
        user_id.clone(),
    ) {
        Ok(token) => {
            println!("✅ Generated authentication token:");
            println!();
            println!("{}", token);
            println!();
            println!("   Expires in: {} seconds", expiry);
            if let Some(pid) = project_id {
                println!("   Project ID: {}", pid);
            }
            if let Some(uid) = user_id {
                println!("   User ID:    {}", uid);
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to generate token");
            eprintln!("❌ Failed to generate token: {}", e);
            process::exit(1);
        }
    }
}
