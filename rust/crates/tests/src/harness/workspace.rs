use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// Manages a temporary directory for a test scenario.
/// Cleans up automatically when dropped.
pub struct TestWorkspace {
    pub temp_dir: TempDir,
}

impl TestWorkspace {
    /// Creates a new empty workspace.
    pub fn new() -> Self {
        Self {
            temp_dir: tempdir().expect("Failed to create temp dir"),
        }
    }

    /// Returns the root path of the workspace.
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Creates a file with content within the workspace.
    /// Automatically creates parent directories.
    pub fn create_file(&self, rel_path: &str, content: &str) {
        let file_path = self.path().join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dirs");
        }
        fs::write(file_path, content).expect("Failed to write file");
    }

    /// Creates a directory within the workspace.
    pub fn create_directory(&self, rel_path: &str) {
        let dir_path = self.path().join(rel_path);
        fs::create_dir_all(dir_path).expect("Failed to create directory");
    }

    /// Reads a file from the workspace.
    pub fn read_file(&self, rel_path: &str) -> String {
        let file_path = self.path().join(rel_path);
        fs::read_to_string(file_path).expect("Failed to read file")
    }

    /// Check if a file exists in the workspace.
    pub fn file_exists(&self, rel_path: &str) -> bool {
        self.path().join(rel_path).exists()
    }

    /// Get the absolute path to a file in the workspace.
    pub fn absolute_path(&self, rel_path: &str) -> PathBuf {
        self.path().join(rel_path)
    }

    /// Create a TypeScript configuration file.
    pub fn create_tsconfig(&self) {
        let tsconfig = serde_json::json!({
            "compilerOptions": {
                "target": "ES2022",
                "module": "ESNext",
                "moduleResolution": "node",
                "esModuleInterop": true,
                "allowSyntheticDefaultImports": true,
                "strict": true,
                "skipLibCheck": true,
                "forceConsistentCasingInFileNames": true,
                "resolveJsonModule": true,
                "isolatedModules": true,
                "noEmit": true
            },
            "include": ["src/**/*"],
            "exclude": ["node_modules"]
        });

        self.create_file(
            "tsconfig.json",
            &serde_json::to_string_pretty(&tsconfig).unwrap(),
        );
    }

    /// Create a package.json file for a TypeScript/JavaScript project.
    pub fn create_package_json(&self, name: &str) {
        let package_json = serde_json::json!({
            "name": name,
            "version": "1.0.0",
            "type": "module",
            "dependencies": {},
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        });

        self.create_file(
            "package.json",
            &serde_json::to_string_pretty(&package_json).unwrap(),
        );
    }

    /// Create a basic TypeScript project structure.
    pub fn setup_typescript_project(&self, name: &str) {
        self.create_package_json(name);
        self.create_tsconfig();
        self.create_directory("src");
    }

    /// Create a TypeScript project with LSP configuration
    pub fn setup_typescript_project_with_lsp(&self, name: &str) {
        self.setup_typescript_project(name);
        self.setup_lsp_config();
    }

    /// Create LSP configuration file for the workspace
    pub fn setup_lsp_config(&self) {
        // Use LspSetupHelper to create config with absolute paths
        crate::harness::LspSetupHelper::setup_lsp_config(self);
    }

    /// Create a Python project structure.
    pub fn setup_python_project(&self, name: &str) {
        self.create_pyproject_toml(name);
        self.create_requirements_txt();
        self.create_directory("src");
        self.create_file("src/__init__.py", "# Python project");
        self.create_file(
            "README.md",
            &format!("# {}\n\nA Python test project.", name),
        );
    }

    /// Create a pyproject.toml file for a Python project.
    pub fn create_pyproject_toml(&self, name: &str) {
        let content = format!(
            r#"
[build-system]
requires = ["setuptools>=61.0", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "{}"
version = "0.1.0"
description = "A test Python project"
readme = "README.md"
requires-python = ">=3.8"
dependencies = []

[project.optional-dependencies]
test = ["pytest>=7.0.0", "pytest-cov>=4.0.0"]
dev = ["black>=22.0.0", "isort>=5.0.0", "mypy>=1.0.0"]

[tool.setuptools.packages.find]
where = ["src"]

[tool.black]
line-length = 88
target-version = ['py38']

[tool.isort]
profile = "black"

[tool.mypy]
python_version = "3.8"
warn_return_any = true
warn_unused_configs = true
disallow_untyped_defs = true
"#,
            name
        );

        self.create_file("pyproject.toml", &content);
    }

    /// Create a requirements.txt file.
    pub fn create_requirements_txt(&self) {
        let content = r#"
# Core dependencies
requests>=2.25.0
pydantic>=1.8.0

# Development dependencies
pytest>=7.0.0
pytest-asyncio>=0.21.0
black>=22.0.0
isort>=5.0.0
mypy>=1.0.0
flake8>=5.0.0
"#;
        self.create_file("requirements.txt", content);
    }

    /// Create a Rust project structure.
    pub fn setup_rust_project(&self, name: &str) {
        self.create_cargo_toml(name);
        self.create_directory("src");
        self.create_file("src/lib.rs", "// Rust library");
        self.create_file("README.md", &format!("# {}\n\nA Rust test project.", name));
    }

    /// Create a Cargo.toml file for a Rust project.
    pub fn create_cargo_toml(&self, name: &str) {
        let content = format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"
description = "A test Rust project"
readme = "README.md"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
tokio = {{ version = "1.0", features = ["full"] }}
anyhow = "1.0"

[dev-dependencies]
tempfile = "3.0"
assert_cmd = "2.0"
predicates = "3.0"

[[bin]]
name = "{}"
path = "src/main.rs"

[lib]
name = "{}"
path = "src/lib.rs"
"#,
            name,
            name,
            name.replace("-", "_")
        );

        self.create_file("Cargo.toml", &content);
    }

    /// Create a monorepo workspace structure.
    pub fn setup_monorepo_workspace(&self, name: &str) {
        // Root package.json for workspace
        let workspace_package = serde_json::json!({
            "name": name,
            "version": "1.0.0",
            "private": true,
            "workspaces": [
                "packages/*",
                "apps/*"
            ],
            "scripts": {
                "build": "npm run build --workspaces",
                "test": "npm run test --workspaces",
                "lint": "npm run lint --workspaces"
            },
            "devDependencies": {
                "typescript": "^5.0.0",
                "eslint": "^8.0.0",
                "@typescript-eslint/eslint-plugin": "^6.0.0",
                "@typescript-eslint/parser": "^6.0.0"
            }
        });

        self.create_file(
            "package.json",
            &serde_json::to_string_pretty(&workspace_package).unwrap(),
        );

        // Create workspace directories
        self.create_directory("packages");
        self.create_directory("apps");
        self.create_directory("tools");

        // Create lerna.json
        let lerna_config = serde_json::json!({
            "version": "independent",
            "npmClient": "npm",
            "packages": [
                "packages/*",
                "apps/*"
            ]
        });

        self.create_file(
            "lerna.json",
            &serde_json::to_string_pretty(&lerna_config).unwrap(),
        );
    }

    /// Create a large file structure for performance testing.
    pub fn create_large_file_structure(&self, depth: usize, files_per_dir: usize) {
        self._create_large_structure("", depth, files_per_dir, 0);
    }

    fn _create_large_structure(
        &self,
        base_path: &str,
        remaining_depth: usize,
        files_per_dir: usize,
        level: usize,
    ) {
        if remaining_depth == 0 {
            return;
        }

        // Create files at current level
        for i in 0..files_per_dir {
            let file_name = format!("file_{}_{}.ts", level, i);
            let file_path = if base_path.is_empty() {
                file_name
            } else {
                format!("{}/{}", base_path, file_name)
            };

            let content = format!(
                r#"
// Generated file at level {} index {}
export interface Data{}_{}  {{
    id: number;
    value: string;
    level: {};
    index: {};
}}

export function process{}_{}(data: Data{}_{}): string {{
    return `Level {} Index {} - ${{data.value}}`;
}}

export const LEVEL_{}_{}  = {};
"#,
                level, i, level, i, level, i, level, i, level, i, level, i, level, i, level
            );

            self.create_file(&file_path, &content);
        }

        // Create subdirectories and recurse
        if remaining_depth > 1 {
            for i in 0..3 {
                // Create 3 subdirectories per level
                let dir_name = format!("level_{}_{}", level, i);
                let dir_path = if base_path.is_empty() {
                    dir_name.clone()
                } else {
                    format!("{}/{}", base_path, dir_name)
                };

                self.create_directory(&dir_path);
                self._create_large_structure(
                    &dir_path,
                    remaining_depth - 1,
                    files_per_dir,
                    level + 1,
                );
            }
        }
    }

    /// Create a project with intentional errors for error testing.
    pub fn setup_error_project(&self) {
        // Create TypeScript files with various types of errors
        self.create_file(
            "syntax_error.ts",
            r#"
// File with syntax errors
interface User {
    id: number;
    name: string;
    // Missing closing brace

function broken() {
    console.log("unclosed"
    // Missing closing parenthesis and brace
"#,
        );

        self.create_file(
            "type_error.ts",
            r#"
// File with type errors
interface User {
    id: number;
    name: string;
}

function processUser(user: User): string {
    return user.nonExistentProperty; // Type error
}

const user: User = {
    id: "not a number", // Type error
    name: 123 // Type error
};
"#,
        );

        self.create_file(
            "import_error.ts",
            r#"
// File with import errors
import { NonExistent } from './does-not-exist';
import { User } from './type_error'; // This should work

function useNonExistent(x: NonExistent): void {
    console.log(x);
}

export function validFunction(): string {
    return "this works";
}
"#,
        );

        // Create corrupted file
        let corrupted_path = self.path().join("corrupted.ts");
        std::fs::write(&corrupted_path, b"\xFF\xFE\xFD\x00Invalid UTF-8").unwrap();
    }

    /// Create a multi-language project structure.
    pub fn setup_multi_language_project(&self, name: &str) {
        // Root configuration
        self.create_file(
            "README.md",
            &format!(
                "# {}\n\nMulti-language test project with TypeScript, Python, and Rust.",
                name
            ),
        );

        // TypeScript part
        self.create_directory("typescript");
        let ts_package = serde_json::json!({
            "name": format!("{}-typescript", name),
            "version": "1.0.0",
            "type": "module",
            "scripts": {
                "build": "tsc",
                "test": "jest"
            },
            "dependencies": {
                "express": "^4.18.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0",
                "@types/express": "^4.17.0",
                "jest": "^29.0.0"
            }
        });

        self.create_file(
            "typescript/package.json",
            &serde_json::to_string_pretty(&ts_package).unwrap(),
        );

        self.create_file(
            "typescript/tsconfig.json",
            r#"
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "node",
    "esModuleInterop": true,
    "strict": true,
    "outDir": "./dist"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#,
        );

        self.create_file(
            "typescript/src/index.ts",
            r#"
import express from 'express';

const app = express();
const PORT = process.env.PORT || 3000;

app.get('/', (req, res) => {
    res.json({ message: 'Hello from TypeScript!' });
});

app.listen(PORT, () => {
    console.log(`Server running on port ${PORT}`);
});
"#,
        );

        // Python part
        self.create_directory("python");
        self.create_file(
            "python/pyproject.toml",
            &format!(
                r#"
[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[project]
name = "{}-python"
version = "0.1.0"
description = "Python component"
dependencies = [
    "fastapi>=0.104.0",
    "uvicorn>=0.24.0"
]

[project.optional-dependencies]
dev = ["pytest>=7.0.0", "black>=22.0.0"]
"#,
                name
            ),
        );

        self.create_file(
            "python/main.py",
            r#"
from fastapi import FastAPI
import uvicorn

app = FastAPI()

@app.get("/")
async def root():
    return {"message": "Hello from Python!"}

@app.get("/health")
async def health():
    return {"status": "healthy"}

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
"#,
        );

        // Rust part
        self.create_directory("rust");
        self.create_file(
            "rust/Cargo.toml",
            &format!(
                r#"
[package]
name = "{}-rust"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = {{ version = "1.0", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#,
                name
            ),
        );

        self.create_file(
            "rust/src/main.rs",
            r#"
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    text: String,
    timestamp: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = Message {
        text: "Hello from Rust!".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    };

    println!("{}", serde_json::to_string_pretty(&message)?);

    Ok(())
}
"#,
        );

        self.create_file(
            "rust/src/lib.rs",
            r#"
//! Multi-language project Rust component

pub mod models {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct SharedData {
        pub id: String,
        pub value: i32,
        pub metadata: std::collections::HashMap<String, String>,
    }

    impl SharedData {
        pub fn new(id: String, value: i32) -> Self {
            Self {
                id,
                value,
                metadata: std::collections::HashMap::new(),
            }
        }
    }
}

pub use models::*;
"#,
        );
    }
}

impl Default for TestWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
