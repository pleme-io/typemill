use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::path::PathBuf;

use super::ProjectFixtures;

impl ProjectFixtures {
    /// Create a Rust project for multi-language testing
    pub async fn create_rust_project(
        workspace: &TestWorkspace,
        client: &mut TestClient,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        // Create Cargo.toml
        let cargo_toml = workspace.path().join("Cargo.toml");
        let cargo_content = r#"
[package]
name = "test-rust-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
uuid = { version = "1.0", features = ["v4"] }

[dev-dependencies]
assert_cmd = "2.0"
tempfile = "3.0"
"#;

        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": cargo_toml.to_string_lossy(),
                    "content": cargo_content
                }),
            )
            .await?;
        created_files.push(cargo_toml);

        // Create src directory
        let src_dir = workspace.path().join("src");
        std::fs::create_dir_all(&src_dir)?;

        // Create lib.rs
        let lib_file = src_dir.join("lib.rs");
        let lib_content = r#"
//! Test Rust project for multi-language integration testing.

pub mod models;
pub mod services;
pub mod utils;

pub use models::*;
pub use services::*;
pub use utils::*;

/// Project version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the library
pub fn init() -> anyhow::Result<()> {
    println!("Initializing Rust test project v{}", VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }
}
"#;

        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": lib_file.to_string_lossy(),
                    "content": lib_content
                }),
            )
            .await?;
        created_files.push(lib_file);

        // Create models.rs
        let models_file = src_dir.join("models.rs");
        let models_content = r#"
//! Data models for the Rust project.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    User,
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

impl User {
    pub fn new(name: String, email: String, role: UserRole) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            email,
            role,
            created_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }

    pub fn display_name(&self) -> String {
        format!("{} ({:?})", self.name, self.role)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.trim().is_empty() {
            anyhow::bail!("Name cannot be empty");
        }

        if !self.email.contains('@') {
            anyhow::bail!("Invalid email format");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub owner_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub tags: Vec<String>,
}

impl Project {
    pub fn new(name: String, description: String, owner_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            owner_id,
            created_at: chrono::Utc::now(),
            is_active: true,
            tags: Vec::new(),
        }
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "Test User".to_string(),
            "test@example.com".to_string(),
            UserRole::User,
        );

        assert_eq!(user.name, "Test User");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, UserRole::User);
        assert!(!user.is_admin());
        assert!(user.validate().is_ok());
    }

    #[test]
    fn test_project_tags() {
        let mut project = Project::new(
            "Test Project".to_string(),
            "A test project".to_string(),
            Uuid::new_v4(),
        );

        project.add_tag("rust".to_string());
        project.add_tag("testing".to_string());

        assert!(project.has_tag("rust"));
        assert!(project.has_tag("testing"));
        assert!(!project.has_tag("python"));

        project.remove_tag("rust");
        assert!(!project.has_tag("rust"));
        assert!(project.has_tag("testing"));
    }
}
"#;

        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": models_file.to_string_lossy(),
                    "content": models_content
                }),
            )
            .await?;
        created_files.push(models_file);

        Ok(created_files)
    }
}