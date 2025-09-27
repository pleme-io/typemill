use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::path::PathBuf;

/// Comprehensive project fixtures for testing various scenarios
pub struct ProjectFixtures;

impl ProjectFixtures {
    /// Create a large TypeScript project for performance testing
    pub async fn create_large_typescript_project(
        workspace: &TestWorkspace,
        client: &TestClient,
        file_count: usize,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        // Create directory structure
        let src_dir = workspace.path().join("src");
        std::fs::create_dir_all(&src_dir)?;

        let components_dir = src_dir.join("components");
        std::fs::create_dir_all(&components_dir)?;

        let services_dir = src_dir.join("services");
        std::fs::create_dir_all(&services_dir)?;

        let utils_dir = src_dir.join("utils");
        std::fs::create_dir_all(&utils_dir)?;

        let types_dir = src_dir.join("types");
        std::fs::create_dir_all(&types_dir)?;

        // Calculate files per directory
        let files_per_dir = file_count / 4;

        // Create type files
        for i in 0..files_per_dir {
            let file_path = types_dir.join(format!("types{}.ts", i));
            let content = format!(r#"
export interface Entity{} {{
    id: number;
    name: string;
    created: Date;
    metadata: Record<string, any>;
}}

export interface EntityFilter{} {{
    namePattern?: string;
    createdAfter?: Date;
    metadataKeys?: string[];
}}

export type EntityStatus{} = 'active' | 'inactive' | 'pending' | 'archived';

export interface EntityWithStatus{} extends Entity{} {{
    status: EntityStatus{};
    lastModified: Date;
}}

export class EntityValidator{} {{
    static validate(entity: Entity{}): boolean {{
        return entity.id > 0 && entity.name.length > 0;
    }}

    static validateStatus(status: EntityStatus{}): boolean {{
        return ['active', 'inactive', 'pending', 'archived'].includes(status);
    }}
}}
"#, i, i, i, i, i, i, i, i, i);

            client.call_tool("create_file", json!({
                "file_path": file_path.to_string_lossy(),
                "content": content
            })).await?;

            created_files.push(file_path);
        }

        // Create utility files
        for i in 0..files_per_dir {
            let file_path = utils_dir.join(format!("utils{}.ts", i));
            let content = format!(r#"
import {{ Entity{}, EntityFilter{}, EntityStatus{}, EntityValidator{} }} from '../types/types{}';

export class EntityUtils{} {{
    static formatName(entity: Entity{}): string {{
        return entity.name.charAt(0).toUpperCase() + entity.name.slice(1);
    }}

    static filterEntities(entities: Entity{}[], filter: EntityFilter{}): Entity{}[] {{
        return entities.filter(entity => {{
            if (filter.namePattern && !entity.name.includes(filter.namePattern)) {{
                return false;
            }}
            if (filter.createdAfter && entity.created < filter.createdAfter) {{
                return false;
            }}
            if (filter.metadataKeys) {{
                const hasAllKeys = filter.metadataKeys.every(key => key in entity.metadata);
                if (!hasAllKeys) return false;
            }}
            return EntityValidator{}.validate(entity);
        }});
    }}

    static sortByName(entities: Entity{}[]): Entity{}[] {{
        return [...entities].sort((a, b) => a.name.localeCompare(b.name));
    }}

    static groupByStatus(entities: Entity{}[]): Map<string, Entity{}[]> {{
        const groups = new Map<string, Entity{}[]>();
        for (const entity of entities) {{
            const status = 'status' in entity ? (entity as any).status : 'unknown';
            if (!groups.has(status)) {{
                groups.set(status, []);
            }}
            groups.get(status)!.push(entity);
        }}
        return groups;
    }}
}}

export function createEntity{}(name: string, metadata: Record<string, any> = {{}}): Entity{} {{
    return {{
        id: Math.floor(Math.random() * 1000000),
        name,
        created: new Date(),
        metadata
    }};
}}

export async function batchCreateEntities{}(names: string[]): Promise<Entity{}[]> {{
    return names.map(name => createEntity{}(name));
}}
"#, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i);

            client.call_tool("create_file", json!({
                "file_path": file_path.to_string_lossy(),
                "content": content
            })).await?;

            created_files.push(file_path);
        }

        // Create service files
        for i in 0..files_per_dir {
            let file_path = services_dir.join(format!("service{}.ts", i));
            let content = format!(r#"
import {{ Entity{}, EntityFilter{}, EntityWithStatus{}, EntityStatus{} }} from '../types/types{}';
import {{ EntityUtils{}, createEntity{}, batchCreateEntities{} }} from '../utils/utils{}';

export class EntityService{} {{
    private entities: Map<number, Entity{}> = new Map();
    private cache: Map<string, Entity{}[]> = new Map();

    async loadEntity{}(id: number): Promise<Entity{} | null> {{
        if (this.entities.has(id)) {{
            return this.entities.get(id) || null;
        }}

        // Simulate async loading
        await new Promise(resolve => setTimeout(resolve, Math.random() * 100));

        const entity = createEntity{}(`Entity_${{id}}`);
        entity.id = id;
        this.entities.set(id, entity);
        return entity;
    }}

    async saveEntity{}(entity: Entity{}): Promise<boolean> {{
        try {{
            this.entities.set(entity.id, entity);
            this.invalidateCache();
            return true;
        }} catch (error) {{
            console.error('Failed to save entity:', error);
            return false;
        }}
    }}

    async findEntities{}(filter: EntityFilter{}): Promise<Entity{}[]> {{
        const cacheKey = JSON.stringify(filter);
        if (this.cache.has(cacheKey)) {{
            return this.cache.get(cacheKey) || [];
        }}

        const allEntities = Array.from(this.entities.values());
        const filtered = EntityUtils{}.filterEntities(allEntities, filter);
        this.cache.set(cacheKey, filtered);
        return filtered;
    }}

    async deleteEntity{}(id: number): Promise<boolean> {{
        const deleted = this.entities.delete(id);
        if (deleted) {{
            this.invalidateCache();
        }}
        return deleted;
    }}

    private invalidateCache(): void {{
        this.cache.clear();
    }}

    async bulkCreate{}(count: number): Promise<Entity{}[]> {{
        const names = Array.from({{ length: count }}, (_, i) => `BulkEntity_${{i}}`);
        const entities = await batchCreateEntities{}(names);

        for (const entity of entities) {{
            this.entities.set(entity.id, entity);
        }}

        return entities;
    }}

    getStatistics(): {{ total: number; cacheSize: number }} {{
        return {{
            total: this.entities.size,
            cacheSize: this.cache.size
        }};
    }}
}}
"#, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i);

            client.call_tool("create_file", json!({
                "file_path": file_path.to_string_lossy(),
                "content": content
            })).await?;

            created_files.push(file_path);
        }

        // Create component files
        for i in 0..files_per_dir {
            let file_path = components_dir.join(format!("component{}.ts", i));
            let content = format!(r#"
import {{ Entity{}, EntityFilter{}, EntityWithStatus{} }} from '../types/types{}';
import {{ EntityService{} }} from '../services/service{}';
import {{ EntityUtils{} }} from '../utils/utils{}';

export interface ComponentProps{} {{
    entities: Entity{}[];
    onEntitySelect?: (entity: Entity{}) => void;
    onEntityUpdate?: (entity: Entity{}) => Promise<void>;
    filter?: EntityFilter{};
}}

export class EntityComponent{} {{
    private service: EntityService{};
    private selectedEntity: Entity{} | null = null;

    constructor(private props: ComponentProps{}) {{
        this.service = new EntityService{}();
    }}

    async initialize(): Promise<void> {{
        try {{
            await this.loadInitialData();
            this.setupEventHandlers();
        }} catch (error) {{
            console.error('Component initialization failed:', error);
        }}
    }}

    private async loadInitialData(): Promise<void> {{
        if (this.props.filter) {{
            const filteredEntities = await this.service.findEntities{}(this.props.filter);
            this.props.entities.push(...filteredEntities);
        }}
    }}

    private setupEventHandlers(): void {{
        // Simulate event handling
        console.log('Event handlers setup for Component{}');
    }}

    async selectEntity{}(id: number): Promise<void> {{
        const entity = await this.service.loadEntity{}(id);
        if (entity) {{
            this.selectedEntity = entity;
            if (this.props.onEntitySelect) {{
                this.props.onEntitySelect(entity);
            }}
        }}
    }}

    async updateEntity{}(updates: Partial<Entity{}>): Promise<boolean> {{
        if (!this.selectedEntity) return false;

        const updatedEntity = {{ ...this.selectedEntity, ...updates }};
        const success = await this.service.saveEntity{}(updatedEntity);

        if (success && this.props.onEntityUpdate) {{
            await this.props.onEntityUpdate(updatedEntity);
            this.selectedEntity = updatedEntity;
        }}

        return success;
    }}

    render(): string {{
        const sortedEntities = EntityUtils{}.sortByName(this.props.entities);
        return `<div>Component{} with ${{sortedEntities.length}} entities</div>`;
    }}

    destroy(): void {{
        this.selectedEntity = null;
        console.log('Component{} destroyed');
    }}
}}

export function createComponent{}(entities: Entity{}[], filter?: EntityFilter{}): EntityComponent{} {{
    return new EntityComponent{}({{ entities, filter }});
}}
"#, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i);

            client.call_tool("create_file", json!({
                "file_path": file_path.to_string_lossy(),
                "content": content
            })).await?;

            created_files.push(file_path);
        }

        Ok(created_files)
    }

    /// Create a Python project for multi-language testing
    pub async fn create_python_project(
        workspace: &TestWorkspace,
        client: &TestClient,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        // Create directory structure
        let src_dir = workspace.path().join("python_project");
        std::fs::create_dir_all(&src_dir)?;

        // Create __init__.py
        let init_file = src_dir.join("__init__.py");
        client.call_tool("create_file", json!({
            "file_path": init_file.to_string_lossy(),
            "content": "\"\"\"Python project for testing.\"\"\"\n__version__ = \"1.0.0\"\n"
        })).await?;
        created_files.push(init_file);

        // Create models.py
        let models_file = src_dir.join("models.py");
        let models_content = r#"
"""Data models for the application."""

from dataclasses import dataclass
from datetime import datetime
from typing import Dict, List, Optional, Union
from enum import Enum


class UserRole(Enum):
    """User role enumeration."""
    ADMIN = "admin"
    USER = "user"
    GUEST = "guest"


@dataclass
class User:
    """User data model."""
    id: int
    name: str
    email: str
    role: UserRole
    created_at: datetime
    metadata: Dict[str, Union[str, int, bool]]

    def __post_init__(self):
        """Validate user data after initialization."""
        if not self.email or "@" not in self.email:
            raise ValueError("Invalid email address")

        if not self.name or len(self.name.strip()) == 0:
            raise ValueError("Name cannot be empty")

    def is_admin(self) -> bool:
        """Check if user is an administrator."""
        return self.role == UserRole.ADMIN

    def get_display_name(self) -> str:
        """Get formatted display name."""
        return f"{self.name} ({self.role.value})"


@dataclass
class Project:
    """Project data model."""
    id: int
    name: str
    description: str
    owner_id: int
    created_at: datetime
    is_active: bool = True
    tags: List[str] = None

    def __post_init__(self):
        """Initialize default values."""
        if self.tags is None:
            self.tags = []

    def add_tag(self, tag: str) -> None:
        """Add a tag to the project."""
        if tag not in self.tags:
            self.tags.append(tag)

    def remove_tag(self, tag: str) -> None:
        """Remove a tag from the project."""
        if tag in self.tags:
            self.tags.remove(tag)
"#;

        client.call_tool("create_file", json!({
            "file_path": models_file.to_string_lossy(),
            "content": models_content
        })).await?;
        created_files.push(models_file);

        // Create services.py
        let services_file = src_dir.join("services.py");
        let services_content = r#"
"""Business logic services."""

import asyncio
from datetime import datetime
from typing import List, Optional, Dict, Any
from .models import User, Project, UserRole


class UserService:
    """Service for managing users."""

    def __init__(self):
        self._users: Dict[int, User] = {}
        self._next_id = 1

    async def create_user(self, name: str, email: str, role: UserRole = UserRole.USER) -> User:
        """Create a new user."""
        user = User(
            id=self._next_id,
            name=name,
            email=email,
            role=role,
            created_at=datetime.now(),
            metadata={}
        )

        self._users[user.id] = user
        self._next_id += 1

        # Simulate async processing
        await asyncio.sleep(0.1)

        return user

    async def get_user(self, user_id: int) -> Optional[User]:
        """Get user by ID."""
        await asyncio.sleep(0.05)  # Simulate database lookup
        return self._users.get(user_id)

    async def update_user(self, user_id: int, **kwargs) -> Optional[User]:
        """Update user properties."""
        user = await self.get_user(user_id)
        if not user:
            return None

        for key, value in kwargs.items():
            if hasattr(user, key):
                setattr(user, key, value)

        return user

    async def delete_user(self, user_id: int) -> bool:
        """Delete a user."""
        if user_id in self._users:
            del self._users[user_id]
            return True
        return False

    async def list_users(self, role: Optional[UserRole] = None) -> List[User]:
        """List all users, optionally filtered by role."""
        users = list(self._users.values())

        if role:
            users = [user for user in users if user.role == role]

        await asyncio.sleep(0.1)  # Simulate database query
        return users

    def get_user_count(self) -> int:
        """Get total number of users."""
        return len(self._users)


class ProjectService:
    """Service for managing projects."""

    def __init__(self, user_service: UserService):
        self.user_service = user_service
        self._projects: Dict[int, Project] = {}
        self._next_id = 1

    async def create_project(self, name: str, description: str, owner_id: int) -> Optional[Project]:
        """Create a new project."""
        # Verify owner exists
        owner = await self.user_service.get_user(owner_id)
        if not owner:
            return None

        project = Project(
            id=self._next_id,
            name=name,
            description=description,
            owner_id=owner_id,
            created_at=datetime.now()
        )

        self._projects[project.id] = project
        self._next_id += 1

        return project

    async def get_project(self, project_id: int) -> Optional[Project]:
        """Get project by ID."""
        return self._projects.get(project_id)

    async def list_user_projects(self, user_id: int) -> List[Project]:
        """List projects owned by a user."""
        return [
            project for project in self._projects.values()
            if project.owner_id == user_id
        ]

    async def search_projects(self, query: str) -> List[Project]:
        """Search projects by name or description."""
        query_lower = query.lower()
        return [
            project for project in self._projects.values()
            if query_lower in project.name.lower() or
               query_lower in project.description.lower()
        ]
"#;

        client.call_tool("create_file", json!({
            "file_path": services_file.to_string_lossy(),
            "content": services_content
        })).await?;
        created_files.push(services_file);

        // Create utils.py
        let utils_file = src_dir.join("utils.py");
        let utils_content = r#"
"""Utility functions."""

import re
import hashlib
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Union


def validate_email(email: str) -> bool:
    """Validate email format."""
    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
    return bool(re.match(pattern, email))


def hash_password(password: str, salt: str = "") -> str:
    """Hash a password with optional salt."""
    combined = f"{password}{salt}"
    return hashlib.sha256(combined.encode()).hexdigest()


def format_timestamp(timestamp: datetime) -> str:
    """Format timestamp for display."""
    if timestamp.tzinfo is None:
        timestamp = timestamp.replace(tzinfo=timezone.utc)

    return timestamp.strftime("%Y-%m-%d %H:%M:%S UTC")


def sanitize_string(text: str, max_length: int = 255) -> str:
    """Sanitize and truncate string input."""
    if not isinstance(text, str):
        text = str(text)

    # Remove control characters
    text = re.sub(r'[\x00-\x1f\x7f-\x9f]', '', text)

    # Truncate if too long
    if len(text) > max_length:
        text = text[:max_length-3] + "..."

    return text.strip()


def deep_merge_dicts(dict1: Dict[str, Any], dict2: Dict[str, Any]) -> Dict[str, Any]:
    """Recursively merge two dictionaries."""
    result = dict1.copy()

    for key, value in dict2.items():
        if key in result and isinstance(result[key], dict) and isinstance(value, dict):
            result[key] = deep_merge_dicts(result[key], value)
        else:
            result[key] = value

    return result


def chunk_list(lst: List[Any], chunk_size: int) -> List[List[Any]]:
    """Split a list into chunks of specified size."""
    return [lst[i:i + chunk_size] for i in range(0, len(lst), chunk_size)]


class CircularBuffer:
    """A simple circular buffer implementation."""

    def __init__(self, size: int):
        self.size = size
        self.buffer: List[Any] = []
        self.index = 0

    def append(self, item: Any) -> None:
        """Add an item to the buffer."""
        if len(self.buffer) < self.size:
            self.buffer.append(item)
        else:
            self.buffer[self.index] = item

        self.index = (self.index + 1) % self.size

    def get_all(self) -> List[Any]:
        """Get all items in insertion order."""
        if len(self.buffer) < self.size:
            return self.buffer.copy()

        return self.buffer[self.index:] + self.buffer[:self.index]

    def clear(self) -> None:
        """Clear the buffer."""
        self.buffer.clear()
        self.index = 0
"#;

        client.call_tool("create_file", json!({
            "file_path": utils_file.to_string_lossy(),
            "content": utils_content
        })).await?;
        created_files.push(utils_file);

        // Create requirements.txt
        let requirements_file = workspace.path().join("requirements.txt");
        client.call_tool("create_file", json!({
            "file_path": requirements_file.to_string_lossy(),
            "content": "pytest>=7.0.0\npytest-asyncio>=0.21.0\nmypy>=1.0.0\nblack>=22.0.0\nflake8>=5.0.0\n"
        })).await?;
        created_files.push(requirements_file);

        Ok(created_files)
    }

    /// Create a Rust project for multi-language testing
    pub async fn create_rust_project(
        workspace: &TestWorkspace,
        client: &TestClient,
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

        client.call_tool("create_file", json!({
            "file_path": cargo_toml.to_string_lossy(),
            "content": cargo_content
        })).await?;
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

        client.call_tool("create_file", json!({
            "file_path": lib_file.to_string_lossy(),
            "content": lib_content
        })).await?;
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

        client.call_tool("create_file", json!({
            "file_path": models_file.to_string_lossy(),
            "content": models_content
        })).await?;
        created_files.push(models_file);

        Ok(created_files)
    }

    /// Create a monorepo project structure
    pub async fn create_monorepo_project(
        workspace: &TestWorkspace,
        client: &TestClient,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        // Create root package.json for workspace
        let root_package = workspace.path().join("package.json");
        let root_content = r#"
{
  "name": "test-monorepo",
  "version": "1.0.0",
  "private": true,
  "workspaces": [
    "packages/*",
    "apps/*"
  ],
  "scripts": {
    "build": "lerna run build",
    "test": "lerna run test",
    "lint": "lerna run lint"
  },
  "devDependencies": {
    "lerna": "^6.0.0",
    "typescript": "^4.9.0",
    "@typescript-eslint/eslint-plugin": "^5.0.0",
    "@typescript-eslint/parser": "^5.0.0",
    "eslint": "^8.0.0"
  }
}
"#;

        client.call_tool("create_file", json!({
            "file_path": root_package.to_string_lossy(),
            "content": root_content
        })).await?;
        created_files.push(root_package);

        // Create packages directory structure
        let packages_dir = workspace.path().join("packages");
        std::fs::create_dir_all(&packages_dir)?;

        let apps_dir = workspace.path().join("apps");
        std::fs::create_dir_all(&apps_dir)?;

        // Create shared library package
        let shared_dir = packages_dir.join("shared");
        std::fs::create_dir_all(&shared_dir)?;

        let shared_package = shared_dir.join("package.json");
        client.call_tool("create_file", json!({
            "file_path": shared_package.to_string_lossy(),
            "content": r#"
{
  "name": "@monorepo/shared",
  "version": "1.0.0",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "test": "jest"
  },
  "dependencies": {
    "uuid": "^9.0.0"
  },
  "devDependencies": {
    "typescript": "^4.9.0",
    "@types/uuid": "^9.0.0"
  }
}
"#
        })).await?;
        created_files.push(shared_package);

        let shared_index = shared_dir.join("index.ts");
        client.call_tool("create_file", json!({
            "file_path": shared_index.to_string_lossy(),
            "content": r#"
export interface BaseEntity {
    id: string;
    createdAt: Date;
    updatedAt: Date;
}

export interface User extends BaseEntity {
    name: string;
    email: string;
}

export interface Project extends BaseEntity {
    name: string;
    description: string;
    ownerId: string;
}

export function generateId(): string {
    return Math.random().toString(36).substr(2, 9);
}

export function validateEmail(email: string): boolean {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

export class Logger {
    static info(message: string): void {
        console.log(`[INFO] ${new Date().toISOString()} ${message}`);
    }

    static error(message: string): void {
        console.error(`[ERROR] ${new Date().toISOString()} ${message}`);
    }

    static warn(message: string): void {
        console.warn(`[WARN] ${new Date().toISOString()} ${message}`);
    }
}
"#
        })).await?;
        created_files.push(shared_index);

        // Create frontend app
        let frontend_dir = apps_dir.join("frontend");
        std::fs::create_dir_all(&frontend_dir)?;

        let frontend_package = frontend_dir.join("package.json");
        client.call_tool("create_file", json!({
            "file_path": frontend_package.to_string_lossy(),
            "content": r#"
{
  "name": "@monorepo/frontend",
  "version": "1.0.0",
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  },
  "dependencies": {
    "@monorepo/shared": "1.0.0",
    "next": "^13.0.0",
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.0.0",
    "typescript": "^4.9.0"
  }
}
"#
        })).await?;
        created_files.push(frontend_package);

        let frontend_app = frontend_dir.join("app.tsx");
        client.call_tool("create_file", json!({
            "file_path": frontend_app.to_string_lossy(),
            "content": r#"
import { User, Project, generateId, validateEmail, Logger } from '@monorepo/shared';
import React, { useState, useEffect } from 'react';

interface AppProps {
    initialUsers?: User[];
}

export function App({ initialUsers = [] }: AppProps) {
    const [users, setUsers] = useState<User[]>(initialUsers);
    const [projects, setProjects] = useState<Project[]>([]);

    useEffect(() => {
        Logger.info('App component mounted');
        loadInitialData();
    }, []);

    const loadInitialData = async () => {
        try {
            // Simulate API calls
            Logger.info('Loading initial data...');

            // In a real app, these would be API calls
            const mockUsers: User[] = [
                {
                    id: generateId(),
                    name: 'John Doe',
                    email: 'john@example.com',
                    createdAt: new Date(),
                    updatedAt: new Date()
                }
            ];

            setUsers(mockUsers);
            Logger.info(`Loaded ${mockUsers.length} users`);
        } catch (error) {
            Logger.error(`Failed to load data: ${error}`);
        }
    };

    const addUser = (name: string, email: string) => {
        if (!validateEmail(email)) {
            Logger.error('Invalid email format');
            return;
        }

        const newUser: User = {
            id: generateId(),
            name,
            email,
            createdAt: new Date(),
            updatedAt: new Date()
        };

        setUsers(prev => [...prev, newUser]);
        Logger.info(`Added user: ${name}`);
    };

    const createProject = (name: string, description: string, ownerId: string) => {
        const newProject: Project = {
            id: generateId(),
            name,
            description,
            ownerId,
            createdAt: new Date(),
            updatedAt: new Date()
        };

        setProjects(prev => [...prev, newProject]);
        Logger.info(`Created project: ${name}`);
    };

    return (
        <div>
            <h1>Monorepo Test App</h1>
            <div>
                <h2>Users ({users.length})</h2>
                {users.map(user => (
                    <div key={user.id}>
                        {user.name} - {user.email}
                    </div>
                ))}
            </div>
            <div>
                <h2>Projects ({projects.length})</h2>
                {projects.map(project => (
                    <div key={project.id}>
                        {project.name} - {project.description}
                    </div>
                ))}
            </div>
        </div>
    );
}

export default App;
"#
        })).await?;
        created_files.push(frontend_app);

        Ok(created_files)
    }

    /// Create an error-prone project for testing error handling
    pub async fn create_error_project(
        workspace: &TestWorkspace,
        client: &TestClient,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        // Create files with various types of errors

        // File with syntax errors
        let syntax_error_file = workspace.path().join("syntax_errors.ts");
        client.call_tool("create_file", json!({
            "file_path": syntax_error_file.to_string_lossy(),
            "content": r#"
// This file contains intentional syntax errors for testing

interface User {
    id: number;
    name: string;
    // Missing closing brace

function brokenFunction() {
    console.log("missing closing brace"
    // Missing closing parenthesis and brace

const unclosedArray = [1, 2, 3;
// Missing closing bracket

class BrokenClass {
    constructor(public id: number {
        // Missing closing parenthesis
    }
// Missing closing brace for class

export { User, brokenFunction, BrokenClass;
// Should work despite errors above
"#
        })).await?;
        created_files.push(syntax_error_file);

        // File with type errors
        let type_error_file = workspace.path().join("type_errors.ts");
        client.call_tool("create_file", json!({
            "file_path": type_error_file.to_string_lossy(),
            "content": r#"
// This file contains intentional type errors

interface User {
    id: number;
    name: string;
}

function processUser(user: User): string {
    // Type error: accessing non-existent property
    return user.nonExistentProperty;
}

function addNumbers(a: number, b: number): number {
    // Type error: returning string instead of number
    return "not a number";
}

const user: User = {
    id: "should be number", // Type error
    name: 123, // Type error
    extraProperty: "not allowed" // Type error
};

// Type error: passing wrong types
const result = addNumbers("not", "numbers");

// Valid code mixed with errors
export function validFunction(x: number): number {
    return x * 2;
}

export const validConstant = "this works";
"#
        })).await?;
        created_files.push(type_error_file);

        // File with import errors
        let import_error_file = workspace.path().join("import_errors.ts");
        client.call_tool("create_file", json!({
            "file_path": import_error_file.to_string_lossy(),
            "content": r#"
// This file contains intentional import errors

import { NonExistentType } from './does-not-exist';
import { AnotherMissing } from './also-missing';
import { } from './empty-import';
import * as Missing from './missing-module';

// Circular import (if this file is imported elsewhere)
import { importErrorFile } from './import_errors';

// Using undefined imports
function useUndefinedTypes(param: NonExistentType): AnotherMissing {
    return Missing.someFunction(param);
}

// Valid imports that might work
import { validFunction } from './type_errors';

export function workingFunction(): number {
    return validFunction(42);
}
"#
        })).await?;
        created_files.push(import_error_file);

        Ok(created_files)
    }

    /// Create a performance test project with configurable complexity
    pub async fn create_performance_project(
        workspace: &TestWorkspace,
        client: &TestClient,
        complexity_level: usize,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        let base_count = complexity_level * 10;

        // Create many interface files
        for i in 0..base_count {
            let file_path = workspace.path().join(format!("perf_interface_{}.ts", i));
            let content = format!(r#"
export interface PerfInterface{i} {{
    id{i}: number;
    data{i}: string;
    nested{i}: {{
        value{i}: boolean;
        array{i}: number[];
        map{i}: Record<string, any>;
    }};
}}

export type Union{i} = 'type{i}A' | 'type{i}B' | 'type{i}C';

export interface Extended{i} extends PerfInterface{i} {{
    additional{i}: Union{i};
    computed{i}: () => string;
}}
"#, i = i);

            client.call_tool("create_file", json!({
                "file_path": file_path.to_string_lossy(),
                "content": content
            })).await?;

            created_files.push(file_path);
        }

        // Create implementation files that use the interfaces
        for i in 0..base_count / 2 {
            let file_path = workspace.path().join(format!("perf_impl_{}.ts", i));
            let imports = (0..5).map(|j| {
                let idx = (i * 5 + j) % base_count;
                format!("import {{ PerfInterface{idx}, Extended{idx}, Union{idx} }} from './perf_interface_{idx}';", idx = idx)
            }).collect::<Vec<_>>().join("\n");

            let content = format!(r#"
{imports}

export class PerfClass{i} {{
    private data: Map<number, any> = new Map();

    processData(items: any[]): any[] {{
        return items.map((item, index) => ({{
            ...item,
            processed: true,
            index,
            timestamp: Date.now()
        }}));
    }}

    async asyncOperation(): Promise<any[]> {{
        await new Promise(resolve => setTimeout(resolve, 1));
        return this.processData([]);
    }}

    complexComputation(input: number): number {{
        let result = input;
        for (let j = 0; j < 1000; j++) {{
            result = Math.sin(result) * Math.cos(result);
        }}
        return result;
    }}
}}
"#, imports = imports, i = i);

            client.call_tool("create_file", json!({
                "file_path": file_path.to_string_lossy(),
                "content": content
            })).await?;

            created_files.push(file_path);
        }

        Ok(created_files)
    }
}