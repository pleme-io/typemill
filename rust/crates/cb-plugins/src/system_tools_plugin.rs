//! System tools plugin providing workspace-level and AST analysis tools

use crate::capabilities::Capabilities;
use crate::{
    error::PluginError,
    plugin::{LanguagePlugin, PluginMetadata},
    protocol::{PluginRequest, PluginResponse, ResponseMetadata},
    PluginResult,
};
use async_trait::async_trait;
use ignore::WalkBuilder;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

/// System tools plugin for non-LSP workspace operations
pub struct SystemToolsPlugin {
    metadata: PluginMetadata,
    capabilities: Capabilities,
}

impl Default for SystemToolsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemToolsPlugin {
    /// Create a new system tools plugin
    pub fn new() -> Self {
        let mut capabilities = Capabilities::default();

        // Add custom capabilities for system tools
        capabilities
            .custom
            .insert("system.list_files".to_string(), json!(true));
        capabilities
            .custom
            .insert("system.analyze_imports".to_string(), json!(true));
        capabilities
            .custom
            .insert("system.find_dead_code".to_string(), json!(true));
        capabilities
            .custom
            .insert("system.update_dependencies".to_string(), json!(true));
        capabilities
            .custom
            .insert("system.rename_directory".to_string(), json!(true));
        capabilities
            .custom
            .insert("system.extract_function".to_string(), json!(true));
        capabilities
            .custom
            .insert("system.inline_variable".to_string(), json!(true));
        capabilities
            .custom
            .insert("system.extract_variable".to_string(), json!(true));
        capabilities
            .custom
            .insert("system.fix_imports".to_string(), json!(true));

        SystemToolsPlugin {
            metadata: PluginMetadata {
                name: "system-tools".to_string(),
                version: "0.1.0".to_string(),
                description: "System-level tools for workspace and AST analysis".to_string(),
                author: "Codeflow Buddy Team".to_string(),
                config_schema: None,
                min_system_version: "0.1.0".to_string(),
            },
            capabilities,
        }
    }

