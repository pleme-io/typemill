// Force linker to include language plugins for inventory collection
// These extern crate declarations ensure the plugins are linked into any
// binary that depends on cb-services, allowing inventory to discover them
extern crate cb_lang_markdown;
extern crate cb_lang_rust;
extern crate cb_lang_typescript;

pub mod services;

// Re-export commonly used types at crate root for convenience
pub use services::{
    ChecksumValidator, DryRunGenerator, DryRunResult, PlanConverter, PostApplyValidator,
    ValidationConfig, ValidationResult,
};
