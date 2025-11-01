//! Contains all plan result types.
use crate::planning::edit::EditPlanMetadata;
use serde::Serialize;

/// Result of applying an edit plan
#[derive(Debug, Clone, Serialize)]
pub struct EditPlanResult {
    /// Whether all edits were applied successfully
    pub success: bool,
    /// List of files that were modified
    pub modified_files: Vec<String>,
    /// Error messages if any edits failed
    pub errors: Option<Vec<String>>,
    /// Original plan metadata
    pub plan_metadata: EditPlanMetadata,
}
