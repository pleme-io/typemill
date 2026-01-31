use super::{LspAdapterPlugin, PluginRequest, PluginResponse, PluginResult, PluginSystemError};
use mill_plugin_api::SymbolKind;
use serde_json::{json, Value};

impl LspAdapterPlugin {
    /// Convert LSP response to plugin response
    pub(crate) fn translate_response(
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
            "search_workspace_symbols" => {
                // LSP returns SymbolInformation[]
                self.normalize_workspace_symbols(lsp_result, request)?
            }
            "get_hover" => {
                // LSP returns Hover | null
                self.normalize_hover(lsp_result)?
            }
            "get_completions" => {
                // LSP returns CompletionList | CompletionItem[]
                self.normalize_completions(lsp_result)?
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
            _ => Err(PluginSystemError::serialization_error(
                "Invalid completion response format",
            )),
        }
    }

    /// Normalize LSP workspace symbol responses with filtering
    fn normalize_workspace_symbols(
        &self,
        lsp_result: Value,
        request: &PluginRequest,
    ) -> PluginResult<Value> {
        let symbols = match lsp_result {
            Value::Array(symbols) => symbols,
            Value::Null => return Ok(Value::Array(vec![])),
            single => vec![single],
        };

        // Check for kind filter
        let kind_filter = if let Some(kind_val) = request.get_param("kind") {
            // Deserialize string/int to SymbolKind enum
            serde_json::from_value::<SymbolKind>(kind_val.clone()).ok()
        } else {
            None
        };

        if let Some(target_kind) = kind_filter {
            let filtered: Vec<Value> = symbols
                .into_iter()
                .filter(|symbol| {
                    if let Some(kind_num) = symbol.get("kind").and_then(|k| k.as_u64()) {
                        if let Some(sym_kind) = SymbolKind::from_lsp_kind(kind_num) {
                            return sym_kind == target_kind;
                        }
                    }
                    false
                })
                .collect();
            // LspAdapterPlugin normally returns raw data for pass-through methods,
            // but for normalized methods it usually wraps in object?
            // "search_workspace_symbols" previously fell into pass-through (_ => lsp_result),
            // so it returned Array directly (not wrapped in object).
            // So we should return Array directly here to match previous behavior if no filter,
            // or filtered Array.
            Ok(Value::Array(filtered))
        } else {
            Ok(Value::Array(symbols))
        }
    }

    /// Normalize LSP workspace edit responses
    #[allow(dead_code)] // Reserved for future workspace edit support
    fn normalize_workspace_edit(&self, lsp_result: Value) -> PluginResult<Value> {
        // WorkspaceEdit is already in the correct format
        Ok(json!({ "workspace_edit": lsp_result }))
    }
}
