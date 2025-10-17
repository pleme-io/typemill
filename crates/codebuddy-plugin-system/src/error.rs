//! Plugin system error types

use thiserror::Error;

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin system error types
#[derive(Error, Debug, Clone)]
pub enum PluginError {
    /// Plugin not found for the given file or method
    #[error("No plugin found for file '{file}' or method '{method}'")]
    PluginNotFound { file: String, method: String },

    /// Plugin failed to handle request
    #[error("Plugin '{plugin}' failed to handle request: {message}")]
    PluginRequestFailed { plugin: String, message: String },

    /// Plugin configuration error
    #[error("Plugin configuration error: {message}")]
    ConfigurationError { message: String },

    /// Plugin initialization error
    #[error("Plugin '{plugin}' initialization failed: {message}")]
    InitializationError { plugin: String, message: String },

    /// Plugin version compatibility error
    #[error("Plugin '{plugin}' version '{version}' is incompatible with system version '{system_version}'")]
    VersionIncompatible {
        plugin: String,
        version: String,
        system_version: String,
    },

    /// Method not supported by plugin
    #[error("Method '{method}' not supported by plugin '{plugin}'")]
    MethodNotSupported { method: String, plugin: String },

    /// Multiple plugins claim support for the same file/method with equal priority
    #[error("Ambiguous plugin selection for method '{method}': plugins {plugins:?} all have priority {priority}")]
    AmbiguousPluginSelection {
        method: String,
        plugins: Vec<String>,
        priority: u32,
    },

    /// Plugin loading/unloading error
    #[error("Plugin lifecycle error: {message}")]
    LifecycleError { message: String },

    /// Serialization/deserialization error
    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    /// IO error (file access, etc.)
    #[error("IO error: {message}")]
    IoError { message: String },

    /// Generic plugin error
    #[error("Plugin error: {message}")]
    Generic { message: String },
}

impl PluginError {
    /// Create a plugin not found error
    pub fn plugin_not_found(file: impl Into<String>, method: impl Into<String>) -> Self {
        Self::PluginNotFound {
            file: file.into(),
            method: method.into(),
        }
    }

    /// Create a plugin request failed error
    pub fn request_failed(plugin: impl Into<String>, message: impl Into<String>) -> Self {
        Self::PluginRequestFailed {
            plugin: plugin.into(),
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create an initialization error
    pub fn initialization_error(plugin: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InitializationError {
            plugin: plugin.into(),
            message: message.into(),
        }
    }

    /// Create a version incompatible error
    pub fn version_incompatible(
        plugin: impl Into<String>,
        version: impl Into<String>,
        system_version: impl Into<String>,
    ) -> Self {
        Self::VersionIncompatible {
            plugin: plugin.into(),
            version: version.into(),
            system_version: system_version.into(),
        }
    }

    /// Create a method not supported error
    pub fn method_not_supported(method: impl Into<String>, plugin: impl Into<String>) -> Self {
        Self::MethodNotSupported {
            method: method.into(),
            plugin: plugin.into(),
        }
    }

    /// Create an ambiguous plugin selection error
    pub fn ambiguous_selection(
        method: impl Into<String>,
        plugins: Vec<String>,
        priority: u32,
    ) -> Self {
        Self::AmbiguousPluginSelection {
            method: method.into(),
            plugins,
            priority,
        }
    }

    /// Create a lifecycle error
    pub fn lifecycle_error(message: impl Into<String>) -> Self {
        Self::LifecycleError {
            message: message.into(),
        }
    }

    /// Create a serialization error
    pub fn serialization_error(message: impl Into<String>) -> Self {
        Self::SerializationError {
            message: message.into(),
        }
    }

    /// Create an IO error
    pub fn io_error(message: impl Into<String>) -> Self {
        Self::IoError {
            message: message.into(),
        }
    }

    /// Create a generic error
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(err: serde_json::Error) -> Self {
        Self::serialization_error(err.to_string())
    }
}

impl From<std::io::Error> for PluginError {
    fn from(err: std::io::Error) -> Self {
        Self::io_error(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_error_creation() {
        let error = PluginError::plugin_not_found("test.ts", "find_definition");
        assert!(matches!(error, PluginError::PluginNotFound { .. }));
        assert!(error.to_string().contains("test.ts"));
        assert!(error.to_string().contains("find_definition"));
    }

    #[test]
    fn test_error_conversion() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_error.is_err());

        let plugin_error: PluginError = json_error.unwrap_err().into();
        assert!(matches!(
            plugin_error,
            PluginError::SerializationError { .. }
        ));
    }

    #[test]
    fn test_ambiguous_selection_error() {
        let plugins = vec!["typescript".to_string(), "javascript".to_string()];
        let error = PluginError::ambiguous_selection("find_definition", plugins, 50);

        assert!(matches!(
            error,
            PluginError::AmbiguousPluginSelection { .. }
        ));
        assert!(error.to_string().contains("typescript"));
        assert!(error.to_string().contains("javascript"));
        assert!(error.to_string().contains("priority 50"));
    }
}
