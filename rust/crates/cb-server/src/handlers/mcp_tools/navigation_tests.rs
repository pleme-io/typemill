//! Integration tests for navigation tools including get_document_symbols

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::handlers::McpDispatcher;
    use crate::state::AppState;
    use crate::services::{FileService, SymbolService, EditingService, ImportService};
    use crate::systems::lsp::MockLspService;
    use std::sync::Arc;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use serde_json::{json, Value};
    use lsp_types::{
        DocumentSymbol, SymbolKind, Range, Position,
        DocumentSymbolResponse, SymbolInformation, Location, Uri
    };

    /// Create a test AppState with mock services
    fn create_test_app_state() -> Arc<AppState> {
        let mock_lsp = MockLspService::new();
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
    async fn test_get_document_symbols_hierarchical() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_document_symbols()
            .returning(|_| {
                Ok(DocumentSymbolResponse::Nested(vec![
                    DocumentSymbol {
                        name: "MyClass".to_string(),
                        detail: Some("class MyClass".to_string()),
                        kind: SymbolKind::CLASS,
                        tags: None,
                        deprecated: Some(false),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 10, character: 1 }
                        },
                        selection_range: Range {
                            start: Position { line: 0, character: 6 },
                            end: Position { line: 0, character: 13 }
                        },
                        children: Some(vec![
                            DocumentSymbol {
                                name: "constructor".to_string(),
                                detail: Some("constructor()".to_string()),
                                kind: SymbolKind::CONSTRUCTOR,
                                tags: None,
                                deprecated: Some(false),
                                range: Range {
                                    start: Position { line: 1, character: 2 },
                                    end: Position { line: 3, character: 3 }
                                },
                                selection_range: Range {
                                    start: Position { line: 1, character: 2 },
                                    end: Position { line: 1, character: 13 }
                                },
                                children: None,
                            },
                            DocumentSymbol {
                                name: "myMethod".to_string(),
                                detail: Some("myMethod(): void".to_string()),
                                kind: SymbolKind::METHOD,
                                tags: None,
                                deprecated: Some(false),
                                range: Range {
                                    start: Position { line: 5, character: 2 },
                                    end: Position { line: 7, character: 3 }
                                },
                                selection_range: Range {
                                    start: Position { line: 5, character: 2 },
                                    end: Position { line: 5, character: 10 }
                                },
                                children: None,
                            }
                        ]),
                    },
                    DocumentSymbol {
                        name: "helperFunction".to_string(),
                        detail: Some("function helperFunction()".to_string()),
                        kind: SymbolKind::FUNCTION,
                        tags: None,
                        deprecated: Some(false),
                        range: Range {
                            start: Position { line: 12, character: 0 },
                            end: Position { line: 14, character: 1 }
                        },
                        selection_range: Range {
                            start: Position { line: 12, character: 9 },
                            end: Position { line: 12, character: 23 }
                        },
                        children: None,
                    }
                ]))
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file(r#"
class MyClass {
  constructor() {
    // constructor
  }

  myMethod(): void {
    console.log('method');
  }
}

function helperFunction() {
  return 42;
}
"#).unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path
        });

        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response["symbols"].is_array());

        let symbols = response["symbols"].as_array().unwrap();
        assert_eq!(symbols.len(), 2);

        // Check class symbol
        assert_eq!(symbols[0]["name"], "MyClass");
        assert_eq!(symbols[0]["kind"], "Class");
        assert!(symbols[0]["children"].is_array());

        let class_children = symbols[0]["children"].as_array().unwrap();
        assert_eq!(class_children.len(), 2);
        assert_eq!(class_children[0]["name"], "constructor");
        assert_eq!(class_children[1]["name"], "myMethod");

        // Check function symbol
        assert_eq!(symbols[1]["name"], "helperFunction");
        assert_eq!(symbols[1]["kind"], "Function");
    }

    #[tokio::test]
    async fn test_get_document_symbols_flat() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_document_symbols()
            .returning(|path| {
                let uri = Uri::from_file_path(path).unwrap();
                Ok(DocumentSymbolResponse::Flat(vec![
                    SymbolInformation {
                        name: "MY_CONSTANT".to_string(),
                        kind: SymbolKind::CONSTANT,
                        tags: None,
                        deprecated: Some(false),
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position { line: 0, character: 0 },
                                end: Position { line: 0, character: 20 }
                            }
                        },
                        container_name: None,
                    },
                    SymbolInformation {
                        name: "myVariable".to_string(),
                        kind: SymbolKind::VARIABLE,
                        tags: None,
                        deprecated: Some(false),
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position { line: 1, character: 0 },
                                end: Position { line: 1, character: 15 }
                            }
                        },
                        container_name: None,
                    },
                    SymbolInformation {
                        name: "MyInterface".to_string(),
                        kind: SymbolKind::INTERFACE,
                        tags: None,
                        deprecated: Some(false),
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position { line: 3, character: 0 },
                                end: Position { line: 6, character: 1 }
                            }
                        },
                        container_name: None,
                    }
                ]))
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file(r#"
const MY_CONSTANT = 42;
let myVariable = "test";

interface MyInterface {
  prop: string;
}
"#).unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path
        });

        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response["symbols"].is_array());

        let symbols = response["symbols"].as_array().unwrap();
        assert_eq!(symbols.len(), 3);

        assert_eq!(symbols[0]["name"], "MY_CONSTANT");
        assert_eq!(symbols[0]["kind"], "Constant");

        assert_eq!(symbols[1]["name"], "myVariable");
        assert_eq!(symbols[1]["kind"], "Variable");

        assert_eq!(symbols[2]["name"], "MyInterface");
        assert_eq!(symbols[2]["kind"], "Interface");
    }

    #[tokio::test]
    async fn test_get_document_symbols_empty_file() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_document_symbols()
            .returning(|_| {
                Ok(DocumentSymbolResponse::Nested(vec![]))
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("// Empty file with just comments").unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path
        });

        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response["symbols"].is_array());
        assert_eq!(response["symbols"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_get_document_symbols_invalid_file() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_document_symbols()
            .returning(|_| {
                Err(crate::error::ServerError::FileNotFound("/nonexistent.ts".into()))
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let args = json!({
            "file_path": "/nonexistent.ts"
        });

        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_document_symbols_with_deprecated_symbols() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_document_symbols()
            .returning(|_| {
                Ok(DocumentSymbolResponse::Nested(vec![
                    DocumentSymbol {
                        name: "deprecatedFunction".to_string(),
                        detail: Some("@deprecated Use newFunction instead".to_string()),
                        kind: SymbolKind::FUNCTION,
                        tags: Some(vec![lsp_types::SymbolTag::DEPRECATED]),
                        deprecated: Some(true),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 2, character: 1 }
                        },
                        selection_range: Range {
                            start: Position { line: 0, character: 9 },
                            end: Position { line: 0, character: 27 }
                        },
                        children: None,
                    },
                    DocumentSymbol {
                        name: "newFunction".to_string(),
                        detail: Some("function newFunction()".to_string()),
                        kind: SymbolKind::FUNCTION,
                        tags: None,
                        deprecated: Some(false),
                        range: Range {
                            start: Position { line: 4, character: 0 },
                            end: Position { line: 6, character: 1 }
                        },
                        selection_range: Range {
                            start: Position { line: 4, character: 9 },
                            end: Position { line: 4, character: 20 }
                        },
                        children: None,
                    }
                ]))
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file(r#"
/** @deprecated Use newFunction instead */
function deprecatedFunction() {
  return "old";
}

function newFunction() {
  return "new";
}
"#).unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path
        });

        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let symbols = response["symbols"].as_array().unwrap();
        assert_eq!(symbols.len(), 2);

        // Check deprecated symbol
        assert_eq!(symbols[0]["name"], "deprecatedFunction");
        assert_eq!(symbols[0]["deprecated"], true);
        assert!(symbols[0]["detail"].as_str().unwrap().contains("deprecated"));

        // Check normal symbol
        assert_eq!(symbols[1]["name"], "newFunction");
        assert_eq!(symbols[1]["deprecated"], false);
    }

    #[tokio::test]
    async fn test_get_document_symbols_invalid_args() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        // Test with missing file_path
        let args = json!({});
        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_err());

        // Test with null file_path
        let args = json!({
            "file_path": null
        });
        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_err());

        // Test with number instead of string
        let args = json!({
            "file_path": 12345
        });
        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_document_symbols_complex_hierarchy() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();
        mock_lsp.expect_document_symbols()
            .returning(|_| {
                Ok(DocumentSymbolResponse::Nested(vec![
                    DocumentSymbol {
                        name: "namespace".to_string(),
                        detail: Some("namespace MyNamespace".to_string()),
                        kind: SymbolKind::NAMESPACE,
                        tags: None,
                        deprecated: Some(false),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 20, character: 1 }
                        },
                        selection_range: Range {
                            start: Position { line: 0, character: 10 },
                            end: Position { line: 0, character: 21 }
                        },
                        children: Some(vec![
                            DocumentSymbol {
                                name: "OuterClass".to_string(),
                                detail: Some("class OuterClass".to_string()),
                                kind: SymbolKind::CLASS,
                                tags: None,
                                deprecated: Some(false),
                                range: Range {
                                    start: Position { line: 1, character: 2 },
                                    end: Position { line: 15, character: 3 }
                                },
                                selection_range: Range {
                                    start: Position { line: 1, character: 8 },
                                    end: Position { line: 1, character: 18 }
                                },
                                children: Some(vec![
                                    DocumentSymbol {
                                        name: "innerProperty".to_string(),
                                        detail: Some("innerProperty: string".to_string()),
                                        kind: SymbolKind::PROPERTY,
                                        tags: None,
                                        deprecated: Some(false),
                                        range: Range {
                                            start: Position { line: 2, character: 4 },
                                            end: Position { line: 2, character: 25 }
                                        },
                                        selection_range: Range {
                                            start: Position { line: 2, character: 4 },
                                            end: Position { line: 2, character: 17 }
                                        },
                                        children: None,
                                    }
                                ]),
                            }
                        ]),
                    }
                ]))
            });

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        let file = create_temp_ts_file("// Complex nested structure").unwrap();
        let file_path = file.path().to_str().unwrap();

        let args = json!({
            "file_path": file_path
        });

        let result = dispatcher.call_tool_for_test("get_document_symbols", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let symbols = response["symbols"].as_array().unwrap();

        // Verify deep nesting
        assert_eq!(symbols[0]["name"], "namespace");
        assert!(symbols[0]["children"].is_array());

        let namespace_children = symbols[0]["children"].as_array().unwrap();
        assert_eq!(namespace_children[0]["name"], "OuterClass");
        assert!(namespace_children[0]["children"].is_array());

        let class_children = namespace_children[0]["children"].as_array().unwrap();
        assert_eq!(class_children[0]["name"], "innerProperty");
        assert_eq!(class_children[0]["kind"], "Property");
    }
}