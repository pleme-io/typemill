//! Services for coordinating complex operations

pub mod ast_service;
pub mod file_service;
pub mod git_service;
pub mod import_service;
pub mod lock_manager;
pub mod operation_queue;
pub mod planner;
pub mod workflow_executor;

#[cfg(test)]
pub mod tests;

// #[cfg(test)]
// pub mod phase2_tests; // Disabled due to private method access

pub use ast_service::DefaultAstService;
pub use file_service::FileService;
pub use git_service::GitService;
pub use import_service::ImportService;
pub use lock_manager::{LockManager, LockType};
pub use operation_queue::{FileOperation, OperationQueue, OperationType, QueueStats};
