//! tests: Comprehensive integration testing framework for Codeflow Buddy
//!
//! This crate provides a robust testing infrastructure including test harnesses,
//! mock implementations, helper utilities, and end-to-end test suites. It ensures
//! complete coverage of all system functionality including refactoring workflows,
//! plugin system operations, LSP integration, and performance validation.
//! The testing framework validates both correctness and performance characteristics.
#![allow(unused_variables)]

pub use mill_test_support::{harness, helpers, mocks};

#[cfg(test)]
pub mod test_helpers;

#[cfg(test)]
pub mod dry_run_integration;

#[cfg(test)]
pub mod resilience_tests;

// Unified Refactoring API integration tests (Proposal 30)
#[cfg(test)]
pub mod comprehensive_refactoring_test;

#[cfg(test)]
pub mod test_rename_integration;

#[cfg(test)]
pub mod test_rename_with_imports;

#[cfg(test)]
pub mod test_comprehensive_rename_coverage;

#[cfg(test)]
pub mod test_cross_workspace_import_updates;

#[cfg(test)]
pub mod test_file_discovery_bug;

#[cfg(test)]
pub mod test_scope_presets;

#[cfg(test)]
pub mod test_cargo_package_rename;

#[cfg(test)]
pub mod test_consolidation;

#[cfg(test)]
pub mod test_extract_integration;

#[cfg(test)]
pub mod test_inline_integration;

#[cfg(test)]
pub mod test_move_integration;

#[cfg(test)]
pub mod test_move_with_imports;

#[cfg(test)]
pub mod test_rust_refactoring;

#[cfg(test)]
pub mod test_rust_cargo_edge_cases;

#[cfg(test)]
pub mod test_reorder_integration;

#[cfg(test)]
pub mod test_transform_integration;

#[cfg(test)]
pub mod test_delete_integration;

// Note: test_workspace_apply_integration was deleted in Phase 5 (unified API replaced separate plan/apply workflow)

// Workspace package creation tests (Proposal 50)
#[cfg(test)]
pub mod test_workspace_create;

// Workspace dependency extraction tests (Proposal 50)
#[cfg(test)]
pub mod test_workspace_extract_deps;

// Workspace member management tests (Proposal 50)
#[cfg(test)]
pub mod test_workspace_update_members;

// Workspace find/replace tests
#[cfg(test)]
pub mod test_workspace_find_replace;

// Cross-platform compatibility tests
#[cfg(test)]
pub mod test_cross_platform;

// TypeScript manual integration tests
#[cfg(test)]
pub mod test_typescript_manual;

#[cfg(test)]
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
