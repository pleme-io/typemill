use super::{LspAdapterPlugin, PluginError, PluginRequest, PluginResponse, PluginResult};
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