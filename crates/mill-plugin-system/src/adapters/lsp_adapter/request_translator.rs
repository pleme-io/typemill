use super::{LspAdapterPlugin, PluginRequest, PluginResult, PluginSystemError};
use serde_json::{json, Value};
use url::Url;

/// Check if LSP method cache is enabled via environment variables
/// Returns true if cache should be used, false if disabled
fn is_lsp_method_cache_enabled() -> bool {
    // Check master switch first
    if let Ok(val) = std::env::var("TYPEMILL_DISABLE_CACHE") {
        if val == "1" || val.to_lowercase() == "true" {
            return false;
        }
    }

    // Check LSP-specific switch
    if let Ok(val) = std::env::var("TYPEMILL_DISABLE_LSP_METHOD_CACHE") {
        if val == "1" || val.to_lowercase() == "true" {
            return false;
        }
    }

    true
}

impl LspAdapterPlugin {
    /// Convert plugin request to LSP method and params
    pub(crate) async fn translate_request(
        &self,
        request: &PluginRequest,
    ) -> PluginResult<(String, Value)> {
        // Check cache first (if enabled)
        if is_lsp_method_cache_enabled() {
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
                return Err(PluginSystemError::method_not_supported(
                    &request.method,
                    &self.metadata.name,
                ));
            }
        };

        // Cache the translation (if enabled)
        if is_lsp_method_cache_enabled() {
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
                PluginSystemError::configuration_error(format!(
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
            "workspace/symbol" => {
                if let Some(query) = request.get_string_param("query") {
                    params = json!({ "query": query });

                    // Forward 'kind' parameter if present to allow server-side filtering (optimization)
                    // This allows DirectLspAdapter to filter symbols before aggregating them,
                    // reducing memory usage and transfer overhead.
                    if let Some(kind) = request.get_param("kind") {
                        if let Value::Object(ref mut map) = params {
                            map.insert("kind".to_string(), kind.clone());
                        }
                    }
                } else {
                    return Err(PluginSystemError::configuration_error(
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
                    return Err(PluginSystemError::configuration_error(
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
}
