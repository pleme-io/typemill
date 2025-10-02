use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Represents a registered workspace (typically a Docker container running a development environment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// A unique identifier for the container, typically the container name or ID.
    pub id: String,
    /// The primary programming language of the workspace (e.g., "python", "typescript").
    pub language: String,
    /// A user-friendly name for the project within the workspace.
    pub project_name: String,
}

/// Manages the collection of registered workspaces in a thread-safe manner.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceManager {
    workspaces: Arc<DashMap<String, Workspace>>,
}

impl WorkspaceManager {
    /// Creates a new, empty `WorkspaceManager`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new workspace or updates an existing one.
    pub fn register(&self, workspace: Workspace) {
        self.workspaces.insert(workspace.id.clone(), workspace);
    }

    /// Retrieves a list of all registered workspaces.
    pub fn list(&self) -> Vec<Workspace> {
        self.workspaces
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
}
