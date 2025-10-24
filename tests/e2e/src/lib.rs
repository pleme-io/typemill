//! tests: Comprehensive integration testing framework for Codeflow Buddy
//!
//! This crate provides a robust testing infrastructure including test harnesses,
//! mock implementations, helper utilities, and end-to-end test suites. It ensures
//! complete coverage of all system functionality including refactoring workflows,
//! plugin system operations, LSP integration, and performance validation.
//! The testing framework validates both correctness and performance characteristics.

pub use mill_test_support::{harness, helpers, mocks};

#[cfg(test)]
pub mod test_helpers;

#[cfg(test)]
pub mod dry_run_integration;

#[cfg(test)]
pub mod resilience_tests;

// Unified Refactoring API integration tests (Proposal 30)
#[cfg(test)]
pub mod test_rename_integration;

#[cfg(test)]
pub mod test_rename_with_imports;

#[cfg(test)]
pub mod test_comprehensive_rename_coverage;

#[cfg(test)]
pub mod test_cross_workspace_import_updates;

#[cfg(test)]

#[cfg(test)]
pub mod test_file_discovery_bug;

#[cfg(test)]
pub mod test_scope_presets;

#[cfg(test)]
pub mod test_cargo_package_rename;

#[cfg(test)]
pub mod test_consolidation;

#[cfg(test)]

#[cfg(test)]
pub mod test_extract_integration;

#[cfg(test)]
pub mod test_inline_integration;

#[cfg(test)]

#[cfg(test)]
pub mod test_move_integration;

#[cfg(test)]
pub mod test_move_with_imports;

#[cfg(test)]
pub mod test_rust_refactoring;

#[cfg(test)]
pub mod test_rust_cargo_edge_cases;

#[cfg(test)]

#[cfg(test)]
pub mod test_reorder_integration;

#[cfg(test)]

#[cfg(test)]
pub mod test_transform_integration;

#[cfg(test)]

#[cfg(test)]
pub mod test_delete_integration;

#[cfg(test)]
pub mod test_workspace_apply_integration;

// Unified Analysis API integration tests (Proposal 40)
#[cfg(test)]
pub mod test_analyze_quality;

#[cfg(test)]
pub mod test_analyze_dead_code;

#[cfg(test)]

#[cfg(test)]
pub mod test_analyze_dependencies;

#[cfg(test)]

#[cfg(test)]
pub mod test_analyze_structure;

#[cfg(test)]

#[cfg(test)]
pub mod test_analyze_documentation;

#[cfg(test)]

#[cfg(test)]
pub mod test_analyze_tests;

#[cfg(test)]

#[cfg(test)]
pub mod test_analyze_batch;

#[cfg(test)]

#[cfg(test)]
pub mod test_suggestions_dead_code;

#[cfg(test)]

// Workspace package creation tests (Proposal 50)
#[cfg(test)]
pub mod test_workspace_create;

#[cfg(test)]

// Module dependency analysis tests (Proposal 50)
#[cfg(test)]
pub mod test_analyze_module_dependencies;

#[cfg(test)]

// Workspace dependency extraction tests (Proposal 50)
#[cfg(test)]
pub mod test_workspace_extract_deps;

#[cfg(test)]

// Workspace member management tests (Proposal 50)
#[cfg(test)]
pub mod test_workspace_update_members;

#[cfg(test)]

// Workspace find/replace tests
#[cfg(test)]
pub mod test_workspace_find_replace;

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
