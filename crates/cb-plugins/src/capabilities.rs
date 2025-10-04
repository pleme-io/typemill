//! Plugin capability definitions
//!
//! Capabilities allow plugins to declare what functionality they support,
//! enabling the plugin manager to route requests appropriately.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Tool scope defines whether a tool operates on files or workspace-wide
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolScope {
    /// Tool operates on a specific file (requires file_path parameter)
    File,
    /// Tool operates at workspace level (no file_path required)
    Workspace,
}

/// Complete set of capabilities a plugin can provide
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Capabilities {
    /// Navigation capabilities (go-to-definition, find references, etc.)
    pub navigation: NavigationCapabilities,
    /// Code editing capabilities (rename, format, etc.)
    pub editing: EditingCapabilities,
    /// Refactoring capabilities (extract function, inline variable, etc.)
    pub refactoring: RefactoringCapabilities,
    /// Code intelligence capabilities (hover, completions, etc.)
    pub intelligence: IntelligenceCapabilities,
    /// Diagnostic capabilities (errors, warnings, linting)
    pub diagnostics: DiagnosticCapabilities,
    /// Language-specific custom capabilities
    pub custom: HashMap<String, Value>,
}

/// Navigation-related capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NavigationCapabilities {
    /// Go to definition support
    pub go_to_definition: bool,
    /// Find references support
    pub find_references: bool,
    /// Find implementations support
    pub find_implementations: bool,
    /// Find type definition support
    pub find_type_definition: bool,
    /// Workspace symbol search support
    pub workspace_symbols: bool,
    /// Document symbol support
    pub document_symbols: bool,
    /// Call hierarchy support
    pub call_hierarchy: bool,
}

/// Code editing capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EditingCapabilities {
    /// Symbol renaming support
    pub rename: bool,
    /// Document formatting support
    pub format_document: bool,
    /// Range formatting support
    pub format_range: bool,
    /// Code actions support (quick fixes)
    pub code_actions: bool,
    /// Import organization support
    pub organize_imports: bool,
    /// Auto-import support
    pub auto_imports: bool,
}

/// Refactoring capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RefactoringCapabilities {
    /// Extract function/method support
    pub extract_function: bool,
    /// Extract variable support
    pub extract_variable: bool,
    /// Inline variable support
    pub inline_variable: bool,
    /// Inline function support
    pub inline_function: bool,
    /// Move refactoring support
    pub move_refactor: bool,
}

/// Code intelligence capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntelligenceCapabilities {
    /// Hover information support
    pub hover: bool,
    /// Code completion support
    pub completions: bool,
    /// Signature help support
    pub signature_help: bool,
    /// Inlay hints support
    pub inlay_hints: bool,
    /// Semantic highlighting support
    pub semantic_highlighting: bool,
}

/// Diagnostic capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosticCapabilities {
    /// Error/warning diagnostics
    pub diagnostics: bool,
    /// Custom linting rules
    pub linting: bool,
    /// Pull diagnostics support
    pub pull_diagnostics: bool,
}

