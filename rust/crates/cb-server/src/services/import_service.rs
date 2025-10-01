//! Service for managing import updates across the codebase

use crate::{ServerError, ServerResult};
use cb_api::DependencyUpdate;
use cb_ast::{update_import_paths, ImportPathResolver};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info};

/// Service for managing import path updates
pub struct ImportService {
    /// Project root directory
    project_root: PathBuf,
}

impl ImportService {
    /// Create a new import service
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
        }
    }

    /// Update imports after a file rename
    pub async fn update_imports_for_rename(
        &self,
        old_path: &Path,
        new_path: &Path,
        dry_run: bool,
    ) -> ServerResult<ImportUpdateReport> {
        info!(
            old_path = ?old_path,
            new_path = ?new_path,
            dry_run = dry_run,
            "Updating imports for rename"
        );

        // Convert to absolute paths if needed
        let old_abs = if old_path.is_absolute() {
            old_path.to_path_buf()
        } else {
            self.project_root.join(old_path)
        };

        let new_abs = if new_path.is_absolute() {
            new_path.to_path_buf()
        } else {
            self.project_root.join(new_path)
        };

        // Find and update imports - CRITICAL FIX: Pass dry_run flag through!
        debug!(
            old_abs = ?old_abs,
            new_abs = ?new_abs,
            project_root = ?self.project_root,
            dry_run = dry_run,
            "Calling update_import_paths"
        );
        let result = update_import_paths(&old_abs, &new_abs, &self.project_root, dry_run)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to update imports: {}", e)))?;

        debug!(
            files_updated = result.updated_files.len(),
            imports_updated = result.imports_updated,
            "update_import_paths result"
        );

        // Create report
        let report = ImportUpdateReport {
            files_updated: result.updated_files.len(),
            imports_updated: result.imports_updated,
            failed_files: result.failed_files.len(),
            updated_paths: result
                .updated_files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            errors: result
                .failed_files
                .iter()
                .map(|(p, e)| format!("{}: {}", p.display(), e))
                .collect(),
        };

        if dry_run {
            info!(
                files_affected = report.files_updated,
                imports_affected = report.imports_updated,
                "Dry run complete - no files were actually modified"
            );
        } else {
            info!(
                files_updated = report.files_updated,
                imports_updated = report.imports_updated,
                "Import update complete"
            );
        }

        Ok(report)
    }

    /// Find all files that would be affected by a rename
    pub async fn find_affected_files(&self, file_path: &Path) -> ServerResult<Vec<PathBuf>> {
        let resolver = ImportPathResolver::new(&self.project_root);

        // Get all project files
        let project_files = self.find_all_source_files().await?;

        // Find files importing the target
        let affected = resolver
            .find_affected_files(file_path, &project_files)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to find affected files: {}", e)))?;

        Ok(affected)
    }

    /// Find all source files in the project
    async fn find_all_source_files(&self) -> ServerResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];

        Self::collect_files(&self.project_root, &mut files, &extensions).await?;

        Ok(files)
    }

    /// Recursively collect files with given extensions
    fn collect_files<'a>(
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
        extensions: &'a [&str],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ServerResult<()>> + Send + 'a>> {
        Box::pin(async move {
            // Skip ignored directories
            if let Some(name) = dir.file_name() {
                let name_str = name.to_string_lossy();
                if matches!(
                    name_str.as_ref(),
                    "node_modules" | ".git" | "dist" | "build" | "target" | ".next" | ".nuxt"
                ) {
                    return Ok(());
                }
            }

            let mut read_dir = tokio::fs::read_dir(dir)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to read entry: {}", e)))?
            {
                let path = entry.path();

                if path.is_dir() {
                    Self::collect_files(&path, files, extensions).await?;
                } else if let Some(ext) = path.extension() {
                    if extensions.contains(&ext.to_str().unwrap_or("")) {
                        files.push(path);
                    }
                }
            }

            Ok(())
        })
    }

    /// Check if a file imports another file
    pub async fn check_import_dependency(
        &self,
        source_file: &Path,
        target_file: &Path,
    ) -> ServerResult<bool> {
        let content = tokio::fs::read_to_string(source_file)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        let target_stem = target_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Simple check for import references
        Ok(content.contains(target_stem)
            && (content.contains("import") || content.contains("require")))
    }

    /// Update an import reference in a file using AST-based transformation
    pub async fn update_import_reference(
        &self,
        file_path: &Path,
        update: &DependencyUpdate,
    ) -> ServerResult<bool> {
        use swc_common::{sync::Lrc, FileName, FilePathMapping, SourceMap};
        use swc_ecma_ast::{ModuleDecl, ModuleItem};
        use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
        use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

        // Read file content
        let content = match fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(e) => {
                debug!(
                    file_path = %file_path.display(),
                    error = %e,
                    "Could not read file for dependency update"
                );
                return Ok(false); // File doesn't exist, skip update
            }
        };

        // Check if the file contains the old reference
        if !content.contains(&update.old_reference) {
            return Ok(false);
        }

        // Parse, transform, and emit the updated code
        // This is done in a separate scope to ensure non-Send types are dropped before await
        let updated_content = {
            // Set up SWC parser
            let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let file_name = Lrc::new(FileName::Real(file_path.to_path_buf()));
            let source_file = cm.new_source_file(file_name, content.clone());

            // Determine syntax based on file extension
            let syntax = match file_path.extension().and_then(|ext| ext.to_str()) {
                Some("ts") | Some("tsx") => Syntax::Typescript(TsSyntax {
                    tsx: file_path.extension().and_then(|e| e.to_str()) == Some("tsx"),
                    decorators: true,
                    ..Default::default()
                }),
                _ => Syntax::Es(Default::default()),
            };

            // Parse the file
            let lexer = Lexer::new(syntax, Default::default(), StringInput::from(&*source_file), None);
            let mut parser = Parser::new_from(lexer);

            let module = match parser.parse_module() {
                Ok(module) => module,
                Err(e) => {
                    error!(
                        file_path = %file_path.display(),
                        error = ?e,
                        "Failed to parse file for import update"
                    );
                    return Err(ServerError::Internal(format!(
                        "Failed to parse file: {:?}",
                        e
                    )));
                }
            };

            // Transform imports
            let mut updated = false;
            let new_items: Vec<ModuleItem> = module
                .body
                .into_iter()
                .map(|item| {
                    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = &item {
                        if import_decl.src.value.as_ref() == update.old_reference {
                            updated = true;
                            let mut new_import = import_decl.clone();
                            new_import.src = Box::new(swc_ecma_ast::Str {
                                span: import_decl.src.span,
                                value: update.new_reference.clone().into(),
                                raw: None,
                            });
                            return ModuleItem::ModuleDecl(ModuleDecl::Import(new_import));
                        }
                    }
                    item
                })
                .collect();

            if !updated {
                debug!(
                    file_path = %file_path.display(),
                    old_ref = %update.old_reference,
                    "No matching imports found to update"
                );
                return Ok(false);
            }

            // Create new module with updated imports
            let new_module = swc_ecma_ast::Module {
                body: new_items,
                ..module
            };

            // Emit the updated code
            let mut buf = vec![];
            {
                let mut emitter = Emitter {
                    cfg: Default::default(),
                    cm: cm.clone(),
                    comments: None,
                    wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
                };

                emitter.emit_module(&new_module).map_err(|e| {
                    ServerError::Internal(format!("Failed to emit updated code: {:?}", e))
                })?;
            }

            String::from_utf8(buf).map_err(|e| {
                ServerError::Internal(format!("Failed to convert emitted code to string: {}", e))
            })?
        };

        // Write the updated content back to the file
        fs::write(file_path, updated_content).await.map_err(|e| {
            ServerError::Internal(format!(
                "Failed to write dependency update to {}: {}",
                file_path.display(),
                e
            ))
        })?;

        info!(
            file_path = %file_path.display(),
            old_ref = %update.old_reference,
            new_ref = %update.new_reference,
            "Successfully updated import reference using AST"
        );

        Ok(true)
    }
}

/// Report of import update operations
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportUpdateReport {
    /// Number of files that were updated
    pub files_updated: usize,
    /// Total number of import statements updated
    pub imports_updated: usize,
    /// Number of files that failed to update
    pub failed_files: usize,
    /// Paths of successfully updated files
    pub updated_paths: Vec<String>,
    /// Error messages for failed updates
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;

    #[tokio::test]
    async fn test_import_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let service = ImportService::new(temp_dir.path());

        assert_eq!(service.project_root, temp_dir.path());
    }

    #[tokio::test]
    async fn test_find_source_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        std::fs::create_dir(temp_dir.path().join("src")).unwrap();
        std::fs::write(temp_dir.path().join("src/index.ts"), "export {}").unwrap();
        std::fs::write(temp_dir.path().join("src/utils.js"), "module.exports = {}").unwrap();

        // Create node_modules that should be ignored
        std::fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        std::fs::write(temp_dir.path().join("node_modules/lib.js"), "ignore me").unwrap();

        let service = ImportService::new(temp_dir.path());
        let files = service.find_all_source_files().await.unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("index.ts")));
        assert!(files.iter().any(|p| p.ends_with("utils.js")));
    }
}
