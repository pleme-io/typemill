//! Prune planning service for Unified Refactoring API
//!
//! Provides delete planning for:
//! - Symbol deletion (AST-based - placeholder)
//! - File deletion (via FileService)
//! - Directory deletion (via FileService)

use crate::handlers::common::calculate_checksum;
use futures::stream::StreamExt;
use lsp_types::{Location, Position, Range, TextEdit, Uri};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::{
    DeletePlan, DeletionTarget, PlanMetadata, PlanSummary, PlanWarning,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};
use url::Url;

/// Planning service for prune operations
pub struct PrunePlanner;

impl PrunePlanner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrunePlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PrunePlanParams {
    pub target: PruneTarget,
    #[serde(default)]
    pub options: PruneOptions,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PruneTarget {
    #[allow(dead_code)] // Used for dispatching but read via pattern matching elsewhere
    pub kind: String, // "symbol" | "file" | "directory"
    pub path: String,
    #[serde(default)]
    pub selector: Option<PruneSelector>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Reserved for future symbol deletion implementation
pub(crate) struct PruneSelector {
    pub line: u32,
    pub character: u32,
    #[serde(default)]
    pub symbol_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PruneOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "crate::default_true")]
    #[allow(dead_code)] // Deserialized but checked via options field in parent
    pub dry_run: bool,
    #[serde(default)]
    pub cleanup_imports: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)] // Reserved for future test cleanup implementation
    pub remove_tests: Option<bool>,
    #[serde(default)]
    pub force: Option<bool>,
}

impl Default for PruneOptions {
    fn default() -> Self {
        Self {
            dry_run: true, // Safe default: preview mode
            cleanup_imports: None,
            remove_tests: None,
            force: None,
        }
    }
}

impl PrunePlanner {
    /// Helper to remove a specific identifier from an import statement
    /// Returns Some(new_line) if we can keep the import with other identifiers
    /// Returns None if the entire line should be deleted
    fn remove_import_identifier(&self, line: &str, identifier: &str) -> Option<String> {
        // Check if this is an import statement with curly braces
        if !line.trim_start().starts_with("import") || !line.contains('{') || !line.contains('}') {
            return Some(line.to_string()); // Not a destructured import, keep line as is
        }

        // Extract the part between curly braces
        let start_brace = line.find('{')?;
        let end_brace = line.find('}')?;
        let imports_section = &line[start_brace + 1..end_brace];

        // Split by comma and filter out the identifier to remove
        let identifiers: Vec<&str> = imports_section
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && *s != identifier)
            .collect();