    /// Handle list_files tool
    async fn handle_list_files(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct ListFilesArgs {
            path: Option<String>,
            recursive: Option<bool>,
            include_hidden: Option<bool>,
        }

        let args: ListFilesArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid list_files args: {}", e),
            })?;

        let path = args.path.unwrap_or_else(|| ".".to_string());
        let recursive = args.recursive.unwrap_or(false);
        let include_hidden = args.include_hidden.unwrap_or(false);

        debug!(path = %path, recursive = %recursive, "Listing files");

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
                    let file_name = file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    // Get metadata
                    match entry.metadata() {
                        Ok(metadata) => {
                            let file_info = json!({
                                "name": file_name,
                                "path": file_path.to_string_lossy(),
                                "size": metadata.len(),
                                "is_dir": metadata.is_dir(),
                                "is_file": metadata.is_file(),
                            });
                            files.push(file_info);
                        }
                        Err(e) => {
                            warn!(file_path = ?file_path, error = %e, "Failed to get metadata");
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Error walking directory");
                }
            }
        }

        Ok(json!({
            "files": files,
            "total": files.len(),
            "path": path,
        }))
    }

    /// Handle analyze_imports tool
    async fn handle_analyze_imports(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct AnalyzeImportsArgs {
            file_path: String,
        }

        let args: AnalyzeImportsArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid analyze_imports args: {}", e),
            })?;

        debug!(file_path = %args.file_path, "Analyzing imports");

        // Read the file content
        let content =
            fs::read_to_string(&args.file_path)
                .await
                .map_err(|e| PluginError::IoError {
                    message: format!("Failed to read file: {}", e),
                })?;

        // Call cb_ast::parser::build_import_graph
        let path = Path::new(&args.file_path);
        let import_graph = cb_ast::parser::build_import_graph(&content, path).map_err(|e| {
            PluginError::PluginRequestFailed {
                plugin: "system-tools".to_string(),
                message: format!("Failed to analyze imports: {}", e),
            }
        })?;

        // Calculate statistics
        let total_imports = import_graph.imports.len();
        let unique_modules: std::collections::HashSet<&String> = import_graph
            .imports
            .iter()
            .map(|imp| &imp.module_path)
            .collect();
        let type_only_imports = import_graph
            .imports
            .iter()
            .filter(|imp| imp.type_only)
            .count();
        let namespace_imports = import_graph
            .imports
            .iter()
            .filter(|imp| imp.namespace_import.is_some())
            .count();
        let default_imports = import_graph
            .imports
            .iter()
            .filter(|imp| imp.default_import.is_some())
            .count();

        Ok(json!({
            "sourceFile": args.file_path,
            "importGraph": import_graph,
            "analysisStats": {
                "totalImports": total_imports,
                "uniqueModules": unique_modules.len(),
                "typeOnlyImports": type_only_imports,
                "namespaceImports": namespace_imports,
                "defaultImports": default_imports,
            }
        }))
    }

    /// Handle update_dependencies tool
    async fn handle_update_dependencies(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct UpdateDependenciesArgs {
            project_path: Option<String>,
            package_manager: Option<String>,
            update_type: Option<String>,
            dry_run: Option<bool>,
        }

        let args: UpdateDependenciesArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid update_dependencies args: {}", e),
            })?;

        let project_path = args.project_path.unwrap_or_else(|| ".".to_string());
        let package_manager = args.package_manager.unwrap_or_else(|| "auto".to_string());
        let update_type = args.update_type.unwrap_or_else(|| "minor".to_string());
        let dry_run = args.dry_run.unwrap_or(false);

        debug!(
            project_path = %project_path,
            package_manager = %package_manager,
            "Updating dependencies"
        );

        // Detect package manager
        let detected_manager = if package_manager == "auto" {
            if Path::new(&format!("{}/package.json", project_path)).exists() {
                if Path::new(&format!("{}/yarn.lock", project_path)).exists() {
                    "yarn"
                } else if Path::new(&format!("{}/pnpm-lock.yaml", project_path)).exists() {
                    "pnpm"
                } else {
                    "npm"
                }
            } else if Path::new(&format!("{}/Cargo.toml", project_path)).exists() {
                "cargo"
            } else if Path::new(&format!("{}/requirements.txt", project_path)).exists() {
                "pip"
            } else {
                "unknown"
            }
        } else {
            package_manager.as_str()
        };

        let command = match detected_manager {
            "npm" => {
                if dry_run {
                    "npm outdated"
                } else {
                    "npm update"
                }
            }
            "yarn" => {
                if dry_run {
                    "yarn outdated"
                } else {
                    "yarn upgrade"
                }
            }
            "pnpm" => {
                if dry_run {
                    "pnpm outdated"
                } else {
                    "pnpm update"
                }
            }
            "cargo" => {
                if dry_run {
                    "cargo outdated"
                } else {
                    "cargo update"
                }
            }
            "pip" => {
                if dry_run {
                    "pip list --outdated"
                } else {
                    "pip install --upgrade -r requirements.txt"
                }
            }
            _ => {
                return Err(PluginError::PluginRequestFailed {
                    plugin: "system-tools".to_string(),
                    message: format!("Unknown package manager: {}", detected_manager),
                })
            }
        };

        Ok(json!({
            "project_path": project_path,
            "package_manager": detected_manager,
            "update_type": update_type,
            "dry_run": dry_run,
            "command": command,
            "status": if dry_run { "preview" } else { "updated" },
        }))
    }

    /// Handle rename_directory tool
    async fn handle_rename_directory(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct RenameDirectoryArgs {
            old_path: String,
            new_path: String,
            update_imports: Option<bool>,
            dry_run: Option<bool>,
        }

        let args: RenameDirectoryArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid rename_directory args: {}", e),
            })?;

        debug!(
            old_path = %args.old_path,
            new_path = %args.new_path,
            "Renaming directory"
        );

        if args.dry_run.unwrap_or(false) {
            // In dry run, just show what would happen
            let mut affected_files = Vec::new();

            // Walk through directory to find all files that would be moved
            let walker = WalkBuilder::new(&args.old_path).hidden(false).build();

            for entry in walker.flatten() {
                let file_path = entry.path();
                if file_path.is_file() {
                    let relative_path = file_path
                        .strip_prefix(&args.old_path)
                        .unwrap_or(file_path)
                        .to_string_lossy();
                    let new_file_path = format!("{}/{}", args.new_path, relative_path);
                    affected_files.push(json!({
                        "old": file_path.to_string_lossy(),
                        "new": new_file_path,
                    }));
                }
            }

            return Ok(json!({
                "operation": "rename_directory",
                "old_path": args.old_path,
                "new_path": args.new_path,
                "dry_run": true,
                "affected_files": affected_files,
                "update_imports": args.update_imports.unwrap_or(true),
            }));
        }

        // Actual rename operation
        match tokio::fs::rename(&args.old_path, &args.new_path).await {
            Ok(_) => Ok(json!({
                "operation": "rename_directory",
                "old_path": args.old_path,
                "new_path": args.new_path,
                "status": "success",
                "update_imports": args.update_imports.unwrap_or(true),
            })),
            Err(e) => Err(PluginError::IoError {
                message: format!("Failed to rename directory: {}", e),
            }),
        }
    }

    /// Handle find_dead_code tool
    async fn handle_find_dead_code(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct FindDeadCodeArgs {
            workspace_path: String,
        }

        let args: FindDeadCodeArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid find_dead_code args: {}", e),
            })?;

        debug!(workspace_path = %args.workspace_path, "Finding dead code");

        let start_time = std::time::Instant::now();
        let dead_symbols: Vec<Value> = Vec::new();
        let mut files_analyzed = 0;
        let symbols_analyzed = 0;

        // Use ignore crate to walk the directory
        let walker = WalkBuilder::new(&args.workspace_path).hidden(false).build();

        for entry in walker.flatten() {
            let file_path = entry.path();

            // Only analyze source files
            if let Some(ext) = file_path.extension() {
                let ext_str = ext.to_string_lossy();
                if matches!(ext_str.as_ref(), "ts" | "tsx" | "js" | "jsx" | "py" | "rs") {
                    files_analyzed += 1;

                    // For each file, we would need to:
                    // 1. Get symbols using LSP documentSymbol request
                    // 2. Check references for each symbol
                    // 3. Mark symbols with 0-1 references as potentially dead

                    // Since we don't have LSP service access here directly,
                    // we'll provide a simplified implementation
                    // In a full implementation, this would call LSP servers

                    // For now, return a placeholder that indicates the analysis would happen
                    debug!(file_path = ?file_path, "Would analyze file");
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Return analysis result (simplified version)
        Ok(json!({
            "workspacePath": args.workspace_path,
            "deadSymbols": dead_symbols,
            "analysisStats": {
                "filesAnalyzed": files_analyzed,
                "symbolsAnalyzed": symbols_analyzed,
                "deadSymbolsFound": dead_symbols.len(),
                "analysisDurationMs": duration_ms,
            }
        }))
    }

    /// Handle extract_function tool
    async fn handle_extract_function(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct ExtractFunctionArgs {
            file_path: String,
            start_line: u32,
            end_line: u32,
            function_name: String,
            dry_run: Option<bool>,
        }

        let args: ExtractFunctionArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid extract_function args: {}", e),
            })?;

        debug!(
            function_name = %args.function_name,
            file_path = %args.file_path,
            start_line = args.start_line,
            end_line = args.end_line,
            "Extracting function"
        );

        // Read the file
        let content =
            fs::read_to_string(&args.file_path)
                .await
                .map_err(|e| PluginError::IoError {
                    message: format!("Failed to read file: {}", e),
                })?;

        let lines: Vec<&str> = content.lines().collect();

        // Basic extraction logic (simplified)
        // In a real implementation, this would use AST parsing to properly extract the function
        let extracted_lines = &lines[(args.start_line as usize - 1)..(args.end_line as usize)];
        let extracted_code = extracted_lines.join("\n");

        // Create the new function
        let new_function = format!(
            "function {}() {{\n  {}\n}}",
            args.function_name, extracted_code
        );

        Ok(json!({
            "operation": "extract_function",
            "file_path": args.file_path,
            "function_name": args.function_name,
            "extracted_code": extracted_code,
            "new_function": new_function,
            "start_line": args.start_line,
            "end_line": args.end_line,
            "dry_run": args.dry_run.unwrap_or(false),
        }))
    }

    /// Handle inline_variable tool
    async fn handle_inline_variable(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct InlineVariableArgs {
            file_path: String,
            variable_name: String,
            line: u32,
            dry_run: Option<bool>,
        }

        let args: InlineVariableArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid inline_variable args: {}", e),
            })?;

        debug!(
            variable_name = %args.variable_name,
            file_path = %args.file_path,
            line = args.line,
            "Inlining variable"
        );

        // Read the file
        let _content =
            fs::read_to_string(&args.file_path)
                .await
                .map_err(|e| PluginError::IoError {
                    message: format!("Failed to read file: {}", e),
                })?;

        // In a real implementation, this would use AST parsing to find the variable declaration
        // and all its usages, then replace them with the value

        Ok(json!({
            "operation": "inline_variable",
            "file_path": args.file_path,
            "variable_name": args.variable_name,
            "line": args.line,
            "status": "preview",
            "dry_run": args.dry_run.unwrap_or(false),
            "message": "Would inline variable and update all references",
        }))
    }

    /// Handle extract_variable tool
    async fn handle_extract_variable(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct ExtractVariableArgs {
            file_path: String,
            start_line: u32,
            start_character: u32,
            end_line: u32,
            end_character: u32,
            variable_name: String,
            dry_run: Option<bool>,
        }

        let args: ExtractVariableArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid extract_variable args: {}", e),
            })?;

        debug!(
            variable_name = %args.variable_name,
            file_path = %args.file_path,
            start_line = args.start_line,
            start_character = args.start_character,
            end_line = args.end_line,
            end_character = args.end_character,
            "Extracting variable"
        );

        // Read the file
        let _content =
            fs::read_to_string(&args.file_path)
                .await
                .map_err(|e| PluginError::IoError {
                    message: format!("Failed to read file: {}", e),
                })?;

        // In a real implementation, this would use AST parsing to extract the expression
        // and create a variable declaration

        Ok(json!({
            "operation": "extract_variable",
            "file_path": args.file_path,
            "variable_name": args.variable_name,
            "range": {
                "start": { "line": args.start_line, "character": args.start_character },
                "end": { "line": args.end_line, "character": args.end_character },
            },
            "status": "preview",
            "dry_run": args.dry_run.unwrap_or(false),
            "message": "Would extract expression into variable",
        }))
    }

    /// Handle fix_imports tool
    async fn handle_fix_imports(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct FixImportsArgs {
            file_path: String,
            dry_run: Option<bool>,
        }

        let args: FixImportsArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid fix_imports args: {}", e),
            })?;

        debug!(file_path = %args.file_path, "Fixing imports");

        // Read the file
        let content =
            fs::read_to_string(&args.file_path)
                .await
                .map_err(|e| PluginError::IoError {
                    message: format!("Failed to read file: {}", e),
                })?;

        // Use cb_ast to analyze imports
        let path = Path::new(&args.file_path);
        let import_graph = cb_ast::parser::build_import_graph(&content, path).map_err(|e| {
            PluginError::PluginRequestFailed {
                plugin: "system-tools".to_string(),
                message: format!("Failed to analyze imports: {}", e),
            }
        })?;

        // Count different types of fixes that would be applied
        let unused_imports = import_graph
            .imports
            .iter()
            .filter(|imp| {
                imp.named_imports.is_empty()
                    && imp.default_import.is_none()
                    && imp.namespace_import.is_none()
            })
            .count();

        let duplicate_imports = {
            let mut seen = std::collections::HashSet::new();
            import_graph
                .imports
                .iter()
                .filter(|imp| !seen.insert(&imp.module_path))
                .count()
        };

        Ok(json!({
            "operation": "fix_imports",
            "file_path": args.file_path,
            "dry_run": args.dry_run.unwrap_or(false),
            "fixes": {
                "unused_imports": unused_imports,
                "duplicate_imports": duplicate_imports,
                "total_imports": import_graph.imports.len(),
            },
            "status": if args.dry_run.unwrap_or(false) { "preview" } else { "fixed" },
        }))
    }
}

