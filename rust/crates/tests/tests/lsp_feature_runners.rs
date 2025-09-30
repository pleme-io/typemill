//! Generic test runners for LSP features
//!
//! This module contains the actual test logic for each LSP feature.
//! Each runner function is parameterized to accept a fixture struct,
//! making them reusable across multiple languages.

use cb_api::LspService;
use serde_json::json;
use tests::harness::test_fixtures::*;
use tests::harness::LspTestBuilder;

/// Run a "go to definition" test with the given test case
pub async fn run_go_to_definition_test(case: &GoToDefinitionTestCase, use_real_lsp: bool) {
    let mut builder = LspTestBuilder::new(case.language_id);
    if use_real_lsp {
        builder = builder.with_real_lsp();
    }
    for (path, content) in case.files {
        builder = builder.with_file(path, content);
    }

    if use_real_lsp {
        let (service, workspace) = builder.build().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let message = cb_api::Message {
            id: Some(format!("real-def-{}", case.language_id)),
            method: "textDocument/definition".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                }
            }),
        };

        let response = service.request(message).await.unwrap();
        let locations = response.params.as_array().unwrap();
        assert!(
            !locations.is_empty(),
            "Real LSP server should return a definition for {}",
            case.language_id
        );
        assert!(
            locations[0]["uri"]
                .as_str()
                .unwrap()
                .contains(case.expected_location.0),
            "Definition should be in {}",
            case.expected_location.0
        );
    } else {
        let (mock, workspace) = builder.build_mock().await.unwrap();

        mock.set_response(
            "textDocument/definition",
            json!([{
                "uri": format!("file://{}/{}", workspace.path().display(), case.expected_location.0),
                "range": {
                    "start": {
                        "line": case.expected_location.1,
                        "character": case.expected_location.2
                    },
                    "end": {
                        "line": case.expected_location.1,
                        "character": case.expected_location.2 + 5
                    }
                }
            }]),
        );

        let message = cb_api::Message {
            id: Some("1".to_string()),
            method: "textDocument/definition".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                }
            }),
        };

        let response = mock.request(message).await.unwrap();
        let locations = response.params.as_array().unwrap();
        assert!(!locations.is_empty(), "Should return at least one location");
        assert!(
            locations[0]["uri"]
                .as_str()
                .unwrap()
                .contains(case.expected_location.0),
            "Definition should be in {}",
            case.expected_location.0
        );
    }
}

/// Run a "find references" test with the given test case
pub async fn run_find_references_test(case: &FindReferencesTestCase, use_real_lsp: bool) {
    let mut builder = LspTestBuilder::new(case.language_id);
    if use_real_lsp {
        builder = builder.with_real_lsp();
    }
    for (path, content) in case.files {
        builder = builder.with_file(path, content);
    }

    if use_real_lsp {
        let (service, workspace) = builder.build().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let message = cb_api::Message {
            id: Some(format!("real-refs-{}", case.language_id)),
            method: "textDocument/references".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                },
                "context": {"includeDeclaration": true}
            }),
        };

        let response = service.request(message).await.unwrap();
        let references = response.params.as_array().unwrap();
        assert!(
            !references.is_empty(),
            "Real LSP should find references for {}",
            case.language_id
        );
    } else {
        let (mock, workspace) = builder.build_mock().await.unwrap();

        // Generate mock references
        let mut mock_refs = vec![];
        for i in 0..case.expected_min_count {
            mock_refs.push(json!({
                "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0),
                "range": {
                    "start": {"line": i, "character": 0},
                    "end": {"line": i, "character": 10}
                }
            }));
        }

        mock.set_response("textDocument/references", json!(mock_refs));

        let message = cb_api::Message {
            id: Some("1".to_string()),
            method: "textDocument/references".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                },
                "context": {"includeDeclaration": true}
            }),
        };

        let response = mock.request(message).await.unwrap();
        let references = response.params.as_array().unwrap();
        assert!(
            references.len() >= case.expected_min_count,
            "Should find at least {} references",
            case.expected_min_count
        );
    }
}

