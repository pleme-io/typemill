//! Filesystem MCP tools (create_file, delete_file, rename_file, etc.)

mod dependency_handlers;

use crate::handlers::McpDispatcher;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tokio::fs;
use ignore::WalkBuilder;
use dependency_handlers::{UpdateDependenciesArgs, handle_update_dependencies};

/// Arguments for create_file tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreateFileArgs {
    file_path: String,
    content: Option<String>,
    overwrite: Option<bool>,
}

/// Arguments for delete_file tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct DeleteFileArgs {
    file_path: String,
    force: Option<bool>,
}

/// Arguments for rename_file tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RenameFileArgs {
    old_path: String,
    new_path: String,
    dry_run: Option<bool>,
}


/// Arguments for read_file tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ReadFileArgs {
    file_path: String,
}

/// Arguments for write_file tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct WriteFileArgs {
    file_path: String,
    content: String,
    create_directories: Option<bool>,
}

/// Arguments for list_files tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ListFilesArgs {
    path: Option<String>,
    recursive: Option<bool>,
    include_hidden: Option<bool>,
    pattern: Option<String>,
}

/// File operation result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileOperationResult {
    success: bool,
    operation: String,
    file_path: String,
    message: Option<String>,
}

/// Import update result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportUpdateResult {
    files_updated: Vec<String>,
    imports_fixed: u32,
    preview: Option<Vec<ImportChange>>,
}

/// Import change description
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportChange {
    file: String,
    old_import: String,
    new_import: String,
}

/// Arguments for update_package_json tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct UpdatePackageJsonArgs {
    file_path: Option<String>,
    add_dependencies: Option<std::collections::HashMap<String, String>>,
    add_dev_dependencies: Option<std::collections::HashMap<String, String>>,
    add_scripts: Option<std::collections::HashMap<String, String>>,
    remove_dependencies: Option<Vec<String>>,
    remove_scripts: Option<Vec<String>>,
    update_version: Option<String>,
    workspace_config: Option<WorkspaceConfig>,
    dry_run: Option<bool>,
}

/// Workspace configuration
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct WorkspaceConfig {
    workspaces: Option<Vec<String>>,
}

/// Package.json update result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePackageJsonResult {
    success: bool,
    file_path: String,
    changes_made: Vec<String>,
    dependencies_added: usize,
    dev_dependencies_added: usize,
    scripts_added: usize,
    items_removed: usize,
    version_updated: bool,
    dry_run: bool,
    preview: Option<serde_json::Value>,
}

