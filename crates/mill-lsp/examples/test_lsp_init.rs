use mill_lsp::lsp_system::client::LspClient;
use mill_config::config::LspServerConfig;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("=== Starting minimal LSP test ===");

    // Use hardcoded path to typescript-language-server
    let ts_lsp = "/home/developer/.nvm/versions/node/v22.20.0/bin/typescript-language-server";

    println!("Using typescript-language-server at: {}", ts_lsp);

    let config = LspServerConfig {
        extensions: vec!["ts".to_string()],
        command: vec![ts_lsp.to_string(), "--stdio".to_string()],
        root_dir: Some("/tmp".into()),
        restart_interval: None,
        initialization_options: None,
    };

    println!("Creating LSP client...");
    println!("This will attempt to initialize the LSP server with 60s timeout");
    println!("Watch for LSP stderr output and initialization messages...\n");

    match tokio::time::timeout(Duration::from_secs(70), LspClient::new(config)).await {
        Ok(Ok(client)) => {
            println!("\n✅ SUCCESS! LSP client initialized");
            drop(client);
        }
        Ok(Err(e)) => {
            println!("\n❌ FAILED: LSP client creation failed: {}", e);
        }
        Err(_) => {
            println!("\n❌ TIMEOUT: LSP client took longer than 70 seconds");
        }
    }
}