/// Run a "hover" test with the given test case
pub async fn run_hover_test(case: &HoverTestCase, use_real_lsp: bool) {
    let mut builder = LspTestBuilder::new(case.language_id);
    if use_real_lsp {
        builder = builder.with_real_lsp();
    }
    for (path, content) in case.files {
        builder = builder.with_file(path, content);
    }

    if use_real_lsp {
        let (service, workspace) = builder.build().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let message = cb_api::Message {
            id: Some(format!("real-hover-{}", case.language_id)),
            method: "textDocument/hover".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                }
            }),
        };

        let response = service.request(message).await.unwrap();
        assert!(response.params.is_object() || response.params.is_null());
    } else {
        let (mock, workspace) = builder.build_mock().await.unwrap();

        if case.should_have_contents {
            mock.set_response(
                "textDocument/hover",
                json!({
                    "contents": {
                        "kind": "markdown",
                        "value": "Mock hover content"
                    },
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 10}
                    }
                }),
            );
        } else {
            mock.set_response("textDocument/hover", json!(null));
        }

        let message = cb_api::Message {
            id: Some("1".to_string()),
            method: "textDocument/hover".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                }
            }),
        };

        let response = mock.request(message).await.unwrap();
        let hover_data = &response.params;
        if case.should_have_contents {
            assert!(hover_data.is_object());
            assert!(hover_data.get("contents").is_some());
        }
    }
}

/// Run a "document symbols" test with the given test case
pub async fn run_document_symbols_test(case: &DocumentSymbolsTestCase, use_real_lsp: bool) {
    let mut builder = LspTestBuilder::new(case.language_id);
    if use_real_lsp {
        builder = builder.with_real_lsp();
    }
    for (path, content) in case.files {
        builder = builder.with_file(path, content);
    }

    if use_real_lsp {
        let (service, workspace) = builder.build().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let message = cb_api::Message {
            id: Some(format!("real-symbols-{}", case.language_id)),
            method: "textDocument/documentSymbol".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.document_path)
                }
            }),
        };

        let response = service.request(message).await.unwrap();
        assert!(response.params.is_array() || response.params.is_object());
    } else {
        let (mock, workspace) = builder.build_mock().await.unwrap();

        // Generate mock symbols
        let mut mock_symbols = vec![];
        for i in 0..case.expected_min_count {
            mock_symbols.push(json!({
                "name": format!("Symbol{}", i),
                "kind": 5,
                "range": {
                    "start": {"line": i, "character": 0},
                    "end": {"line": i, "character": 10}
                }
            }));
        }

        mock.set_response("textDocument/documentSymbol", json!(mock_symbols));

        let message = cb_api::Message {
            id: Some("1".to_string()),
            method: "textDocument/documentSymbol".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.document_path)
                }
            }),
        };

        let response = mock.request(message).await.unwrap();
        let symbols = response.params.as_array().unwrap();
        assert!(
            symbols.len() >= case.expected_min_count,
            "Should return at least {} symbols",
            case.expected_min_count
        );
    }
}

/// Run a "workspace symbols" test with the given test case
pub async fn run_workspace_symbols_test(case: &WorkspaceSymbolsTestCase, use_real_lsp: bool) {
    let mut builder = LspTestBuilder::new(case.language_id);
    if use_real_lsp {
        builder = builder.with_real_lsp();
    }
    for (path, content) in case.files {
        builder = builder.with_file(path, content);
    }

    if use_real_lsp {
        let (service, _workspace) = builder.build().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let message = cb_api::Message {
            id: Some(format!("real-ws-symbols-{}", case.language_id)),
            method: "workspace/symbol".to_string(),
            params: json!({"query": case.query}),
        };

        let response = service.request(message).await.unwrap();
        assert!(response.params.is_array() || response.params.is_null());
    } else {
        let (mock, workspace) = builder.build_mock().await.unwrap();

        if case.should_find_symbols {
            mock.set_response(
                "workspace/symbol",
                json!([{
                    "name": "MockSymbol",
                    "kind": 5,
                    "location": {
                        "uri": format!("file://{}/{}", workspace.path().display(), case.files[0].0),
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 10}
                        }
                    }
                }]),
            );
        } else {
            mock.set_response("workspace/symbol", json!([]));
        }

        let message = cb_api::Message {
            id: Some("1".to_string()),
            method: "workspace/symbol".to_string(),
            params: json!({"query": case.query}),
        };

        let response = mock.request(message).await.unwrap();
        let symbols = response.params.as_array().unwrap();
        if case.should_find_symbols {
            assert!(!symbols.is_empty(), "Should find workspace symbols");
        }
    }
}

