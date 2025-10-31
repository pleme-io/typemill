//! TypeScript project factory implementation
//!
//! Handles creation of new TypeScript/JavaScript packages with proper workspace integration.

use mill_lang_common::project_factory::{
    derive_package_name, resolve_package_path, update_workspace_manifest,
    validate_package_path_not_exists, write_project_file, WorkspaceManifestDetector,
};
use mill_plugin_api::project_factory::{
    CreatePackageConfig, CreatePackageResult, PackageInfo, PackageType, ProjectFactory, Template,
};
use mill_plugin_api::{PluginError, PluginResult, WorkspaceSupport};
use std::fs;
use std::path::Path;
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

        // Resolve and validate paths
        let workspace_root = Path::new(&config.workspace_root);
        let package_path = resolve_package_path(workspace_root, &config.package_path)?;
        validate_package_path_not_exists(&package_path)?;

        // Derive package name
        let package_name = derive_package_name(&package_path)?;

        debug!(package_name = %package_name, "Derived package name");

        // Create directory structure
        create_directory_structure(&package_path)?;

        // Generate and write files
        let mut created_files = Vec::new();

        // Write package.json
        let package_json_path = package_path.join("package.json");
        let package_json_content = generate_package_json(&package_name, config.package_type);
        write_project_file(&package_json_path, &package_json_content)?;
        created_files.push(package_json_path.display().to_string());

        // Write tsconfig.json
        let tsconfig_path = package_path.join("tsconfig.json");
        let tsconfig_content = generate_tsconfig();
        write_project_file(&tsconfig_path, &tsconfig_content)?;
        created_files.push(tsconfig_path.display().to_string());

        // Write entry file
        let entry_file_path = package_path.join(entry_file(config.package_type));
        let entry_content = generate_entry_content(&package_name, config.package_type);
        write_project_file(&entry_file_path, &entry_content)?;
        created_files.push(entry_file_path.display().to_string());

        // Create baseline files (README, .gitignore, tests) for minimal template
        let baseline = create_baseline_files(&package_path, &package_name)?;
        created_files.extend(baseline);

        // Create additional files for full template (.eslintrc.json)
        if matches!(config.template, Template::Full) {
            let additional = create_full_template_extras(&package_path)?;
            created_files.extend(additional);
        }

        // Update workspace if requested
        let workspace_updated = if config.add_to_workspace {
            let workspace_support = crate::workspace_support::TypeScriptWorkspaceSupport;
            update_workspace_manifest(
                workspace_root,
                &package_path,
                "package.json",
                &TypeScriptManifestDetector,
                |content, member| workspace_support.add_workspace_member(content, member),
            )?
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

/// Workspace manifest detector for TypeScript projects
struct TypeScriptManifestDetector;

impl WorkspaceManifestDetector for TypeScriptManifestDetector {
    fn is_workspace_manifest(&self, content: &str) -> bool {
        crate::workspace_support::TypeScriptWorkspaceSupport.is_workspace_manifest(content)
    }
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

fn create_baseline_files(package_path: &Path, package_name: &str) -> PluginResult<Vec<String>> {
    let mut created = Vec::new();

    // README.md
    let readme_path = package_path.join("README.md");
    let readme_content = format!(
        "# {}\n\nTODO: Add project description\n\n## Installation\n\n```bash\nnpm install {}\n```\n\n## Usage\n\nTODO: Add usage examples\n",
        package_name, package_name
    );
    write_project_file(&readme_path, &readme_content)?;
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
    write_project_file(&gitignore_path, gitignore_content)?;
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
    write_project_file(&test_path, &test_content)?;
    created.push(test_path.display().to_string());

    Ok(created)
}

fn create_full_template_extras(package_path: &Path) -> PluginResult<Vec<String>> {
    let mut created = Vec::new();

    // .eslintrc.json (Full template only)
    let eslintrc_path = package_path.join(".eslintrc.json");
    let eslintrc_content = r#"{
  "parser": "@typescript-eslint/parser",
  "extends": [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended"
  ],
  "parserOptions": {
    "ecmaVersion": 2022,
    "sourceType": "module"
  },
  "rules": {}
}
"#;
    write_project_file(&eslintrc_path, eslintrc_content)?;
    created.push(eslintrc_path.display().to_string());

    Ok(created)
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
        assert!(content.contains("\"target\": \"ES2022\""));
        assert!(content.contains("\"lib\": [\"ES2022\"]"));
        assert!(content.contains("\"outDir\": \"./dist\""));
        assert!(content.contains("\"strict\": true"));
    }
}