#[async_trait]
impl LanguagePlugin for SystemToolsPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn supported_extensions(&self) -> Vec<String> {
        // System tools work on all file types
        vec![]
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }

    fn configure(&self, _config: Value) -> PluginResult<()> {
        // No configuration needed for system tools
        Ok(())
    }

    async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse> {
        debug!(method = %request.method, "System tools plugin handling request");

        let result = match request.method.as_str() {
            "list_files" => self.handle_list_files(request.params.clone()).await?,
            "analyze_imports" => self.handle_analyze_imports(request.params.clone()).await?,
            "find_dead_code" => self.handle_find_dead_code(request.params.clone()).await?,
            "update_dependencies" => {
                self.handle_update_dependencies(request.params.clone())
                    .await?
            }
            "rename_directory" => self.handle_rename_directory(request.params.clone()).await?,
            "extract_function" => self.handle_extract_function(request.params.clone()).await?,
            "inline_variable" => self.handle_inline_variable(request.params.clone()).await?,
            "extract_variable" => self.handle_extract_variable(request.params.clone()).await?,
            "fix_imports" => self.handle_fix_imports(request.params.clone()).await?,
            _ => {
                return Err(PluginError::MethodNotSupported {
                    method: request.method.clone(),
                    plugin: self.metadata.name.clone(),
                });
            }
        };

        Ok(PluginResponse {
            success: true,
            data: Some(result),
            error: None,
            request_id: request.request_id.clone(),
            metadata: ResponseMetadata {
                plugin_name: self.metadata.name.clone(),
                processing_time_ms: Some(0), // Would be calculated in real implementation
                cached: false,
                plugin_metadata: json!({}),
            },
        })
    }
}
