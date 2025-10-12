use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::path::PathBuf;

use super::ProjectFixtures;

impl ProjectFixtures {
    /// Create a monorepo project structure
    pub async fn create_monorepo_project(
        workspace: &TestWorkspace,
        client: &mut TestClient,
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

        client
            .call_tool(
                "create_file",
                json!({
                    "file_path": root_package.to_string_lossy(),
                    "content": root_content
                }),
            )
            .await?;
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
        client
            .call_tool(
                "create_file",
                json!({
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
                }),
            )
            .await?;
        created_files.push(shared_package);

        let shared_index = shared_dir.join("index.ts");
        client
            .call_tool(
                "create_file",
                json!({
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
                }),
            )
            .await?;
        created_files.push(shared_index);

        // Create frontend app
        let frontend_dir = apps_dir.join("frontend");
        std::fs::create_dir_all(&frontend_dir)?;

        let frontend_package = frontend_dir.join("package.json");
        client
            .call_tool(
                "create_file",
                json!({
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
                }),
            )
            .await?;
        created_files.push(frontend_package);

        let frontend_app = frontend_dir.join("app.tsx");
        client
            .call_tool(
                "create_file",
                json!({
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
                }),
            )
            .await?;
        created_files.push(frontend_app);

        Ok(created_files)
    }
}
