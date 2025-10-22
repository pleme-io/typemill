//! Tool handler for fine-grained dependency management.

use super::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct UpdateDependenciesArgs {
    file_path: String,
    add_dependencies: Option<HashMap<String, String>>,
    add_dev_dependencies: Option<HashMap<String, String>>,
    remove_dependencies: Option<Vec<String>>,
    update_version: Option<String>,
    add_scripts: Option<HashMap<String, String>>,
    remove_scripts: Option<Vec<String>>,
    dry_run: Option<bool>,
}

pub struct DependencyHandler;

impl DependencyHandler {
    pub fn new() -> Self {
        Self
    }

    async fn handle_package_json(&self, args: &UpdateDependenciesArgs) -> ServerResult<()> {
        debug!(file_path = %args.file_path, "Handling package.json dependency update");

        let content =
            fs::read_to_string(&args.file_path)
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Failed to read file: {}", e),
                })?;

        let mut json_val: Value =
            serde_json::from_str(&content).map_err(|e| ServerError::Runtime {
                message: format!("Failed to parse JSON: {}", e),
            })?;

        if let Some(map) = json_val.as_object_mut() {
            // Update version
            if let Some(version) = &args.update_version {
                map.insert("version".to_string(), json!(version));
            }

            // Handle dependencies
            if let Some(deps_to_add) = &args.add_dependencies {
                let deps = map
                    .entry("dependencies")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .unwrap();
                for (name, version) in deps_to_add {
                    deps.insert(name.clone(), json!(version));
                }
            }

            // Handle devDependencies
            if let Some(dev_deps_to_add) = &args.add_dev_dependencies {
                let dev_deps = map
                    .entry("devDependencies")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .unwrap();
                for (name, version) in dev_deps_to_add {
                    dev_deps.insert(name.clone(), json!(version));
                }
            }

            // Handle removing dependencies
            if let Some(deps_to_remove) = &args.remove_dependencies {
                if let Some(deps) = map.get_mut("dependencies").and_then(|v| v.as_object_mut()) {
                    for name in deps_to_remove {
                        deps.remove(name);
                    }
                }
                if let Some(dev_deps) = map
                    .get_mut("devDependencies")
                    .and_then(|v| v.as_object_mut())
                {
                    for name in deps_to_remove {
                        dev_deps.remove(name);
                    }
                }
            }

            // Handle scripts
            if let Some(scripts_to_add) = &args.add_scripts {
                let scripts = map
                    .entry("scripts")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .unwrap();
                for (name, command) in scripts_to_add {
                    scripts.insert(name.clone(), json!(command));
                }
            }
            if let Some(scripts_to_remove) = &args.remove_scripts {
                if let Some(scripts) = map.get_mut("scripts").and_then(|v| v.as_object_mut()) {
                    for name in scripts_to_remove {
                        scripts.remove(name);
                    }
                }
            }
        }

        let updated_content =
            serde_json::to_string_pretty(&json_val).map_err(|e| ServerError::Runtime {
                message: format!("Failed to serialize JSON: {}", e),
            })?;

        fs::write(&args.file_path, updated_content)
            .await
            .map_err(|e| ServerError::Runtime {
                message: format!("Failed to write file: {}", e),
            })?;
        Ok(())
    }

    async fn handle_cargo_toml(&self, args: &UpdateDependenciesArgs) -> ServerResult<()> {
        debug!(file_path = %args.file_path, "Handling Cargo.toml dependency update");

        let content =
            fs::read_to_string(&args.file_path)
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Failed to read file: {}", e),
                })?;

        let mut toml_val: toml::Value =
            toml::from_str(&content).map_err(|e| ServerError::Runtime {
                message: format!("Failed to parse TOML: {}", e),
            })?;

        if let Some(table) = toml_val.as_table_mut() {
            // Update version
            if let Some(version) = &args.update_version {
                if let Some(package) = table.get_mut("package").and_then(|v| v.as_table_mut()) {
                    package.insert("version".to_string(), toml::Value::String(version.clone()));
                }
            }

            // Handle dependencies
            if let Some(deps_to_add) = &args.add_dependencies {
                let deps = table
                    .entry("dependencies".to_string())
                    .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
                    .as_table_mut()
                    .unwrap();
                for (name, version) in deps_to_add {
                    deps.insert(name.clone(), toml::Value::String(version.clone()));
                }
            }

            // Handle dev-dependencies
            if let Some(dev_deps_to_add) = &args.add_dev_dependencies {
                let dev_deps = table
                    .entry("dev-dependencies".to_string())
                    .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
                    .as_table_mut()
                    .unwrap();
                for (name, version) in dev_deps_to_add {
                    dev_deps.insert(name.clone(), toml::Value::String(version.clone()));
                }
            }

            // Handle removing dependencies
            if let Some(deps_to_remove) = &args.remove_dependencies {
                if let Some(deps) = table.get_mut("dependencies").and_then(|v| v.as_table_mut()) {
                    for name in deps_to_remove {
                        deps.remove(name);
                    }
                }
                if let Some(dev_deps) = table
                    .get_mut("dev-dependencies")
                    .and_then(|v| v.as_table_mut())
                {
                    for name in deps_to_remove {
                        dev_deps.remove(name);
                    }
                }
            }
        }

        let updated_content =
            toml::to_string_pretty(&toml_val).map_err(|e| ServerError::Runtime {
                message: format!("Failed to serialize TOML: {}", e),
            })?;

        fs::write(&args.file_path, updated_content)
            .await
            .map_err(|e| ServerError::Runtime {
                message: format!("Failed to write file: {}", e),
            })?;
        Ok(())
    }

    async fn handle_requirements_txt(&self, args: &UpdateDependenciesArgs) -> ServerResult<()> {
        debug!(file_path = %args.file_path, "Handling requirements.txt dependency update");

        let content =
            fs::read_to_string(&args.file_path)
                .await
                .map_err(|e| ServerError::Runtime {
                    message: format!("Failed to read file: {}", e),
                })?;

        let mut lines: Vec<String> = content.lines().map(String::from).collect();

        // Handle removing dependencies
        if let Some(deps_to_remove) = &args.remove_dependencies {
            lines.retain(|line| !deps_to_remove.iter().any(|dep| line.starts_with(dep)));
        }

        // Handle adding dependencies
        if let Some(deps_to_add) = &args.add_dependencies {
            for (name, version) in deps_to_add {
                lines.push(format!("{}=={}", name, version));
            }
        }

        let updated_content = lines.join("\n");
        fs::write(&args.file_path, updated_content)
            .await
            .map_err(|e| ServerError::Runtime {
                message: format!("Failed to write file: {}", e),
            })?;
        Ok(())
    }
}

