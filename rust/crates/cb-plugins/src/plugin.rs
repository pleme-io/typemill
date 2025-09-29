//! Core plugin trait and metadata definitions

use crate::{Capabilities, PluginRequest, PluginResponse, PluginResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// Core trait that all language plugins must implement
#[async_trait]
pub trait LanguagePlugin: Send + Sync {
    /// Plugin metadata for identification and versioning
    fn metadata(&self) -> PluginMetadata;

    /// File extensions this plugin supports (e.g., ["ts", "tsx", "js"])
    fn supported_extensions(&self) -> Vec<String>;

    /// Capabilities this plugin provides
    fn capabilities(&self) -> Capabilities;

    /// Handle a code intelligence request
    async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse>;

    /// Configure the plugin with language-specific settings
    fn configure(&self, config: Value) -> PluginResult<()>;

    /// Lifecycle hook: called when a file is opened
    fn on_file_open(&self, _path: &Path) -> PluginResult<()> {
        Ok(())
    }

    /// Lifecycle hook: called when a file is saved
    fn on_file_save(&self, _path: &Path) -> PluginResult<()> {
        Ok(())
    }

    /// Lifecycle hook: called when a file is closed
    fn on_file_close(&self, _path: &Path) -> PluginResult<()> {
        Ok(())
    }

    /// Check if plugin can handle a specific file
    fn can_handle_file(&self, file_path: &Path) -> bool {
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            self.supported_extensions().contains(&extension.to_string())
        } else {
            false
        }
    }

    /// Tool definitions this plugin provides
    fn tool_definitions(&self) -> Vec<Value>;

    /// Initialize the plugin with necessary services
    async fn initialize(&mut self) -> PluginResult<()> {
        Ok(())
    }

    /// Cleanup resources when plugin is unloaded
    async fn shutdown(&mut self) -> PluginResult<()> {
        Ok(())
    }
}

/// Plugin metadata for identification and compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Human-readable plugin name
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Plugin author/organization
    pub author: String,
    /// Plugin description
    pub description: String,
    /// Minimum plugin system version required
    pub min_system_version: String,
    /// Plugin-specific configuration schema (JSON Schema)
    pub config_schema: Option<Value>,
}

impl PluginMetadata {
    /// Create new plugin metadata
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            author: author.into(),
            description: String::new(),
            min_system_version: crate::PLUGIN_SYSTEM_VERSION.to_string(),
            config_schema: None,
        }
    }

    /// Set plugin description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set minimum system version requirement
    pub fn with_min_system_version(mut self, version: impl Into<String>) -> Self {
        self.min_system_version = version.into();
        self
    }

    /// Set configuration schema
    pub fn with_config_schema(mut self, schema: Value) -> Self {
        self.config_schema = Some(schema);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TestPlugin;

    #[async_trait]
    impl LanguagePlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("test", "1.0.0", "test author")
        }

        fn supported_extensions(&self) -> Vec<String> {
            vec!["test".to_string()]
        }

        fn capabilities(&self) -> Capabilities {
            Capabilities::default()
        }

        async fn handle_request(&self, _request: PluginRequest) -> PluginResult<PluginResponse> {
            Ok(PluginResponse::empty())
        }

        fn configure(&self, _config: Value) -> PluginResult<()> {
            Ok(())
        }

        fn tool_definitions(&self) -> Vec<Value> {
            vec![]
        }
    }

    #[test]
    fn test_can_handle_file() {
        let plugin = TestPlugin;
        assert!(plugin.can_handle_file(&PathBuf::from("file.test")));
        assert!(!plugin.can_handle_file(&PathBuf::from("file.other")));
        assert!(!plugin.can_handle_file(&PathBuf::from("no_extension")));
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = PluginMetadata::new("test", "1.0.0", "author")
            .with_description("Test plugin")
            .with_min_system_version("0.1.0");

        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.author, "author");
        assert_eq!(metadata.description, "Test plugin");
        assert_eq!(metadata.min_system_version, "0.1.0");
    }
}
