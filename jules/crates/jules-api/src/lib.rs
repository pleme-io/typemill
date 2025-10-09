//! Jules API client library
//!
//! This crate provides a Rust client for the Jules API, enabling programmatic
//! access to Jules's AI coding agent capabilities.

pub mod client;
pub mod auth;
pub mod pagination;
pub mod error;
pub mod types;
pub mod config;

// Re-exports
pub use client::JulesClient;
pub use config::Config;
pub use error::JulesError;
pub use types::{Activity, Session, Source, CreateSessionRequest, SourcesResponse, SessionsResponse, ActivitiesResponse};

pub type Result<T> = std::result::Result<T, JulesError>;