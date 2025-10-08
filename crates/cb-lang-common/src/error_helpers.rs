//! Error construction utilities and helpers
//!
//! This module provides ergonomic builders and macros for creating `PluginError` instances
//! with rich context. Reduces boilerplate from repetitive error construction patterns.

use cb_plugin_api::PluginError;
use std::collections::HashMap;
use std::path::Path;

/// Builder for constructing plugin errors with context
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::error_helpers::ErrorBuilder;
///
/// let error = ErrorBuilder::parse("Invalid syntax")
///     .with_path(&file_path)
///     .with_line(42)
///     .build();
/// ```
pub struct ErrorBuilder {
    kind: ErrorKind,
    message: String,
    context: HashMap<String, String>,
}

/// Error kind classification
#[derive(Debug, Clone, Copy)]
enum ErrorKind {
    Parse,
    Manifest,
    Internal,
}

impl ErrorBuilder {
    /// Create a parse error builder
    pub fn parse(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Parse,
            message: msg.into(),
            context: HashMap::new(),
        }
    }

    /// Create a manifest error builder
    pub fn manifest(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Manifest,
            message: msg.into(),
            context: HashMap::new(),
        }
    }

    /// Create an internal error builder
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Internal,
            message: msg.into(),
            context: HashMap::new(),
        }
    }

    /// Add file path context
    pub fn with_path(mut self, path: &Path) -> Self {
        self.context
            .insert("path".to_string(), path.display().to_string());
        self
    }

    /// Add line number context
    pub fn with_line(mut self, line: u32) -> Self {
        self.context.insert("line".to_string(), line.to_string());
        self
    }

    /// Add column number context
    pub fn with_column(mut self, column: u32) -> Self {
        self.context
            .insert("column".to_string(), column.to_string());
        self
    }

    /// Add source snippet context
    pub fn with_source_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.context.insert("source".to_string(), snippet.into());
        self
    }

    /// Add custom context key-value
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Get formatted context as string
    ///
    /// Useful for logging or debugging before building the error
    ///
    /// # Example
    ///
    /// ```rust
    /// use cb_lang_common::error_helpers::ErrorBuilder;
    ///
    /// let builder = ErrorBuilder::parse("Invalid syntax")
    ///     .with_line(42)
    ///     .with_column(10);
    ///
    /// let context = builder.format_context();
    /// // HashMap iteration order is not guaranteed, check both keys are present
    /// assert!(context.contains("line=42"));
    /// assert!(context.contains("column=10"));
    /// ```
    pub fn format_context(&self) -> String {
        if self.context.is_empty() {
            String::new()
        } else {
            let context_parts: Vec<String> = self
                .context
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            context_parts.join(", ")
        }
    }

    /// Build the final PluginError
    pub fn build(self) -> PluginError {
        let mut final_message = self.message;

        // Append context if any
        if !self.context.is_empty() {
            final_message.push_str(" (");
            let mut first = true;
            for (key, value) in self.context.iter() {
                if !first {
                    final_message.push_str(", ");
                }
                final_message.push_str(&format!("{}: {}", key, value));
                first = false;
            }
            final_message.push(')');
        }

        match self.kind {
            ErrorKind::Parse => PluginError::parse(final_message),
            ErrorKind::Manifest => PluginError::manifest(final_message),
            ErrorKind::Internal => PluginError::internal(final_message),
        }
    }
}

/// Convenience macro for creating parse errors with formatting
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::parse_error;
///
/// return Err(parse_error!("Expected identifier, found {}", token));
/// ```
#[macro_export]
macro_rules! parse_error {
    ($($arg:tt)*) => {
        cb_plugin_api::PluginError::parse(format!($($arg)*))
    };
}

/// Convenience macro for creating manifest errors with formatting
#[macro_export]
macro_rules! manifest_error {
    ($($arg:tt)*) => {
        cb_plugin_api::PluginError::manifest(format!($($arg)*))
    };
}

/// Convenience macro for creating internal errors with formatting
#[macro_export]
macro_rules! internal_error {
    ($($arg:tt)*) => {
        cb_plugin_api::PluginError::internal(format!($($arg)*))
    };
}

/// Helper function for I/O errors in manifest operations
pub fn io_to_manifest_error(error: std::io::Error, path: &Path) -> PluginError {
    ErrorBuilder::manifest(format!("Failed to read file: {}", error))
        .with_path(path)
        .build()
}

/// Helper function for I/O errors in parsing operations
pub fn io_to_parse_error(error: std::io::Error, context: &str) -> PluginError {
    ErrorBuilder::parse(format!("I/O error: {}", error))
        .with_context("context", context)
        .build()
}

/// Helper function for JSON deserialization errors
pub fn json_error(error: serde_json::Error, context: &str) -> PluginError {
    ErrorBuilder::parse(format!("Failed to parse JSON: {}", error))
        .with_context("context", context)
        .build()
}

/// Helper function for TOML deserialization errors
pub fn toml_error(error: impl std::fmt::Display, context: &str) -> PluginError {
    ErrorBuilder::manifest(format!("Failed to parse TOML: {}", error))
        .with_context("context", context)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_error_builder_parse() {
        let error = ErrorBuilder::parse("Syntax error")
            .with_line(42)
            .with_column(10)
            .build();

        let msg = format!("{:?}", error);
        assert!(msg.contains("Syntax error"));
        assert!(msg.contains("line: 42"));
        assert!(msg.contains("column: 10"));
    }

    #[test]
    fn test_error_builder_manifest() {
        let path = PathBuf::from("/tmp/test.toml");
        let error = ErrorBuilder::manifest("Missing field")
            .with_path(&path)
            .build();

        let msg = format!("{:?}", error);
        assert!(msg.contains("Missing field"));
        assert!(msg.contains("path:"));
    }

    #[test]
    fn test_error_builder_internal() {
        let error = ErrorBuilder::internal("Unexpected state")
            .with_context("state", "invalid")
            .build();

        let msg = format!("{:?}", error);
        assert!(msg.contains("Unexpected state"));
        assert!(msg.contains("state: invalid"));
    }

    #[test]
    fn test_parse_error_macro() {
        let token = "EOF";
        let error = parse_error!("Expected identifier, found {}", token);
        let msg = format!("{:?}", error);
        assert!(msg.contains("Expected identifier, found EOF"));
    }
}
