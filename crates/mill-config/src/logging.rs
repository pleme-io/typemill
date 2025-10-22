//! Centralized logging initialization with environment variable support

use crate::{AppConfig, LogFormat};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing subscriber with environment variable support
///
/// Environment variables (in priority order):
/// - `RUST_LOG`: Standard Rust log filter (takes precedence over all)
/// - `LOG_LEVEL`: Set log level (trace, debug, info, warn, error)
/// - `LOG_FORMAT`: Override format (json, pretty)
///
/// # Examples
///
/// ```bash
/// # Development with debug logging
/// LOG_LEVEL=debug cargo run
///
/// # Production with JSON logs
/// LOG_LEVEL=info LOG_FORMAT=json ./codebuddy serve
///
/// # Module-specific filtering (most powerful)
/// RUST_LOG=cb_handlers=debug,cb_lsp=info cargo run
/// ```
pub fn initialize(config: &AppConfig) {
    // Parse log level from config
    let log_level = config.logging.level.parse().unwrap_or(tracing::Level::INFO);

    // Create env filter (RUST_LOG takes precedence over config)
    let env_filter = EnvFilter::from_default_env().add_directive(log_level.into());

    // Check for LOG_FORMAT env override
    let format = std::env::var("LOG_FORMAT")
        .ok()
        .and_then(|f| match f.to_lowercase().as_str() {
            "json" => Some(LogFormat::Json),
            "pretty" | "human" => Some(LogFormat::Pretty),
            _ => None,
        })
        .unwrap_or_else(|| config.logging.format.clone());

    // Initialize based on format
    // IMPORTANT: Always write to stderr to keep stdout clean for JSON-RPC
    match format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .json()
                        // .with_current_span(true) // Disabled due to performance issues in high-throughput tests
                        .with_writer(std::io::stderr),
                )
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().pretty().with_writer(std::io::stderr))
                .init();
        }
    }
}

/// Create a request span with standard fields for context propagation
///
/// Use this at transport layer to automatically add request context to all
/// nested logs within the request handler.
///
/// # Example
///
/// ```rust
/// use cb_core::logging::request_span;
///
/// let request_id = "req-12345";
/// let span = request_span(request_id, "websocket");
/// let _enter = span.enter();
///
/// // All logs within this scope automatically include:
/// // - request_id
/// // - transport (websocket or stdio)
/// tracing::info!("Processing request");
/// ```
pub fn request_span(request_id: &str, transport: &str) -> tracing::Span {
    tracing::info_span!(
        "request",
        request_id = %request_id,
        transport = %transport
    )
}
