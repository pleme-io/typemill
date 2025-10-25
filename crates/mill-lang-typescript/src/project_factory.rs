//! TypeScript project factory implementation
//!
//! Handles creation of new TypeScript/JavaScript packages with proper workspace integration.

use mill_plugin_api::project_factory::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, PackageType, ProjectFactory, Template,
};
use mill_plugin_api::{PluginError, PluginResult};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error};

/// TypeScript project factory implementation
#[derive(Default)]
pub struct TypeScriptProjectFactory;

impl ProjectFactory for TypeScriptProjectFactory {
    fn create_package(&self, config: &CreatePackageConfig) -> PluginResult<CreatePackageResult> {
        debug!(
            package_path = %config.package_path,
            package_type = ?config.package_type,
            template = ?config.template,
            "Creating TypeScript package"
        );

        // Resolve paths
        let workspace_root = Path::new(&config.workspace_root);
        let package_path = resolve_package_path(workspace_root, &config.package_path)?;

        // Validate package path doesn't exist
        if package_path.exists() {
            return Err(PluginError::invalid_input(format!(
                "Package already exists at {}",
                package_path.display()
            )));
        }

        // Derive package name
        let package_name = package_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                PluginError::invalid_input(format!(
                    "Invalid package path: {}",
                    package_path.display()
                ))
            })?
            .to_string();

        debug!(package_name = %package_name, "Derived package name");

        // Create directory structure
        create_directory_structure(&package_path)?;

        // Generate and write files
        let mut created_files = Vec::new();

        // Write package.json
        let package_json_path = package_path.join("package.json");
        let package_json_content = generate_package_json(&package_name, config.package_type);
        write_file(&package_json_path, &package_json_content)?;
        created_files.push(package_json_path.display().to_string());

        // Write tsconfig.json
        let tsconfig_path = package_path.join("tsconfig.json");
        let tsconfig_content = generate_tsconfig();
        write_file(&tsconfig_path, &tsconfig_content)?;
        created_files.push(tsconfig_path.display().to_string());

        // Write entry file
        let entry_file_path = package_path.join(entry_file(config.package_type));
        let entry_content = generate_entry_content(&package_name, config.package_type);
        write_file(&entry_file_path, &entry_content)?;
        created_files.push(entry_file_path.display().to_string());

        // Create additional files for full template
        if matches!(config.template, Template::Full) {
            let additional = create_full_template(&package_path, &package_name)?;
            created_files.extend(additional);
        }

        // Update workspace if requested
        let workspace_updated = if config.add_to_workspace {
            update_workspace_members(workspace_root, &package_path)?
        } else {
            false
        };

        Ok(CreatePackageResult {
            created_files,
            workspace_updated,
            package_info: PackageInfo {
                name: package_name,
                version: "0.1.0".to_string(),
                manifest_path: package_json_path.display().to_string(),
            },
        })
    }
}

// Helper functions

fn resolve_package_path(workspace_root: &Path, package_path: &str) -> PluginResult<PathBuf> {
    let path = Path::new(package_path);

    // Reject paths with parent directory components to prevent traversal
    use std::path::Component;
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(PluginError::invalid_input(format!(
                "Package path cannot contain '..' components: {}",
                package_path
            )));
        }
    }

    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    // Canonicalize both paths for comparison (handles symlinks, . and .. after join)
    let canonical_root = workspace_root.canonicalize().map_err(|e| {
        PluginError::internal(format!("Failed to canonicalize workspace root: {}", e))
    })?;

    // For the resolved path, we need to canonicalize the parent since the target doesn't exist yet
    let canonical_resolved = if let Some(parent) = resolved.parent() {
        if parent.exists() {
            let canonical_parent = parent.canonicalize().map_err(|e| {
                PluginError::internal(format!("Failed to canonicalize parent directory: {}", e))
            })?;
            resolved.file_name()
                .map(|name| canonical_parent.join(name))
                .ok_or_else(|| PluginError::invalid_input("Invalid package path"))?
        } else {
            // Parent doesn't exist yet, we'll create it - just verify it would be within workspace
            resolved.clone()
        }
    } else {
        resolved.clone()
    };

    if !canonical_resolved.starts_with(&canonical_root) {
        return Err(PluginError::invalid_input(format!(
            "Package path {} is outside workspace",
            package_path
        )));
    }

    Ok(resolved)
}

