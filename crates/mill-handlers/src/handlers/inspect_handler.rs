//! Inspect handler for aggregated code intelligence
//!
//! Implements the `inspect_code` tool which aggregates multiple LSP operations
//! into a single unified response.

use super::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_plugin_system::PluginRequest;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, warn};

pub struct InspectHandler;

impl InspectHandler {
    pub fn new() -> Self {
        Self
    }

    /// Parse input parameters from tool call
    fn parse_params(tool_call: &ToolCall) -> ServerResult<InspectParams> {
        let args = tool_call
            .arguments
            .clone()
            .ok_or_else(|| ServerError::invalid_request("Missing arguments for inspect_code"))?;

        let params: InspectParams = serde_json::from_value(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid inspect_code parameters: {}", e))
        })?;

        // Validate that we have either position-based or name-based lookup
        if params.symbol_name.is_none() && (params.line.is_none() || params.character.is_none()) {
            return Err(ServerError::invalid_request(
                "Must provide either symbolName or both line and character",
            ));
        }

        Ok(params)
    }

    /// Get symbol position by name (if needed)
    async fn resolve_symbol_position(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        file_path: &PathBuf,
        symbol_name: &str,
    ) -> ServerResult<Option<(u32, u32)>> {
        debug!(
            file = %file_path.display(),
            symbol = %symbol_name,
            "Resolving symbol position by name"
        );

        let request = PluginRequest::new("get_document_symbols".to_string(), file_path.clone());

        match context.plugin_manager.handle_request(request).await {
            Ok(response) => {
                if let Some(data) = response.data {
                    if let Some(symbols) = data.as_array() {
                        // Search for symbol by name
                        for symbol in symbols {
                            if let Some(name) = symbol.get("name").and_then(|n| n.as_str()) {
                                if name == symbol_name {
                                    // Extract position from symbol
                                    let position = symbol
                                        .get("selectionRange")
                                        .or_else(|| symbol.get("range"))
                                        .and_then(|r| r.get("start"));

                                    if let Some(pos) = position {
                                        let line = pos.get("line").and_then(|v| v.as_u64());
                                        let character =
                                            pos.get("character").and_then(|v| v.as_u64());

                                        if let (Some(l), Some(c)) = (line, character) {
                                            debug!(
                                                symbol = %symbol_name,
                                                line = l,
                                                character = c,
                                                "Resolved symbol position"
                                            );
                                            return Ok(Some((l as u32, c as u32)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(None)
            }
            Err(e) => {
                warn!(
                    error = %e,
                    file = %file_path.display(),
                    "Failed to get document symbols for name resolution"
                );
                Ok(None)
            }
        }
    }

    /// Aggregate LSP operations based on include flags
    async fn aggregate_intelligence(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        params: &InspectParams,
        line: u32,
        character: u32,
    ) -> ServerResult<InspectResult> {
        let file_path = PathBuf::from(&params.file_path);
        let mut result = InspectResult::default();

        // Include lists based on detail level
        const BASIC_INCLUDE: &[&str] = &["definition", "typeInfo"];
        const DEEP_INCLUDE: &[&str] = &[
            "definition",
            "typeInfo",
            "references",
            "implementations",
            "callHierarchy",
            "diagnostics",
        ];

        // Get include list: explicit include > detailLevel > default (basic)
        let include: Vec<String> = match &params.include {
            Some(inc) => inc.clone(),
            None => {
                let level = params.detail_level.as_deref().unwrap_or("basic");
                match level {
                    "deep" => DEEP_INCLUDE.iter().map(|s| s.to_string()).collect(),
                    _ => BASIC_INCLUDE.iter().map(|s| s.to_string()).collect(),
                }
            }
        };

        debug!(
            file = %file_path.display(),
            line = line,
            character = character,
            include = ?include,
            "Aggregating code intelligence"
        );

        // Execute LSP operations in parallel where possible
        let mut tasks = Vec::new();

        for item in include {
            let method = match item.as_str() {
                "definition" => "find_definition",
                "typeInfo" => "get_hover",
                "references" => "find_references",
                "implementations" => "find_implementations",
                "callHierarchy" => "prepare_call_hierarchy",
                "diagnostics" => "get_diagnostics",
                _ => {
                    warn!(include_item = %item, "Unknown include item, skipping");
                    continue;
                }
            };

            let mut request = PluginRequest::new(method.to_string(), file_path.clone());
            request = request.with_position(line, character);

            tasks.push(async move {
                (
                    item.clone(),
                    context.plugin_manager.handle_request(request).await,
                )
            });
        }

        // Execute all tasks
        for task in tasks {
            let (item, response_result) = task.await;

            match response_result {
                Ok(response) => {
                    let content = response.data.unwrap_or(json!(null));

                    match item.as_str() {
                        "definition" => {
                            result.definition = Some(self.apply_pagination(&content, params));
                        }
                        "typeInfo" => {
                            result.type_info = Some(content);
                        }
                        "references" => {
                            result.references = Some(self.apply_pagination(&content, params));
                        }
                        "implementations" => {
                            result.implementations = Some(self.apply_pagination(&content, params));
                        }
                        "callHierarchy" => {
                            result.call_hierarchy = Some(self.apply_pagination(&content, params));
                        }
                        "diagnostics" => {
                            result.diagnostics = Some(self.apply_pagination(&content, params));
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    warn!(
                        include_item = %item,
                        error = %e,
                        "Failed to fetch intelligence data"
                    );
                    // Continue with other operations even if one fails
                }
            }
        }

        Ok(result)
    }

    /// Apply pagination to list results
    fn apply_pagination(&self, content: &Value, params: &InspectParams) -> Value {
        if let Some(arr) = content.as_array() {
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit;

            let paginated: Vec<Value> = arr
                .iter()
                .skip(offset)
                .take(limit.unwrap_or(usize::MAX))
                .cloned()
                .collect();

            json!({
                "items": paginated,
                "total": arr.len(),
                "offset": offset,
                "limit": limit
            })
        } else {
            content.clone()
        }
    }
}

impl Default for InspectHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for InspectHandler {
    fn tool_names(&self) -> &[&str] {
        &["inspect_code"]
    }


    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        debug!(tool_name = %tool_call.name, "Handling inspect_code");

        let params = Self::parse_params(tool_call)?;

        // Determine the position to inspect
        let (line, character) = if let Some(symbol_name) = &params.symbol_name {
            // Name-based lookup: resolve symbol to position
            let file_path = PathBuf::from(&params.file_path);
            match self
                .resolve_symbol_position(context, &file_path, symbol_name)
                .await?
            {
                Some((l, c)) => (l, c),
                None => {
                    return Err(ServerError::invalid_request(format!(
                        "Symbol '{}' not found in file",
                        symbol_name
                    )));
                }
            }
        } else {
            // Position-based lookup: use provided coordinates (0-based)
            let line = params.line.unwrap();
            let character = params.character.unwrap();
            (line as u32, character as u32)
        };

        // Aggregate intelligence data
        let result = self
            .aggregate_intelligence(context, &params, line, character)
            .await?;

        // Serialize result
        let result_json = serde_json::to_value(&result).map_err(|e| {
            ServerError::internal(format!("Failed to serialize inspect result: {}", e))
        })?;

        Ok(json!({
            "content": result_json
        }))
    }
}

/// Input parameters for inspect_code
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InspectParams {
    /// File path to inspect
    file_path: String,
    /// Line number (0-based, optional if symbolName is provided)
    line: Option<u64>,
    /// Character position (0-based, optional if symbolName is provided)
    character: Option<u64>,
    /// Symbol name for name-based lookup (alternative to line/character)
    symbol_name: Option<String>,
    /// What intelligence to include (defaults based on detailLevel)
    #[serde(default)]
    include: Option<Vec<String>>,
    /// Detail level: "basic" (definition, typeInfo) | "deep" (all fields)
    #[serde(default, alias = "detailLevel")]
    detail_level: Option<String>,
    /// Maximum number of results per list field (pagination)
    #[serde(default)]
    limit: Option<usize>,
    /// Offset for pagination (skip first N items)
    #[serde(default)]
    offset: Option<usize>,
}

/// Aggregated inspection result
#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectResult {
    /// Symbol definition location
    #[serde(skip_serializing_if = "Option::is_none")]
    definition: Option<Value>,
    /// Type information and documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    type_info: Option<Value>,
    /// All references to the symbol
    #[serde(skip_serializing_if = "Option::is_none")]
    references: Option<Value>,
    /// Interface implementations
    #[serde(skip_serializing_if = "Option::is_none")]
    implementations: Option<Value>,
    /// Call hierarchy (incoming/outgoing calls)
    #[serde(skip_serializing_if = "Option::is_none")]
    call_hierarchy: Option<Value>,
    /// Diagnostics (errors/warnings) at position
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_params_position_based() {
        let tool_call = ToolCall {
            name: "inspect_code".to_string(),
            arguments: Some(json!({
                "filePath": "src/main.rs",
                "line": 10,
                "character": 5,
                "include": ["definition", "references"]
            })),
        };

        let params = InspectHandler::parse_params(&tool_call).unwrap();
        assert_eq!(params.file_path, "src/main.rs");
        assert_eq!(params.line, Some(10));
        assert_eq!(params.character, Some(5));
        assert!(params.symbol_name.is_none());
        assert_eq!(params.include.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_parse_params_name_based() {
        let tool_call = ToolCall {
            name: "inspect_code".to_string(),
            arguments: Some(json!({
                "filePath": "src/main.rs",
                "symbolName": "main"
            })),
        };

        let params = InspectHandler::parse_params(&tool_call).unwrap();
        assert_eq!(params.file_path, "src/main.rs");
        assert_eq!(params.symbol_name, Some("main".to_string()));
        assert!(params.line.is_none());
        assert!(params.character.is_none());
    }

    #[test]
    fn test_parse_params_missing_position() {
        let tool_call = ToolCall {
            name: "inspect_code".to_string(),
            arguments: Some(json!({
                "filePath": "src/main.rs"
            })),
        };

        let result = InspectHandler::parse_params(&tool_call);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_pagination() {
        let handler = InspectHandler::new();
        let items = json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let params = InspectParams {
            file_path: "test.rs".to_string(),
            line: Some(1),
            character: Some(0),
            symbol_name: None,
            include: None,
            detail_level: None,
            limit: Some(3),
            offset: Some(2),
        };

        let result = handler.apply_pagination(&items, &params);
        let result_items = result.get("items").unwrap().as_array().unwrap();

        assert_eq!(result_items.len(), 3);
        assert_eq!(result_items[0], 3);
        assert_eq!(result_items[1], 4);
        assert_eq!(result_items[2], 5);
        assert_eq!(result.get("total").unwrap().as_u64().unwrap(), 10);
    }

    #[test]
    fn test_apply_pagination_no_limit() {
        let handler = InspectHandler::new();
        let items = json!([1, 2, 3]);

        let params = InspectParams {
            file_path: "test.rs".to_string(),
            line: Some(1),
            character: Some(0),
            symbol_name: None,
            include: None,
            detail_level: None,
            limit: None,
            offset: Some(1),
        };

        let result = handler.apply_pagination(&items, &params);
        let result_items = result.get("items").unwrap().as_array().unwrap();

        assert_eq!(result_items.len(), 2);
        assert_eq!(result_items[0], 2);
        assert_eq!(result_items[1], 3);
    }
}
