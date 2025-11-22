#![allow(clippy::new_without_default)]

pub mod handlers;
pub mod language_plugin_registry;
pub mod utils;

// Re-export for convenience
pub use language_plugin_registry::LanguagePluginRegistry;

/// Serde helper for fields that default to `true`.
/// Use with `#[serde(default = "crate::default_true")]`
pub fn default_true() -> bool {
    true
}
