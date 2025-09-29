//! tests: Comprehensive integration testing framework for Codeflow Buddy
//!
//! This crate provides a robust testing infrastructure including test harnesses,
//! mock implementations, helper utilities, and end-to-end test suites. It ensures
//! complete coverage of all system functionality including refactoring workflows,
//! plugin system operations, LSP integration, and performance validation.
//! The testing framework validates both correctness and performance characteristics.

pub mod harness;
pub mod helpers;
pub mod mocks;

#[cfg(test)]
pub mod contract_tests;

#[cfg(test)]
pub mod resilience_tests;

pub use harness::{TestClient, TestWorkspace};
pub use helpers::*;
pub use mocks::{MockAstService, MockLspService};

use thiserror::Error;

/// Test harness errors
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TestHarnessError {
    #[error("Setup error: {message}")]
    Setup { message: String },

    #[error("Test execution error: {message}")]
    Execution { message: String },

    #[error("Assertion error: {message}")]
    Assertion { message: String },
}

impl TestHarnessError {
    /// Create a setup error
    pub fn setup(message: impl Into<String>) -> Self {
        Self::Setup {
            message: message.into(),
        }
    }

    /// Create an execution error
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Execution {
            message: message.into(),
        }
    }

    /// Create an assertion error
    pub fn assertion(message: impl Into<String>) -> Self {
        Self::Assertion {
            message: message.into(),
        }
    }
}
