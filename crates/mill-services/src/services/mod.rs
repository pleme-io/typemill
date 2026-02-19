//! Services for coordinating complex operations.
//!
//! This module is organized into several submodules, each representing a distinct
//! service domain, to improve code organization and clarity.

// --- Domain-Specific Submodules ---

/// AST (Abstract Syntax Tree) operations, including parsing and import analysis.
pub mod ast;
/// Services for coordinating concurrent operations, such as locking and queuing.
pub mod coordination;
/// Filesystem operations, including file I/O and git integration.
pub mod filesystem;
/// Services for generating, converting, and executing operational plans.
pub mod planning;
/// Services for validation, such as checksums and post-application checks.
pub mod validation;

// --- Other Service Modules ---

pub mod app_state_factory;
pub mod move_service;
pub mod perf_env;
pub mod perf_metrics;
pub mod reference_updater;
pub mod registry_builder;

// --- Testing ---

#[cfg(test)]
pub mod tests;

// --- Public Re-exports for Backward Compatibility ---

// Re-export items from the new submodules to maintain the public API.

pub use self::ast::ast_service::DefaultAstService;
pub use self::ast::import_service::ImportService;
pub use self::coordination::lock_manager::{LockManager, LockType};
pub use self::coordination::operation_queue::{
    FileOperation, OperationQueue, OperationType, QueueStats,
};
pub use self::coordination::workflow_executor::{self, WorkflowExecutor};
pub use self::filesystem::file_service::{self, FileService};
pub use self::filesystem::git_service::{self, GitService};
pub use self::planning::converter::{self, PlanConverter};
pub use self::planning::executor::{self, ExecutionOptions, ExecutionResult, PlanExecutor};
pub use self::planning::planner::{self, Planner};
pub use self::validation::checksum::{self, ChecksumValidator};
pub use self::validation::dry_run::{self, DryRunGenerator, DryRunResult};
pub use self::validation::post_apply::{self, PostApplyValidator};

// Re-export items from modules that were not moved.
pub use self::move_service::MoveService;
pub use self::registry_builder::build_language_plugin_registry;
