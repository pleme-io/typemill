mod cli;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use cb_api::AstService;
use cb_ast::AstCache;
use cb_server::handlers::plugin_dispatcher::{AppState, PluginDispatcher};
use cb_server::services::DefaultAstService;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

#[tokio::main]
async fn main() {
    cli::run().await;
}

pub async fn run_stdio_mode() {
    debug!("Initializing stdio mode MCP server");
    debug!(
        "Current working directory in run_stdio_mode: {:?}",
        std::env::current_dir()
    );

    // Create AppState similar to the test implementation
    let app_state = match create_app_state().await {
        Ok(state) => state,
        Err(e) => {
            error!(error = %e, "Failed to create app state");
            return;
        }
    };

    let dispatcher = Arc::new(PluginDispatcher::new(app_state));
    debug!("About to call dispatcher.initialize()");
    if let Err(e) = dispatcher.initialize().await {
        error!(error = %e, "Failed to initialize dispatcher");
        return;
    }
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
                        match dispatcher.dispatch(mcp_message).await {
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

pub async fn run_websocket_server() {
    run_websocket_server_with_port(3000).await;
}

pub async fn run_websocket_server_with_port(port: u16) {
    // Create AppState similar to the test implementation
    let app_state = match create_app_state().await {
        Ok(state) => state,
        Err(e) => {
            error!(error = %e, "Failed to create app state");
            return;
        }
    };

    let dispatcher = Arc::new(PluginDispatcher::new(app_state));
    if let Err(e) = dispatcher.initialize().await {
        error!(error = %e, "Failed to initialize dispatcher");
        return;
    }

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

async fn create_app_state() -> Result<Arc<AppState>, std::io::Error> {
    // Use current working directory as project root for production
    let project_root = std::env::current_dir()?;
    debug!(project_root = %project_root.display(), "Server project root set");

    // Create shared AST cache for performance optimization
    let ast_cache = Arc::new(AstCache::new());
    debug!("Created shared AST cache");

    let ast_service: Arc<dyn AstService> = Arc::new(DefaultAstService::new(ast_cache.clone()));
    let lock_manager = Arc::new(cb_server::services::LockManager::new());
    let file_service = Arc::new(cb_server::services::FileService::new(
        project_root.clone(),
        ast_cache.clone(),
        lock_manager.clone(),
    ));
    let operation_queue = Arc::new(cb_server::services::OperationQueue::new(
        lock_manager.clone(),
    ));
    let planner = cb_server::services::planner::DefaultPlanner::new();
    let plugin_manager = Arc::new(cb_plugins::PluginManager::new());
    let workflow_executor = cb_server::services::workflow_executor::DefaultWorkflowExecutor::new(
        plugin_manager.clone(),
    );

    Ok(Arc::new(AppState {
        ast_service,
        file_service,
        planner,
        workflow_executor,
        project_root,
        lock_manager,
        operation_queue,
    }))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(dispatcher): State<Arc<PluginDispatcher>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, dispatcher))
}

async fn handle_socket(mut socket: WebSocket, dispatcher: Arc<PluginDispatcher>) {
    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => {
                let response = match serde_json::from_str(&text) {
                    Ok(mcp_message) => dispatcher.dispatch(mcp_message).await,
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
