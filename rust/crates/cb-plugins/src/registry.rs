//! Plugin registry for managing loaded plugins

use crate::{LanguagePlugin, PluginMetadata, Capabilities, PluginError, PluginResult};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Registry for managing loaded language plugins
pub struct PluginRegistry {
    /// Map of plugin name to plugin instance
    plugins: HashMap<String, Arc<dyn LanguagePlugin>>,
    /// Map of file extension to supporting plugins
    extension_map: HashMap<String, Vec<String>>,
    /// Map of method to supporting plugins
    method_map: HashMap<String, Vec<String>>,
    /// Plugin metadata cache
    metadata_cache: HashMap<String, PluginMetadata>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            extension_map: HashMap::new(),
            method_map: HashMap::new(),
            metadata_cache: HashMap::new(),
        }
    }

    /// Register a new plugin
    pub fn register_plugin(
        &mut self,
        name: impl Into<String>,
        plugin: Arc<dyn LanguagePlugin>,
    ) -> PluginResult<()> {
        let name = name.into();
        let metadata = plugin.metadata();
        let capabilities = plugin.capabilities();

        // Validate plugin metadata
        self.validate_plugin_metadata(&metadata)?;

        // Check for duplicate plugin names
        if self.plugins.contains_key(&name) {
            warn!("Plugin '{}' is already registered, replacing", name);
        }

        // Update extension mappings
        for extension in plugin.supported_extensions() {
            self.extension_map
                .entry(extension)
                .or_insert_with(Vec::new)
                .push(name.clone());
        }

        // Update method mappings based on capabilities
        self.update_method_mappings(&name, &capabilities);

        // Store plugin and metadata
        self.plugins.insert(name.clone(), plugin);
        self.metadata_cache.insert(name.clone(), metadata);

        info!("Registered plugin '{}'", name);
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister_plugin(&mut self, name: &str) -> PluginResult<()> {
        if let Some(plugin) = self.plugins.remove(name) {
            // Remove from extension mappings
            for extension in plugin.supported_extensions() {
                if let Some(plugins) = self.extension_map.get_mut(&extension) {
                    plugins.retain(|p| p != name);
                    if plugins.is_empty() {
                        self.extension_map.remove(&extension);
                    }
                }
            }

            // Remove from method mappings
            for plugins in self.method_map.values_mut() {
                plugins.retain(|p| p != name);
            }
            self.method_map.retain(|_, plugins| !plugins.is_empty());

            // Remove metadata
            self.metadata_cache.remove(name);

            info!("Unregistered plugin '{}'", name);
            Ok(())
        } else {
            Err(PluginError::plugin_not_found(name, "unregister"))
        }
    }

    /// Find plugins that can handle a specific file
    pub fn find_plugins_for_file(&self, file_path: &Path) -> Vec<String> {
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            self.extension_map
                .get(extension)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Find plugins that support a specific method
    pub fn find_plugins_for_method(&self, method: &str) -> Vec<String> {
        self.method_map
            .get(method)
            .cloned()
            .unwrap_or_default()
    }

    /// Find the best plugin for a file and method combination
    pub fn find_best_plugin(&self, file_path: &Path, method: &str) -> PluginResult<String> {
        // Special handling for system tools that don't have file associations
        if matches!(method, "list_files" | "analyze_imports" | "find_dead_code") {
            // Check if we have the system plugin registered
            if self.plugins.contains_key("system") {
                return Ok("system".to_string());
            }
        }

        let file_plugins = self.find_plugins_for_file(file_path);
        let method_plugins = self.find_plugins_for_method(method);

        // Find intersection of plugins that support both the file and method
        let compatible_plugins: Vec<String> = file_plugins
            .into_iter()
            .filter(|plugin| method_plugins.contains(plugin))
            .collect();

        match compatible_plugins.len() {
            0 => Err(PluginError::plugin_not_found(
                file_path.to_string_lossy(),
                method,
            )),
            1 => Ok(compatible_plugins[0].clone()),
            _ => {
                // Multiple plugins support this combination
                // For now, use priority-based selection (first wins)
                // In the future, this could be more sophisticated
                debug!(
                    "Multiple plugins support file {:?} and method '{}': {:?}",
                    file_path, method, compatible_plugins
                );
                Ok(compatible_plugins[0].clone())
            }
        }
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn LanguagePlugin>> {
        self.plugins.get(name).cloned()
    }

    /// Get plugin metadata
    pub fn get_plugin_metadata(&self, name: &str) -> Option<&PluginMetadata> {
        self.metadata_cache.get(name)
    }

    /// Get all registered plugin names
    pub fn get_plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get all supported file extensions
    pub fn get_supported_extensions(&self) -> Vec<String> {
        self.extension_map.keys().cloned().collect()
    }

    /// Get all supported methods
    pub fn get_supported_methods(&self) -> Vec<String> {
        self.method_map.keys().cloned().collect()
    }

    /// Check if a method is supported for a specific file
    pub fn is_method_supported(&self, file_path: &Path, method: &str) -> bool {
        self.find_best_plugin(file_path, method).is_ok()
    }

    /// Get capabilities for a specific plugin
    pub fn get_plugin_capabilities(&self, name: &str) -> Option<Capabilities> {
        self.plugins.get(name).map(|plugin| plugin.capabilities())
    }

    /// Get statistics about the registry
    pub fn get_statistics(&self) -> RegistryStatistics {
        RegistryStatistics {
            total_plugins: self.plugins.len(),
            supported_extensions: self.extension_map.len(),
            supported_methods: self.method_map.len(),
            average_methods_per_plugin: if self.plugins.is_empty() {
                0.0
            } else {
                self.method_map.values().map(|v| v.len()).sum::<usize>() as f64
                    / self.plugins.len() as f64
            },
        }
    }

    /// Validate plugin metadata
    fn validate_plugin_metadata(&self, metadata: &PluginMetadata) -> PluginResult<()> {
        if metadata.name.is_empty() {
            return Err(PluginError::configuration_error("Plugin name cannot be empty"));
        }

        if metadata.version.is_empty() {
            return Err(PluginError::configuration_error("Plugin version cannot be empty"));
        }

        // Basic semver validation (could be more sophisticated)
        if !metadata.version.chars().any(|c| c.is_ascii_digit()) {
            return Err(PluginError::configuration_error(
                "Plugin version must contain at least one digit",
            ));
        }

        // Validate minimum system version compatibility
        if metadata.min_system_version.as_str() > crate::PLUGIN_SYSTEM_VERSION {
            return Err(PluginError::version_incompatible(
                metadata.name.clone(),
                metadata.version.clone(),
                crate::PLUGIN_SYSTEM_VERSION,
            ));
        }

        Ok(())
    }

    /// Update method mappings based on plugin capabilities
    fn update_method_mappings(&mut self, plugin_name: &str, capabilities: &Capabilities) {
        let methods = [
            // Navigation methods
            ("find_definition", capabilities.navigation.go_to_definition),
            ("find_references", capabilities.navigation.find_references),
            ("find_implementations", capabilities.navigation.find_implementations),
            ("find_type_definition", capabilities.navigation.find_type_definition),
            ("search_workspace_symbols", capabilities.navigation.workspace_symbols),
            ("get_document_symbols", capabilities.navigation.document_symbols),
            ("prepare_call_hierarchy", capabilities.navigation.call_hierarchy),
            ("get_call_hierarchy_incoming_calls", capabilities.navigation.call_hierarchy),
            ("get_call_hierarchy_outgoing_calls", capabilities.navigation.call_hierarchy),

            // Editing methods
            ("rename_symbol", capabilities.editing.rename),
            ("format_document", capabilities.editing.format_document),
            ("format_range", capabilities.editing.format_range),
            ("get_code_actions", capabilities.editing.code_actions),
            ("organize_imports", capabilities.editing.organize_imports),

            // Refactoring methods
            ("extract_function", capabilities.refactoring.extract_function),
            ("extract_variable", capabilities.refactoring.extract_variable),
            ("inline_variable", capabilities.refactoring.inline_variable),

            // Intelligence methods
            ("get_hover", capabilities.intelligence.hover),
            ("get_completions", capabilities.intelligence.completions),
            ("get_signature_help", capabilities.intelligence.signature_help),

            // Diagnostic methods
            ("get_diagnostics", capabilities.diagnostics.diagnostics),
        ];

        // Add standard methods
        for (method, supported) in methods {
            if supported {
                self.method_map
                    .entry(method.to_string())
                    .or_insert_with(Vec::new)
                    .push(plugin_name.to_string());
            }
        }

        // Add custom methods
        for custom_method in capabilities.custom.keys() {
            self.method_map
                .entry(custom_method.clone())
                .or_insert_with(Vec::new)
                .push(plugin_name.to_string());
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the plugin registry
#[derive(Debug, Clone)]
pub struct RegistryStatistics {
    /// Total number of registered plugins
    pub total_plugins: usize,
    /// Number of supported file extensions
    pub supported_extensions: usize,
    /// Number of supported methods
    pub supported_methods: usize,
    /// Average number of methods supported per plugin
    pub average_methods_per_plugin: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PluginRequest, PluginResponse, PluginMetadata};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::path::PathBuf;

    struct TestPlugin {
        name: String,
        extensions: Vec<String>,
        capabilities: Capabilities,
    }

    #[async_trait]
    impl LanguagePlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new(&self.name, "1.0.0", "test")
        }

        fn supported_extensions(&self) -> Vec<String> {
            self.extensions.clone()
        }

        fn capabilities(&self) -> Capabilities {
            self.capabilities.clone()
        }

        async fn handle_request(&self, _request: PluginRequest) -> PluginResult<PluginResponse> {
            Ok(PluginResponse::empty())
        }

        fn configure(&self, _config: Value) -> PluginResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_plugin_registration() {
        let mut registry = PluginRegistry::new();

        let mut capabilities = Capabilities::default();
        capabilities.navigation.go_to_definition = true;

        let plugin = Arc::new(TestPlugin {
            name: "test-plugin".to_string(),
            extensions: vec!["test".to_string()],
            capabilities,
        });

        assert!(registry.register_plugin("test-plugin", plugin).is_ok());
        assert!(registry.get_plugin("test-plugin").is_some());
        assert_eq!(registry.get_plugin_names(), vec!["test-plugin"]);
    }

    #[test]
    fn test_plugin_discovery() {
        let mut registry = PluginRegistry::new();

        let mut capabilities = Capabilities::default();
        capabilities.navigation.go_to_definition = true;

        let plugin = Arc::new(TestPlugin {
            name: "test-plugin".to_string(),
            extensions: vec!["test".to_string()],
            capabilities,
        });

        registry.register_plugin("test-plugin", plugin).unwrap();

        let file_path = PathBuf::from("example.test");
        let plugins = registry.find_plugins_for_file(&file_path);
        assert_eq!(plugins, vec!["test-plugin"]);

        let method_plugins = registry.find_plugins_for_method("find_definition");
        assert_eq!(method_plugins, vec!["test-plugin"]);

        let best_plugin = registry.find_best_plugin(&file_path, "find_definition").unwrap();
        assert_eq!(best_plugin, "test-plugin");
    }

    #[test]
    fn test_plugin_unregistration() {
        let mut registry = PluginRegistry::new();

        let plugin = Arc::new(TestPlugin {
            name: "test-plugin".to_string(),
            extensions: vec!["test".to_string()],
            capabilities: Capabilities::default(),
        });

        registry.register_plugin("test-plugin", plugin).unwrap();
        assert!(registry.get_plugin("test-plugin").is_some());

        assert!(registry.unregister_plugin("test-plugin").is_ok());
        assert!(registry.get_plugin("test-plugin").is_none());
        assert!(registry.get_plugin_names().is_empty());
    }

    #[test]
    fn test_method_support_checking() {
        let mut registry = PluginRegistry::new();

        let mut capabilities = Capabilities::default();
        capabilities.navigation.go_to_definition = true;

        let plugin = Arc::new(TestPlugin {
            name: "test-plugin".to_string(),
            extensions: vec!["test".to_string()],
            capabilities,
        });

        registry.register_plugin("test-plugin", plugin).unwrap();

        let file_path = PathBuf::from("example.test");
        assert!(registry.is_method_supported(&file_path, "find_definition"));
        assert!(!registry.is_method_supported(&file_path, "find_references"));
    }

    #[test]
    fn test_registry_statistics() {
        let mut registry = PluginRegistry::new();

        let mut capabilities = Capabilities::default();
        capabilities.navigation.go_to_definition = true;
        capabilities.navigation.find_references = true;

        let plugin = Arc::new(TestPlugin {
            name: "test-plugin".to_string(),
            extensions: vec!["test".to_string(), "example".to_string()],
            capabilities,
        });

        registry.register_plugin("test-plugin", plugin).unwrap();

        let stats = registry.get_statistics();
        assert_eq!(stats.total_plugins, 1);
        assert_eq!(stats.supported_extensions, 2);
        assert_eq!(stats.supported_methods, 2);
        assert_eq!(stats.average_methods_per_plugin, 2.0);
    }
}