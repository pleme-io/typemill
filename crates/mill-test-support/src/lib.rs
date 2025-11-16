//! Test support utilities and fixtures for TypeMill integration tests

pub mod harness;
pub mod helpers;
pub mod mocks;

// Re-export commonly used helpers
pub use helpers::create_test_config;

// Re-export plugin testing utilities for convenience
pub use harness::{IntegrationTestHarness, plugin_assertions, plugin_fixtures};

/// Get the path to test fixtures directory
pub fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}
