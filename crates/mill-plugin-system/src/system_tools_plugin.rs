//! System tools plugin providing workspace-level and AST analysis tools

use crate::capabilities::Capabilities;
use crate::{
    error::PluginError,
    plugin::{LanguagePlugin, PluginMetadata},
    protocol::{PluginRequest, PluginResponse, ResponseMetadata},
    PluginResult,
};
use async_trait::async_trait;
use mill_plugin_api::language::detect_package_manager;
use ignore::WalkBuilder;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, warn};

/// System tools plugin for non-LSP workspace operations
pub struct SystemToolsPlugin {
    metadata: PluginMetadata,
    capabilities: Capabilities,
    /// Language plugin registry for AST operations
    plugin_registry: Arc<mill_plugin_api::PluginRegistry>,
}

impl SystemToolsPlugin {
    /// Creates a new instance of the `SystemToolsPlugin` with injected registry.
    ///
    /// This plugin provides system-level tools that work across all file types, including:
    /// - File system operations (list_files)
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
    pub fn new(plugin_registry: Arc<mill_plugin_api::PluginRegistry>) -> Self {
        let mut capabilities = Capabilities::default();

        // Add custom capabilities for system tools
        capabilities
            .custom
            .insert("system.list_files".to_string(), json!(true));
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

        // Add extract_module_to_package only if Rust plugin is available
        let has_rust_plugin = plugin_registry
            .all()
            .iter()
            .any(|p| p.metadata().name == "rust");
        if has_rust_plugin {
            capabilities
                .custom
                .insert("system.extract_module_to_package".to_string(), json!(true));
        }

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
    #[allow(unused_variables)] // params only used with lang-rust feature
    async fn handle_extract_module_to_package(&self, params: Value) -> PluginResult<Value> {
        // Check if Rust plugin is available at runtime
        let has_rust = self
            .plugin_registry
            .all()
            .iter()
            .any(|p| p.metadata().name == "rust");

        if !has_rust {
            return Err(PluginError::MethodNotSupported {
                method: "extract_module_to_package".to_string(),
                plugin: "system-tools (requires Rust plugin)".to_string(),
            });
        }

        // Deserialize parameters - no cfg guard needed, we check capabilities at runtime
        let parsed: mill_ast::package_extractor::ExtractModuleToPackageParams =
            serde_json::from_value(params.clone()).map_err(|e| {
                PluginError::SerializationError {
                    message: format!("Invalid extract_module_to_package args: {}", e),
                }
            })?;

        debug!(
            source_package = %parsed.source_package,
            module_path = %parsed.module_path,
            target_package_path = %parsed.target_package_path,
            target_package_name = %parsed.target_package_name,
            "Extracting module to package"
        );

        // Call the planning function from cb-ast with injected registry
        // cb-ast is now language-agnostic and uses capability-based dispatch
        let edit_plan =
            mill_ast::package_extractor::plan_extract_module_to_package_with_registry(
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
        let mut tools = vec![
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
            json!({
                "name": "analyze.quality",
                "description": "Analyze code quality metrics including complexity, code smells, maintainability, and readability.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["complexity", "smells", "maintainability", "readability"],
                            "description": "Type of quality analysis to perform"
                        },
                        "scope": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["file", "directory", "workspace"] },
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        },
                        "options": {
                            "type": "object",
                            "properties": {
                                "thresholds": { "type": "object" },
                                "include_suggestions": { "type": "boolean" }
                            }
                        }
                    },
                    "required": ["kind", "scope"]
                }
            }),
            json!({
                "name": "analyze.dead_code",
                "description": "Detect unused code including imports, symbols, parameters, variables, types, and unreachable code.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["unused_imports", "unused_symbols", "unused_parameters", "unused_variables", "unused_types", "unreachable_code"],
                            "description": "Type of dead code detection to perform"
                        },
                        "scope": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["file", "directory", "workspace"] },
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    "required": ["kind", "scope"]
                }
            }),
            json!({
                "name": "analyze.dependencies",
                "description": "Analyze dependency relationships including imports, dependency graph, circular dependencies, coupling, and cohesion.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["imports", "graph", "circular", "coupling", "cohesion", "depth"],
                            "description": "Type of dependency analysis to perform"
                        },
                        "scope": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["file", "directory", "workspace"] },
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    "required": ["kind", "scope"]
                }
            }),
            json!({
                "name": "analyze.structure",
                "description": "Analyze code structure including symbols, hierarchy, interfaces, inheritance, and module organization.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["symbols", "hierarchy", "interfaces", "inheritance", "modules"],
                            "description": "Type of structure analysis to perform"
                        },
                        "scope": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["file", "directory", "workspace"] },
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    "required": ["kind", "scope"]
                }
            }),
            json!({
                "name": "analyze.documentation",
                "description": "Analyze documentation quality including coverage, quality, style consistency, examples, and TODO items.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["coverage", "quality", "style", "examples", "todos"],
                            "description": "Type of documentation analysis to perform"
                        },
                        "scope": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["file", "directory", "workspace"] },
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    "required": ["kind", "scope"]
                }
            }),
            json!({
                "name": "analyze.tests",
                "description": "Analyze test quality including coverage, test quality, assertions, and test organization.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["coverage", "quality", "assertions", "organization"],
                            "description": "Type of test analysis to perform"
                        },
                        "scope": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["file", "directory", "workspace"] },
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    "required": ["kind", "scope"]
                }
            }),
            json!({
                "name": "analyze.batch",
                "description": "Executes multiple analysis queries in a single batch for optimized performance. Leverages shared AST parsing to analyze code efficiently across different categories and kinds. Returns an aggregated result for all queries.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "queries": {
                            "type": "array",
                            "description": "An array of analysis queries to execute in a batch.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "command": {
                                        "type": "string",
                                        "description": "The analysis command to run, e.g., 'analyze.quality'."
                                    },
                                    "kind": {
                                        "type": "string",
                                        "description": "The specific kind of analysis to perform, e.g., 'complexity'."
                                    },
                                    "scope": {
                                        "type": "object",
                                        "description": "The scope of the analysis (file, directory, workspace).",
                                        "properties": {
                                            "type": { "type": "string", "enum": ["file", "directory", "workspace", "symbol"] },
                                            "path": { "type": "string" },
                                            "include": { "type": "array", "items": { "type": "string" } },
                                            "exclude": { "type": "array", "items": { "type": "string" } }
                                        },
                                        "required": ["type"]
                                    },
                                    "options": {
                                        "type": "object",
                                        "description": "Optional parameters for the analysis."
                                    }
                                },
                                "required": ["command", "kind", "scope"]
                            }
                        }
                    },
                    "required": ["queries"]
                }
            }),
            // Note: rename_file and rename_directory are handled by FileOperationHandler
            // and WorkspaceHandler respectively, not by this plugin
        ];

        // Conditionally add Rust-specific tools based on runtime plugin availability
        let has_rust_plugin = self
            .plugin_registry
            .all()
            .iter()
            .any(|p| p.metadata().name == "rust");

        if has_rust_plugin {
            tools.push(json!({
                "name": "extract_module_to_package",
                "description": "Extract a module from an existing package into a new standalone package. Currently supports Rust and TypeScript. Automatically updates imports and package manifests. Note: Language support temporarily reduced during unified API refactoring.",
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
            }));
        }

        tools
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