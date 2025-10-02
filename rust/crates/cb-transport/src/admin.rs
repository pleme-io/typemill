//! Admin endpoints for runtime log level control and health checks

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use cb_api::ApiResult;
use cb_server::workspaces::{Workspace, WorkspaceManager};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::info;

/// Admin server state
#[derive(Clone)]
pub struct AdminState {
    /// Application version
    pub version: String,
    /// Server start time
    pub start_time: std::time::Instant,
    /// Workspace manager
    pub workspace_manager: Arc<WorkspaceManager>,
}

/// Log level change request
#[derive(Debug, Deserialize)]
pub struct LogLevelRequest {
    /// New log level (trace, debug, info, warn, error)
    pub level: String,
}

/// Log level response
#[derive(Debug, Serialize)]
pub struct LogLevelResponse {
    /// Status of the operation
    pub status: String,
    /// Previous log level
    pub previous_level: Option<String>,
    /// New log level
    pub new_level: String,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Application version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Current log level
    pub log_level: String,
}

/// Start the admin HTTP server on a separate port
pub async fn start_admin_server(
    port: u16,
    workspace_manager: Arc<WorkspaceManager>,
) -> ApiResult<()> {
    let state = AdminState {
        version: env!("CARGO_PKG_VERSION").to_string(),
        start_time: std::time::Instant::now(),
        workspace_manager,
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/healthz", get(health_check)) // Kubernetes style
        .route("/admin/log-level", post(set_log_level))
        .route("/admin/log-level", get(get_log_level))
        .route("/workspaces", get(list_workspaces))
        .route("/workspaces/register", post(register_workspace))
        .layer(ServiceBuilder::new())
        .with_state(Arc::new(state));

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    info!("Admin server listening on {}", addr);
    info!("Available endpoints:");
    info!("  GET  /health - Health check");
    info!("  GET  /healthz - Kubernetes health check");
    info!("  POST /admin/log-level - Set log level");
    info!("  GET  /admin/log-level - Get current log level");
    info!("  GET  /workspaces - List registered workspaces");
    info!("  POST /workspaces/register - Register a new workspace");

    axum::serve(listener, app).await?;
    Ok(())
}

/// Health check endpoint
async fn health_check(State(state): State<Arc<AdminState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        log_level: get_current_log_level(),
    })
}

/// Set log level endpoint
async fn set_log_level(
    State(_state): State<Arc<AdminState>>,
    Json(request): Json<LogLevelRequest>,
) -> Result<Json<LogLevelResponse>, StatusCode> {
    let previous_level = get_current_log_level();

    // Validate log level
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&request.level.to_lowercase().as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Log level changes are not supported at runtime without reload handle
    // This is a documented limitation - see ARCHITECTURE.md
    tracing::warn!(
        previous_level = %previous_level,
        requested_level = %request.level,
        "Runtime log level change not supported - restart server with RUST_LOG={}",
        request.level
    );

    Ok(Json(LogLevelResponse {
        status: "not_supported".to_string(),
        previous_level: Some(previous_level),
        new_level: format!("restart_required_with_RUST_LOG={}", request.level),
    }))
}

/// Get current log level endpoint
async fn get_log_level(State(_state): State<Arc<AdminState>>) -> Json<Value> {
    Json(json!({
        "current_level": get_current_log_level(),
        "available_levels": ["trace", "debug", "info", "warn", "error"]
    }))
}

/// Get current log level (simplified implementation)
fn get_current_log_level() -> String {
    // In a real implementation, this would query the actual tracing filter
    // For now, we'll use the environment variable or default to "info"
    std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".to_string())
        .split(',')
        .next()
        .unwrap_or("info")
        .split('=')
        .next_back()
        .unwrap_or("info")
        .to_string()
}

/// Register a new workspace
async fn register_workspace(
    State(state): State<Arc<AdminState>>,
    Json(workspace): Json<Workspace>,
) -> Result<Json<Value>, (StatusCode, String)> {
    info!(workspace_id = %workspace.id, "Registering new workspace");
    state.workspace_manager.register(workspace);
    Ok(Json(json!({ "status": "registered" })))
}

/// List all registered workspaces
async fn list_workspaces(State(state): State<Arc<AdminState>>) -> Json<Vec<Workspace>> {
    Json(state.workspace_manager.list())
}
