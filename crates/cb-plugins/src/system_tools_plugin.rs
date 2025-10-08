//! System tools plugin providing workspace-level and AST analysis tools

use crate::capabilities::Capabilities;
use crate::{
    error::PluginError,
    plugin::{LanguagePlugin, PluginMetadata},
    protocol::{PluginRequest, PluginResponse, ResponseMetadata},
    PluginResult,
};
use async_trait::async_trait;
use cb_core::language::detect_package_manager;
use ignore::WalkBuilder;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, warn};

/// System tools plugin for non-LSP workspace operations
pub struct SystemToolsPlugin {
    metadata: PluginMetadata,
    capabilities: Capabilities,
    /// Language plugin registry for AST operations
    plugin_registry: Arc<cb_plugin_api::PluginRegistry>,
}

impl Default for SystemToolsPlugin {
    /// **DEPRECATED**: Do not use Default in production.
    ///
    /// This implementation exists only for backward compatibility and testing.
    /// Production code MUST use `SystemToolsPlugin::new(registry)` with a
    /// registry obtained from `cb_services::services::build_language_plugin_registry()`.
    ///
    /// # Why this exists
    ///
    /// Due to circular dependency constraints (cb-plugins cannot depend on cb-services),
    /// we cannot call the centralized builder here. This Default impl creates a
    /// minimal registry for non-production use only.
    ///
    /// # Production usage
    ///
    /// ```rust,ignore
    /// let registry = cb_services::services::build_language_plugin_registry();
    /// let plugin = SystemToolsPlugin::new(registry);
    /// ```
    fn default() -> Self {
        // IMPORTANT: This is NOT production code. Production code uses new() with
        // a registry from cb_services::services::build_language_plugin_registry()
        #[allow(unused_mut)] // mut needed when language features are enabled
        let plugin_registry = {
            let mut registry = cb_plugin_api::PluginRegistry::new();
            #[cfg(feature = "lang-rust")]
            registry.register(Arc::new(cb_lang_rust::RustPlugin::new()));
            #[cfg(feature = "lang-go")]
            registry.register(Arc::new(cb_lang_go::GoPlugin::new()));
            #[cfg(feature = "lang-typescript")]
            registry.register(Arc::new(cb_lang_typescript::TypeScriptPlugin::new()));
            Arc::new(registry)
        };
        Self::new(plugin_registry)
    }
}

