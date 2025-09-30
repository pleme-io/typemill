//! Core data structures for the Intent-Based Workflow Engine.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a high-level user or AI goal.
/// This is the primary input to the workflow engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// The unique name of the intent, e.g., "refactor.renameSymbol".
    pub name: String,
    /// A flexible JSON object containing the parameters for the intent.
    pub params: Value,
}

/// Metadata about a workflow's characteristics and complexity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    /// Complexity score based on the number of steps.
    pub complexity: usize,
}

/// Represents a concrete, multi-step plan to fulfill an Intent.
/// This is the primary output of the Planner service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// A descriptive name for the workflow, derived from the intent.
    pub name: String,
    /// The ordered sequence of steps to be executed.
    pub steps: Vec<Step>,
    /// Metadata about this workflow's complexity and characteristics.
    pub metadata: WorkflowMetadata,
}

/// Represents a single, atomic action within a Workflow.
/// Each step corresponds to a call to a specific tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// The name of the tool to be called for this step (e.g., "find_references").
    pub tool: String,
    /// The JSON parameters to be passed to the tool.
    pub params: Value,
    /// A human-readable description of what this step does.
    pub description: String,
    /// Whether this step requires user confirmation before execution.
    /// This is for future interactive workflow support.
    #[serde(default)]
    pub requires_confirmation: Option<bool>,
}
