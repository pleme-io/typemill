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
    /// The URL of the agent running in the workspace (e.g., "http://python-workspace:8000").
    pub agent_url: String,
}

/// Manages the collection of registered workspaces in a thread-safe manner.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceManager {
    // The key is a tuple of (user_id, workspace_id) to enforce multi-tenancy.
    workspaces: Arc<DashMap<(String, String), Workspace>>,
}

impl WorkspaceManager {
    /// Creates a new, empty `WorkspaceManager`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new workspace or updates an existing one for a specific user.
    pub fn register(&self, user_id: &str, workspace: Workspace) {
        self.workspaces
            .insert((user_id.to_string(), workspace.id.clone()), workspace);
    }

    /// Retrieves a list of all registered workspaces for a specific user.
    pub fn list(&self, user_id: &str) -> Vec<Workspace> {
        self.workspaces
            .iter()
            .filter(|entry| entry.key().0 == user_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Retrieves a specific workspace by its ID for a given user.
    pub fn get(&self, user_id: &str, id: &str) -> Option<Workspace> {
        self.workspaces
            .get(&(user_id.to_string(), id.to_string()))
            .map(|entry| entry.value().clone())
    }
}
