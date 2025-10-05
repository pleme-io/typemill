//! Plugin registry for managing loaded plugins

use crate::{Capabilities, LanguagePlugin, PluginError, PluginMetadata, PluginResult};
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
    /// Priority overrides for plugin selection (plugin_name -> priority)
    priority_overrides: HashMap<String, u32>,
    /// Whether to error on ambiguous selection
    error_on_ambiguity: bool,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            extension_map: HashMap::new(),
            method_map: HashMap::new(),
            metadata_cache: HashMap::new(),
            priority_overrides: HashMap::new(),
            error_on_ambiguity: false,
        }
    }

    /// Set priority overrides for plugin selection
    ///
    /// Priority overrides allow runtime configuration of plugin selection order.
    /// These overrides take precedence over the plugin's metadata priority.
    ///
    /// # Arguments
    /// * `overrides` - Map of plugin name to priority value (higher = more preferred)
    ///
    /// # Example
    /// ```text
    /// let mut overrides = HashMap::new();
    /// overrides.insert("typescript-plugin".to_string(), 100);
    /// overrides.insert("generic-lsp-plugin".to_string(), 50);
    /// registry.set_priority_overrides(overrides);
    /// ```
    pub fn set_priority_overrides(&mut self, overrides: HashMap<String, u32>) {
        self.priority_overrides = overrides;
    }

    /// Set whether to error on ambiguous selection
    ///
    /// When enabled, the registry will return an error if multiple plugins
    /// with the same priority can handle a request. When disabled (default),
    /// it falls back to lexicographic ordering for deterministic selection.
    ///
    /// # Arguments
    /// * `error_on_ambiguity` - true to error on ambiguous selection, false to use fallback
    pub fn set_error_on_ambiguity(&mut self, error_on_ambiguity: bool) {
        self.error_on_ambiguity = error_on_ambiguity;
    }

    /// Get the effective priority for a plugin
    ///
    /// Priority resolution follows this order:
    /// 1. Runtime overrides (from `set_priority_overrides`)
    /// 2. Plugin metadata priority
    /// 3. Default priority (50)
    ///
    /// Higher priorities are preferred when multiple plugins can handle a request.
    fn get_plugin_priority(&self, plugin_name: &str) -> u32 {
        // Check override first
        if let Some(&priority) = self.priority_overrides.get(plugin_name) {
            return priority;
        }

        // Use plugin metadata priority
        if let Some(metadata) = self.metadata_cache.get(plugin_name) {
            return metadata.priority;
        }

        // Default priority
        50
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
                .or_default()
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
        self.method_map.get(method).cloned().unwrap_or_default()
    }

    /// Find the best plugin for a file and method combination using scope-aware priority
    ///
    /// This is the core plugin selection algorithm that uses a sophisticated multi-tiered
    /// approach to select the most appropriate plugin for a given request.
    ///
    /// # Selection Strategy
    ///
    /// 1. **Tool Scope Detection**: Determines if the tool is File-scoped or Workspace-scoped
    ///    - File-scoped tools (e.g., `find_definition`, `rename_symbol`) require a specific file context
    ///    - Workspace-scoped tools (e.g., `search_workspace_symbols`, `list_files`) operate globally
    ///
    /// 2. **Candidate Filtering**:
    ///    - **File-scoped**: Plugins must match BOTH file extension AND method capability
    ///    - **Workspace-scoped**: Plugins only need to support the method
    ///
    /// 3. **Priority-Based Selection**:
    ///    - Selects plugin with highest effective priority (see `get_plugin_priority`)
    ///    - Ties broken by lexicographic order (deterministic, reproducible)
    ///    - Optionally errors on ambiguity if `error_on_ambiguity` is true
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file being operated on (used for extension matching)
    /// * `method` - The tool/method name being invoked (e.g., "find_definition")
    ///
    /// # Returns
    ///
    /// * `Ok(plugin_name)` - Name of the selected plugin
    /// * `Err(PluginNotFound)` - No plugin supports this file type and method
    /// * `Err(AmbiguousPluginSelection)` - Multiple plugins with same priority (if `error_on_ambiguity` enabled)
    ///
    /// # Performance
    ///
    /// - Time Complexity: O(n) where n = number of candidate plugins
    /// - Typical latency: 141ns (1 plugin) to 1.7µs (20 plugins)
    ///
    /// # Example
    ///
    /// ```text
    /// let plugin = registry.find_best_plugin(
    ///     Path::new("src/main.ts"),
    ///     "find_definition"
    /// )?;
    /// // Returns: "typescript-plugin"
    /// ```
    pub fn find_best_plugin(&self, file_path: &Path, method: &str) -> PluginResult<String> {
        let file_plugins = self.find_plugins_for_file(file_path);
        let method_plugins = self.find_plugins_for_method(method);

        // Determine tool scope from first plugin that supports this method
        let tool_scope = method_plugins
            .first()
            .and_then(|plugin_name| self.get_plugin(plugin_name))
            .and_then(|plugin| {
                let caps = plugin.capabilities();
                caps.get_tool_scope(method)
            });

        let candidate_plugins = match tool_scope {
            Some(crate::ToolScope::File) => {
                // File-scoped tools require both file extension AND method match
                file_plugins
                    .into_iter()
                    .filter(|plugin| method_plugins.contains(plugin))
                    .collect()
            }
            Some(crate::ToolScope::Workspace) | None => {
                // Workspace-scoped tools only need method match
                // Also use this as fallback when scope is unknown
                method_plugins
            }
        };

        // Select best plugin from candidates based on priority
        if let Some(best) = self.select_by_priority(&candidate_plugins, method)? {
            return Ok(best);
        }

        // No compatible plugins found
        Err(PluginError::plugin_not_found(
            file_path.to_string_lossy(),
            method,
        ))
    }

    /// Select the best plugin from a list based on priority
    ///
    /// Returns None if the list is empty
    /// Returns Err if there's an ambiguous selection (multiple plugins with same priority)
    /// Returns Ok(Some(plugin_name)) otherwise
    fn select_by_priority(&self, plugins: &[String], method: &str) -> PluginResult<Option<String>> {
        if plugins.is_empty() {
            return Ok(None);
        }

        if plugins.len() == 1 {
            return Ok(Some(plugins[0].clone()));
        }

        // Find maximum priority
        let max_priority = plugins
            .iter()
            .map(|p| self.get_plugin_priority(p))
            .max()
            .unwrap_or(50);

        // Get all plugins with max priority
        let mut best_plugins: Vec<&String> = plugins
            .iter()
            .filter(|p| self.get_plugin_priority(p) == max_priority)
            .collect();

        // Check for ambiguity
        if best_plugins.len() > 1 {
            if self.error_on_ambiguity {
                return Err(PluginError::AmbiguousPluginSelection {
                    method: method.to_string(),
                    plugins: best_plugins.iter().map(|p| p.to_string()).collect(),
                    priority: max_priority,
                });
            } else {
                // Deterministic fallback: sort by name and pick first
                best_plugins.sort();
                debug!(
                    method = %method,
                    candidates = ?best_plugins,
                    selected = %best_plugins[0],
                    priority = max_priority,
                    "Multiple plugins with same priority, using lexicographic order"
                );
            }
        }

        Ok(Some(best_plugins[0].clone()))
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
            return Err(PluginError::configuration_error(
                "Plugin name cannot be empty",
            ));
        }

        if metadata.version.is_empty() {
            return Err(PluginError::configuration_error(
                "Plugin version cannot be empty",
            ));
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
            (
                "find_implementations",
                capabilities.navigation.find_implementations,
            ),
            (
                "find_type_definition",
                capabilities.navigation.find_type_definition,
            ),
            (
                "search_workspace_symbols",
                capabilities.navigation.workspace_symbols,
            ),
            (
                "get_document_symbols",
                capabilities.navigation.document_symbols,
            ),
            (
                "prepare_call_hierarchy",
                capabilities.navigation.call_hierarchy,
            ),
            (
                "get_call_hierarchy_incoming_calls",
                capabilities.navigation.call_hierarchy,
            ),
            (
                "get_call_hierarchy_outgoing_calls",
                capabilities.navigation.call_hierarchy,
            ),
            // Editing methods
            ("rename_symbol", capabilities.editing.rename),
            ("format_document", capabilities.editing.format_document),
            ("format_range", capabilities.editing.format_range),
            ("get_code_actions", capabilities.editing.code_actions),
            ("organize_imports", capabilities.editing.organize_imports),
            // Refactoring methods
            (
                "extract_function",
                capabilities.refactoring.extract_function,
            ),
            (
                "extract_variable",
                capabilities.refactoring.extract_variable,
            ),
            ("inline_variable", capabilities.refactoring.inline_variable),
            // Intelligence methods
            ("get_hover", capabilities.intelligence.hover),
            ("get_completions", capabilities.intelligence.completions),
            (
                "get_signature_help",
                capabilities.intelligence.signature_help,
            ),
            // Diagnostic methods
            ("get_diagnostics", capabilities.diagnostics.diagnostics),
        ];

        // Add standard methods
        for (method, supported) in methods {
            if supported {
                self.method_map
                    .entry(method.to_string())
                    .or_default()
                    .push(plugin_name.to_string());
            }
        }

        // Add custom methods
        for custom_method in capabilities.custom.keys() {
            self.method_map
                .entry(custom_method.clone())
                .or_default()
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
    use crate::{PluginMetadata, PluginRequest, PluginResponse};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::path::PathBuf;

    struct TestPlugin {
        name: String,
        extensions: Vec<String>,
        capabilities: Capabilities,
        metadata: Option<PluginMetadata>,
    }

    #[async_trait]
    impl LanguagePlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            self.metadata
                .clone()
                .unwrap_or_else(|| PluginMetadata::new(&self.name, "1.0.0", "test"))
        }

        fn supported_extensions(&self) -> Vec<String> {
            self.extensions.clone()
        }

        fn tool_definitions(&self) -> Vec<Value> {
            vec![]
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
            metadata: None,
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
            metadata: None,
        });

        registry.register_plugin("test-plugin", plugin).unwrap();

        let file_path = PathBuf::from("example.test");
        let plugins = registry.find_plugins_for_file(&file_path);
        assert_eq!(plugins, vec!["test-plugin"]);

        let method_plugins = registry.find_plugins_for_method("find_definition");
        assert_eq!(method_plugins, vec!["test-plugin"]);

        let best_plugin = registry
            .find_best_plugin(&file_path, "find_definition")
            .unwrap();
        assert_eq!(best_plugin, "test-plugin");
    }

    #[test]
    fn test_plugin_unregistration() {
        let mut registry = PluginRegistry::new();

        let plugin = Arc::new(TestPlugin {
            name: "test-plugin".to_string(),
            extensions: vec!["test".to_string()],
            capabilities: Capabilities::default(),
            metadata: None,
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
            metadata: None,
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
            metadata: None,
        });

        registry.register_plugin("test-plugin", plugin).unwrap();

        let stats = registry.get_statistics();
        assert_eq!(stats.total_plugins, 1);
        assert_eq!(stats.supported_extensions, 2);
        assert_eq!(stats.supported_methods, 2);
        assert_eq!(stats.average_methods_per_plugin, 2.0);
    }

    #[test]
    fn test_scope_aware_file_tool_selection() {
        // Test that file-scoped tools require both file extension AND method match
        let mut registry = PluginRegistry::new();

        // Plugin 1: Supports TypeScript files + find_definition (file-scoped)
        let mut caps1 = Capabilities::default();
        caps1.navigation.go_to_definition = true;
        let plugin1 = Arc::new(TestPlugin {
            name: "ts-plugin".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps1,
            metadata: None,
        });

        // Plugin 2: Supports find_definition but not TypeScript files
        let mut caps2 = Capabilities::default();
        caps2.navigation.go_to_definition = true;
        let plugin2 = Arc::new(TestPlugin {
            name: "generic-plugin".to_string(),
            extensions: vec!["js".to_string()],
            capabilities: caps2,
            metadata: None,
        });

        registry.register_plugin("ts-plugin", plugin1).unwrap();
        registry.register_plugin("generic-plugin", plugin2).unwrap();

        // File-scoped tool should only match plugin with matching file extension
        let ts_file = PathBuf::from("example.ts");
        let best = registry
            .find_best_plugin(&ts_file, "find_definition")
            .unwrap();
        assert_eq!(best, "ts-plugin");

        let js_file = PathBuf::from("example.js");
        let best = registry
            .find_best_plugin(&js_file, "find_definition")
            .unwrap();
        assert_eq!(best, "generic-plugin");
    }

    #[test]
    fn test_scope_aware_workspace_tool_selection() {
        // Test that workspace-scoped tools only need method match
        let mut registry = PluginRegistry::new();

        // Plugin supports workspace-scoped tool (search_workspace_symbols)
        let mut caps = Capabilities::default();
        caps.navigation.workspace_symbols = true;
        let plugin = Arc::new(TestPlugin {
            name: "workspace-plugin".to_string(),
            extensions: vec!["ts".to_string()], // File extension shouldn't matter
            capabilities: caps,
            metadata: None,
        });

        registry
            .register_plugin("workspace-plugin", plugin)
            .unwrap();

        // Workspace tool should work regardless of file extension
        let ts_file = PathBuf::from("example.ts");
        let best = registry
            .find_best_plugin(&ts_file, "search_workspace_symbols")
            .unwrap();
        assert_eq!(best, "workspace-plugin");

        let py_file = PathBuf::from("example.py");
        let best = registry
            .find_best_plugin(&py_file, "search_workspace_symbols")
            .unwrap();
        assert_eq!(best, "workspace-plugin");
    }

    #[test]
    fn test_priority_based_selection() {
        // Test that priority-based selection works correctly
        let mut registry = PluginRegistry::new();

        // Plugin 1: Priority 40
        let mut caps1 = Capabilities::default();
        caps1.navigation.go_to_definition = true;
        let mut meta1 = PluginMetadata::new("low-priority", "1.0.0", "test");
        meta1.priority = 40;

        let plugin1 = Arc::new(TestPlugin {
            name: "low-priority".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps1,
            metadata: Some(meta1),
        });

        // Plugin 2: Priority 60
        let mut caps2 = Capabilities::default();
        caps2.navigation.go_to_definition = true;
        let mut meta2 = PluginMetadata::new("high-priority", "1.0.0", "test");
        meta2.priority = 60;

        let plugin2 = Arc::new(TestPlugin {
            name: "high-priority".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps2,
            metadata: Some(meta2),
        });

        registry.register_plugin("low-priority", plugin1).unwrap();
        registry.register_plugin("high-priority", plugin2).unwrap();

        // High priority plugin should be selected
        let ts_file = PathBuf::from("example.ts");
        let best = registry
            .find_best_plugin(&ts_file, "find_definition")
            .unwrap();
        assert_eq!(best, "high-priority");
    }

    #[test]
    fn test_priority_override() {
        // Test that priority overrides work correctly
        let mut registry = PluginRegistry::new();

        // Plugin 1: Default priority 50
        let mut caps1 = Capabilities::default();
        caps1.navigation.go_to_definition = true;
        let plugin1 = Arc::new(TestPlugin {
            name: "plugin1".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps1,
            metadata: None,
        });

        // Plugin 2: Default priority 50
        let mut caps2 = Capabilities::default();
        caps2.navigation.go_to_definition = true;
        let plugin2 = Arc::new(TestPlugin {
            name: "plugin2".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps2,
            metadata: None,
        });

        registry.register_plugin("plugin1", plugin1).unwrap();
        registry.register_plugin("plugin2", plugin2).unwrap();

        // Set priority override for plugin1
        let mut overrides = HashMap::new();
        overrides.insert("plugin1".to_string(), 100);
        registry.set_priority_overrides(overrides);

        let ts_file = PathBuf::from("example.ts");
        let best = registry
            .find_best_plugin(&ts_file, "find_definition")
            .unwrap();
        assert_eq!(best, "plugin1");
    }

    #[test]
    fn test_ambiguous_selection_error() {
        // Test that ambiguous selection is detected when error_on_ambiguity is true
        let mut registry = PluginRegistry::new();
        registry.set_error_on_ambiguity(true);

        // Two plugins with same priority
        let mut caps1 = Capabilities::default();
        caps1.navigation.go_to_definition = true;
        let plugin1 = Arc::new(TestPlugin {
            name: "plugin1".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps1,
            metadata: None,
        });

        let mut caps2 = Capabilities::default();
        caps2.navigation.go_to_definition = true;
        let plugin2 = Arc::new(TestPlugin {
            name: "plugin2".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps2,
            metadata: None,
        });

        registry.register_plugin("plugin1", plugin1).unwrap();
        registry.register_plugin("plugin2", plugin2).unwrap();

        let ts_file = PathBuf::from("example.ts");
        let result = registry.find_best_plugin(&ts_file, "find_definition");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginError::AmbiguousPluginSelection { .. }
        ));
    }

    #[test]
    fn test_ambiguous_selection_fallback() {
        // Test that lexicographic fallback works when error_on_ambiguity is false
        let mut registry = PluginRegistry::new();
        registry.set_error_on_ambiguity(false);

        // Two plugins with same priority
        let mut caps1 = Capabilities::default();
        caps1.navigation.go_to_definition = true;
        let plugin1 = Arc::new(TestPlugin {
            name: "zebra-plugin".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps1,
            metadata: None,
        });

        let mut caps2 = Capabilities::default();
        caps2.navigation.go_to_definition = true;
        let plugin2 = Arc::new(TestPlugin {
            name: "alpha-plugin".to_string(),
            extensions: vec!["ts".to_string()],
            capabilities: caps2,
            metadata: None,
        });

        registry.register_plugin("zebra-plugin", plugin1).unwrap();
        registry.register_plugin("alpha-plugin", plugin2).unwrap();

        let ts_file = PathBuf::from("example.ts");
        let best = registry
            .find_best_plugin(&ts_file, "find_definition")
            .unwrap();

        // Should select "alpha-plugin" due to lexicographic order
        assert_eq!(best, "alpha-plugin");
    }
}
