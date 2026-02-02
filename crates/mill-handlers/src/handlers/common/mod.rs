//! Common utilities shared across refactoring handlers
//!
//! This module provides shared functionality used by rename, move, and other
//! refactoring operations to avoid code duplication.

use crate::handlers::tools::extensions::get_concrete_app_state;
use mill_foundation::errors::MillResult as ServerResult;
use mill_foundation::protocol::RefactorPlan;
use mill_handler_api::ToolHandlerContext;
use mill_services::services::{ExecutionOptions, ExecutionResult, PlanExecutor};

pub mod checksums;

pub use checksums::calculate_checksum;
pub(crate) use checksums::{
    calculate_checksums_for_directory_rename, calculate_checksums_for_edits,
};

use async_trait::async_trait;
use mill_handler_api::LspAdapter;
use mill_services::services::reference_updater::LspImportFinder;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Wrapper to adapt LspAdapter to LspImportFinder trait
pub struct LspFinderWrapper(pub Arc<dyn LspAdapter>);

#[async_trait]
impl LspImportFinder for LspFinderWrapper {
    async fn find_files_that_import(&self, file_path: &Path) -> Result<Vec<PathBuf>, String> {
        self.0.find_files_that_import(file_path).await
    }

    async fn find_files_that_import_directory(
        &self,
        dir_path: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        self.0.find_files_that_import_directory(dir_path).await
    }
}

/// Execute a refactoring plan using the file service from the app state
///
/// This is a shared helper function used by refactoring handlers (inline, extract,
/// delete, rename, move) to avoid code duplication. It handles getting the concrete
/// app state, creating the executor, and executing the plan with default options.
///
/// # Arguments
/// * `context` - The tool handler context containing the app state
/// * `plan` - The refactoring plan to execute
///
/// # Returns
/// The execution result containing applied files, warnings, and validation results
pub async fn execute_refactor_plan(
    context: &ToolHandlerContext,
    plan: RefactorPlan,
) -> ServerResult<ExecutionResult> {
    // Get concrete AppState to access concrete FileService
    let concrete_state = get_concrete_app_state(&context.app_state)?;
    let executor = PlanExecutor::new(concrete_state.file_service.clone());
    executor
        .execute_plan(plan, ExecutionOptions::default())
        .await
}

/// Estimate impact based on number of affected files
pub fn estimate_impact(affected_files: usize) -> String {
    if affected_files <= 3 {
        "low"
    } else if affected_files <= 10 {
        "medium"
    } else {
        "high"
    }
    .to_string()
}

/// Detect language from file path extension
pub fn detect_language(file_path: &str) -> &'static str {
    use std::path::Path;
    let path = Path::new(file_path);
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") | Some("pyi") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("swift") => "swift",
        Some("cs") => "csharp",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("hpp") | Some("cc") | Some("cxx") => "cpp",
        _ => "unknown",
    }
}
