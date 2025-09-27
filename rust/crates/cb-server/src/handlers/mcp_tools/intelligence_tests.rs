//! Integration tests for intelligence tools (get_hover, get_completions, get_signature_help)

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::handlers::McpDispatcher;
    use crate::state::AppState;
    use crate::services::{FileService, SymbolService, EditingService, ImportService, LspService};
    use crate::systems::lsp::MockLspService;
    use std::sync::Arc;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use serde_json::{json, Value};
    use lsp_types::{
        Hover, HoverContents, MarkupContent, MarkupKind,
        CompletionItem, CompletionItemKind, SignatureHelp,
        SignatureInformation, ParameterInformation, ParameterLabel
    };

    /// Create a test AppState with mock services
    fn create_test_app_state() -> Arc<AppState> {
        let mut mock_lsp = MockLspService::new();

        // Setup default mock responses
        mock_lsp.expect_is_initialized()
            .returning(|_| true);

        Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        })
    }

    /// Create a temporary TypeScript file with content
    fn create_temp_ts_file(content: &str) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        Ok(file)
    }

    #[tokio::test]
    async fn test_get_hover_success() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        // Setup mock LSP response for hover
        let mock_lsp = MockLspService::new();
        let mut mock_lsp_clone = mock_lsp.clone();
        mock_lsp_clone.expect_hover()
            .withf(|path, line, char| {
                path.ends_with("test.ts") && *line == 1 && *char == 5
            })
            .returning(|_, _, _| {
                Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: "```typescript\nfunction testFunction(): void\n```\nTest function documentation".to_string(),
                    }),
                    range: None,
                }))
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp_clone.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp_clone.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp_clone),
        });

        // Register tools
        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("function testFunction() {\n  console.log('test');\n}").unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path,
            "line": 1,
            "character": 5
        });

        let result = dispatcher.call_tool_for_test("get_hover", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response["contents"].is_string());
        assert!(response["contents"].as_str().unwrap().contains("testFunction"));
    }

    #[tokio::test]
    async fn test_get_hover_no_info() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_hover()
            .returning(|_, _, _| Ok(None));

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("// Empty file").unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path,
            "line": 1,
            "character": 1
        });

        let result = dispatcher.call_tool_for_test("get_hover", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["contents"], Value::Null);
    }

    #[tokio::test]
    async fn test_get_completions_success() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_completions()
            .returning(|_, _, _, _| {
                Ok(vec![
                    CompletionItem {
                        label: "console".to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some("Console object".to_string()),
                        documentation: None,
                        deprecated: Some(false),
                        preselect: Some(false),
                        sort_text: None,
                        filter_text: None,
                        insert_text: Some("console".to_string()),
                        insert_text_format: None,
                        insert_text_mode: None,
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: None,
                    },
                    CompletionItem {
                        label: "const".to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        detail: Some("const keyword".to_string()),
                        documentation: None,
                        deprecated: Some(false),
                        preselect: Some(false),
                        sort_text: None,
                        filter_text: None,
                        insert_text: Some("const ".to_string()),
                        insert_text_format: None,
                        insert_text_mode: None,
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: None,
                    },
                ])
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("con").unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path,
            "line": 1,
            "character": 3
        });

        let result = dispatcher.call_tool_for_test("get_completions", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response["items"].is_array());
        let items = response["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["label"], "console");
        assert_eq!(items[1]["label"], "const");
    }

    #[tokio::test]
    async fn test_get_completions_with_trigger_character() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_completions()
            .withf(|_, _, _, trigger| trigger == &Some(".".to_string()))
            .returning(|_, _, _, _| {
                Ok(vec![
                    CompletionItem {
                        label: "log".to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("(method) Console.log(...data: any[]): void".to_string()),
                        documentation: None,
                        deprecated: Some(false),
                        preselect: Some(false),
                        sort_text: None,
                        filter_text: None,
                        insert_text: Some("log".to_string()),
                        insert_text_format: None,
                        insert_text_mode: None,
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: None,
                    },
                ])
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("console.").unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path,
            "line": 1,
            "character": 8,
            "trigger_character": "."
        });

        let result = dispatcher.call_tool_for_test("get_completions", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response["items"].is_array());
        let items = response["items"].as_array().unwrap();
        assert!(items.len() > 0);
        assert_eq!(items[0]["label"], "log");
    }

    #[tokio::test]
    async fn test_get_signature_help_success() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_signature_help()
            .returning(|_, _, _, _| {
                Ok(Some(SignatureHelp {
                    signatures: vec![
                        SignatureInformation {
                            label: "testFunction(param1: string, param2: number): void".to_string(),
                            documentation: Some(lsp_types::Documentation::String(
                                "Test function with two parameters".to_string()
                            )),
                            parameters: Some(vec![
                                ParameterInformation {
                                    label: ParameterLabel::Simple("param1: string".to_string()),
                                    documentation: Some(lsp_types::Documentation::String(
                                        "First parameter".to_string()
                                    )),
                                },
                                ParameterInformation {
                                    label: ParameterLabel::Simple("param2: number".to_string()),
                                    documentation: Some(lsp_types::Documentation::String(
                                        "Second parameter".to_string()
                                    )),
                                },
                            ]),
                            active_parameter: None,
                        },
                    ],
                    active_signature: Some(0),
                    active_parameter: Some(0),
                }))
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("testFunction(").unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path,
            "line": 1,
            "character": 13,
            "trigger_character": "("
        });

        let result = dispatcher.call_tool_for_test("get_signature_help", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response["signatures"].is_array());
        let signatures = response["signatures"].as_array().unwrap();
        assert_eq!(signatures.len(), 1);
        assert!(signatures[0]["label"].as_str().unwrap().contains("testFunction"));
        assert_eq!(response["activeSignature"], 0);
        assert_eq!(response["activeParameter"], 0);
    }

    #[tokio::test]
    async fn test_get_signature_help_no_signature() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_signature_help()
            .returning(|_, _, _, _| Ok(None));

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("// No function call").unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path,
            "line": 1,
            "character": 1
        });

        let result = dispatcher.call_tool_for_test("get_signature_help", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response["signatures"].is_array());
        assert_eq!(response["signatures"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_intelligence_tools_invalid_args() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        // Test get_hover with missing file_path
        let args = json!({
            "line": 1,
            "character": 5
        });
        let result = dispatcher.call_tool_for_test("get_hover", args).await;
        assert!(result.is_err());

        // Test get_completions with invalid line
        let args = json!({
            "file_path": "/test.ts",
            "line": 0,  // Invalid: lines are 1-based
            "character": 5
        });
        let result = dispatcher.call_tool_for_test("get_completions", args).await;
        assert!(result.is_err());

        // Test get_signature_help with invalid character
        let args = json!({
            "file_path": "/test.ts",
            "line": 1,
            "character": -1  // Invalid: negative character position
        });
        let result = dispatcher.call_tool_for_test("get_signature_help", args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_intelligence_tools_with_errors() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();

        // Mock LSP errors
        mock_lsp.expect_hover()
            .returning(|_, _, _| Err(crate::error::ServerError::LspError("LSP server error".into())));
        mock_lsp.expect_completions()
            .returning(|_, _, _, _| Err(crate::error::ServerError::LspError("Completions failed".into())));
        mock_lsp.expect_signature_help()
            .returning(|_, _, _, _| Err(crate::error::ServerError::LspError("Signature help failed".into())));

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("test content").unwrap();
        let file_path = file.path().to_str().unwrap();

        // Test get_hover error handling
        let args = json!({
            "file_path": file_path,
            "line": 1,
            "character": 1
        });
        let result = dispatcher.call_tool_for_test("get_hover", args.clone()).await;
        assert!(result.is_err());

        // Test get_completions error handling
        let result = dispatcher.call_tool_for_test("get_completions", args.clone()).await;
        assert!(result.is_err());

        // Test get_signature_help error handling
        let result = dispatcher.call_tool_for_test("get_signature_help", args).await;
        assert!(result.is_err());
    }
}