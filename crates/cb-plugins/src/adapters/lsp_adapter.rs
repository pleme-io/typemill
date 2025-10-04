//! LSP adapter plugin that translates between the plugin system and LSP protocol
//!
//! This adapter serves as a bridge, allowing the plugin system to work with
//! existing LSP servers without requiring changes to the LSP implementation.

use crate::{
    Capabilities, DiagnosticCapabilities, EditingCapabilities, IntelligenceCapabilities,
    LanguagePlugin, NavigationCapabilities, PluginError, PluginMetadata, PluginRequest,
    PluginResponse, PluginResult, RefactoringCapabilities,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};
use url::Url;

/// Trait for LSP service integration
/// This allows the adapter to work with any LSP implementation
#[async_trait]
pub trait LspService: Send + Sync {
    /// Send a request to the LSP server and get a response
    async fn request(&self, method: &str, params: Value) -> Result<Value, String>;

    /// Check if the service supports a specific file extension
    fn supports_extension(&self, extension: &str) -> bool;

    /// Get the service name for debugging
    fn service_name(&self) -> String;
}

/// LSP adapter plugin that bridges plugin system with LSP servers
pub struct LspAdapterPlugin {
    /// Plugin metadata
    metadata: PluginMetadata,
    /// Supported file extensions
    extensions: Vec<String>,
    /// Computed capabilities based on LSP support
    capabilities: Capabilities,
    /// LSP service for handling requests
    lsp_service: Arc<dyn LspService>,
    /// Method mapping cache for performance
    method_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl LspAdapterPlugin {
    /// Create a new LSP adapter plugin
    pub fn new(
        name: impl Into<String>,
        extensions: Vec<String>,
        lsp_service: Arc<dyn LspService>,
    ) -> Self {
        let name = name.into();
        let metadata = PluginMetadata::new(&name, env!("CARGO_PKG_VERSION"), "Codeflow Buddy")
            .with_description("LSP adapter plugin for protocol translation")
            .with_min_system_version(env!("CARGO_PKG_VERSION"));

        // Create comprehensive capabilities for LSP-based functionality
        let capabilities = Capabilities {
            navigation: NavigationCapabilities {
                go_to_definition: true,
                find_references: true,
                find_implementations: true,
                find_type_definition: true,
                workspace_symbols: true,
                document_symbols: true,
                call_hierarchy: true,
            },
            editing: EditingCapabilities {
                rename: true,
                format_document: true,
                format_range: true,
                code_actions: true,
                organize_imports: true,
                auto_imports: false, // Depends on LSP server
            },
            refactoring: RefactoringCapabilities {
                extract_function: false, // Requires AST support
                extract_variable: false, // Requires AST support
                inline_variable: false,  // Requires AST support
                inline_function: false,  // Requires AST support
                move_refactor: false,    // Requires AST support
            },
            intelligence: IntelligenceCapabilities {
                hover: true,
                completions: true,
                signature_help: true,
                inlay_hints: true,
                semantic_highlighting: true,
            },
            diagnostics: DiagnosticCapabilities {
                diagnostics: true,
                linting: false, // Depends on LSP server
                pull_diagnostics: true,
            },
            custom: HashMap::new(),
        };

        Self {
            metadata,
            extensions,
            capabilities,
            lsp_service,
            method_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a TypeScript/JavaScript LSP adapter
    pub fn typescript(lsp_service: Arc<dyn LspService>) -> Self {
        let mut adapter = Self::new(
            "typescript-lsp-adapter",
            vec![
                "ts".to_string(),
                "tsx".to_string(),
                "js".to_string(),
                "jsx".to_string(),
            ],
            lsp_service,
        );

        // Apply standard capabilities
        Self::apply_standard_capabilities(&mut adapter);

        adapter
    }

    /// Create a Python LSP adapter
    pub fn python(lsp_service: Arc<dyn LspService>) -> Self {
        let mut adapter = Self::new(
            "python-lsp-adapter",
            vec!["py".to_string(), "pyi".to_string()],
            lsp_service,
        );

        // Apply standard capabilities
        Self::apply_standard_capabilities(&mut adapter);

        // Add Python-specific custom capabilities
        adapter
            .capabilities
            .custom
            .insert("python.format_imports".to_string(), json!(true));

        adapter
    }

    /// Create a Go LSP adapter
    pub fn go(lsp_service: Arc<dyn LspService>) -> Self {
        let mut adapter = Self::new("go-lsp-adapter", vec!["go".to_string()], lsp_service);

        // Apply standard capabilities
        Self::apply_standard_capabilities(&mut adapter);

        // Add Go-specific custom capabilities
        adapter
            .capabilities
            .custom
            .insert("go.generate".to_string(), json!(true));
        adapter
            .capabilities
            .custom
            .insert("go.organize_imports".to_string(), json!(true));

        adapter
    }

    /// Create a Rust LSP adapter
    pub fn rust(lsp_service: Arc<dyn LspService>) -> Self {
        let mut adapter = Self::new("rust-lsp-adapter", vec!["rs".to_string()], lsp_service);

        // Apply standard capabilities
        Self::apply_standard_capabilities(&mut adapter);

        // Add Rust-specific custom capabilities
        adapter
            .capabilities
            .custom
            .insert("rust.expand_macro".to_string(), json!(true));

        adapter
    }

    /// Apply a standard set of capabilities that should work for most modern LSP servers.
    fn apply_standard_capabilities(adapter: &mut Self) {
        // Enable common editing capabilities
        adapter.capabilities.editing.auto_imports = true;
        adapter.capabilities.editing.organize_imports = true;

        // Enable common refactoring capabilities
        adapter.capabilities.refactoring.extract_function = true;
        adapter.capabilities.refactoring.extract_variable = true;
        adapter.capabilities.refactoring.inline_variable = true;
    }

    /// Convert plugin request to LSP method and params
    async fn translate_request(&self, request: &PluginRequest) -> PluginResult<(String, Value)> {
        // Check cache first
        {
            let cache = self.method_cache.lock().await;
            if let Some(lsp_method) = cache.get(&request.method) {
                return Ok((
                    lsp_method.clone(),
                    self.build_lsp_params(request, lsp_method)?,
                ));
            }
        }

        // Translate method to LSP equivalent
        let lsp_method = match request.method.as_str() {
            // Navigation methods
            "find_definition" => "textDocument/definition",
            "find_references" => "textDocument/references",
            "find_implementations" => "textDocument/implementation",
            "find_type_definition" => "textDocument/typeDefinition",
            "search_workspace_symbols" => "workspace/symbol",
            "get_document_symbols" => "textDocument/documentSymbol",
            "prepare_call_hierarchy" => "textDocument/prepareCallHierarchy",
            "get_call_hierarchy_incoming_calls" => "callHierarchy/incomingCalls",
            "get_call_hierarchy_outgoing_calls" => "callHierarchy/outgoingCalls",

            // Editing methods
            "rename_symbol" => "textDocument/rename",
            "format_document" => "textDocument/formatting",
            "format_range" => "textDocument/rangeFormatting",
            "get_code_actions" => "textDocument/codeAction",
            "organize_imports" => "textDocument/codeAction", // With specific params

            // Intelligence methods
            "get_hover" => "textDocument/hover",
            "get_completions" => "textDocument/completion",
            "get_signature_help" => "textDocument/signatureHelp",

            // Diagnostic methods
            "get_diagnostics" => "textDocument/diagnostic",

            // Custom methods (pass through)
            method if method.contains('.') => method,

            _ => {
                return Err(PluginError::method_not_supported(
                    &request.method,
                    &self.metadata.name,
                ));
            }
        };

        // Cache the translation
        {
            let mut cache = self.method_cache.lock().await;
            cache.insert(request.method.clone(), lsp_method.to_string());
        }

        let params = self.build_lsp_params(request, lsp_method)?;
        Ok((lsp_method.to_string(), params))
    }

    /// Build LSP parameters from plugin request
    fn build_lsp_params(&self, request: &PluginRequest, lsp_method: &str) -> PluginResult<Value> {
        // Convert file path to absolute path if needed
        let abs_path = if request.file_path.is_absolute() {
            request.file_path.clone()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .join(&request.file_path)
        };

        // Create proper file:// URI using the url crate
        let file_uri = Url::from_file_path(&abs_path)
            .map_err(|_| {
                PluginError::configuration_error(format!(
                    "Invalid file path: {}",
                    abs_path.display()
                ))
            })?
            .to_string();

        let mut params = json!({
            "textDocument": {
                "uri": file_uri
            }
        });

        // Add position if available
        if let Some(position) = request.position {
            params["position"] = position.to_lsp_position();
        }

        // Add range if available
        if let Some(range) = request.range {
            params["range"] = range.to_lsp_range();
        }

        // Method-specific parameter handling
        match lsp_method {
            "textDocument/references" => {
                params["context"] = json!({
                    "includeDeclaration": request.get_bool_param("include_declaration").unwrap_or(true)
                });
            }
            "textDocument/rename" => {
                if let Some(new_name) = request.get_string_param("new_name") {
                    params["newName"] = json!(new_name);
                } else {
                    return Err(PluginError::configuration_error(
                        "rename_symbol requires new_name parameter",
                    ));
                }
            }
            "workspace/symbol" => {
                if let Some(query) = request.get_string_param("query") {
                    params = json!({ "query": query });
                } else {
                    return Err(PluginError::configuration_error(
                        "search_workspace_symbols requires query parameter",
                    ));
                }
            }
            "textDocument/codeAction" => {
                if request.method == "organize_imports" {
                    params["context"] = json!({
                        "only": ["source.organizeImports"],
                        "diagnostics": []
                    });
                } else {
                    params["context"] = json!({
                        "diagnostics": request.get_param("diagnostics").unwrap_or(&json!([]))
                    });
                }
            }
            "callHierarchy/incomingCalls" | "callHierarchy/outgoingCalls" => {
                // Call hierarchy methods need the item parameter
                if let Some(item) = request.get_param("item") {
                    params = json!({ "item": item });
                } else {
                    return Err(PluginError::configuration_error(
                        "call hierarchy methods require item parameter",
                    ));
                }
            }
            _ => {
                // Copy any additional parameters from the request
                if let Value::Object(request_params) = &request.params {
                    if let Value::Object(params_obj) = &mut params {
                        for (key, value) in request_params {
                            if !params_obj.contains_key(key) {
                                params_obj.insert(key.clone(), value.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(params)
    }

    /// Convert LSP response to plugin response
    fn translate_response(
        &self,
        lsp_result: Value,
        request: &PluginRequest,
    ) -> PluginResult<PluginResponse> {
        // Handle different LSP response formats
        let data = match request.method.as_str() {
            "find_definition"
            | "find_references"
            | "find_implementations"
            | "find_type_definition" => {
                // LSP returns Location[] or LocationLink[]
                self.normalize_locations(lsp_result)?
            }
            "get_document_symbols" => {
                // LSP returns DocumentSymbol[] or SymbolInformation[]
                self.normalize_symbols(lsp_result)?
            }
            "get_hover" => {
                // LSP returns Hover | null
                self.normalize_hover(lsp_result)?
            }
            "get_completions" => {
                // LSP returns CompletionList | CompletionItem[]
                self.normalize_completions(lsp_result)?
            }
            "rename_symbol" => {
                // LSP returns WorkspaceEdit
                self.normalize_workspace_edit(lsp_result)?
            }
            _ => {
                // Pass through other responses
                lsp_result
            }
        };

        Ok(PluginResponse::success(data, &self.metadata.name))
    }

    /// Normalize LSP location responses
    fn normalize_locations(&self, lsp_result: Value) -> PluginResult<Value> {
        match lsp_result {
            Value::Array(locations) => Ok(json!({ "locations": locations })),
            Value::Null => Ok(json!({ "locations": [] })),
            single_location => Ok(json!({ "locations": [single_location] })),
        }
    }

    /// Normalize LSP symbol responses
    fn normalize_symbols(&self, lsp_result: Value) -> PluginResult<Value> {
        match lsp_result {
            Value::Array(symbols) => Ok(json!({ "symbols": symbols })),
            Value::Null => Ok(json!({ "symbols": [] })),
            _ => Ok(json!({ "symbols": [lsp_result] })),
        }
    }

    /// Normalize LSP hover responses
    fn normalize_hover(&self, lsp_result: Value) -> PluginResult<Value> {
        match lsp_result {
            Value::Null => Ok(json!({ "hover": null })),
            hover => Ok(json!({ "hover": hover })),
        }
    }

    /// Normalize LSP completion responses
    fn normalize_completions(&self, lsp_result: Value) -> PluginResult<Value> {
        match &lsp_result {
            Value::Object(obj) if obj.contains_key("items") => {
                // CompletionList format
                Ok(lsp_result)
            }
            Value::Array(items) => {
                // CompletionItem[] format
                Ok(json!({
                    "items": items,
                    "isIncomplete": false
                }))
            }
            Value::Null => Ok(json!({
                "items": [],
                "isIncomplete": false
            })),
            _ => Err(PluginError::serialization_error(
                "Invalid completion response format",
            )),
        }
    }

    /// Normalize LSP workspace edit responses
    fn normalize_workspace_edit(&self, lsp_result: Value) -> PluginResult<Value> {
        // WorkspaceEdit is already in the correct format
        Ok(json!({ "workspace_edit": lsp_result }))
    }
}

#[async_trait]
impl LanguagePlugin for LspAdapterPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn supported_extensions(&self) -> Vec<String> {
        self.extensions.clone()
    }

    fn tool_definitions(&self) -> Vec<Value> {
        vec![
            // Navigation Tools
            json!({
                "name": "find_definition",
                "description": "Find the definition of a symbol by name and kind in a file. Returns definitions for all matching symbols.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "symbol_name": { "type": "string", "description": "The name of the symbol" },
                        "symbol_kind": { "type": "string", "description": "The kind of symbol (function, class, variable, method, etc.)" }
                    },
                    "required": ["file_path", "symbol_name"]
                }
            }),
            json!({
                "name": "find_references",
                "description": "Find all references to a symbol by name and kind in a file. Returns references for all matching symbols.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "symbol_name": { "type": "string", "description": "The name of the symbol" },
                        "symbol_kind": { "type": "string", "description": "The kind of symbol (function, class, variable, method, etc.)" },
                        "include_declaration": { "type": "boolean", "description": "Whether to include the declaration", "default": true }
                    },
                    "required": ["file_path", "symbol_name"]
                }
            }),
            json!({
                "name": "find_implementations",
                "description": "Find all implementations of an interface or abstract class. Useful for finding concrete classes that implement an interface.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "symbol_name": { "type": "string", "description": "The name of the interface or abstract class" },
                        "symbol_kind": { "type": "string", "description": "The kind of symbol (interface, class, etc.)" }
                    },
                    "required": ["file_path", "symbol_name"]
                }
            }),
            json!({
                "name": "find_type_definition",
                "description": "Find the type definition of a symbol. For variables, this finds the type declaration rather than the variable declaration.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "symbol_name": { "type": "string", "description": "The name of the symbol" },
                        "symbol_kind": { "type": "string", "description": "The kind of symbol (variable, property, etc.)" }
                    },
                    "required": ["file_path", "symbol_name"]
                }
            }),
            json!({
                "name": "search_workspace_symbols",
                "description": "Search for symbols (functions, classes, variables, etc.) across the entire workspace. Useful for finding symbols by name across multiple files.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query for symbol names (supports partial matching)" },
                        "workspace_path": { "type": "string", "description": "Optional workspace path to search within (defaults to current working directory)" }
                    },
                    "required": ["query"]
                }
            }),
            json!({
                "name": "get_document_symbols",
                "description": "Get all symbols (functions, classes, variables, etc.) defined in a specific file. Returns a hierarchical structure of symbols with their locations and types.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" }
                    },
                    "required": ["file_path"]
                }
            }),
            // Refactoring Tools
            json!({
                "name": "rename_symbol",
                "description": "Rename a symbol by name and kind in a file. If multiple symbols match, returns candidate positions and suggests using rename_symbol_strict. By default, this will apply the rename to the files. Use dry_run to preview changes without applying them.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "symbol_name": { "type": "string", "description": "The name of the symbol" },
                        "new_name": { "type": "string", "description": "The new name for the symbol" },
                        "symbol_kind": { "type": "string", "description": "The kind of symbol (function, class, variable, method, etc.)" },
                        "dry_run": { "type": "boolean", "description": "If true, only preview the changes without applying them (default: false)" }
                    },
                    "required": ["file_path", "symbol_name", "new_name"]
                }
            }),
            json!({
                "name": "rename_symbol_strict",
                "description": "Rename a symbol at a specific position in a file. Use this when rename_symbol returns multiple candidates. By default, this will apply the rename to the files. Use dry_run to preview changes without applying them.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "line": { "type": "number", "description": "The line number (1-indexed)" },
                        "character": { "type": "number", "description": "The character position in the line (1-indexed)" },
                        "new_name": { "type": "string", "description": "The new name for the symbol" },
                        "dry_run": { "type": "boolean", "description": "If true, only preview the changes without applying them (default: false)" }
                    },
                    "required": ["file_path", "line", "character", "new_name"]
                }
            }),
            // Editing Tools
            json!({
                "name": "organize_imports",
                "description": "Automatically organizes and sorts import statements in a file according to the language-specific conventions. It removes unused imports, groups them, and sorts them.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The absolute path to the file to organize imports for." }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "get_code_actions",
                "description": "Get available code actions (quick fixes, refactors, organize imports) for a file or specific range. Can apply auto-fixes like removing unused imports, adding missing imports, and organizing imports.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "range": {
                            "type": "object",
                            "description": "Optional range to get code actions for. If not provided, gets actions for entire file.",
                            "properties": {
                                "start": {
                                    "type": "object",
                                    "properties": {
                                        "line": { "type": "number", "description": "Start line (0-indexed)" },
                                        "character": { "type": "number", "description": "Start character (0-indexed)" }
                                    },
                                    "required": ["line", "character"]
                                },
                                "end": {
                                    "type": "object",
                                    "properties": {
                                        "line": { "type": "number", "description": "End line (0-indexed)" },
                                        "character": { "type": "number", "description": "End character (0-indexed)" }
                                    },
                                    "required": ["line", "character"]
                                }
                            },
                            "required": ["start", "end"]
                        }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "format_document",
                "description": "Format a document using the language server's formatter. Applies consistent code style and formatting rules.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file to format" },
                        "options": {
                            "type": "object",
                            "description": "Formatting options",
                            "properties": {
                                "tab_size": { "type": "number", "description": "Size of tabs (default: 2)" },
                                "insert_spaces": { "type": "boolean", "description": "Use spaces instead of tabs (default: true)" },
                                "trim_trailing_whitespace": { "type": "boolean", "description": "Trim trailing whitespace" },
                                "insert_final_newline": { "type": "boolean", "description": "Insert final newline" },
                                "trim_final_newlines": { "type": "boolean", "description": "Trim final newlines" }
                            }
                        }
                    },
                    "required": ["file_path"]
                }
            }),
            // Intelligence Tools
            json!({
                "name": "get_hover",
                "description": "Get hover information (documentation, types, signatures) for a symbol at a specific position. Provides rich context about project-specific APIs and functions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "line": { "type": "number", "description": "The line number (1-indexed)" },
                        "character": { "type": "number", "description": "The character position in the line (0-indexed)" }
                    },
                    "required": ["file_path", "line", "character"]
                }
            }),
            json!({
                "name": "get_completions",
                "description": "Get intelligent code completions for a specific position. Returns project-aware suggestions including imports, methods, properties, and context-specific completions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "line": { "type": "number", "description": "The line number (1-indexed)" },
                        "character": { "type": "number", "description": "The character position in the line (0-indexed)" },
                        "trigger_character": { "type": "string", "description": "Optional trigger character (e.g., \".\", \":\", \">\") that caused the completion request" }
                    },
                    "required": ["file_path", "line", "character"]
                }
            }),
            json!({
                "name": "get_signature_help",
                "description": "Get function signature help at a specific position. Shows function signatures, parameter information, and documentation for the function being called. Critical for AI agents when generating function calls with correct parameters.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "line": { "type": "number", "description": "The line number (1-indexed)" },
                        "character": { "type": "number", "description": "The character position in the line (0-indexed)" },
                        "trigger_character": { "type": "string", "description": "Optional trigger character that invoked signature help (e.g., \"(\", \",\")" }
                    },
                    "required": ["file_path", "line", "character"]
                }
            }),
            // Diagnostics Tools
            json!({
                "name": "get_diagnostics",
                "description": "Get language diagnostics (errors, warnings, hints) for a file. Uses LSP textDocument/diagnostic to pull current diagnostics.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file to get diagnostics for" }
                    },
                    "required": ["file_path"]
                }
            }),
            // File Management Tools
            json!({
                "name": "rename_file",
                "description": "Rename or move a file and automatically update all import statements that reference it. Works with TypeScript, JavaScript, JSX, and TSX files.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "old_path": { "type": "string", "description": "Current path to the file" },
                        "new_path": { "type": "string", "description": "New path for the file (can be in a different directory)" },
                        "dry_run": { "type": "boolean", "description": "Preview changes without applying them (default: false)" }
                    },
                    "required": ["old_path", "new_path"]
                }
            }),
            json!({
                "name": "create_file",
                "description": "Create a new file with optional content and notify relevant LSP servers. Ensures proper LSP workspace synchronization for newly created files.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path where the new file should be created" },
                        "content": { "type": "string", "description": "Initial content for the file (default: empty string)" },
                        "overwrite": { "type": "boolean", "description": "Whether to overwrite existing file if it exists (default: false)" },
                        "dry_run": { "type": "boolean", "description": "Preview changes without applying them (default: false)" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "delete_file",
                "description": "Delete a file and notify relevant LSP servers. Ensures proper LSP workspace synchronization and cleanup for deleted files.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file to delete" },
                        "force": { "type": "boolean", "description": "Force deletion even if file has uncommitted changes (default: false)" },
                        "dry_run": { "type": "boolean", "description": "Preview changes without applying them (default: false)" }
                    },
                    "required": ["file_path"]
                }
            }),
            // Call Hierarchy Tools
            json!({
                "name": "prepare_call_hierarchy",
                "description": "Prepare for a call hierarchy request.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file" },
                        "line": { "type": "number", "description": "The line number (1-indexed)" },
                        "character": { "type": "number", "description": "The character position in the line (0-indexed)" }
                    },
                    "required": ["file_path", "line", "character"]
                }
            }),
            json!({
                "name": "get_call_hierarchy_incoming_calls",
                "description": "Get incoming calls for a call hierarchy item.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "item": {
                            "type": "object",
                            "description": "The call hierarchy item"
                        }
                    },
                    "required": ["item"]
                }
            }),
            json!({
                "name": "get_call_hierarchy_outgoing_calls",
                "description": "Get outgoing calls for a call hierarchy item.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "item": {
                            "type": "object",
                            "description": "The call hierarchy item"
                        }
                    },
                    "required": ["item"]
                }
            }),
            // LSP Notification Tools
            json!({
                "name": "notify_file_opened",
                "description": "Notify the LSP server that a file has been opened. This helps ensure the language server is aware of files for proper project indexing and symbol resolution.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file that was opened" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "notify_file_saved",
                "description": "Notify the LSP server that a file has been saved. Triggers on_file_save hooks on all plugins that support the file extension.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file that was saved" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "notify_file_closed",
                "description": "Notify the LSP server that a file has been closed. Triggers on_file_close hooks on all plugins that support the file extension.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "The path to the file that was closed" }
                    },
                    "required": ["file_path"]
                }
            }),
        ]
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }

    async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse> {
        debug!("LSP adapter handling request: {}", request.method);

        // Skip file extension check for workspace-level operations
        let is_workspace_operation = matches!(request.method.as_str(), "search_workspace_symbols");

        // Check if we support the file extension (skip for workspace operations)
        if !is_workspace_operation && !self.can_handle_file(&request.file_path) {
            return Err(PluginError::plugin_not_found(
                request.file_path.to_string_lossy(),
                &request.method,
            ));
        }

        // Translate plugin request to LSP request
        let (lsp_method, lsp_params) = self.translate_request(&request).await?;

        debug!(
            "Translated to LSP method: {} with params: {}",
            lsp_method, lsp_params
        );

        // Send request to LSP service
        match self.lsp_service.request(&lsp_method, lsp_params).await {
            Ok(lsp_result) => {
                debug!("LSP service returned result");
                self.translate_response(lsp_result, &request)
            }
            Err(err) => {
                error!("LSP service error: {}", err);
                Err(PluginError::request_failed(&self.metadata.name, err))
            }
        }
    }

    fn configure(&self, _config: Value) -> PluginResult<()> {
        // LSP adapters generally don't need additional configuration
        // The LSP service handles its own configuration
        Ok(())
    }

    fn on_file_open(&self, path: &Path) -> PluginResult<()> {
        debug!(
            path = %path.display(),
            plugin = %self.metadata.name,
            "File opened - hook triggered"
        );

        // Note: The actual LSP textDocument/didOpen notification is sent by
        // the DirectLspAdapter in plugin_dispatcher.rs via LspClient::notify_file_opened().
        // This hook serves as a notification point for the plugin to be aware of file lifecycle.
        // Future enhancements could add plugin-specific logic here (e.g., invalidate caches,
        // update internal state, etc.)

        Ok(())
    }

    fn on_file_save(&self, path: &Path) -> PluginResult<()> {
        debug!(
            path = %path.display(),
            plugin = %self.metadata.name,
            "File saved - hook triggered"
        );

        // Note: Future implementation could send textDocument/didSave notification
        // when notify_file_saved tool is added to the MCP API

        Ok(())
    }

    fn on_file_close(&self, path: &Path) -> PluginResult<()> {
        debug!(
            path = %path.display(),
            plugin = %self.metadata.name,
            "File closed - hook triggered"
        );

        // Note: Future implementation could send textDocument/didClose notification
        // when notify_file_closed tool is added to the MCP API

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct MockLspService {
        name: String,
        extensions: Vec<String>,
    }

    #[async_trait]
    impl LspService for MockLspService {
        async fn request(&self, method: &str, _params: Value) -> Result<Value, String> {
            match method {
                "textDocument/definition" => Ok(json!([{
                    "uri": "file:///test.ts",
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 10 }
                    }
                }])),
                "textDocument/hover" => Ok(json!({
                    "contents": "test hover content"
                })),
                _ => Ok(json!(null)),
            }
        }

        fn supports_extension(&self, extension: &str) -> bool {
            self.extensions.contains(&extension.to_string())
        }

        fn service_name(&self) -> String {
            self.name.clone()
        }
    }

    #[tokio::test]
    async fn test_lsp_adapter_basic_functionality() {
        let lsp_service = Arc::new(MockLspService {
            name: "test-lsp".to_string(),
            extensions: vec!["ts".to_string()],
        });

        let adapter = LspAdapterPlugin::typescript(lsp_service);

        assert_eq!(adapter.metadata().name, "typescript-lsp-adapter");
        assert!(adapter.supported_extensions().contains(&"ts".to_string()));
        assert!(adapter.capabilities().navigation.go_to_definition);
        assert!(adapter.capabilities().intelligence.hover);
    }

    #[tokio::test]
    async fn test_request_translation() {
        let lsp_service = Arc::new(MockLspService {
            name: "test-lsp".to_string(),
            extensions: vec!["ts".to_string()],
        });

        let adapter = LspAdapterPlugin::typescript(lsp_service);

        let request =
            PluginRequest::new("find_definition", PathBuf::from("test.ts")).with_position(10, 20);

        let response = adapter.handle_request(request).await.unwrap();
        assert!(response.success);
        assert!(response.data.is_some());

        let data = response.data.unwrap();
        assert!(data.get("locations").is_some());
    }

    #[tokio::test]
    async fn test_hover_request() {
        let lsp_service = Arc::new(MockLspService {
            name: "test-lsp".to_string(),
            extensions: vec!["ts".to_string()],
        });

        let adapter = LspAdapterPlugin::typescript(lsp_service);

        let request =
            PluginRequest::new("get_hover", PathBuf::from("test.ts")).with_position(5, 10);

        let response = adapter.handle_request(request).await.unwrap();
        assert!(response.success);

        let data = response.data.unwrap();
        assert!(data.get("hover").is_some());
    }

    #[tokio::test]
    async fn test_unsupported_method() {
        let lsp_service = Arc::new(MockLspService {
            name: "test-lsp".to_string(),
            extensions: vec!["ts".to_string()],
        });

        let adapter = LspAdapterPlugin::typescript(lsp_service);

        let request = PluginRequest::new("unsupported_method", PathBuf::from("test.ts"));

        let result = adapter.handle_request(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginError::MethodNotSupported { .. }
        ));
    }

    #[tokio::test]
    async fn test_language_specific_adapters() {
        let lsp_service = Arc::new(MockLspService {
            name: "test-lsp".to_string(),
            extensions: vec!["py".to_string()],
        });

        let python_adapter = LspAdapterPlugin::python(lsp_service.clone());
        assert_eq!(python_adapter.metadata().name, "python-lsp-adapter");
        assert!(python_adapter
            .supported_extensions()
            .contains(&"py".to_string()));
        // Python adapter should have the custom capability we added
        assert!(python_adapter
            .capabilities()
            .custom
            .contains_key("python.format_imports"));

        let go_adapter = LspAdapterPlugin::go(lsp_service.clone());
        assert_eq!(go_adapter.metadata().name, "go-lsp-adapter");
        // Go adapter should have both capabilities
        assert!(go_adapter.capabilities().custom.contains_key("go.generate"));
        assert!(go_adapter
            .capabilities()
            .custom
            .contains_key("go.organize_imports"));

        let rust_adapter = LspAdapterPlugin::rust(lsp_service);
        assert_eq!(rust_adapter.metadata().name, "rust-lsp-adapter");
        assert!(rust_adapter
            .capabilities()
            .custom
            .contains_key("rust.expand_macro"));
    }

    #[tokio::test]
    async fn test_consistent_capabilities() {
        let lsp_service = Arc::new(MockLspService {
            name: "test-consistency-lsp".to_string(),
            extensions: vec![
                "ts".to_string(),
                "py".to_string(),
                "go".to_string(),
                "rs".to_string(),
            ],
        });

        let ts_adapter = LspAdapterPlugin::typescript(lsp_service.clone());
        let py_adapter = LspAdapterPlugin::python(lsp_service.clone());
        let go_adapter = LspAdapterPlugin::go(lsp_service.clone());
        let rs_adapter = LspAdapterPlugin::rust(lsp_service.clone());

        let adapters = vec![ts_adapter, py_adapter, go_adapter, rs_adapter];

        for adapter in adapters {
            let caps = adapter.capabilities();
            assert!(
                caps.editing.auto_imports,
                "auto_imports should be enabled for {}",
                adapter.metadata.name
            );
            assert!(
                caps.editing.organize_imports,
                "organize_imports should be enabled for {}",
                adapter.metadata.name
            );
            assert!(
                caps.refactoring.extract_function,
                "extract_function should be enabled for {}",
                adapter.metadata.name
            );
            assert!(
                caps.refactoring.extract_variable,
                "extract_variable should be enabled for {}",
                adapter.metadata.name
            );
            assert!(
                caps.refactoring.inline_variable,
                "inline_variable should be enabled for {}",
                adapter.metadata.name
            );
        }
    }
}