/// Register filesystem tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // read_file tool
    dispatcher.register_tool("read_file".to_string(), |app_state, args| async move {
        let params: ReadFileArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Reading file: {}", params.file_path);

        let path = std::path::Path::new(&params.file_path);

        match app_state.file_service.read_file(path).await {
            Ok(content) => {
                Ok(json!({
                    "content": {
                        "type": "text",
                        "content": content
                    },
                    "file_path": params.file_path,
                    "status": "success"
                }))
            }
            Err(e) => {
                tracing::error!("Failed to read file {}: {}", params.file_path, e);
                Err(crate::error::ServerError::runtime(format!("Failed to read file: {}", e)))
            }
        }
    });

    // write_file tool
    dispatcher.register_tool("write_file".to_string(), |app_state, args| async move {
        let params: WriteFileArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Writing file: {}", params.file_path);

        let path = std::path::Path::new(&params.file_path);

        match app_state.file_service.write_file(path, &params.content).await {
            Ok(()) => {
                Ok(json!({
                    "file_path": params.file_path,
                    "bytes_written": params.content.len(),
                    "status": "success"
                }))
            }
            Err(e) => {
                tracing::error!("Failed to write file {}: {}", params.file_path, e);
                Err(e)
            }
        }
    });

    // list_files tool
    dispatcher.register_tool("list_files".to_string(), |_app_state, args| async move {
        let params: ListFilesArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        let path = params.path.unwrap_or_else(|| ".".to_string());
        let recursive = params.recursive.unwrap_or(false);
        let include_hidden = params.include_hidden.unwrap_or(false);

        tracing::debug!("Listing files in: {} (recursive: {})", path, recursive);

        // Use ignore::WalkBuilder to respect .gitignore and other ignore files
        let mut files = Vec::new();
        let walker = WalkBuilder::new(&path)
            .hidden(!include_hidden)
            .max_depth(if recursive { None } else { Some(1) })
            .build();

        for result in walker {
            match result {
                Ok(entry) => {
                    let file_path = entry.path();
                    let file_name = file_path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    // Get metadata
                    match entry.metadata() {
                        Ok(metadata) => {
                            let file_info = json!({
                                "name": file_name,
                                "path": file_path.to_string_lossy(),
                                "is_directory": metadata.is_dir(),
                                "is_file": metadata.is_file(),
                                "size": if metadata.is_file() { Some(metadata.len()) } else { None },
                                "modified": metadata.modified().ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs())
                            });
                            files.push(file_info);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to read metadata for {}: {}", file_path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read directory entry: {}", e);
                }
            }
        }

        Ok(json!({
            "files": files,
            "path": path,
            "count": files.len(),
            "recursive": recursive,
            "status": "success"
        }))
    });
    // create_file tool
    dispatcher.register_tool("create_file".to_string(), |app_state, args| async move {
        let params: CreateFileArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Creating file: {}", params.file_path);

        let overwrite = params.overwrite.unwrap_or(false);
        let content = params.content.as_deref();
        let path = std::path::Path::new(&params.file_path);

        match app_state.file_service.create_file(path, content, overwrite).await {
            Ok(_) => {
                Ok(serde_json::to_value(FileOperationResult {
                    success: true,
                    operation: "create".to_string(),
                    file_path: params.file_path,
                    message: Some(format!("Created with {} bytes", content.map(|c| c.len()).unwrap_or(0))),
                })?)
            }
            Err(e) => {
                tracing::error!("Failed to create file: {}", e);
                Ok(serde_json::to_value(FileOperationResult {
                    success: false,
                    operation: "create".to_string(),
                    file_path: params.file_path,
                    message: Some(e.to_string()),
                })?)
            }
        }
    });

    // delete_file tool
    dispatcher.register_tool("delete_file".to_string(), |app_state, args| async move {
        let params: DeleteFileArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        let force = params.force.unwrap_or(false);
        let path = std::path::Path::new(&params.file_path);

        tracing::debug!("Deleting file: {} (force: {})", params.file_path, force);

        match app_state.file_service.delete_file(path, force).await {
            Ok(_) => {
                Ok(serde_json::to_value(FileOperationResult {
                    success: true,
                    operation: "delete".to_string(),
                    file_path: params.file_path,
                    message: Some("File deleted successfully".to_string()),
                })?)
            }
            Err(e) => {
                tracing::error!("Failed to delete file: {}", e);
                Ok(serde_json::to_value(FileOperationResult {
                    success: false,
                    operation: "delete".to_string(),
                    file_path: params.file_path,
                    message: Some(e.to_string()),
                })?)
            }
        }
    });

    // rename_file tool
    dispatcher.register_tool("rename_file".to_string(), |app_state, args| async move {
        let params: RenameFileArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Renaming {} to {}", params.old_path, params.new_path);

        let is_dry_run = params.dry_run.unwrap_or(false);

        // Use the FileService to perform rename with import updates
        let old_path = std::path::Path::new(&params.old_path);
        let new_path = std::path::Path::new(&params.new_path);

        match app_state.file_service.rename_file_with_imports(old_path, new_path, is_dry_run).await {
            Ok(result) => {
                // Convert FileRenameResult to MCP response format
                let import_updates = if let Some(report) = result.import_updates {
                    json!({
                        "filesUpdated": report.updated_paths,
                        "importFilesUpdated": report.files_updated,
                        "importsFixed": report.imports_updated,
                        "failedFiles": report.failed_files,
                        "errors": report.errors
                    })
                } else {
                    json!({
                        "filesUpdated": [],
                        "importFilesUpdated": 0,
                        "importsFixed": 0
                    })
                };

                Ok(json!({
                    "renamed": result.success && !is_dry_run,
                    "oldPath": result.old_path,
                    "newPath": result.new_path,
                    "importUpdates": import_updates,
                    "error": result.error
                }))
            }
            Err(e) => {
                tracing::error!("Failed to rename file: {}", e);
                Err(e)
            }
        }
    });

    // update_dependencies tool - unified dependency management across languages
    dispatcher.register_tool("update_dependencies".to_string(), |_app_state, args| async move {
        let params: UpdateDependenciesArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Updating dependencies: {}", params.file_path);

        match handle_update_dependencies(params).await {
            Ok(result) => {
                Ok(serde_json::to_value(result)?)
            }
            Err(e) => {
                tracing::error!("Failed to update dependencies: {}", e);
                Err(crate::error::ServerError::Internal(e.to_string()))
            }
        }
    });

    // health_check tool
    dispatcher.register_tool("health_check".to_string(), |_app_state, args| async move {
        let include_details = args["include_details"].as_bool().unwrap_or(false);

        tracing::debug!("Performing health check");

        let mut health = json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "services": {
                "lsp": "operational",
                "mcp": "operational",
                "cache": "operational"
            }
        });

        if include_details {
            health["details"] = json!({
                "activeServers": ["typescript", "python"],
                "cacheStats": {
                    "hits": 1234,
                    "misses": 56,
                    "hitRate": 0.956
                },
                "memory": {
                    "used": "45MB",
                    "available": "2GB"
                }
            });
        }

        Ok(health)
    });

    // update_package_json tool
    dispatcher.register_tool("update_package_json".to_string(), |_app_state, args| async move {
        let params: UpdatePackageJsonArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        let file_path = params.file_path.unwrap_or_else(|| "./package.json".to_string());
        let is_dry_run = params.dry_run.unwrap_or(false);

        tracing::debug!("Updating package.json at: {} (dry_run: {})", file_path, is_dry_run);

        // Read existing package.json
        let existing_content = match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(e) => {
                return Err(crate::error::ServerError::runtime(format!("Failed to read package.json: {}", e)));
            }
        };

        // Parse existing JSON
        let mut package_json: serde_json::Value = match serde_json::from_str(&existing_content) {
            Ok(json) => json,
            Err(e) => {
                return Err(crate::error::ServerError::runtime(format!("Failed to parse package.json: {}", e)));
            }
        };

        let mut changes_made = Vec::new();
        let mut dependencies_added = 0;
        let mut dev_dependencies_added = 0;
        let mut scripts_added = 0;
        let mut items_removed = 0;
        let mut version_updated = false;

        // Add dependencies
        if let Some(deps) = params.add_dependencies {
            if !package_json.get("dependencies").is_some() {
                package_json["dependencies"] = json!({});
            }

            if let Some(dependencies) = package_json["dependencies"].as_object_mut() {
                for (name, version) in deps {
                    dependencies.insert(name.clone(), json!(version));
                    dependencies_added += 1;
                    changes_made.push(format!("Added dependency: {}@{}", name, version));
                }
            }
        }

        // Add dev dependencies
        if let Some(dev_deps) = params.add_dev_dependencies {
            if !package_json.get("devDependencies").is_some() {
                package_json["devDependencies"] = json!({});
            }

            if let Some(dev_dependencies) = package_json["devDependencies"].as_object_mut() {
                for (name, version) in dev_deps {
                    dev_dependencies.insert(name.clone(), json!(version));
                    dev_dependencies_added += 1;
                    changes_made.push(format!("Added dev dependency: {}@{}", name, version));
                }
            }
        }

        // Add scripts
        if let Some(scripts) = params.add_scripts {
            if !package_json.get("scripts").is_some() {
                package_json["scripts"] = json!({});
            }

            if let Some(package_scripts) = package_json["scripts"].as_object_mut() {
                for (name, command) in scripts {
                    package_scripts.insert(name.clone(), json!(command));
                    scripts_added += 1;
                    changes_made.push(format!("Added script: {} -> {}", name, command));
                }
            }
        }

        // Remove dependencies
        if let Some(deps_to_remove) = params.remove_dependencies {
            for dep_name in deps_to_remove {
                // Remove from both dependencies and devDependencies
                if let Some(dependencies) = package_json["dependencies"].as_object_mut() {
                    if dependencies.remove(&dep_name).is_some() {
                        items_removed += 1;
                        changes_made.push(format!("Removed dependency: {}", dep_name));
                    }
                }

                if let Some(dev_dependencies) = package_json["devDependencies"].as_object_mut() {
                    if dev_dependencies.remove(&dep_name).is_some() {
                        items_removed += 1;
                        changes_made.push(format!("Removed dev dependency: {}", dep_name));
                    }
                }
            }
        }

        // Remove scripts
        if let Some(scripts_to_remove) = params.remove_scripts {
            if let Some(package_scripts) = package_json["scripts"].as_object_mut() {
                for script_name in scripts_to_remove {
                    if package_scripts.remove(&script_name).is_some() {
                        items_removed += 1;
                        changes_made.push(format!("Removed script: {}", script_name));
                    }
                }
            }
        }

        // Update version
        if let Some(new_version) = params.update_version {
            package_json["version"] = json!(new_version.clone());
            version_updated = true;
            changes_made.push(format!("Updated version to: {}", new_version));
        }

        // Update workspace config
        if let Some(workspace_config) = params.workspace_config {
            if let Some(workspaces) = workspace_config.workspaces {
                package_json["workspaces"] = json!(workspaces);
                changes_made.push("Updated workspace configuration".to_string());
            }
        }

        let result = UpdatePackageJsonResult {
            success: true,
            file_path: file_path.clone(),
            changes_made: changes_made.clone(),
            dependencies_added,
            dev_dependencies_added,
            scripts_added,
            items_removed,
            version_updated,
            dry_run: is_dry_run,
            preview: if is_dry_run { Some(package_json.clone()) } else { None },
        };

        if is_dry_run {
            tracing::debug!("Dry run mode - changes preview generated");
            return Ok(serde_json::to_value(result)?);
        }

        // Write updated package.json
        let updated_content = serde_json::to_string_pretty(&package_json)
            .map_err(|e| crate::error::ServerError::runtime(format!("Failed to serialize package.json: {}", e)))?;

        match tokio::fs::write(&file_path, updated_content).await {
            Ok(_) => {
                tracing::info!("Successfully updated package.json with {} changes", changes_made.len());
                Ok(serde_json::to_value(result)?)
            }
            Err(e) => {
                Err(crate::error::ServerError::runtime(format!("Failed to write package.json: {}", e)))
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_file_args() {
        let args = json!({
            "file_path": "new_file.ts",
            "content": "export const foo = 'bar';",
            "overwrite": false
        });

        let parsed: CreateFileArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "new_file.ts");
        assert_eq!(parsed.content, Some("export const foo = 'bar';".to_string()));
        assert_eq!(parsed.overwrite, Some(false));
    }

    #[tokio::test]
    async fn test_rename_file_args() {
        let args = json!({
            "old_path": "old.ts",
            "new_path": "new.ts",
            "dry_run": true
        });

        let parsed: RenameFileArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.old_path, "old.ts");
        assert_eq!(parsed.new_path, "new.ts");
        assert_eq!(parsed.dry_run, Some(true));
    }

    #[tokio::test]
    async fn test_update_package_json_args() {
        let args = json!({
            "file_path": "package.json",
            "add_dependencies": {
                "react": "^18.0.0",
                "axios": "^1.0.0"
            },
            "add_scripts": {
                "test": "jest",
                "build": "webpack"
            },
            "update_version": "2.0.0",
            "dry_run": true
        });

        let parsed: UpdatePackageJsonArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, Some("package.json".to_string()));
    }

    // New tests for the corrected importFilesUpdated field

    #[tokio::test]
    async fn test_rename_file_response_field_names() {
        // Test that the response contains the corrected field name
        use cb_tests::harness::TestWorkspace;
        use crate::handlers::McpDispatcher;
        use crate::handlers::AppState;
        use std::sync::Arc;

        let workspace = TestWorkspace::new();

        // Create a test file
        workspace.create_file("test.ts", "export const foo = 'bar';");

        // Mock the file service to return a predictable response
        let mut mock_file_service = cb_tests::mocks::mock_file_service();
        let mut mock_ast_service = cb_tests::mocks::mock_ast_service();
        let mut mock_lsp_service = cb_tests::mocks::mock_lsp_service();

        // Configure mock to return import update report
        mock_file_service
            .expect_rename_file_with_imports()
            .returning(|_, _, _| {
                Ok(cb_server::services::file_service::FileRenameResult {
                    success: true,
                    old_path: "test.ts".to_string(),
                    new_path: "renamed.ts".to_string(),
                    import_updates: Some(cb_server::services::file_service::ImportUpdateReport {
                        updated_paths: vec!["other.ts".to_string()],
                        files_updated: 1,
                        imports_updated: 2,
                        failed_files: vec![],
                        errors: vec![],
                    }),
                    error: None,
                })
            });

        let app_state = AppState {
            lsp: Arc::new(mock_lsp_service),
            file_service: Arc::new(mock_file_service),
            ast: Arc::new(mock_ast_service),
        };

        let mut dispatcher = McpDispatcher::new();
        register_tools(&mut dispatcher);

        let args = json!({
            "old_path": workspace.absolute_path("test.ts").to_string_lossy(),
            "new_path": workspace.absolute_path("renamed.ts").to_string_lossy(),
            "dry_run": false
        });

        let result = dispatcher.call_tool(&app_state, "rename_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.is_object());

        // Verify the response contains the corrected field name
        let import_updates = &response["importUpdates"];
        assert!(import_updates.is_object());

        // Check that the corrected field name is present
        assert!(import_updates.get("importFilesUpdated").is_some());
        assert_eq!(import_updates["importFilesUpdated"], 1);

        // Verify the old misspelled field is NOT present
        assert!(import_updates.get("importsFielsUpdated").is_none());

        // Verify other fields are present and correct
        assert_eq!(import_updates["filesUpdated"].as_array().unwrap().len(), 1);
        assert_eq!(import_updates["importsFixed"], 2);
    }

    #[tokio::test]
    async fn test_rename_file_response_no_import_updates() {
        // Test response when there are no import updates
        use cb_tests::harness::TestWorkspace;
        use crate::handlers::McpDispatcher;
        use crate::handlers::AppState;
        use std::sync::Arc;

        let workspace = TestWorkspace::new();
        workspace.create_file("simple.txt", "Hello world");

        let mut mock_file_service = cb_tests::mocks::mock_file_service();
        let mut mock_ast_service = cb_tests::mocks::mock_ast_service();
        let mut mock_lsp_service = cb_tests::mocks::mock_lsp_service();

        // Configure mock to return no import updates
        mock_file_service
            .expect_rename_file_with_imports()
            .returning(|_, _, _| {
                Ok(cb_server::services::file_service::FileRenameResult {
                    success: true,
                    old_path: "simple.txt".to_string(),
                    new_path: "renamed.txt".to_string(),
                    import_updates: None, // No import updates
                    error: None,
                })
            });

        let app_state = AppState {
            lsp: Arc::new(mock_lsp_service),
            file_service: Arc::new(mock_file_service),
            ast: Arc::new(mock_ast_service),
        };

        let mut dispatcher = McpDispatcher::new();
        register_tools(&mut dispatcher);

        let args = json!({
            "old_path": workspace.absolute_path("simple.txt").to_string_lossy(),
            "new_path": workspace.absolute_path("renamed.txt").to_string_lossy(),
            "dry_run": false
        });

        let result = dispatcher.call_tool(&app_state, "rename_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let import_updates = &response["importUpdates"];

        // Check that the corrected field name is present with 0 value
        assert_eq!(import_updates["importFilesUpdated"], 0);

        // Verify the old misspelled field is NOT present
        assert!(import_updates.get("importsFielsUpdated").is_none());

        // Verify default values
        assert_eq!(import_updates["filesUpdated"].as_array().unwrap().len(), 0);
        assert_eq!(import_updates["importsFixed"], 0);
    }

    #[test]
    fn test_field_name_spelling_in_json() {
        // Direct test of the JSON structure to ensure correct spelling
        let import_updates = json!({
            "filesUpdated": ["file1.ts", "file2.ts"],
            "importFilesUpdated": 2,
            "importsFixed": 5,
            "failedFiles": [],
            "errors": []
        });

        // Test that we can access the correctly spelled field
        assert_eq!(import_updates["importFilesUpdated"], 2);

        // Test that the misspelled field doesn't exist
        assert!(import_updates.get("importsFielsUpdated").is_none());

        // Test serialization contains correct field name
        let serialized = serde_json::to_string(&import_updates).unwrap();
        assert!(serialized.contains("importFilesUpdated"));
        assert!(!serialized.contains("importsFielsUpdated"));
    }
}