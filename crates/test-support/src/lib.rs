//! Test support utilities and fixtures for Codebuddy integration tests

pub mod harness;
pub mod helpers;
pub mod mocks;

/// Get the path to test fixtures directory
pub fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}
