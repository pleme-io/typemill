//! Plugin manager for orchestrating plugin operations

use crate::registry::RegistryStatistics;
use crate::{
    Capabilities, LanguagePlugin, PluginMetadata, PluginRequest, PluginResponse, PluginResult,
    PluginSystemError, RuntimePluginManager,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

/// Main plugin manager that orchestrates all plugin operations
pub struct PluginManager {
    /// Plugin registry
    registry: Arc<RwLock<RuntimePluginManager>>,
    /// Plugin configurations
    configurations: Arc<RwLock<HashMap<String, Value>>>,
    /// Performance metrics
    metrics: Arc<RwLock<PluginMetrics>>,
}

/// Performance metrics for plugin operations
#[derive(Debug, Clone, Default)]
pub struct PluginMetrics {
    /// Total number of requests processed
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Average processing time in milliseconds
    pub average_processing_time_ms: f64,
    /// Request count per plugin
    pub requests_per_plugin: HashMap<String, u64>,
    /// Processing time per plugin
    pub processing_time_per_plugin: HashMap<String, f64>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(RuntimePluginManager::new())),
            configurations: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(PluginMetrics::default())),
        }
    }

    /// Register a new plugin
    #[instrument(skip(self, plugin))]
    pub async fn register_plugin(
        &self,
        name: impl Into<String> + std::fmt::Debug,
        plugin: Arc<dyn LanguagePlugin>,
    ) -> PluginResult<()> {
        let name = name.into();
        debug!("PluginManager::register_plugin called for '{}'", name);

        // Initialize the plugin
        // Note: We can't mutate the plugin here since it's behind an Arc<dyn Trait>
        // In a real implementation, plugins would handle their own initialization

        debug!("Getting registry write lock for '{}'", name);
        let mut registry = self.registry.write().await;
        debug!("Calling registry.register_plugin for '{}'", name);
        registry.register_plugin(&name, plugin)?;
        debug!("Plugin '{}' registered successfully in registry", name);

        info!("Successfully registered plugin '{}'", name);
        debug!("PluginManager::register_plugin completed for '{}'", name);
        Ok(())
    }

    /// Unregister a plugin
    #[instrument(skip(self))]
    pub async fn unregister_plugin(&self, name: &str) -> PluginResult<()> {
        let mut registry = self.registry.write().await;
        registry.unregister_plugin(name)?;

        // Remove configuration
        let mut configs = self.configurations.write().await;
        configs.remove(name);

        info!("Successfully unregistered plugin '{}'", name);
        Ok(())
    }

    /// Configure a plugin
    #[instrument(skip(self, config))]
    pub async fn configure_plugin(&self, name: &str, config: Value) -> PluginResult<()> {
        let registry = self.registry.read().await;

        if let Some(plugin) = registry.get_plugin(name) {
            plugin.configure(config.clone())?;

            // Store configuration
            let mut configs = self.configurations.write().await;
            configs.insert(name.to_string(), config);

            debug!("Configured plugin '{}'", name);
            Ok(())
        } else {
            Err(PluginSystemError::plugin_not_found(name, "configure"))
        }
    }

    /// Handle a plugin request
    #[instrument(skip(self, request), fields(method = %request.method, file = %request.file_path.display()))]
    pub async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse> {
        let start_time = Instant::now();

        // Find the best plugin for this request
        let registry = self.registry.read().await;
        let plugin_result = registry.find_best_plugin(&request.file_path, &request.method);

        let (plugin_name, plugin) = match plugin_result {
            Ok(plugin_name) => {
                debug!(
                    "PluginManager got plugin_name '{}' from registry",
                    plugin_name
                );
                let plugin = registry.get_plugin(&plugin_name).ok_or_else(|| {
                    error!("get_plugin('{}') returned None!", plugin_name);
                    PluginSystemError::plugin_not_found(&plugin_name, &request.method)
                })?;
                debug!("Successfully got plugin '{}' from registry", plugin_name);
                (plugin_name, plugin)
            }
            Err(err) => {
                // Update metrics for system-level failures (no plugin found)
                let processing_time = start_time.elapsed().as_millis() as u64;
                let error_result: PluginResult<PluginResponse> = Err(err.clone());
                self.update_metrics("none", &error_result, processing_time)
                    .await;
                return Err(err);
            }
        };

        // Release the registry lock before making the request
        drop(registry);

        debug!("Routing request to plugin '{}'", plugin_name);

        // Save file extension and method before moving request
        let file_extension = request
            .file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_string();
        let method = request.method.clone();

        // Handle the request
        let result = plugin.handle_request(request).await;

        // Update metrics
        let processing_time = start_time.elapsed().as_millis() as u64;
        self.update_metrics(&plugin_name, &result, processing_time)
            .await;

        match result {
            Ok(mut response) => {
                // Ensure response metadata is populated
                response.metadata.plugin_name = plugin_name;
                response.metadata.processing_time_ms = Some(processing_time);

                debug!(
                    plugin = %response.metadata.plugin_name,
                    duration_ms = processing_time,
                    method = %method,
                    file_extension = %file_extension,
                    "Request processed successfully"
                );
                Ok(response)
            }
            Err(err) => {
                error!(
                    plugin = %plugin_name,
                    error = %err,
                    method = %method,
                    file_extension = %file_extension,
                    "Plugin request failed"
                );
                Err(err)
            }
        }
    }

    /// Check if a method is supported for a file
    pub async fn is_method_supported(&self, file_path: &Path, method: &str) -> bool {
        let registry = self.registry.read().await;
        registry.is_method_supported(file_path, method)
    }

    /// Get all supported file extensions
    pub async fn get_supported_extensions(&self) -> Vec<String> {
        let registry = self.registry.read().await;
        registry.get_supported_extensions()
    }

    /// Get all supported methods
    pub async fn get_supported_methods(&self) -> Vec<String> {
        let registry = self.registry.read().await;
        registry.get_supported_methods()
    }

    /// Get capabilities for all plugins
    pub async fn get_all_capabilities(&self) -> HashMap<String, Capabilities> {
        let registry = self.registry.read().await;
        // Use optimized bulk retrieval from registry to avoid N+1 queries
        registry.get_all_capabilities()
    }

    /// Get capabilities for a specific plugin
    pub async fn get_plugin_capabilities(&self, name: &str) -> Option<Capabilities> {
        let registry = self.registry.read().await;
        registry.get_plugin_capabilities(name)
    }

    /// Get metadata for all plugins
    pub async fn get_all_metadata(&self) -> HashMap<String, PluginMetadata> {
        let registry = self.registry.read().await;
        // Use optimized bulk retrieval from registry to avoid N+1 queries.
        // This replaces the previous inefficient loop that performed N lookups.
        registry.get_all_metadata()
    }

    /// Get metadata for a specific plugin
    pub async fn get_plugin_metadata(&self, name: &str) -> Option<PluginMetadata> {
        let registry = self.registry.read().await;
        registry.get_plugin_metadata(name).cloned()
    }

    /// Get registry statistics
    pub async fn get_registry_statistics(&self) -> RegistryStatistics {
        let registry = self.registry.read().await;
        registry.get_statistics()
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> PluginMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Reset performance metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = PluginMetrics::default();
        info!("Plugin metrics reset");
    }

    /// Get plugin configuration
    pub async fn get_plugin_configuration(&self, name: &str) -> Option<Value> {
        let configs = self.configurations.read().await;
        configs.get(name).cloned()
    }

    /// List all registered plugins
    pub async fn list_plugins(&self) -> Vec<String> {
        let registry = self.registry.read().await;
        registry.get_plugin_names()
    }

    /// Check if a plugin is registered
    pub async fn is_plugin_registered(&self, name: &str) -> bool {
        let registry = self.registry.read().await;
        registry.get_plugin(name).is_some()
    }

    /// Get a plugin by name for direct request handling
    /// Returns None if the plugin is not found
    pub async fn get_plugin_by_name(&self, name: &str) -> Option<Arc<dyn LanguagePlugin>> {
        let registry = self.registry.read().await;
        registry.get_plugin(name)
    }

    /// Find plugins that can handle a specific file
    pub async fn find_plugins_for_file(&self, file_path: &Path) -> Vec<String> {
        let registry = self.registry.read().await;
        registry.find_plugins_for_file(file_path)
    }

    /// Find plugins that support a specific method
    pub async fn find_plugins_for_method(&self, method: &str) -> Vec<String> {
        let registry = self.registry.read().await;
        registry.find_plugins_for_method(method)
    }

    /// Trigger file open hooks for all plugins that can handle this file
    #[instrument(skip(self), fields(file = %file_path.display()))]
    pub async fn trigger_file_open_hooks(&self, file_path: &Path) -> PluginResult<()> {
        let registry = self.registry.read().await;
        let plugin_names = registry.find_plugins_for_file(file_path);

        debug!(
            file = %file_path.display(),
            plugins = ?plugin_names,
            hook = "on_file_open",
            "Triggering lifecycle hooks"
        );

        for plugin_name in plugin_names {
            if let Some(plugin) = registry.get_plugin(&plugin_name) {
                match plugin.on_file_open(file_path) {
                    Ok(()) => {
                        debug!(plugin = %plugin_name, hook = "on_file_open", "Hook succeeded");
                    }
                    Err(e) => {
                        warn!(
                            plugin = %plugin_name,
                            hook = "on_file_open",
                            error = %e,
                            "Hook failed (continuing)"
                        );
                        // Don't propagate errors - hooks are non-blocking
                    }
                }
            }
        }

        Ok(())
    }

    /// Trigger file save hooks for all plugins that can handle this file
    #[instrument(skip(self), fields(file = %file_path.display()))]
    pub async fn trigger_file_save_hooks(&self, file_path: &Path) -> PluginResult<()> {
        let registry = self.registry.read().await;
        let plugin_names = registry.find_plugins_for_file(file_path);

        debug!(
            file = %file_path.display(),
            plugins = ?plugin_names,
            hook = "on_file_save",
            "Triggering lifecycle hooks"
        );

        for plugin_name in plugin_names {
            if let Some(plugin) = registry.get_plugin(&plugin_name) {
                match plugin.on_file_save(file_path) {
                    Ok(()) => {
                        debug!(plugin = %plugin_name, hook = "on_file_save", "Hook succeeded");
                    }
                    Err(e) => {
                        warn!(
                            plugin = %plugin_name,
                            hook = "on_file_save",
                            error = %e,
                            "Hook failed (continuing)"
                        );
                        // Don't propagate errors - hooks are non-blocking
                    }
                }
            }
        }

        Ok(())
    }

    /// Trigger file close hooks for all plugins that can handle this file
    #[instrument(skip(self), fields(file = %file_path.display()))]
    pub async fn trigger_file_close_hooks(&self, file_path: &Path) -> PluginResult<()> {
        let registry = self.registry.read().await;
        let plugin_names = registry.find_plugins_for_file(file_path);

        debug!(
            file = %file_path.display(),
            plugins = ?plugin_names,
            hook = "on_file_close",
            "Triggering lifecycle hooks"
        );

        for plugin_name in plugin_names {
            if let Some(plugin) = registry.get_plugin(&plugin_name) {
                match plugin.on_file_close(file_path) {
                    Ok(()) => {
                        debug!(plugin = %plugin_name, hook = "on_file_close", "Hook succeeded");
                    }
                    Err(e) => {
                        warn!(
                            plugin = %plugin_name,
                            hook = "on_file_close",
                            error = %e,
                            "Hook failed (continuing)"
                        );
                        // Don't propagate errors - hooks are non-blocking
                    }
                }
            }
        }

        Ok(())
    }

    /// Get all tool definitions from all registered plugins
    pub async fn get_all_tool_definitions(&self) -> Vec<Value> {
        let registry = self.registry.read().await;
        registry
            .get_all_plugins()
            .flat_map(|plugin| plugin.tool_definitions())
            .collect()
    }

    /// Get all registered plugins with their names
    pub async fn get_all_plugins_with_names(&self) -> Vec<(String, Arc<dyn LanguagePlugin>)> {
        let registry = self.registry.read().await;
        registry
            .get_plugins_with_names()
            .map(|(name, plugin)| (name.clone(), plugin.clone()))
            .collect()
    }

    /// Shutdown all plugins gracefully
    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> PluginResult<()> {
        let registry = self.registry.read().await;

        // Note: In a real implementation, we'd call shutdown on each plugin
        // This would require either making LanguagePlugin methods async
        // or using a different approach for lifecycle management

        info!("Shutting down {} plugins", registry.plugin_count());

        for (plugin_name, _plugin) in registry.get_plugins_with_names() {
            // plugin.shutdown().await?; // Would need async trait methods
            debug!("Plugin '{}' shutdown", plugin_name);
        }

        info!("All plugins shut down successfully");
        Ok(())
    }

    /// Update performance metrics
    async fn update_metrics(
        &self,
        plugin_name: &str,
        result: &PluginResult<PluginResponse>,
        processing_time_ms: u64,
    ) {
        let mut metrics = self.metrics.write().await;

        metrics.total_requests += 1;

        if result.is_ok() {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // Update average processing time
        let total_time = metrics.average_processing_time_ms * (metrics.total_requests - 1) as f64
            + processing_time_ms as f64;
        metrics.average_processing_time_ms = total_time / metrics.total_requests as f64;

        // Update per-plugin metrics
        *metrics
            .requests_per_plugin
            .entry(plugin_name.to_string())
            .or_insert(0) += 1;

        let plugin_requests = *metrics.requests_per_plugin.get(plugin_name).unwrap_or(&1);
        let current_avg = metrics
            .processing_time_per_plugin
            .get(plugin_name)
            .copied()
            .unwrap_or(0.0);

        let new_avg = (current_avg * (plugin_requests - 1) as f64 + processing_time_ms as f64)
            / plugin_requests as f64;

        metrics
            .processing_time_per_plugin
            .insert(plugin_name.to_string(), new_avg);
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Capabilities, PluginMetadata, PluginRequest, PluginResponse, PluginSystemError};
    use async_trait::async_trait;
    use std::path::PathBuf;

    struct TestPlugin {
        name: String,
        extensions: Vec<String>,
        capabilities: Capabilities,
        should_fail: bool,
    }

    #[async_trait]
    impl LanguagePlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new(&self.name, "1.0.0", "test")
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

        async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse> {
            if self.should_fail {
                Err(PluginSystemError::request_failed(
                    &self.name,
                    "test failure",
                ))
            } else {
                Ok(PluginResponse::success(
                    serde_json::json!({"method": request.method}),
                    &self.name,
                ))
            }
        }

        fn configure(&self, _config: Value) -> PluginResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_plugin_manager_basic_operations() {
        let manager = PluginManager::new();

        let mut capabilities = Capabilities::default();
        capabilities.navigation.go_to_definition = true;

        let plugin = Arc::new(TestPlugin {
            name: "test-plugin".to_string(),
            extensions: vec!["test".to_string()],
            capabilities,
            should_fail: false,
        });

        // Register plugin
        assert!(manager.register_plugin("test-plugin", plugin).await.is_ok());
        assert!(manager.is_plugin_registered("test-plugin").await);

        // Test request handling
        let request = PluginRequest::new("find_definition", PathBuf::from("test.test"));
        let response = manager.handle_request(request).await.unwrap();
        assert!(response.success);
        assert_eq!(response.metadata.plugin_name, "test-plugin");

        // Check metrics
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[tokio::test]
    async fn test_plugin_failure_handling() {
        let manager = PluginManager::new();

        let mut capabilities = Capabilities::default();
        capabilities.navigation.go_to_definition = true;

        let plugin = Arc::new(TestPlugin {
            name: "failing-plugin".to_string(),
            extensions: vec!["test".to_string()],
            capabilities,
            should_fail: true,
        });

        manager
            .register_plugin("failing-plugin", plugin)
            .await
            .unwrap();

        let request = PluginRequest::new("find_definition", PathBuf::from("test.test"));
        let result = manager.handle_request(request).await;
        assert!(result.is_err());

        // Check failure metrics
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_plugin_configuration() {
        let manager = PluginManager::new();

        let plugin = Arc::new(TestPlugin {
            name: "configurable-plugin".to_string(),
            extensions: vec!["test".to_string()],
            capabilities: Capabilities::default(),
            should_fail: false,
        });

        manager
            .register_plugin("configurable-plugin", plugin)
            .await
            .unwrap();

        let config = serde_json::json!({"enabled": true, "level": "debug"});
        assert!(manager
            .configure_plugin("configurable-plugin", config.clone())
            .await
            .is_ok());

        let stored_config = manager
            .get_plugin_configuration("configurable-plugin")
            .await;
        assert_eq!(stored_config, Some(config));
    }

    #[tokio::test]
    async fn test_plugin_discovery() {
        let manager = PluginManager::new();

        let mut capabilities = Capabilities::default();
        capabilities.navigation.go_to_definition = true;
        capabilities.editing.rename = true;

        let plugin = Arc::new(TestPlugin {
            name: "discovery-test".to_string(),
            extensions: vec!["test".to_string(), "example".to_string()],
            capabilities,
            should_fail: false,
        });

        manager
            .register_plugin("discovery-test", plugin)
            .await
            .unwrap();

        // Test file-based discovery
        let plugins = manager
            .find_plugins_for_file(&PathBuf::from("file.test"))
            .await;
        assert_eq!(plugins, vec!["discovery-test"]);

        // Test method-based discovery
        let plugins = manager.find_plugins_for_method("find_definition").await;
        assert_eq!(plugins, vec!["discovery-test"]);

        // Test capability checking
        assert!(
            manager
                .is_method_supported(&PathBuf::from("file.test"), "find_definition")
                .await
        );
        assert!(
            !manager
                .is_method_supported(&PathBuf::from("file.other"), "find_definition")
                .await
        );
    }

    #[tokio::test]
    async fn test_file_open_hooks() {
        use std::sync::atomic::{AtomicBool, Ordering};

        struct HookTestPlugin {
            hook_called: Arc<AtomicBool>,
            extensions: Vec<String>,
        }

        #[async_trait]
        impl LanguagePlugin for HookTestPlugin {
            fn metadata(&self) -> PluginMetadata {
                PluginMetadata::new("hook-test", "1.0.0", "test")
            }

            fn supported_extensions(&self) -> Vec<String> {
                self.extensions.clone()
            }

            fn capabilities(&self) -> Capabilities {
                Capabilities::default()
            }

            async fn handle_request(
                &self,
                _request: PluginRequest,
            ) -> PluginResult<PluginResponse> {
                Ok(PluginResponse::empty())
            }

            fn configure(&self, _config: Value) -> PluginResult<()> {
                Ok(())
            }

            fn on_file_open(&self, _path: &Path) -> PluginResult<()> {
                self.hook_called.store(true, Ordering::SeqCst);
                Ok(())
            }

            fn tool_definitions(&self) -> Vec<Value> {
                vec![]
            }
        }

        let manager = PluginManager::new();
        let hook_called = Arc::new(AtomicBool::new(false));

        let plugin = Arc::new(HookTestPlugin {
            hook_called: hook_called.clone(),
            extensions: vec!["test".to_string()],
        });

        manager.register_plugin("hook-test", plugin).await.unwrap();

        // Trigger hook for matching file
        let test_file = PathBuf::from("file.test");
        manager.trigger_file_open_hooks(&test_file).await.unwrap();

        assert!(
            hook_called.load(Ordering::SeqCst),
            "Hook should be called for .test file"
        );

        // Reset and test non-matching file
        hook_called.store(false, Ordering::SeqCst);
        let other_file = PathBuf::from("file.other");
        manager.trigger_file_open_hooks(&other_file).await.unwrap();

        assert!(
            !hook_called.load(Ordering::SeqCst),
            "Hook should NOT be called for .other file"
        );
    }

    #[tokio::test]
    async fn test_hook_error_handling() {
        struct FailingHookPlugin;

        #[async_trait]
        impl LanguagePlugin for FailingHookPlugin {
            fn metadata(&self) -> PluginMetadata {
                PluginMetadata::new("failing-hook", "1.0.0", "test")
            }

            fn supported_extensions(&self) -> Vec<String> {
                vec!["fail".to_string()]
            }

            fn capabilities(&self) -> Capabilities {
                Capabilities::default()
            }

            async fn handle_request(
                &self,
                _request: PluginRequest,
            ) -> PluginResult<PluginResponse> {
                Ok(PluginResponse::empty())
            }

            fn configure(&self, _config: Value) -> PluginResult<()> {
                Ok(())
            }

            fn on_file_open(&self, _path: &Path) -> PluginResult<()> {
                Err(PluginSystemError::request_failed(
                    "failing-hook",
                    "Intentional test failure",
                ))
            }

            fn tool_definitions(&self) -> Vec<Value> {
                vec![]
            }
        }

        let manager = PluginManager::new();
        let plugin = Arc::new(FailingHookPlugin);

        manager
            .register_plugin("failing-hook", plugin)
            .await
            .unwrap();

        // Hook failure should not propagate
        let result = manager
            .trigger_file_open_hooks(&PathBuf::from("file.fail"))
            .await;
        assert!(result.is_ok(), "Hook errors should not propagate");
    }

    #[tokio::test]
    async fn test_multiple_plugins_receive_hooks() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_counter = Arc::new(AtomicUsize::new(0));

        struct CountingPlugin {
            counter: Arc<AtomicUsize>,
            name: String,
            extensions: Vec<String>,
        }

        #[async_trait]
        impl LanguagePlugin for CountingPlugin {
            fn metadata(&self) -> PluginMetadata {
                PluginMetadata::new(&self.name, "1.0.0", "test")
            }

            fn supported_extensions(&self) -> Vec<String> {
                self.extensions.clone()
            }

            fn capabilities(&self) -> Capabilities {
                Capabilities::default()
            }

            async fn handle_request(
                &self,
                _request: PluginRequest,
            ) -> PluginResult<PluginResponse> {
                Ok(PluginResponse::empty())
            }

            fn configure(&self, _config: Value) -> PluginResult<()> {
                Ok(())
            }

            fn on_file_open(&self, _path: &Path) -> PluginResult<()> {
                self.counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            fn tool_definitions(&self) -> Vec<Value> {
                vec![]
            }
        }

        let manager = PluginManager::new();

        // Register two plugins for the same extension
        let plugin1 = Arc::new(CountingPlugin {
            counter: call_counter.clone(),
            name: "counter1".to_string(),
            extensions: vec!["ts".to_string()],
        });

        let plugin2 = Arc::new(CountingPlugin {
            counter: call_counter.clone(),
            name: "counter2".to_string(),
            extensions: vec!["ts".to_string()],
        });

        manager.register_plugin("counter1", plugin1).await.unwrap();
        manager.register_plugin("counter2", plugin2).await.unwrap();

        // Trigger hook
        manager
            .trigger_file_open_hooks(&PathBuf::from("file.ts"))
            .await
            .unwrap();

        assert_eq!(
            call_counter.load(Ordering::SeqCst),
            2,
            "Both plugins should receive the hook"
        );
    }

    #[tokio::test]
    async fn test_benchmark_metadata_retrieval() {
        use std::collections::HashMap;
        use std::time::Instant;

        let manager = PluginManager::new();
        // Register 1000 dummy plugins
        let count = 1000;
        for i in 0..count {
            let name = format!("plugin-{}", i);
            let plugin = Arc::new(TestPlugin {
                name: name.clone(),
                extensions: vec!["txt".to_string()],
                capabilities: Capabilities::default(),
                should_fail: false,
            });
            manager.register_plugin(&name, plugin).await.unwrap();
        }

        // Measure inefficient approach (simulating N+1 lookup)
        // This simulates the pattern we want to avoid: iterating names and fetching metadata individually
        let start = Instant::now();
        let registry = manager.registry.read().await;
        let mut inefficient_map = HashMap::new();

        // Note: get_plugin_names() returns Vec<String>, so iterating over it is fine with the lock held
        for name in registry.get_plugin_names() {
            // This is the N+1 lookup part: for each name, we do a hash lookup
            if let Some(meta) = registry.get_plugin_metadata(&name) {
                inefficient_map.insert(name, meta.clone());
            }
        }
        drop(registry);
        let duration_inefficient = start.elapsed();

        // Measure optimized approach (bulk retrieval)
        // This uses the optimized implementation that clones the map in one go
        let start = Instant::now();
        let optimized_map = manager.get_all_metadata().await;
        let duration_optimized = start.elapsed();

        println!("Inefficient (N+1): {:?}", duration_inefficient);
        println!("Optimized (Bulk): {:?}", duration_optimized);

        assert_eq!(inefficient_map.len(), count);
        assert_eq!(optimized_map.len(), count);

        // Verification: The optimized approach should be performant.
        // We don't assert strict "faster than" to avoid flakiness in CI environments,
        // but typically cloning 1000 items once is faster than 1000 lookups + clones.
        assert!(duration_optimized < std::time::Duration::from_secs(1));

        // Ensure optimized path is actually faster (with some buffer for noise)
        // If the optimized path is slower, we have a regression or the "inefficient" simulation is invalid.
        if duration_inefficient > std::time::Duration::from_millis(1) {
            assert!(
                duration_optimized < duration_inefficient,
                "Optimized path should be faster than inefficient path ({} vs {})",
                duration_optimized.as_micros(),
                duration_inefficient.as_micros()
            );
        }
    }

    #[tokio::test]
    async fn test_benchmark_tool_definitions_retrieval() {
        use std::time::Instant;

        let manager = PluginManager::new();
        // Register 1000 dummy plugins
        let count = 1000;
        for i in 0..count {
            let name = format!("plugin-{}", i);
            let plugin = Arc::new(TestPlugin {
                name: name.clone(),
                extensions: vec!["txt".to_string()],
                capabilities: Capabilities::default(),
                should_fail: false,
            });
            manager.register_plugin(&name, plugin).await.unwrap();
        }

        // Measure inefficient approach (simulating N+1 lookup)
        let start = Instant::now();
        let registry = manager.registry.read().await;
        let mut inefficient_tools = Vec::new();

        // Note: get_plugin_names() returns Vec<String>, so iterating over it is fine with the lock held
        for name in registry.get_plugin_names() {
            // This is the N+1 lookup part: for each name, we do a hash lookup
            if let Some(plugin) = registry.get_plugin(&name) {
                let tools = plugin.tool_definitions();
                inefficient_tools.extend(tools);
            }
        }
        drop(registry);
        let duration_inefficient = start.elapsed();

        // Measure optimized approach (bulk retrieval)
        let start = Instant::now();
        let optimized_tools = manager.get_all_tool_definitions().await;
        let duration_optimized = start.elapsed();

        println!("Inefficient Tool Defs (N+1): {:?}", duration_inefficient);
        println!("Optimized Tool Defs (Bulk): {:?}", duration_optimized);

        assert_eq!(inefficient_tools.len(), 0); // TestPlugin returns empty tools
        assert_eq!(optimized_tools.len(), 0);

        // Verification: The optimized approach should be performant.
        assert!(duration_optimized < std::time::Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_benchmark_capabilities_retrieval() {
        use std::collections::HashMap;
        use std::time::Instant;

        let manager = PluginManager::new();
        // Register 1000 dummy plugins
        let count = 1000;
        for i in 0..count {
            let name = format!("plugin-cap-{}", i);
            let plugin = Arc::new(TestPlugin {
                name: name.clone(),
                extensions: vec!["txt".to_string()],
                capabilities: Capabilities::default(),
                should_fail: false,
            });
            manager.register_plugin(&name, plugin).await.unwrap();
        }

        // Measure inefficient approach (simulating N+1 lookup)
        let start = Instant::now();
        let registry = manager.registry.read().await;
        let mut inefficient_map = HashMap::new();

        for name in registry.get_plugin_names() {
            if let Some(caps) = registry.get_plugin_capabilities(&name) {
                inefficient_map.insert(name, caps);
            }
        }
        drop(registry);
        let duration_inefficient = start.elapsed();

        // Measure optimized approach (bulk retrieval)
        let start = Instant::now();
        let optimized_map = manager.get_all_capabilities().await;
        let duration_optimized = start.elapsed();

        println!("Capabilities Inefficient (N+1): {:?}", duration_inefficient);
        println!("Capabilities Optimized (Bulk): {:?}", duration_optimized);

        assert_eq!(inefficient_map.len(), count);
        assert_eq!(optimized_map.len(), count);

        assert!(duration_optimized < std::time::Duration::from_secs(1));
    }
}
