use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use cb_server::handlers::plugin_dispatcher::{AppState, PluginDispatcher};
use cb_server::systems::LspManager;
use cb_core::config::LspConfig;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Create AppState similar to the test implementation
    let app_state = create_app_state().await;

    let dispatcher = Arc::new(PluginDispatcher::new(app_state));
    dispatcher.initialize().await.expect("Failed to initialize dispatcher");

    let app = Router::new().route("/ws", get(ws_handler)).with_state(dispatcher);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn create_app_state() -> Arc<AppState> {
    let lsp_config = LspConfig::default();
    let lsp_manager = Arc::new(LspManager::new(lsp_config));

    // Use current working directory as project root for production
    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let file_service = Arc::new(cb_server::services::FileService::new(project_root.clone()));
    let lock_manager = Arc::new(cb_server::services::LockManager::new());
    let operation_queue = Arc::new(cb_server::services::OperationQueue::new(lock_manager.clone()));

    Arc::new(AppState {
        lsp: lsp_manager,
        file_service,
        project_root,
        lock_manager,
        operation_queue,
    })
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
                        tracing::error!("Failed to deserialize message: {}", e);
                        continue;
                    }
                };

                match response {
                    Ok(response_message) => {
                        let response_text = serde_json::to_string(&response_message).unwrap();
                        if socket.send(Message::Text(response_text.into())).await.is_err() {
                            break; // client disconnected
                        }
                    }
                    Err(e) => {
                        // Handle dispatch error
                        tracing::error!("Error dispatching message: {}", e);
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
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        }
    }
}