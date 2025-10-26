//! Helper macros for reducing boilerplate in language plugin implementations
//!
//! This module provides three complementary macros for language plugin development:
//!
//! 1. **`define_language_plugin!`** - Plugin scaffolding generator (struct, constants, registration)
//! 2. **`impl_language_plugin_basics!`** - Standard method delegation (metadata, capabilities, as_any)
//! 3. **`impl_capability_delegations!`** - Capability trait delegation (imports, workspace, etc.)
//!
//! These macros work together to eliminate ~70 lines of boilerplate per plugin.
//!
//! # Organization Note
//!
//! All three macros are defined in this single file rather than split into submodules because:
//! - They're always used together (no plugin uses only one)
//! - Declarative macros have zero compile-time cost when not invoked
//! - Single file is easier to maintain and review
//!
//! # Performance
//!
//! Declarative macros (`macro_rules!`) are expanded at call sites only, so having all three
//! in one module doesn't add overhead for consumers that only need some of them.

// ============================================================================
// 1. Plugin Scaffolding Generator
// ============================================================================

/// Comprehensive macro to define a complete language plugin with all scaffolding.
///
/// This macro generates:
/// - Plugin struct definition with fields
/// - METADATA constant
/// - CAPABILITIES constant
/// - new() factory method
/// - mill_plugin! registration block
///
/// # Example
///
/// ```rust,ignore
/// use mill_lang_common::define_language_plugin;
///
/// define_language_plugin! {
///     struct: PythonPlugin,
///     name: "python",
///     extensions: ["py"],
///     manifest: "pyproject.toml",
///     lsp_command: "pylsp",
///     lsp_args: ["pylsp"],
///     source_dir: ".",
///     entry_point: "__init__.py",
///     module_separator: ".",
///     capabilities: [imports, workspace, project_factory],
///     fields: {
///         import_support: import_support::PythonImportSupport,
///         workspace_support: workspace_support::PythonWorkspaceSupport,
///         project_factory: project_factory::PythonProjectFactory,
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_language_plugin {
    (
        struct: $struct_name:ident,
        name: $name:expr,
        extensions: [$($ext:expr),+ $(,)?],
        manifest: $manifest:expr,
        lsp_command: $lsp_cmd:expr,
        lsp_args: [$($lsp_arg:expr),+ $(,)?],
        source_dir: $source_dir:expr,
        entry_point: $entry_point:expr,
        module_separator: $module_sep:expr,
        capabilities: [$($cap:ident),+ $(,)?],
        fields: {
            $($field_name:ident: $field_type:ty),+ $(,)?
        }
        $(, doc: $doc:expr)?
    ) => {
        // Generate mill_plugin! registration
        mill_plugin_api::mill_plugin! {
            name: $name,
            extensions: [$($ext),+],
            manifest: $manifest,
            capabilities: $struct_name::CAPABILITIES,
            factory: $struct_name::new,
            lsp: Some(mill_plugin_api::LspConfig::new($lsp_cmd, &[$($lsp_arg),+]))
        }

        $(#[doc = $doc])?
        #[derive(Default)]
        pub struct $struct_name {
            $(pub(crate) $field_name: $field_type),+
        }

        impl $struct_name {
            /// Static metadata for this language.
            pub const METADATA: mill_plugin_api::LanguageMetadata = mill_plugin_api::LanguageMetadata {
                name: $name,
                extensions: &[$($ext),+],
                manifest_filename: $manifest,
                source_dir: $source_dir,
                entry_point: $entry_point,
                module_separator: $module_sep,
            };

            /// The capabilities of this plugin.
            pub const CAPABILITIES: mill_plugin_api::PluginCapabilities = mill_plugin_api::PluginCapabilities::none()
                $(.$cap())+;

            /// Creates a new, boxed instance of the plugin.
            #[allow(clippy::new_ret_no_self)]
            pub fn new() -> Box<dyn mill_plugin_api::LanguagePlugin> {
                Box::new(Self::default())
            }
        }
    };
}

// ============================================================================
// 2. Capability Trait Delegation
// ============================================================================

/// Macro to generate capability delegation methods for LanguagePlugin implementations.
///
/// This macro eliminates the repetitive boilerplate of delegating capability trait methods
/// to struct fields. Each plugin has slightly different capabilities, so this macro allows
/// selective generation of only the methods needed.
///
/// # Example
///
/// ```rust,ignore
/// use mill_lang_common::impl_capability_delegations;
///
/// impl LanguagePlugin for MyPlugin {
///     // ... core methods like parse(), metadata(), etc. ...
///
///     impl_capability_delegations! {
///         import_support => {
///             import_parser,
///             import_rename_support,
///             import_move_support,
///             import_mutation_support,
///             import_advanced_support,
///         },
///         workspace_support => {
///             workspace_support,
///         },
///         project_factory => {
///             project_factory,
///         },
///         this => {
///             module_reference_scanner: ModuleReferenceScanner,
///             refactoring_provider: RefactoringProvider,
///             import_analyzer: ImportAnalyzer,
///             manifest_updater: ManifestUpdater,
///         },
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_capability_delegations {
    // Entry point - processes all delegation blocks
    (
        $($variant:tt)*
    ) => {
        $crate::__impl_capability_delegations_inner!($($variant)*);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __impl_capability_delegations_inner {
    // Pattern with 'this' delegation first (delegates to self, not a field)
    (
        this => {
            $($this_method:ident: $this_trait:ident),+ $(,)?
        } $(,
        $field:ident => {
            $($field_method:ident: $field_trait:ident),+ $(,)?
        })* $(,)?
    ) => {
        $(
            fn $this_method(&self) -> Option<&dyn mill_plugin_api::$this_trait> {
                Some(self)
            }
        )+
        $(
            $(
                fn $field_method(&self) -> Option<&dyn mill_plugin_api::$field_trait> {
                    Some(&self.$field)
                }
            )+
        )*
    };

    // Pattern: field => { method_name: TraitName, ... } (no 'this')
    (
        $($field:ident => {
            $($method:ident: $trait:ident),+ $(,)?
        }),+ $(,)?
    ) => {
        $(
            $(
                fn $method(&self) -> Option<&dyn mill_plugin_api::$trait> {
                    Some(&self.$field)
                }
            )+
        )+
    };
}

// ============================================================================
// 3. Standard LanguagePlugin Methods
// ============================================================================

/// Macro to generate standard LanguagePlugin boilerplate.
///
/// Generates the metadata() and capabilities() methods which are identical
/// across all plugins.
///
/// # Example
///
/// ```rust,ignore
/// use mill_lang_common::impl_language_plugin_basics;
///
/// impl LanguagePlugin for MyPlugin {
///     impl_language_plugin_basics!();
///
///     // ... rest of trait implementation ...
/// }
/// ```
#[macro_export]
macro_rules! impl_language_plugin_basics {
    () => {
        fn metadata(&self) -> &mill_plugin_api::LanguageMetadata {
            &Self::METADATA
        }

        fn capabilities(&self) -> mill_plugin_api::PluginCapabilities {
            Self::CAPABILITIES
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    };
}