fn create_directory_structure(package_path: &Path) -> PluginResult<()> {
    debug!(package_path = %package_path.display(), "Creating directory structure");

    fs::create_dir_all(package_path).map_err(|e| {
        error!(error = %e, package_path = %package_path.display(), "Failed to create package directory");
        PluginError::internal(format!("Failed to create directory: {}", e))
    })?;

    let src_dir = package_path.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| {
        error!(error = %e, src_dir = %src_dir.display(), "Failed to create src directory");
        PluginError::internal(format!("Failed to create src directory: {}", e))
    })?;

    Ok(())
}

fn entry_file(package_type: PackageType) -> &'static str {
    match package_type {
        PackageType::Library => "src/index.ts",
        PackageType::Binary => "src/main.ts",
    }
}

fn generate_package_json(package_name: &str, package_type: PackageType) -> String {
    match package_type {
        PackageType::Library => format!(
            r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "TODO: Add package description",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {{
    "build": "tsc",
    "test": "echo \"Error: no test specified\" && exit 1",
    "lint": "eslint src --ext .ts"
  }},
  "keywords": [],
  "author": "",
  "license": "ISC",
  "devDependencies": {{
    "typescript": "^5.0.0"
  }}
}}
"#,
            package_name
        ),
        PackageType::Binary => format!(
            r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "TODO: Add package description",
  "bin": {{
    "{}": "dist/main.js"
  }},
  "scripts": {{
    "build": "tsc",
    "start": "node dist/main.js",
    "test": "echo \"Error: no test specified\" && exit 1",
    "lint": "eslint src --ext .ts"
  }},
  "keywords": [],
  "author": "",
  "license": "ISC",
  "devDependencies": {{
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }}
}}
"#,
            package_name, package_name
        ),
    }
}

fn generate_tsconfig() -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "commonjs",
    "lib": ["ES2022"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#
    .to_string()
}

fn generate_entry_content(package_name: &str, package_type: PackageType) -> String {
    match package_type {
        PackageType::Library => format!(
            r#"/**
 * {} library
 *
 * TODO: Add library description
 */

export function hello(): string {{
  return "Hello from {}!";
}}
"#,
            package_name, package_name
        ),
        PackageType::Binary => r#"#!/usr/bin/env node

function main(): void {
  console.log("Hello, world!");
}

main();
"#
        .to_string(),
    }
}

fn write_file(path: &Path, content: &str) -> PluginResult<()> {
    debug!(path = %path.display(), "Writing file");
    fs::write(path, content).map_err(|e| {
        error!(error = %e, path = %path.display(), "Failed to write file");
        PluginError::internal(format!("Failed to write file: {}", e))
    })
}

fn create_full_template(package_path: &Path, package_name: &str) -> PluginResult<Vec<String>> {
    let mut created = Vec::new();

    // README.md
    let readme_path = package_path.join("README.md");
    let readme_content = format!(
        "# {}\n\nTODO: Add project description\n\n## Installation\n\n```bash\nnpm install {}\n```\n\n## Usage\n\nTODO: Add usage examples\n",
        package_name, package_name
    );
    write_file(&readme_path, &readme_content)?;
    created.push(readme_path.display().to_string());

    // .gitignore
    let gitignore_path = package_path.join(".gitignore");
    let gitignore_content = r#"# Dependencies
node_modules/

# Build output
dist/

# IDE
.vscode/
.idea/

# OS
.DS_Store
Thumbs.db

# Logs
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Environment
.env
.env.local
"#;
    write_file(&gitignore_path, gitignore_content)?;
    created.push(gitignore_path.display().to_string());

    // tests/index.test.ts
    let tests_dir = package_path.join("tests");
    fs::create_dir_all(&tests_dir)
        .map_err(|e| PluginError::internal(format!("Failed to create tests directory: {}", e)))?;

    let test_path = tests_dir.join("index.test.ts");
    let test_content = format!(
        r#"/**
 * Tests for {}
 */

describe("{}", () => {{
  it("should work", () => {{
    // TODO: Add tests
    expect(true).toBe(true);
  }});
}});
"#,
        package_name, package_name
    );
    write_file(&test_path, &test_content)?;
    created.push(test_path.display().to_string());

    // .eslintrc.json
    let eslintrc_path = package_path.join(".eslintrc.json");
    let eslintrc_content = r#"{
  "parser": "@typescript-eslint/parser",
  "extends": [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended"
  ],
  "parserOptions": {
    "ecmaVersion": 2020,
    "sourceType": "module"
  },
  "rules": {}
}
"#;
    write_file(&eslintrc_path, eslintrc_content)?;
    created.push(eslintrc_path.display().to_string());

    Ok(created)
}

