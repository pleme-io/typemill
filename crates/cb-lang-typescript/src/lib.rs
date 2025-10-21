//! TypeScript/JavaScript Language Plugin for Codebuddy
pub mod import_support;
pub mod imports;
mod manifest;
pub mod parser;
pub mod refactoring;
pub mod workspace_support;

use async_trait::async_trait;
use cb_lang_common::read_manifest;
use cb_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    LanguageMetadata, LanguagePlugin, LspConfig, ManifestData, ParsedSource,
    PluginCapabilities, PluginError, PluginResult, WorkspaceSupport,
};
use cb_plugin_api::codebuddy_plugin;
use std::path::Path;

// Self-register the plugin with the Codebuddy system.
codebuddy_plugin! {
    name: "typescript",
    extensions: ["ts", "tsx", "js", "jsx", "mjs", "cjs"],
    manifest: "package.json",
    capabilities: TypeScriptPlugin::CAPABILITIES,
    factory: TypeScriptPlugin::new,
    lsp: Some(LspConfig::new("typescript-language-server", &["typescript-language-server", "--stdio"]))
}

/// TypeScript/JavaScript language plugin implementation.
#[derive(Default)]
pub struct TypeScriptPlugin {
    import_support: import_support::TypeScriptImportSupport,
    workspace_support: workspace_support::TypeScriptWorkspaceSupport,
}

impl TypeScriptPlugin {
    /// Static metadata for the TypeScript language.
    pub const METADATA: LanguageMetadata = LanguageMetadata {
        name: "typescript",
        extensions: &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
        manifest_filename: "package.json",
        source_dir: "src",
        entry_point: "index.ts",
        module_separator: ".",
    };

    /// The capabilities of this plugin.
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none()
        .with_imports()
        .with_workspace();

    /// Creates a new, boxed instance of the plugin.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<dyn LanguagePlugin> {
        Box::new(Self::default())
    }
}

#[async_trait]
impl LanguagePlugin for TypeScriptPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &Self::METADATA
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        let symbols = parser::extract_symbols(source)?;
        Ok(ParsedSource {
            data: serde_json::json!(
                { "language" : "typescript", "symbols_count" : symbols.len() }
            ),
            symbols,
        })
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::load_package_json(path).await
    }

    fn analyze_detailed_imports(
        &self,
        source: &str,
        file_path: Option<&Path>,
    ) -> PluginResult<codebuddy_foundation::protocol::ImportGraph> {
        parser::analyze_imports(source, file_path)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn import_parser(&self) -> Option<&dyn ImportParser> {
        Some(&self.import_support)
    }

    fn import_rename_support(&self) -> Option<&dyn ImportRenameSupport> {
        Some(&self.import_support)
    }

    fn import_move_support(&self) -> Option<&dyn ImportMoveSupport> {
        Some(&self.import_support)
    }

    fn import_mutation_support(&self) -> Option<&dyn ImportMutationSupport> {
        Some(&self.import_support)
    }

    fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
        Some(&self.import_support)
    }

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        Some(&self.workspace_support)
    }

    // Capability trait discovery methods
    fn module_reference_scanner(&self) -> Option<&dyn cb_plugin_api::ModuleReferenceScanner> {
        Some(self)
    }

    fn refactoring_provider(&self) -> Option<&dyn cb_plugin_api::RefactoringProvider> {
        Some(self)
    }

    fn import_analyzer(&self) -> Option<&dyn cb_plugin_api::ImportAnalyzer> {
        Some(self)
    }

    fn manifest_updater(&self) -> Option<&dyn cb_plugin_api::ManifestUpdater> {
        Some(self)
    }

    fn rewrite_file_references(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        current_file: &Path,
        project_root: &Path,
        rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        self.rewrite_imports_for_rename(
            content,
            old_path,
            new_path,
            current_file,
            project_root,
            rename_info,
        )
        .ok()
    }
}

// ============================================================================
// Capability Trait Implementations
// ============================================================================

impl cb_plugin_api::ModuleReferenceScanner for TypeScriptPlugin {
    fn scan_references(
        &self,
        content: &str,
        module_name: &str,
        scope: cb_plugin_api::ScanScope,
    ) -> cb_plugin_api::PluginResult<Vec<cb_plugin_api::ModuleReference>> {
        Ok(self.find_module_references(content, module_name, scope))
    }
}

impl cb_plugin_api::RefactoringProvider for TypeScriptPlugin {
    fn supports_inline_variable(&self) -> bool {
        // Not yet implemented - return false until we implement the AST-based refactoring
        false
    }