        if identifiers.is_empty() {
            // No identifiers left, delete entire line
            None
        } else if identifiers.len()
            == imports_section
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .count()
        {
            // Identifier wasn't found, keep line as is
            Some(line.to_string())
        } else {
            // Reconstruct the import with remaining identifiers
            let prefix = &line[..start_brace + 1];
            let suffix = &line[end_brace..];
            let new_imports = identifiers.join(", ");
            Some(format!("{} {} {}", prefix, new_imports, suffix))
        }
    }

    /// Generate edits to clean up imports of a symbol in other files
    async fn cleanup_imports(
        &self,
        symbol_name: &str,
        def_file_path: &Path,
        line: u32,
        character: u32,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<HashMap<Uri, Vec<TextEdit>>> {
        let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

        // Get extension to find LSP client
        let extension = def_file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::invalid_request(format!(
                    "File has no extension: {}",
                    def_file_path.display()
                ))
            })?;

        // Get LSP client
        let client_opt = {
            let adapter = context.lsp_adapter.lock().await;
            if let Some(adapter) = adapter.as_ref() {
                adapter.get_or_create_client(extension).await.ok()
            } else {
                None
            }
        };

        if let Some(client) = client_opt {
            // Create Uri from path
            let uri_str = Url::from_file_path(def_file_path)
                .map_err(|_| ServerError::invalid_request("Invalid definition file path"))?
                .to_string();
            let uri: Uri = uri_str
                .parse()
                .map_err(|e| ServerError::internal(format!("Failed to parse URI: {}", e)))?;

            // Find references
            let params = lsp_types::ReferenceParams {
                text_document_position: lsp_types::TextDocumentPositionParams {
                    text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
                    position: Position { line, character },
                },
                context: lsp_types::ReferenceContext {
                    include_declaration: false,
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };

            let response = client
                .send_request(
                    "textDocument/references",
                    serde_json::to_value(params).unwrap(),
                )
                .await
                .map_err(|e| ServerError::internal(format!("Failed to find references: {}", e)))?;

            let references: Vec<Location> = serde_json::from_value(response).unwrap_or_default();

            // Group references by file
            let mut files_with_refs: HashMap<Uri, Vec<Location>> = HashMap::new();
            for reference in references {
                files_with_refs
                    .entry(reference.uri.clone())
                    .or_default()
                    .push(reference);
            }

            // Process each file
            for (file_uri, locations) in files_with_refs {
                // Skip the definition file itself (handled by symbol delete plan)
                if file_uri == uri {
                    continue;
                }

                // Convert Uri to Path via Url
                let url_str = file_uri.as_str();
                let url = Url::parse(url_str)
                    .map_err(|e| ServerError::internal(format!("Failed to parse URI as URL: {}", e)))?;

                let path = url
                    .to_file_path()
                    .map_err(|_| ServerError::internal("Failed to convert URI to path"))?;

                let content = context
                    .app_state
                    .file_service
                    .read_file(&path)
                    .await
                    .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

                let lines: Vec<&str> = content.lines().collect();
                let mut edits = Vec::new();
                let mut processed_lines = HashSet::new();

                for location in locations {
                    let line_idx = location.range.start.line as usize;
                    if line_idx >= lines.len() {
                        continue;
                    }

                    // Avoid processing the same line multiple times
                    if processed_lines.contains(&line_idx) {
                        continue;
                    }
                    processed_lines.insert(line_idx);

                    let line = lines[line_idx];

                    // Only process import statements
                    if !line.trim_start().starts_with("import") {
                        continue;
                    }

                    // Attempt to remove identifier
                    if let Some(new_line) = self.remove_import_identifier(line, symbol_name) {
                        if new_line != line {
                            // Replace line
                            let range = Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: 0,
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: line.len() as u32,
                                },
                            };

                            edits.push(TextEdit {
                                range,
                                new_text: new_line,
                            });
                        }
                    } else {
                        // Delete line (including newline if possible)
                        // To delete the line completely, we delete from start of this line to start of next line
                        let start = Position {
                            line: line_idx as u32,
                            character: 0,
                        };
                        let end = Position {
                            line: (line_idx + 1) as u32,
                            character: 0,
                        };

                        edits.push(TextEdit {
                            range: Range { start, end },
                            new_text: "".to_string(),
                        });
                    }
                }

                if !edits.is_empty() {
                    changes.insert(file_uri, edits);
                }
            }
        }

        Ok(changes)
    }

    /// Generate plan for symbol deletion using AST-based analysis
    pub(crate) async fn plan_symbol_delete(
        &self,
        params: &PrunePlanParams,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(path = %params.target.path, "Planning symbol delete");

        let file_path = Path::new(&params.target.path);

        // Validate selector is provided
        let selector = params.target.selector.as_ref().ok_or_else(|| {
            ServerError::invalid_request("Symbol delete requires selector with line/character")
        })?;

        // Get file extension to find the right language plugin
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                error!(
                    path = %params.target.path,
                    "File has no extension for plugin lookup"
                );
                ServerError::invalid_request(format!(
                    "File has no extension: {}",
                    params.target.path
                ))
            })?;

        debug!(
            extension = %extension,
            "Looking up language plugin for extension"
        );

        // Get the language plugin for this file extension
        let plugin = context
            .app_state
            .language_plugins
            .get_plugin(extension)
            .ok_or_else(|| {
                error!(
                    extension = %extension,
                    "No language plugin found for extension"
                );
                ServerError::not_supported(format!(
                    "No language plugin available for .{} files",
                    extension
                ))
            })?;

        debug!(
            plugin = %plugin.metadata().name,
            "Found language plugin, getting refactoring provider capability"
        );

        // Get the refactoring provider capability from the plugin
        let refactoring_provider = plugin.refactoring_provider().ok_or_else(|| {
            error!(
                plugin = %plugin.metadata().name,
                "Plugin does not support refactoring operations"
            );
            ServerError::not_supported(format!(
                "{} plugin does not support symbol deletion refactoring",
                plugin.metadata().name
            ))
        })?;

        // Read file content
        let content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| {
                error!(error = %e, file_path = %params.target.path, "Failed to read file");
                ServerError::internal(format!("Failed to read file: {}", e))
            })?;

        debug!("File read successfully, calling plan_symbol_delete on RefactoringProvider");

        // Call the plugin's symbol delete planning
        // Note: This is a new method that needs to be added to the RefactoringProvider trait
        let edit_plan = refactoring_provider
            .plan_symbol_delete(
                &content,
                selector.line,
                selector.character,
                &params.target.path,
            )
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    "Plugin symbol delete failed"
                );
                ServerError::internal(format!("Symbol delete failed: {}", e))
            })?;

        // Convert EditPlan to WorkspaceEdit using the converter utility from move module
        let mut workspace_edit =
            crate::handlers::relocate_ops::converter::convert_edit_plan_to_workspace_edit(
                &edit_plan,
            )?;

        let mut warnings = Vec::new();

        // Clean up imports if enabled
        if params.options.cleanup_imports.unwrap_or(true) {
            if let Some(symbol_name) = &selector.symbol_name {
                let cleanup_edits = self
                    .cleanup_imports(
                        symbol_name,
                        file_path,
                        selector.line,
                        selector.character,
                        context,
                    )
                    .await?;

                if !cleanup_edits.is_empty() {
                    let changes = workspace_edit.changes.get_or_insert_with(HashMap::new);
                    for (uri, edits) in cleanup_edits {
                        changes.entry(uri).or_default().extend(edits);
                    }
                }
            } else {
                warnings.push(PlanWarning {
                    code: "MISSING_SYMBOL_NAME".to_string(),
                    message: "Cannot clean up imports without symbol name".to_string(),
                    candidates: None,
                });
            }
        }

        // Calculate file checksums
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            file_path.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Build summary
        let summary = PlanSummary {
            affected_files: 1,
            created_files: 0,
            deleted_files: 0,
        };

        // Determine language from extension via plugin registry
        let language = plugin.metadata().name.to_string();

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "delete".to_string(),
            language,
            estimated_impact: "low".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        info!(
            file_path = %params.target.path,
            line = selector.line,
            "Created AST-based delete plan for symbol"
        );

        Ok(DeletePlan {
            deletions: Vec::new(), // No file deletions, using edits instead
            edits: Some(workspace_edit),
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for file deletion using FileService
    pub(crate) async fn plan_file_delete(
        &self,
        params: &PrunePlanParams,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(
            path = %params.target.path,
            "Planning file delete"
        );

        let file_path = Path::new(&params.target.path);
        let force = params.options.force.unwrap_or(false);

        // Use FileService to generate dry-run plan for file deletion
        let dry_run_result = context
            .app_state
            .file_service
            .delete_file(file_path, force, true)
            .await?;

        // Extract the inner value from DryRunnable
        let result_value = dry_run_result.result;

        // Read file content for checksum before deletion
        let content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| {
                error!(error = %e, file_path = %params.target.path, "Failed to read file");
                ServerError::internal(format!("Failed to read file for checksum: {}", e))
            })?;

        // Calculate checksum
        let mut file_checksums = HashMap::new();
        file_checksums.insert(
            file_path.to_string_lossy().to_string(),
            calculate_checksum(&content),
        );

        // Canonicalize path to ensure proper path handling
        let abs_file_path = tokio::fs::canonicalize(file_path)
            .await
            .unwrap_or_else(|_| file_path.to_path_buf());

        // Create explicit deletion target
        let deletions = vec![DeletionTarget {
            path: abs_file_path.to_string_lossy().to_string(),
            kind: "file".to_string(),
        }];

        // Build summary
        let summary = PlanSummary {
            affected_files: 1,
            created_files: 0,
            deleted_files: 1,
        };

        // Check if there are affected files (imports to clean up)
        let mut warnings = Vec::new();
        let affected_files_count = result_value
            .get("affected_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if affected_files_count > 0 && params.options.cleanup_imports.unwrap_or(true) {
            warnings.push(PlanWarning {
                code: "IMPORT_CLEANUP_REQUIRED".to_string(),
                message: format!(
                    "File deletion will clean up imports in {} dependent files",
                    affected_files_count
                ),
                candidates: None,
            });
        }

        // Determine language from extension
        let language = crate::handlers::common::detect_language(&params.target.path).to_string();

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "delete".to_string(),
            language,
            estimated_impact: if affected_files_count > 5 {
                "high"
            } else if affected_files_count > 0 {
                "medium"
            } else {
                "low"
            }
            .to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(DeletePlan {
            deletions,
            edits: None, // Symbol deletion will use this field
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Generate plan for directory deletion using FileService
    pub(crate) async fn plan_directory_delete(
        &self,
        params: &PrunePlanParams,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<DeletePlan> {
        debug!(
            path = %params.target.path,
            "Planning directory delete"
        );

        let dir_path = Path::new(&params.target.path);

        // Walk directory to collect files and checksums
        // Move directory walking and checking to a blocking task to avoid blocking the async runtime
        let dir_path_buf = dir_path.to_path_buf();
        let target_path_str = params.target.path.clone();

        let (files, abs_dir): (Vec<PathBuf>, PathBuf) = tokio::task::spawn_blocking(move || {
            // Verify it's a directory inside the blocking task using synchronous fs
            if !dir_path_buf.is_dir() {
                return Err(ServerError::invalid_request(format!(
                    "Path is not a directory: {}",
                    target_path_str
                )));
            }

            let abs_dir = std::fs::canonicalize(&dir_path_buf).unwrap_or(dir_path_buf);
            let walker = ignore::WalkBuilder::new(&abs_dir).hidden(false).build();
            let files = walker
                .flatten()
                .filter(|entry| entry.path().is_file())
                .map(|entry| entry.path().to_path_buf())
                .collect();
            Ok((files, abs_dir))
        })
        .await
        .map_err(|e| ServerError::internal(format!("Task failed: {}", e)))??;

        let mut file_checksums = HashMap::new();
        let mut file_count = 0;

        // Process file reads and checksums concurrently
        let results: Vec<_> = futures::stream::iter(files)
            .map(|path| {
                let fs = context.app_state.file_service.clone();
                async move {
                    match fs.read_file(&path).await {
                        Ok(content) => Some((
                            path.to_string_lossy().to_string(),
                            calculate_checksum(&content),
                        )),
                        Err(_) => None,
                    }
                }
            })
            .buffer_unordered(50) // Process 50 files concurrently
            .collect()
            .await;

        for (path_str, checksum) in results.into_iter().flatten() {
            file_checksums.insert(path_str, checksum);
            file_count += 1;
        }

        // Create explicit deletion target for directory
        let deletions = vec![DeletionTarget {
            path: abs_dir.to_string_lossy().to_string(),
            kind: "directory".to_string(),
        }];

        // Build summary
        let summary = PlanSummary {
            affected_files: file_count,
            created_files: 0,
            deleted_files: file_count,
        };

        // Add warnings
        let mut warnings = Vec::new();
        if params.options.cleanup_imports.unwrap_or(true) {
            warnings.push(PlanWarning {
                code: "IMPORT_CLEANUP_REQUIRED".to_string(),
                message: format!(
                    "Directory deletion will clean up imports for {} files",
                    file_count
                ),
                candidates: None,
            });
        }

        // Check if this is a Cargo package
        let cargo_toml = abs_dir.join("Cargo.toml");
        if tokio::fs::try_exists(&cargo_toml).await.unwrap_or(false) {
            warnings.push(PlanWarning {
                code: "CARGO_PACKAGE_DELETE".to_string(),
                message: "Deleting a Cargo package will remove it from workspace members"
                    .to_string(),
                candidates: None,
            });
        }

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "delete".to_string(),
            language: "unknown".to_string(),
            estimated_impact: if file_count > 10 {
                "high"
            } else if file_count > 3 {
                "medium"
            } else {
                "low"
            }
            .to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(DeletePlan {
            deletions,
            edits: None, // Symbol deletion will use this field
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_import_identifier() {
        let planner = PrunePlanner::new();

        // Case 1: Destructured import, remove one identifier
        let line = "import { A, B } from './mod';";
        let result = planner.remove_import_identifier(line, "A");
        assert_eq!(result, Some("import { B } from './mod';".to_string()));

        // Case 2: Destructured import, remove last identifier (should delete line)
        let line = "import { A } from './mod';";
        let result = planner.remove_import_identifier(line, "A");
        assert_eq!(result, None);

        // Case 3: Destructured import, identifier not found (should be no-op)
        let line = "import { A } from './mod';";
        let result = planner.remove_import_identifier(line, "B");
        assert_eq!(result, Some(line.to_string()));

        // Case 4: Default import (should be no-op currently)
        let line = "import A from './mod';";
        let result = planner.remove_import_identifier(line, "A");
        assert_eq!(result, Some(line.to_string()));

        // Case 5: Complex spacing
        let line = "import {  A ,  B  } from './mod';";
        let result = planner.remove_import_identifier(line, "A");
        // Implementation trims whitespace and reconstructs with single spaces
        assert_eq!(result, Some("import { B } from './mod';".to_string()));

        // Case 6: Not an import (should be no-op)
        let line = "const x = 1;";
        let result = planner.remove_import_identifier(line, "x");
        assert_eq!(result, Some(line.to_string()));
    }
}