fn update_workspace_members(workspace_root: &Path, package_path: &Path) -> PluginResult<bool> {
    // Find workspace package.json
    let workspace_manifest = find_workspace_manifest(workspace_root)?;

    debug!(
        workspace_manifest = %workspace_manifest.display(),
        "Found workspace manifest"
    );

    // Read manifest
    let content = fs::read_to_string(&workspace_manifest).map_err(|e| {
        error!(
            error = %e,
            workspace_manifest = %workspace_manifest.display(),
            "Failed to read workspace manifest"
        );
        PluginError::internal(format!("Failed to read workspace package.json: {}", e))
    })?;

    // Calculate relative path
    let workspace_dir = workspace_manifest
        .parent()
        .ok_or_else(|| PluginError::internal("Invalid workspace manifest path"))?;

    let relative_path = pathdiff::diff_paths(package_path, workspace_dir)
        .ok_or_else(|| PluginError::internal("Failed to calculate relative path"))?;

    // Normalize to forward slashes for cross-platform compatibility
    // npm/yarn/pnpm expect forward slashes even on Windows
    let member_str = relative_path
        .to_string_lossy()
        .replace('\\', "/");

    debug!(member = %member_str, "Adding workspace member");

    // Use workspace support to add member
    use mill_plugin_api::WorkspaceSupport;
    let workspace_support = crate::workspace_support::TypeScriptWorkspaceSupport;
    let updated_content = workspace_support.add_workspace_member(&content, &member_str);

    if updated_content != content {
        // Write updated manifest
        fs::write(&workspace_manifest, &updated_content).map_err(|e| {
            error!(
                error = %e,
                workspace_manifest = %workspace_manifest.display(),
                "Failed to write workspace manifest"
            );
            PluginError::internal(format!("Failed to write workspace package.json: {}", e))
        })?;

        Ok(true)
    } else {
        Ok(false)
    }
}

fn find_workspace_manifest(workspace_root: &Path) -> PluginResult<PathBuf> {
    use mill_plugin_api::WorkspaceSupport;
    let workspace_support = crate::workspace_support::TypeScriptWorkspaceSupport;
    let mut current = workspace_root.to_path_buf();

    loop {
        // Check for pnpm-workspace.yaml first
        let pnpm_manifest = current.join("pnpm-workspace.yaml");
        if pnpm_manifest.exists() {
            let content = fs::read_to_string(&pnpm_manifest).map_err(|e| {
                PluginError::internal(format!("Failed to read pnpm-workspace.yaml: {}", e))
            })?;

            if workspace_support.is_workspace_manifest(&content) {
                // Return package.json path for pnpm workspaces (for consistency)
                // but we know pnpm-workspace.yaml exists
                let package_json = current.join("package.json");
                if package_json.exists() {
                    return Ok(package_json);
                }
            }
        }

        // Check for package.json with workspaces
        let manifest = current.join("package.json");
        if manifest.exists() {
            let content = fs::read_to_string(&manifest).map_err(|e| {
                PluginError::internal(format!("Failed to read package.json: {}", e))
            })?;

            // Check if it's a workspace manifest using workspace_support
            if workspace_support.is_workspace_manifest(&content) {
                return Ok(manifest);
            }
        }

        // Move up
        current = current
            .parent()
            .ok_or_else(|| {
                PluginError::invalid_input("No workspace manifest found in hierarchy")
            })?
            .to_path_buf();

        // Stop at root
        if current == current.parent().unwrap_or(&current) {
            break;
        }
    }

    Err(PluginError::invalid_input(
        "No workspace manifest found (package.json or pnpm-workspace.yaml)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_package_json_lib() {
        let content = generate_package_json("test-package", PackageType::Library);
        assert!(content.contains("\"name\": \"test-package\""));
        assert!(content.contains("\"version\": \"0.1.0\""));
        assert!(content.contains("\"main\": \"dist/index.js\""));
        assert!(content.contains("\"types\": \"dist/index.d.ts\""));
        assert!(!content.contains("\"bin\""));
    }

    #[test]
    fn test_generate_package_json_bin() {
        let content = generate_package_json("test-bin", PackageType::Binary);
        assert!(content.contains("\"name\": \"test-bin\""));
        assert!(content.contains("\"bin\""));
        assert!(content.contains("\"test-bin\": \"dist/main.js\""));
        assert!(!content.contains("\"main\""));
    }

    #[test]
    fn test_entry_file() {
        assert_eq!(entry_file(PackageType::Library), "src/index.ts");
        assert_eq!(entry_file(PackageType::Binary), "src/main.ts");
    }

    #[test]
    fn test_generate_tsconfig() {
        let content = generate_tsconfig();
        assert!(content.contains("\"target\": \"ES2020\""));
        assert!(content.contains("\"outDir\": \"./dist\""));
        assert!(content.contains("\"strict\": true"));
    }
}
