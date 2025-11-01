//! File operations service with import awareness

// Module declarations
mod basic_ops;
mod edit_plan;
mod rename;
mod utils;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod path_safety_tests;

// Re-export public types
pub use mill_foundation::protocol::EditPlanResult;
pub use self::utils::DocumentationUpdateReport;

use crate::services::coordination::lock_manager::LockManager;
use crate::services::coordination::operation_queue::OperationQueue;
use crate::services::filesystem::git_service::GitService;
use crate::services::move_service::MoveService;
use crate::services::reference_updater::ReferenceUpdater;
use mill_ast::AstCache;
use mill_config::config::AppConfig;
use mill_foundation::validation::ValidationConfig;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

/// Service for file operations with import update capabilities
pub struct FileService {
    /// Reference updater for handling import updates
    pub reference_updater: ReferenceUpdater,
    /// Language plugin registry
    pub plugin_registry: Arc<mill_plugin_api::PluginRegistry>,
    /// Project root directory
    pub(super) project_root: PathBuf,
    /// Canonicalized project root (cached for performance)
    pub(super) canonical_project_root: PathBuf,
    /// AST cache for invalidation after edits
    pub(super) ast_cache: Arc<AstCache>,
    /// Lock manager for atomic operations
    pub(super) lock_manager: Arc<LockManager>,
    /// Operation queue for serializing file operations
    pub(super) operation_queue: Arc<OperationQueue>,
    /// Git service for git-aware file operations
    #[allow(dead_code)]
    pub(super) git_service: GitService,
    /// Whether to use git for file operations
    pub(super) use_git: bool,
    /// Validation configuration
    pub(super) validation_config: ValidationConfig,
}

impl FileService {
    /// Create a new file service
    pub fn new(
        project_root: impl AsRef<Path>,
        ast_cache: Arc<AstCache>,
        lock_manager: Arc<LockManager>,
        operation_queue: Arc<OperationQueue>,
        config: &AppConfig,
        plugin_registry: Arc<mill_plugin_api::PluginRegistry>,
    ) -> Self {
        let project_root = project_root.as_ref().to_path_buf();

        // Canonicalize project root once for performance and security
        // This cached value is used for all path containment checks
        let canonical_project_root = project_root.canonicalize().unwrap_or_else(|e| {
            tracing::warn!(
                project_root = %project_root.display(),
                error = %e,
                "Failed to canonicalize project root, using as-is (path safety checks may fail)"
            );
            project_root.clone()
        });

        // Determine if we should use git based on:
        // 1. Configuration git.enabled flag
        // 2. Whether the project is actually a git repository
        let is_git_repo = GitService::is_git_repo(&project_root);
        let use_git = config.git.enabled && is_git_repo;

        debug!(
            project_root = %project_root.display(),
            canonical_root = %canonical_project_root.display(),
            git_enabled_in_config = config.git.enabled,
            is_git_repo,
            use_git,
            "Initializing FileService with git support and injected plugin registry"
        );

        Self {
            reference_updater: ReferenceUpdater::new(&project_root),
            plugin_registry,
            project_root,
            canonical_project_root,
            ast_cache,
            lock_manager,
            operation_queue,
            git_service: GitService::new(),
            use_git,
            validation_config: config.validation.clone(),
        }
    }

    /// Create a MoveService for unified move/rename planning
    ///
    /// The MoveService provides the single source of truth for all move and rename operations.
    pub fn move_service(&self) -> MoveService<'_> {
        MoveService::new(
            &self.reference_updater,
            &self.plugin_registry,
            &self.project_root,
        )
    }
}
