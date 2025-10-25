//! Admin endpoints for runtime log level control and health checks

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use mill_auth::{ generate_token , jwt::{ decode , Claims , DecodingKey , Validation } , };
use mill_config::config::AppConfig;
use mill_foundation::protocol::ApiResult;
use mill_workspaces::{ Workspace , WorkspaceManager };
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::{error, info};

/// Admin server state
#[derive(Clone)]
pub struct AdminState {
    /// Application version
    pub version: String,
    /// Server start time
    pub start_time: std::time::Instant,
    /// Workspace manager
    pub workspace_manager: Arc<WorkspaceManager>,
    /// Application configuration
    pub config: Arc<AppConfig>,
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

/// Execute command request
#[derive(Debug, Deserialize)]
pub struct ExecuteCommandRequest {
    /// Shell command to execute
    pub command: String,
}

/// Execute command response
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteCommandResponse {
    /// Exit code of the command
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Generate token request
#[derive(Debug, Deserialize)]
pub struct GenerateTokenRequest {
    /// Optional project ID to embed in token
    pub project_id: Option<String>,
    /// Optional user ID for multi-tenancy
    pub user_id: Option<String>,
    /// Optional custom expiry in seconds (defaults to config value)
    pub expiry_seconds: Option<u64>,
}

/// Generate token response
#[derive(Debug, Serialize)]
pub struct GenerateTokenResponse {
    /// Generated JWT token
    pub token: String,
    /// Token expiry time as Unix timestamp
    pub expires_at: u64,
}

/// Start the admin HTTP server on a separate port
pub async fn start_admin_server(
    port: u16,
    config: Arc<AppConfig>,
    workspace_manager: Arc<WorkspaceManager>,
) -> ApiResult<()> {
    let state = AdminState {
        version: env!("CARGO_PKG_VERSION").to_string(),
        start_time: std::time::Instant::now(),
        workspace_manager,
        config,
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/healthz", get(health_check)) // Kubernetes style
        .route("/admin/log-level", post(set_log_level))
        .route("/admin/log-level", get(get_log_level))
        .route("/auth/generate-token", post(generate_auth_token))
        .route("/workspaces", get(list_workspaces))
        .route("/workspaces/register", post(register_workspace))
        .route("/workspaces/{id}/execute", post(execute_command))
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
    info!("  POST /auth/generate-token - Generate JWT authentication token");
    info!("  GET  /workspaces - List registered workspaces");
    info!("  POST /workspaces/register - Register a new workspace");
    info!("  POST /workspaces/:id/execute - Execute command in workspace");

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

/// Extracts the user_id from the JWT token in the Authorization header.
fn extract_user_id_from_jwt(
    headers: &HeaderMap,
    config: &AppConfig,
) -> Result<String, (StatusCode, String)> {
    // 1. Extract Authorization header
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header".to_string(),
            )
        })?
        .to_str()
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization header".to_string(),
            )
        })?;

    // 2. Extract token (Bearer <token>)
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid Authorization format".to_string(),
        )
    })?;

    // 3. Validate and decode JWT
    let auth_config = config.server.auth.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Auth not configured".to_string(),
        )
    })?;

    let key = DecodingKey::from_secret(auth_config.jwt_secret.as_ref());
    let mut validation = Validation::default();

    // Use config to determine if audience validation is enabled
    validation.validate_aud = auth_config.validate_audience;

    if auth_config.validate_audience {
        let audience = auth_config
            .jwt_audience_override
            .as_ref()
            .unwrap_or(&auth_config.jwt_audience);
        validation.set_audience(&[audience]);
    }

    let token_data = decode::<Claims>(token, &key, &validation)
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e)))?;

    // 4. Extract user_id claim
    token_data.claims.user_id.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Token missing user_id claim".to_string(),
        )
    })
}