impl Capabilities {
    /// Create capabilities with all features enabled (for testing)
    pub fn all_enabled() -> Self {
        Self {
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
                auto_imports: true,
            },
            refactoring: RefactoringCapabilities {
                extract_function: true,
                extract_variable: true,
                inline_variable: true,
                inline_function: true,
                move_refactor: true,
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
                linting: true,
                pull_diagnostics: true,
            },
            custom: HashMap::new(),
        }
    }

    /// Check if a specific capability is supported
    pub fn supports(&self, method: &str) -> bool {
        match method {
            // Navigation capabilities
            "find_definition" => self.navigation.go_to_definition,
            "find_references" => self.navigation.find_references,
            "find_implementations" => self.navigation.find_implementations,
            "find_type_definition" => self.navigation.find_type_definition,
            "search_workspace_symbols" => self.navigation.workspace_symbols,
            "get_document_symbols" => self.navigation.document_symbols,
            "prepare_call_hierarchy" => self.navigation.call_hierarchy,
            "get_call_hierarchy_incoming_calls" => self.navigation.call_hierarchy,
            "get_call_hierarchy_outgoing_calls" => self.navigation.call_hierarchy,

            // Editing capabilities
            "rename_symbol" => self.editing.rename,
            "format_document" => self.editing.format_document,
            "format_range" => self.editing.format_range,
            "get_code_actions" => self.editing.code_actions,
            "organize_imports" => self.editing.organize_imports,

            // Refactoring capabilities
            "extract_function" => self.refactoring.extract_function,
            "extract_variable" => self.refactoring.extract_variable,
            "inline_variable" => self.refactoring.inline_variable,

            // Intelligence capabilities
            "get_hover" => self.intelligence.hover,
            "get_completions" => self.intelligence.completions,
            "get_signature_help" => self.intelligence.signature_help,

            // Diagnostic capabilities
            "get_diagnostics" => self.diagnostics.diagnostics,

            // Custom capabilities
            method if method.contains('.') => self.custom.contains_key(method),

            _ => false,
        }
    }

    /// Add a custom capability
    pub fn add_custom(&mut self, name: String, value: Value) {
        self.custom.insert(name, value);
    }

    /// Get the scope of a tool (File or Workspace)
    pub fn get_tool_scope(&self, method: &str) -> Option<ToolScope> {
        match method {
            // File-scoped navigation tools
            "find_definition"
            | "find_references"
            | "find_implementations"
            | "find_type_definition"
            | "get_document_symbols"
            | "prepare_call_hierarchy"
            | "get_call_hierarchy_incoming_calls"
            | "get_call_hierarchy_outgoing_calls" => Some(ToolScope::File),

            // File-scoped editing tools
            "rename_symbol"
            | "rename_symbol_strict"
            | "format_document"
            | "format_range"
            | "get_code_actions"
            | "organize_imports" => Some(ToolScope::File),

            // File-scoped refactoring tools
            "extract_function" | "extract_variable" | "inline_variable" => Some(ToolScope::File),

            // File-scoped intelligence tools
            "get_hover" | "get_completions" | "get_signature_help" => Some(ToolScope::File),

            // File-scoped diagnostic tools
            "get_diagnostics" => Some(ToolScope::File),

            // File-scoped analysis tools
            "analyze_imports" => Some(ToolScope::File),

            // Workspace-scoped tools
            "search_workspace_symbols"
            | "list_files"
            | "find_dead_code"
            | "bulk_update_dependencies"
            | "rename_file"
            | "rename_directory"
            | "extract_module_to_package" => Some(ToolScope::Workspace),

            // Custom capabilities - default to workspace scope
            _ if self.custom.contains_key(method) => Some(ToolScope::Workspace),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_capabilities() {
        let caps = Capabilities::default();
        assert!(!caps.navigation.go_to_definition);
        assert!(!caps.editing.rename);
        assert!(!caps.refactoring.extract_function);
        assert!(!caps.intelligence.hover);
        assert!(!caps.diagnostics.diagnostics);
    }

    #[test]
    fn test_all_enabled_capabilities() {
        let caps = Capabilities::all_enabled();
        assert!(caps.navigation.go_to_definition);
        assert!(caps.editing.rename);
        assert!(caps.refactoring.extract_function);
        assert!(caps.intelligence.hover);
        assert!(caps.diagnostics.diagnostics);
    }

    #[test]
    fn test_supports_method() {
        let mut caps = Capabilities::default();
        caps.navigation.go_to_definition = true;
        caps.custom
            .insert("typescript.infer_types".to_string(), json!(true));

        assert!(caps.supports("find_definition"));
        assert!(!caps.supports("find_references"));
        assert!(caps.supports("typescript.infer_types"));
        assert!(!caps.supports("unknown_method"));
    }

    #[test]
    fn test_custom_capabilities() {
        let mut caps = Capabilities::default();
        caps.add_custom("rust.expand_macros".to_string(), json!({"supported": true}));

        assert!(caps.supports("rust.expand_macros"));
        assert_eq!(
            caps.custom.get("rust.expand_macros"),
            Some(&json!({"supported": true}))
        );
    }

    #[test]
    fn test_tool_scope() {
        let caps = Capabilities::default();

        // Test file-scoped tools
        assert_eq!(
            caps.get_tool_scope("find_definition"),
            Some(ToolScope::File)
        );
        assert_eq!(caps.get_tool_scope("rename_symbol"), Some(ToolScope::File));
        assert_eq!(caps.get_tool_scope("get_hover"), Some(ToolScope::File));
        assert_eq!(
            caps.get_tool_scope("extract_function"),
            Some(ToolScope::File)
        );

        // Test workspace-scoped tools
        assert_eq!(
            caps.get_tool_scope("search_workspace_symbols"),
            Some(ToolScope::Workspace)
        );
        assert_eq!(
            caps.get_tool_scope("list_files"),
            Some(ToolScope::Workspace)
        );
        assert_eq!(
            caps.get_tool_scope("rename_directory"),
            Some(ToolScope::Workspace)
        );

        // Test unknown tool
        assert_eq!(caps.get_tool_scope("unknown_tool"), None);
    }

    #[test]
    fn test_custom_tool_scope() {
        let mut caps = Capabilities::default();
        caps.add_custom("custom.analysis".to_string(), json!(true));

        // Custom tools default to workspace scope
        assert_eq!(
            caps.get_tool_scope("custom.analysis"),
            Some(ToolScope::Workspace)
        );
    }
}
