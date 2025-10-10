//! Binary entry point for the out-of-process Rust language plugin.
//!
//! This binary wraps the `RustPlugin` implementation in a `PluginServer`,
//! allowing it to run as a standalone JSON-RPC server that communicates
//! over stdio.

// This allows the binary to use the library features of its own crate.
use cb_lang_rust::RustPlugin;
use cb_plugin_api::PluginServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // For now, we don't initialize logging here to avoid dependencies.
    // The core application will capture and log stderr from this process.

    // Instantiate the plugin implementation from the library part of this crate.
    let plugin = RustPlugin::default();

    // Create a new server that will handle the JSON-RPC protocol.
    let server = PluginServer::new(plugin);

    // Run the server's main event loop. This will read from stdin,
    // process requests, and write responses to stdout until the process
    // is terminated.
    server.run().await?;

    Ok(())
}