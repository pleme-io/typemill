use super::*;
use crate::{PluginRequest, PluginError};
use serde_json::json;
use std::path::PathBuf;
use async_trait::async_trait;

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