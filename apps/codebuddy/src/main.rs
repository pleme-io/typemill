mod cli;
mod dispatcher_factory;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use mill_server::handlers::plugin_dispatcher::PluginDispatcher;
use mill_server::workspaces::WorkspaceManager;
use mill_transport::SessionInfo;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

fn warn_if_fuse_enabled() {
    if let Ok(config) = codebuddy_config::config::AppConfig::load() {
        if config.fuse.is_some() {
            eprintln!("⚠️  FUSE configured - requires SYS_ADMIN capability");
            eprintln!("   Only use in trusted development environments");
            eprintln!("   Set \"fuse\": null in config for production");
        }
    }
}

#[tokio::main]
async fn main() {
    warn_if_fuse_enabled();
    cli::run().await;
}

/// Runs the application in stdio mode.
///
/// This mode is used when the application is run from the command line. It
/// reads messages from stdin and writes responses to stdout.
pub async fn run_stdio_mode() {
    debug!("Initializing stdio mode MCP server");
    debug!(
        "Current working directory in run_stdio_mode: {:?}",
        std::env::current_dir()
    );

    // Initialize dispatcher via factory
    let dispatcher = match dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            error!(error = %e, "Failed to initialize dispatcher");
            return;
        }
    };
    let session_info = SessionInfo::default();
    debug!("Plugin dispatcher initialized successfully");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin);

    debug!("Starting stdio message loop");
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                debug!("EOF received, exiting");
                break; // EOF
            }
            Ok(_) => {
                debug!(message = %line.trim(), "Received message");
                match serde_json::from_str(&line) {
                    Ok(mcp_message) => {
                        debug!("Parsed MCP message, dispatching");
                        match dispatcher.dispatch(mcp_message, &session_info).await {
                            Ok(response) => {
                                let response_json = match serde_json::to_string(&response) {
                                    Ok(json) => json,
                                    Err(e) => {
                                        error!(error = %e, "Failed to serialize response");
                                        continue;
                                    }
                                };
                                debug!(response = %response_json, "Sending response");
                                if let Err(e) = stdout.write_all(response_json.as_bytes()).await {
                                    error!(error = %e, "Error writing to stdout");
                                    break;
                                }
                                if let Err(e) = stdout.write_all(b"\n").await {
                                    error!(error = %e, "Error writing newline");
                                    break;
                                }
                                if let Err(e) = stdout.flush().await {
                                    error!(error = %e, "Error flushing stdout");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!(error = %e, "Error dispatching message");
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to parse JSON");
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Error reading from stdin");
                break;
            }
        }
    }
    debug!("Stdio mode exiting");
}

/// Runs the application in websocket mode.
///
/// This mode is used when the application is run as a server. It listens for
/// websocket connections on port 3000 and handles messages from clients.
pub async fn run_websocket_server() {
    run_websocket_server_with_port(3000).await;
}

/// Runs the application in websocket mode on a specific port.
///
/// This mode is used when the application is run as a server. It listens for
/// websocket connections on the specified port and handles messages from
/// clients.
///
/// # Arguments
///
/// * `port` - The port to listen on.
pub async fn run_websocket_server_with_port(port: u16) {
    // Load configuration
    let config = match codebuddy_config::config::AppConfig::load() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!(error = %e, "Failed to load configuration");
            return;
        }
    };

    // Create workspace manager
    let workspace_manager = Arc::new(WorkspaceManager::new());

    // Initialize dispatcher via factory
    let dispatcher = match dispatcher_factory::create_initialized_dispatcher_with_workspace(
        workspace_manager.clone(),
    )
    .await
    {
        Ok(d) => d,
        Err(e) => {
            error!(error = %e, "Failed to initialize dispatcher");
            return;
        }
    };

    // Start admin server on a separate port
    let admin_port = port + 1000; // Admin on port+1000
    let admin_config = config.clone();
    let admin_workspace_manager = workspace_manager.clone();
    tokio::spawn(async move {
        if let Err(e) =
            mill_transport::start_admin_server(admin_port, admin_config, admin_workspace_manager)
                .await
        {
            error!(
                error_category = "admin_server_error",
                error = %e,
                "Admin server failed"
            );
        }
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(dispatcher);

    let bind_addr = format!("127.0.0.1:{}", port);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!(bind_addr = %bind_addr, error = %e, "Failed to bind to address");
            return;
        }
    };

    let addr = match listener.local_addr() {
        Ok(addr) => addr,
        Err(e) => {
            error!(error = %e, "Failed to get local address");
            return;
        }
    };
    info!(addr = %addr, "Server listening");

    if let Err(e) = axum::serve(listener, app).await {
        error!(error = %e, "Server error");
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(dispatcher): State<Arc<PluginDispatcher>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, dispatcher))
}

async fn handle_socket(mut socket: WebSocket, dispatcher: Arc<PluginDispatcher>) {
    let session_info = SessionInfo::default();
    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => {
                let response = match serde_json::from_str(&text) {
                    Ok(mcp_message) => dispatcher.dispatch(mcp_message, &session_info).await,
                    Err(e) => {
                        // Handle deserialization error
                        error!(error = %e, "Failed to deserialize message");
                        continue;
                    }
                };

                match response {
                    Ok(response_message) => {
                        let response_text = match serde_json::to_string(&response_message) {
                            Ok(text) => text,
                            Err(e) => {
                                error!(error = %e, "Failed to serialize response");
                                continue;
                            }
                        };
                        if socket
                            .send(Message::Text(response_text.into()))
                            .await
                            .is_err()
                        {
                            break; // client disconnected
                        }
                    }
                    Err(e) => {
                        // Handle dispatch error
                        error!(error = %e, "Error dispatching message");
                    }
                }
            }
            Some(Ok(Message::Close(_))) | None => {
                break; // client disconnected
            }
            Some(Ok(_)) => {
                // Ignore other message types (binary, ping, pong)
            }
            Some(Err(e)) => {
                error!(error = %e, "WebSocket error");
                break;
            }
        }
    }
}