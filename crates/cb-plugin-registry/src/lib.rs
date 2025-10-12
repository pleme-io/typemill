use cb_plugin_api::{LanguagePlugin, LspConfig, PluginCapabilities};

// Re-export inventory for the macro.
pub use inventory;

/// Describes a language plugin to the core system.
///
/// This struct is created by the `codebuddy_plugin!` macro and collected
/// at link-time by the `inventory` crate.
pub struct PluginDescriptor {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub manifest_filename: &'static str,
    pub capabilities: PluginCapabilities,
    pub factory: fn() -> Box<dyn LanguagePlugin>,
    pub lsp: Option<LspConfig>,
}

// Collect all plugin descriptors into a static collection.
inventory::collect!(PluginDescriptor);

/// Returns an iterator over all registered language plugins.
pub fn iter_plugins() -> impl Iterator<Item = &'static PluginDescriptor> {
    inventory::iter::<PluginDescriptor>.into_iter()
}

/// A macro for language plugins to register themselves.
///
/// This macro creates and submits a `PluginDescriptor` to the `inventory`
/// system, making it discoverable at runtime.
#[macro_export]
macro_rules! codebuddy_plugin {
    (
        name: $name:expr,
        extensions: $extensions:expr,
        manifest: $manifest:expr,
        capabilities: $capabilities:expr,
        factory: $factory:expr,
        lsp: $lsp:expr
    ) => {
        $crate::inventory::submit! {
            $crate::PluginDescriptor {
                name: $name,
                extensions: &$extensions,
                manifest_filename: $manifest,
                capabilities: $capabilities,
                factory: $factory,
                lsp: $lsp,
            }
        }
    };
}