    fn inline_variable(
        &self,
        _params: cb_plugin_api::InlineParams,
    ) -> cb_plugin_api::PluginResult<cb_plugin_api::WorkspaceEdit> {
        // TODO: Implement inline variable refactoring
        // For now, return not supported
        Err(cb_plugin_api::PluginError::not_supported(
            "inline_variable not yet implemented for TypeScript",
        ))
    }

    fn supports_extract_function(&self) -> bool {
        // Not yet implemented - return false until we implement the AST-based refactoring
        false
    }

    fn extract_function(
        &self,
        _params: cb_plugin_api::ExtractParams,
    ) -> cb_plugin_api::PluginResult<cb_plugin_api::WorkspaceEdit> {
        // TODO: Implement extract function refactoring
        // For now, return not supported
        Err(cb_plugin_api::PluginError::not_supported(
            "extract_function not yet implemented for TypeScript",
        ))
    }
}

impl cb_plugin_api::ImportAnalyzer for TypeScriptPlugin {
    fn build_import_graph(
        &self,
        file_path: &Path,
    ) -> cb_plugin_api::PluginResult<codebuddy_foundation::protocol::ImportGraph> {
        // Read the file content
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| cb_plugin_api::PluginError::internal(format!("Failed to read file: {}", e)))?;

        // Use the existing analyze_detailed_imports method
        self.analyze_detailed_imports(&content, Some(file_path))
    }

    fn find_unused_imports(
        &self,
        _file_path: &Path,
    ) -> cb_plugin_api::PluginResult<Vec<String>> {
        // TODO: Implement unused import detection
        // For now, return empty vector
        Ok(Vec::new())
    }
}

// ============================================================================
// Manifest Updater Capability
// ============================================================================

#[async_trait::async_trait]
impl cb_plugin_api::ManifestUpdater for TypeScriptPlugin {
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> cb_plugin_api::PluginResult<String> {
        // Delegate to the inherent method implementation
        TypeScriptPlugin::update_dependency(self, manifest_path, old_name, new_name, new_version)
            .await
    }
}

// ============================================================================
// Plugin-specific helper methods
// ============================================================================

impl TypeScriptPlugin {
    pub async fn update_dependency(
        &self,
        manifest_path: &Path,
        _old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        let content = read_manifest(manifest_path).await?;
        let version = new_version.ok_or_else(|| {
            PluginError::invalid_input("Version required for package.json dependency updates")
        })?;
        manifest::update_dependency(&content, new_name, version)
    }

    pub fn generate_manifest(&self, package_name: &str, dependencies: &[String]) -> String {
        manifest::generate_manifest(package_name, dependencies)
    }

    /// Find module references (minimal implementation for compatibility)
    pub fn find_module_references(
        &self,
        content: &str,
        module_to_find: &str,
        _scope: cb_plugin_api::ScanScope,
    ) -> Vec<cb_plugin_api::ModuleReference> {
        use cb_plugin_api::{ModuleReference, ReferenceKind};
        let mut references = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if (line.contains("import") || line.contains("from")) && line.contains(module_to_find) {
                references.push(ModuleReference {
                    line: line_num + 1,
                    column: 0,
                    length: line.len(),
                    text: line.to_string(),
                    kind: ReferenceKind::Declaration,
                });
            }
        }
        references
    }

    /// Rewrite imports for rename (minimal implementation for compatibility)
    pub fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        importing_file: &Path,
        _project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> PluginResult<(String, usize)> {
        // Use the standalone function with full context
        Ok(import_support::rewrite_imports_for_move_with_context(
            content,
            old_path,
            new_path,
            importing_file,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cb_plugin_api::LanguagePlugin;

    #[test]
    fn test_typescript_capabilities() {
        let plugin = TypeScriptPlugin::new();
        let plugin_trait: &dyn LanguagePlugin = plugin.as_ref();
        let caps = plugin_trait.capabilities();
        assert!(caps.imports, "TypeScript plugin should support imports");
        assert!(caps.workspace, "TypeScript plugin should support workspace");
    }

    #[test]
    fn test_typescript_workspace_support() {
        let plugin = TypeScriptPlugin::new();
        let plugin_trait: &dyn LanguagePlugin = plugin.as_ref();
        assert!(
            plugin_trait.workspace_support().is_some(),
            "TypeScript should have workspace support"
        );
    }
}
