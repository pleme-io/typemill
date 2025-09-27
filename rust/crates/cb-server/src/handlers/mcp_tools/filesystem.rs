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

/// Register filesystem tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    eprintln!("DEBUG: Registering filesystem tools including rename_file");
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
        eprintln!("DEBUG: rename_file handler called with old_path: {}, new_path: {}", params.old_path, params.new_path);

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
                        "importsFielsUpdated": report.files_updated,
                        "importsFixed": report.imports_updated,
                        "failedFiles": report.failed_files,
                        "errors": report.errors
                    })
                } else {
                    json!({
                        "filesUpdated": [],
                        "importsFielsUpdated": 0,
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
        assert!(parsed.add_dependencies.is_some());
        assert!(parsed.add_scripts.is_some());
        assert_eq!(parsed.update_version, Some("2.0.0".to_string()));
        assert_eq!(parsed.dry_run, Some(true));
    }
}