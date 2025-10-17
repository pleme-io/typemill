//! Services for coordinating complex operations

pub mod app_state_factory;
pub mod ast_service;
pub mod checksum_validator;
pub mod dry_run_generator;
pub mod file_service;
pub mod git_service;
pub mod import_service;
pub mod lock_manager;
pub mod move_service;
pub mod operation_queue;
pub mod plan_converter;
pub mod planner;
pub mod post_apply_validator;
pub mod reference_updater;
pub mod registry_builder;
pub mod workflow_executor;

#[cfg(test)]
pub mod tests;

// #[cfg(test)]
// pub mod phase2_tests; // Disabled due to private method access

pub use ast_service::DefaultAstService;
pub use checksum_validator::ChecksumValidator;
pub use dry_run_generator::{DryRunGenerator, DryRunResult};
pub use file_service::FileService;
pub use git_service::GitService;
pub use import_service::ImportService;
pub use lock_manager::{LockManager, LockType};
pub use move_service::MoveService;
pub use operation_queue::{FileOperation, OperationQueue, OperationType, QueueStats};
pub use plan_converter::PlanConverter;
pub use post_apply_validator::{PostApplyValidator, ValidationConfig, ValidationResult};
pub use registry_builder::build_language_plugin_registry;
