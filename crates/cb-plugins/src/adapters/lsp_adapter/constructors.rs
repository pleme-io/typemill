use super::{
    Capabilities, DiagnosticCapabilities, EditingCapabilities, IntelligenceCapabilities,
    LspAdapterPlugin, LspService, NavigationCapabilities, PluginMetadata, RefactoringCapabilities,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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
}