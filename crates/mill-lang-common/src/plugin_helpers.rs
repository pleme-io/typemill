//! Helper macros for reducing boilerplate in language plugin implementations

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
