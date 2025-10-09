//! Authentication module for Jules API
//!
//! This module will contain authentication logic for the Jules API client.
//! Currently a stub implementation.

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// API key for authentication
    pub api_key: String,
}

impl AuthConfig {
    /// Create a new authentication configuration
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

/// Authentication provider trait
pub trait AuthProvider {
    /// Get the current authentication token
    fn get_token(&self) -> &str;
}

impl AuthProvider for AuthConfig {
    fn get_token(&self) -> &str {
        &self.api_key
    }
}
