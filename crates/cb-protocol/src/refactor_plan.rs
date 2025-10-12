use lsp_types::WorkspaceEdit;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a target for deletion (file or directory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionTarget {
    pub path: String,
    pub kind: String, // "file" or "directory"
}

/// Discriminated union type for all refactoring plans
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "plan_type")]
pub enum RefactorPlan {
    RenamePlan(RenamePlan),
    ExtractPlan(ExtractPlan),
    InlinePlan(InlinePlan),
    MovePlan(MovePlan),
    ReorderPlan(ReorderPlan),
    TransformPlan(TransformPlan),
    DeletePlan(DeletePlan),
}

/// Base structure shared by all plans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub plan_version: String, // Always "1.0"
    pub kind: String,
    pub language: String,
    pub estimated_impact: String, // "low" | "medium" | "high"
    pub created_at: String,       // ISO 8601 timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub affected_files: usize,
    pub created_files: usize,
    pub deleted_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWarning {
    pub code: String,
    pub message: String,
    pub candidates: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenamePlan {
    pub edits: WorkspaceEdit,
    pub summary: PlanSummary,
    pub warnings: Vec<PlanWarning>,
    pub metadata: PlanMetadata,
    pub file_checksums: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractPlan {
    pub edits: WorkspaceEdit,
    pub summary: PlanSummary,
    pub warnings: Vec<PlanWarning>,
    pub metadata: PlanMetadata,
    pub file_checksums: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlinePlan {
    pub edits: WorkspaceEdit,
    pub summary: PlanSummary,
    pub warnings: Vec<PlanWarning>,
    pub metadata: PlanMetadata,
    pub file_checksums: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePlan {
    pub edits: WorkspaceEdit,
    pub summary: PlanSummary,
    pub warnings: Vec<PlanWarning>,
    pub metadata: PlanMetadata,
    pub file_checksums: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderPlan {
    pub edits: WorkspaceEdit,
    pub summary: PlanSummary,
    pub warnings: Vec<PlanWarning>,
    pub metadata: PlanMetadata,
    pub file_checksums: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformPlan {
    pub edits: WorkspaceEdit,
    pub summary: PlanSummary,
    pub warnings: Vec<PlanWarning>,
    pub metadata: PlanMetadata,
    pub file_checksums: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePlan {
    pub deletions: Vec<DeletionTarget>,
    pub summary: PlanSummary,
    pub warnings: Vec<PlanWarning>,
    pub metadata: PlanMetadata,
    pub file_checksums: HashMap<String, String>,
}