impl SystemToolsPlugin {
    /// Creates a new instance of the `SystemToolsPlugin` with injected registry.
    ///
    /// This plugin provides system-level tools that work across all file types, including:
    /// - File system operations (list_files, analyze_imports)
    /// - Dependency management (bulk_update_dependencies)
    /// - Code quality tools (optimize_imports)
    /// - Refactoring operations (extract_function, inline_variable, extract_variable)
    ///
    /// The plugin advertises all available tools through its capabilities, even though
    /// some operations (like refactoring) are handled by the plugin dispatcher.
    ///
    /// # Arguments
    ///
    /// * `plugin_registry` - Shared language plugin registry for AST operations
    ///
    /// # Returns
    ///
    /// A new `SystemToolsPlugin` instance with all capabilities registered
    pub fn new(plugin_registry: Arc<cb_plugin_api::PluginRegistry>) -> Self {
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
            .insert("system.bulk_update_dependencies".to_string(), json!(true));

        // Add refactoring tool capabilities (handled by plugin_dispatcher, but advertised here for discovery)
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
            .insert("system.extract_module_to_package".to_string(), json!(true));

        SystemToolsPlugin {
            metadata: PluginMetadata {
                name: "system-tools".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: "System-level tools for workspace and AST analysis".to_string(),
                author: "Codeflow Buddy Team".to_string(),
                config_schema: None,
                min_system_version: env!("CARGO_PKG_VERSION").to_string(),
                priority: 50, // Default priority
            },
            capabilities,
            plugin_registry,
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

        // Determine file extension
        let path = Path::new(&args.file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| PluginError::PluginRequestFailed {
                plugin: "system-tools".to_string(),
                message: "File has no extension".to_string(),
            })?;

        // Find appropriate plugin from injected registry
        let plugin = self
            .plugin_registry
            .find_by_extension(extension)
            .ok_or_else(|| PluginError::PluginRequestFailed {
                plugin: "system-tools".to_string(),
                message: format!("No plugin found for .{} files", extension),
            })?;

        // Build import graph using the plugin
        let import_graph = build_import_graph_with_plugin(&content, path, plugin)?;

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

    /// Handle bulk_update_dependencies tool
    async fn handle_bulk_update_dependencies(&self, params: Value) -> PluginResult<Value> {
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
                message: format!("Invalid bulk_update_dependencies args: {}", e),
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

        // Detect package manager using shared utility
        let detected_manager = if package_manager == "auto" {
            let detected = detect_package_manager(Path::new(&project_path));
            detected.as_str()
        } else {
            package_manager.as_str()
        };

        let (command, args) = match detected_manager {
            "npm" => {
                if dry_run {
                    ("npm", vec!["outdated"])
                } else {
                    ("npm", vec!["update"])
                }
            }
            "yarn" => {
                if dry_run {
                    ("yarn", vec!["outdated"])
                } else {
                    ("yarn", vec!["upgrade"])
                }
            }
            "pnpm" => {
                if dry_run {
                    ("pnpm", vec!["outdated"])
                } else {
                    ("pnpm", vec!["update"])
                }
            }
            "go" => {
                if dry_run {
                    // Go doesn't have a built-in "outdated" command
                    // Use go list to check for available updates
                    ("go", vec!["list", "-u", "-m", "all"])
                } else {
                    // Update all dependencies
                    ("go", vec!["get", "-u", "./..."])
                }
            }
            "cargo" => {
                if dry_run {
                    ("cargo", vec!["outdated"])
                } else {
                    ("cargo", vec!["update"])
                }
            }
            "pip" => {
                if dry_run {
                    ("pip", vec!["list", "--outdated"])
                } else {
                    (
                        "pip",
                        vec!["install", "--upgrade", "-r", "requirements.txt"],
                    )
                }
            }
            _ => {
                return Err(PluginError::PluginRequestFailed {
                    plugin: "system-tools".to_string(),
                    message: format!("Unknown package manager: {}", detected_manager),
                })
            }
        };

        // Execute the command
        let output = tokio::process::Command::new(command)
            .args(&args)
            .current_dir(&project_path)
            .output()
            .await
            .map_err(|e| PluginError::IoError {
                message: format!("Failed to execute command: {}", e),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();
        let exit_code = output.status.code();

        debug!(
            command = %command,
            args = ?args,
            success = %success,
            exit_code = ?exit_code,
            "Command executed"
        );

        Ok(json!({
            "project_path": project_path,
            "package_manager": detected_manager,
            "update_type": update_type,
            "dry_run": dry_run,
            "command": format!("{} {}", command, args.join(" ")),
            "success": success,
            "exit_code": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "status": if dry_run { "preview" } else { "completed" },
        }))
    }

    /// Handle web_fetch tool
    async fn handle_web_fetch(&self, params: Value) -> PluginResult<Value> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct WebFetchArgs {
            url: String,
        }

        let args: WebFetchArgs =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid web_fetch args: {}", e),
            })?;

        debug!(url = %args.url, "Fetching URL content");

        // Use reqwest to fetch the URL content
        let response = reqwest::blocking::get(&args.url).map_err(|e| PluginError::IoError {
            message: format!("Failed to fetch URL: {}", e),
        })?;

        let html_content = response.text().map_err(|e| PluginError::IoError {
            message: format!("Failed to read response text: {}", e),
        })?;

        // Convert HTML to Markdown for easier AI processing
        let markdown_content =
            html2md_rs::to_md::safe_from_html_to_md(html_content).map_err(|e| {
                PluginError::IoError {
                    message: format!("Failed to convert HTML to markdown: {}", e),
                }
            })?;

        Ok(json!({
            "url": args.url,
            "content": markdown_content,
            "status": "success"
        }))
    }

    /// Handle extract_module_to_package tool
    async fn handle_extract_module_to_package(&self, params: Value) -> PluginResult<Value> {
        // Deserialize parameters
        let parsed: cb_ast::package_extractor::ExtractModuleToPackageParams =
            serde_json::from_value(params).map_err(|e| PluginError::SerializationError {
                message: format!("Invalid extract_module_to_package args: {}", e),
            })?;

        debug!(
            source_package = %parsed.source_package,
            module_path = %parsed.module_path,
            target_package_path = %parsed.target_package_path,
            target_package_name = %parsed.target_package_name,
            "Extracting module to package"
        );

        // Call the planning function from cb-ast with injected registry
        let edit_plan = cb_ast::package_extractor::plan_extract_module_to_package_with_registry(
            parsed,
            &self.plugin_registry,
        )
        .await
        .map_err(|e| PluginError::PluginRequestFailed {
            plugin: "system-tools".to_string(),
            message: format!("Failed to plan extract_module_to_package: {}", e),
        })?;

        // Return the edit plan
        Ok(json!({
            "edit_plan": edit_plan,
            "status": "success"
        }))
    }
}

/// Build import graph using a language plugin
fn build_import_graph_with_plugin(
    source: &str,
    path: &Path,
    plugin: &dyn cb_plugin_api::LanguagePlugin,
) -> PluginResult<cb_protocol::ImportGraph> {
    use cb_protocol::{ImportGraph, ImportGraphMetadata, ImportInfo};
    use std::collections::HashSet;

    // For now, TypeScript plugin doesn't have a direct "analyze_imports" method
    // that returns ImportGraph. We need to use the parser module from the plugin.
    //
    // This is a temporary bridge until we add analyze_imports to the trait.
    // For TypeScript files, we'll use cb_lang_typescript::parser::analyze_imports
    let language = plugin.metadata().name.to_lowercase();

    let imports: Vec<ImportInfo> = match language.as_str() {
        "typescript" => {
            // Use TypeScript plugin's parser
            let graph =
                cb_lang_typescript::parser::analyze_imports(source, Some(path)).map_err(|e| {
                    PluginError::PluginRequestFailed {
                        plugin: plugin.metadata().name.to_string(),
                        message: format!("Failed to parse imports: {}", e),
                    }
                })?;
            graph.imports
        }
        "rust" => {
            // Use Rust plugin's parser (returns Vec<ImportInfo> directly)
            cb_lang_rust::parser::parse_imports(source).map_err(|e| {
                PluginError::PluginRequestFailed {
                    plugin: plugin.metadata().name.to_string(),
                    message: format!("Failed to parse imports: {}", e),
                }
            })?
        }
        "go" => {
            // Use Go plugin's parser
            let graph = cb_lang_go::parser::analyze_imports(source, Some(path)).map_err(|e| {
                PluginError::PluginRequestFailed {
                    plugin: plugin.metadata().name.to_string(),
                    message: format!("Failed to parse imports: {}", e),
                }
            })?;
            graph.imports
        }
        _ => {
            return Err(PluginError::PluginRequestFailed {
                plugin: plugin.metadata().name.to_string(),
                message: format!("Unsupported language: {}", language),
            });
        }
    };

    // Detect external dependencies
    let external_dependencies = imports
        .iter()
        .filter_map(|imp| {
            if is_external_dependency(&imp.module_path) {
                Some(imp.module_path.clone())
            } else {
                None
            }
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Ok(ImportGraph {
        source_file: path.to_string_lossy().to_string(),
        imports,
        importers: Vec::new(),
        metadata: ImportGraphMetadata {
            language: language.clone(),
            parsed_at: chrono::Utc::now(),
            parser_version: "1.0.0-plugin".to_string(),
            circular_dependencies: Vec::new(),
            external_dependencies,
        },
    })
}

/// Check if a module path represents an external dependency
fn is_external_dependency(module_path: &str) -> bool {
    if module_path.starts_with("./") || module_path.starts_with("../") {
        return false;
    }
    if module_path.starts_with("/") || module_path.starts_with("src/") {
        return false;
    }
    if module_path.starts_with("@") {
        return true;
    }
    !module_path.contains("/")
        || module_path.contains("node_modules")
        || !module_path.starts_with(".")
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

    fn tool_definitions(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "achieve_intent",
                "description": "Takes a high-level user intent and returns a multi-step workflow plan. Optionally executes the workflow with dry-run support. Can also resume a paused workflow.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "intent": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "The unique name of the intent, e.g., 'refactor.renameSymbol'." },
                                "params": { "type": "object", "description": "Parameters for the intent." }
                            },
                            "required": ["name", "params"]
                        },
                        "execute": {
                            "type": "boolean",
                            "description": "If true, execute the workflow after planning. If false or omitted, only return the plan."
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "If true, execute the workflow in dry-run mode (preview changes without modifying files). Only applies when execute is true."
                        },
                        "workflow_id": {
                            "type": "string",
                            "description": "Optional workflow ID to resume a paused workflow. If provided, the intent parameter is ignored."
                        },
                        "resume_data": {
                            "type": "object",
                            "description": "Optional data to pass when resuming a workflow. Can be used for future features where user provides input."
                        }
                    }
                }
            }),
            json!({
                "name": "list_files",
                "description": "List files and directories in a given path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to list (defaults to current directory)"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "Whether to recursively list subdirectories"
                        },
                        "include_hidden": {
                            "type": "boolean",
                            "description": "Whether to include hidden files"
                        }
                    }
                }
            }),
            json!({
                "name": "analyze_imports",
                "description": "Analyze import statements in a file.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file to analyze"
                        }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "bulk_update_dependencies",
                "description": "Run the package manager's update command (e.g., `npm update`).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project_path": {
                            "type": "string",
                            "description": "Path to the project (defaults to current directory)"
                        },
                        "package_manager": {
                            "type": "string",
                            "description": "Package manager to use (auto, npm, yarn, pnpm, cargo, pip)"
                        },
                        "update_type": {
                            "type": "string",
                            "description": "Type of update (minor, major, patch)"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "Preview changes without applying them"
                        }
                    }
                }
            }),
            json!({
                "name": "extract_function",
                "description": "Extract a block of code into a new function.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "start_line": {
                            "type": "number",
                            "description": "Start line of code to extract"
                        },
                        "end_line": {
                            "type": "number",
                            "description": "End line of code to extract"
                        },
                        "function_name": {
                            "type": "string",
                            "description": "Name for the new function"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "Preview changes without applying them"
                        }
                    },
                    "required": ["file_path", "start_line", "end_line", "function_name"]
                }
            }),
            json!({
                "name": "inline_variable",
                "description": "Inline a variable's value at the specified position.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "line": {
                            "type": "number",
                            "description": "Line number where the variable is declared (1-indexed)"
                        },
                        "character": {
                            "type": "number",
                            "description": "Optional character position in the line (0-indexed, defaults to 0)"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "Preview changes without applying them"
                        }
                    },
                    "required": ["file_path", "line"]
                }
            }),
            json!({
                "name": "extract_variable",
                "description": "Extract an expression into a new variable.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "start_line": {
                            "type": "number",
                            "description": "Start line of expression"
                        },
                        "start_character": {
                            "type": "number",
                            "description": "Start character of expression"
                        },
                        "end_line": {
                            "type": "number",
                            "description": "End line of expression"
                        },
                        "end_character": {
                            "type": "number",
                            "description": "End character of expression"
                        },
                        "variable_name": {
                            "type": "string",
                            "description": "Name for the new variable"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "description": "Preview changes without applying them"
                        }
                    },
                    "required": ["file_path", "start_line", "start_character", "end_line", "end_character", "variable_name"]
                }
            }),
            json!({
                "name": "extract_module_to_package",
                "description": "Extract a module from an existing package into a new standalone package. Works across Rust, TypeScript, Python, Go, and Java. Automatically updates imports and package manifests.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source_package": {
                            "type": "string",
                            "description": "Path to the source package (e.g., 'rust/crates/cb-server', 'packages/api')"
                        },
                        "module_path": {
                            "type": "string",
                            "description": "Dotted path to the module within the source package (e.g., 'services.planner', 'utils.helpers')"
                        },
                        "target_package_path": {
                            "type": "string",
                            "description": "Path where the new package should be created (e.g., 'domains/planner', 'packages/planner')"
                        },
                        "target_package_name": {
                            "type": "string",
                            "description": "Name of the new package (e.g., 'cb-planner', '@org/planner', 'cb_planner')"
                        },
                        "update_imports": {
                            "type": "boolean",
                            "default": true,
                            "description": "Automatically update all import statements across the workspace"
                        },
                        "create_manifest": {
                            "type": "boolean",
                            "default": true,
                            "description": "Auto-generate package manifest (Cargo.toml, package.json, etc.)"
                        },
                        "dry_run": {
                            "type": "boolean",
                            "default": false,
                            "description": "Preview changes without applying them"
                        }
                    },
                    "required": ["source_package", "module_path", "target_package_path", "target_package_name"]
                }
            }),
            json!({
                "name": "web_fetch",
                "description": "Fetch the plain text content of a given URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to fetch content from"
                        }
                    },
                    "required": ["url"]
                }
            }),
            json!({
                "name": "health_check",
                "description": "Get the health status of the server, including uptime, loaded plugins, and paused workflows.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }),
            // Note: rename_file and rename_directory are handled by FileOperationHandler
            // and WorkspaceHandler respectively, not by this plugin
        ]
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
            "bulk_update_dependencies" => {
                self.handle_bulk_update_dependencies(request.params.clone())
                    .await?
            }
            "web_fetch" => self.handle_web_fetch(request.params.clone()).await?,
            "extract_module_to_package" => {
                self.handle_extract_module_to_package(request.params.clone())
                    .await?
            }
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