/// Run a "completion" test with the given test case
pub async fn run_completion_test(case: &CompletionTestCase, use_real_lsp: bool) {
    let mut builder = LspTestBuilder::new(case.language_id);
    if use_real_lsp {
        builder = builder.with_real_lsp();
    }
    for (path, content) in case.files {
        builder = builder.with_file(path, content);
    }

    if use_real_lsp {
        let (service, workspace) = builder.build().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let message = cb_api::Message {
            id: Some(format!("real-completion-{}", case.language_id)),
            method: "textDocument/completion".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                }
            }),
        };

        let response = service.request(message).await.unwrap();
        assert!(response.params.is_object() || response.params.is_array());
    } else {
        let (mock, workspace) = builder.build_mock().await.unwrap();

        if case.should_have_items {
            mock.set_response(
                "textDocument/completion",
                json!({
                    "items": [
                        {
                            "label": "mockItem",
                            "kind": 5,
                            "detail": "Mock completion item"
                        }
                    ]
                }),
            );
        } else {
            mock.set_response("textDocument/completion", json!({"items": []}));
        }

        let message = cb_api::Message {
            id: Some("1".to_string()),
            method: "textDocument/completion".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                }
            }),
        };

        let response = mock.request(message).await.unwrap();
        let completions = &response.params;
        assert!(completions.is_object());
        if case.should_have_items {
            let items = completions.get("items").unwrap().as_array().unwrap();
            assert!(!items.is_empty(), "Should return completion items");
        }
    }
}

/// Run a "rename" test with the given test case
pub async fn run_rename_test(case: &RenameTestCase, use_real_lsp: bool) {
    let mut builder = LspTestBuilder::new(case.language_id);
    if use_real_lsp {
        builder = builder.with_real_lsp();
    }
    for (path, content) in case.files {
        builder = builder.with_file(path, content);
    }

    if use_real_lsp {
        let (service, workspace) = builder.build().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let message = cb_api::Message {
            id: Some(format!("real-rename-{}", case.language_id)),
            method: "textDocument/rename".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                },
                "newName": case.new_name
            }),
        };

        let response = service.request(message).await.unwrap();
        assert!(response.params.is_object());
    } else {
        let (mock, workspace) = builder.build_mock().await.unwrap();

        if case.should_have_changes {
            mock.set_response(
                "textDocument/rename",
                json!({
                    "changes": {
                        format!("file://{}/{}", workspace.path().display(), case.trigger_point.0): [
                            {
                                "range": {
                                    "start": {"line": 0, "character": 0},
                                    "end": {"line": 0, "character": 10}
                                },
                                "newText": case.new_name
                            }
                        ]
                    }
                }),
            );
        } else {
            mock.set_response("textDocument/rename", json!({"changes": {}}));
        }

        let message = cb_api::Message {
            id: Some("1".to_string()),
            method: "textDocument/rename".to_string(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{}/{}", workspace.path().display(), case.trigger_point.0)
                },
                "position": {
                    "line": case.trigger_point.1,
                    "character": case.trigger_point.2
                },
                "newName": case.new_name
            }),
        };

        let response = mock.request(message).await.unwrap();
        let workspace_edit = &response.params;
        assert!(workspace_edit.is_object());
        if case.should_have_changes {
            assert!(workspace_edit.get("changes").is_some());
        }
    }
}