impl Default for DependencyHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for DependencyHandler {
    fn tool_names(&self) -> &[&str] {
        &["update_dependencies"]
    }

    async fn handle_tool_call(
        &self,
        _context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args: UpdateDependenciesArgs =
            serde_json::from_value(tool_call.arguments.clone().unwrap_or_default()).map_err(
                |e| ServerError::InvalidRequest(format!("Invalid update_dependencies args: {}", e)),
            )?;

        let is_dry_run = args.dry_run.unwrap_or(false);

        if is_dry_run {
            // For dry run, return preview without modifying files
            return Ok(json!({
                "preview": true,
                "changes": {
                    "file_path": args.file_path,
                    "add_dependencies": args.add_dependencies,
                    "add_dev_dependencies": args.add_dev_dependencies,
                    "remove_dependencies": args.remove_dependencies,
                    "update_version": args.update_version,
                    "add_scripts": args.add_scripts,
                    "remove_scripts": args.remove_scripts
                }
            }));
        }

        match Path::new(&args.file_path)
            .file_name()
            .and_then(|s| s.to_str())
        {
            Some("package.json") => self.handle_package_json(&args).await?,
            Some("Cargo.toml") => self.handle_cargo_toml(&args).await?,
            Some("requirements.txt") => self.handle_requirements_txt(&args).await?,
            _ => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unsupported file for dependency update: {}",
                    args.file_path
                )))
            }
        };

        Ok(json!({ "success": true }))
    }
}
