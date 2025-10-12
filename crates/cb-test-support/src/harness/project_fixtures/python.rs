use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::path::PathBuf;

use super::ProjectFixtures;

impl ProjectFixtures {
    /// Create a Python project for multi-language testing
    pub async fn create_python_project(
        workspace: &TestWorkspace,
        client: &mut TestClient,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        // Create directory structure
        let src_dir = workspace.path().join("python_project");
        std::fs::create_dir_all(&src_dir)?;

        // Create __init__.py
        let init_file = src_dir.join("__init__.py");
        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": init_file.to_string_lossy(),
                    "content": "\"\"\"Python project for testing.\"\"\"\n__version__ = \"1.0.0\"\n"
                }),
            )
            .await?;
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

        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": services_file.to_string_lossy(),
                    "content": services_content
                }),
            )
            .await?;
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

        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": utils_file.to_string_lossy(),
                    "content": utils_content
                }),
            )
            .await?;
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
}