/// Register a new workspace, scoped to the authenticated user.
async fn register_workspace(
    State(state): State<Arc<AdminState>>,
    headers: HeaderMap,
    Json(workspace): Json<Workspace>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let user_id = extract_user_id_from_jwt(&headers, &state.config)?;
    info!(workspace_id = %workspace.id, user_id = %user_id, "Registering workspace");
    state.workspace_manager.register(&user_id, workspace);
    Ok(Json(json!({ "status": "registered" })))
}

/// List all registered workspaces for the authenticated user.
async fn list_workspaces(
    State(state): State<Arc<AdminState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Workspace>>, (StatusCode, String)> {
    let user_id = extract_user_id_from_jwt(&headers, &state.config)?;
    Ok(Json(state.workspace_manager.list(&user_id)))
}

/// Execute command in a workspace, scoped to the authenticated user.
async fn execute_command(
    State(state): State<Arc<AdminState>>,
    headers: HeaderMap,
    Path(workspace_id): Path<String>,
    Json(request): Json<ExecuteCommandRequest>,
) -> Result<Json<ExecuteCommandResponse>, (StatusCode, String)> {
    let user_id = extract_user_id_from_jwt(&headers, &state.config)?;
    info!(
        workspace_id = %workspace_id,
        command = %request.command,
        user_id = %user_id,
        "Executing command in workspace"
    );

    // Look up workspace for the specific user
    let workspace = state
        .workspace_manager
        .get(&user_id, &workspace_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Workspace '{}' not found for this user", workspace_id),
            )
        })?;

    // Build agent URL
    let agent_url = format!("{}/execute", workspace.agent_url);

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| {
            error!(error = %e, "Failed to create HTTP client");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "HTTP client error".to_string(),
            )
        })?;

    // Forward command to agent
    let response = client
        .post(&agent_url)
        .json(&json!({ "command": request.command }))
        .send()
        .await
        .map_err(|e| {
            error!(
                workspace_id = %workspace_id,
                agent_url = %agent_url,
                error = %e,
                "Failed to send command to workspace agent"
            );
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to reach workspace agent: {}", e),
            )
        })?;

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!(
            workspace_id = %workspace_id,
            status = %status,
            error = %error_text,
            "Agent returned error"
        );
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Agent error ({}): {}", status, error_text),
        ));
    }

    // Parse response
    let result: ExecuteCommandResponse = response.json().await.map_err(|e| {
        error!(
            workspace_id = %workspace_id,
            error = %e,
            "Failed to parse agent response"
        );
        (
            StatusCode::BAD_GATEWAY,
            "Invalid response from workspace agent".to_string(),
        )
    })?;

    info!(
        workspace_id = %workspace_id,
        exit_code = result.exit_code,
        execution_time_ms = result.execution_time_ms,
        "Command execution completed"
    );

    Ok(Json(result))
}

/// Generate JWT authentication token
async fn generate_auth_token(
    State(state): State<Arc<AdminState>>,
    Json(request): Json<GenerateTokenRequest>,
) -> Result<Json<GenerateTokenResponse>, (StatusCode, String)> {
    // Check if authentication is configured
    let auth_config = state.config.server.auth.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Authentication is not configured on this server".to_string(),
        )
    })?;

    // Use custom expiry or default from config
    let expiry_seconds = request
        .expiry_seconds
        .unwrap_or(auth_config.jwt_expiry_seconds);

    // Generate token
    let token = generate_token(
        &auth_config.jwt_secret,
        expiry_seconds,
        &auth_config.jwt_issuer,
        &auth_config.jwt_audience,
        request.project_id,
        request.user_id.clone(),
    )
    .map_err(|e| {
        error!(error = %e, "Failed to generate token");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Token generation failed: {}", e),
        )
    })?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    info!(
        expiry_seconds = expiry_seconds,
        user_id = ?request.user_id,
        "Generated authentication token"
    );

    Ok(Json(GenerateTokenResponse {
        token,
        expires_at: now + expiry_seconds,
    }))
}