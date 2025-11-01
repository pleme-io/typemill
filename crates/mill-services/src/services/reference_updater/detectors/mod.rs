//! Reference detectors for finding affected files
//!
//! Provides generic strategies for detecting which files are affected by a rename operation.
//! Language-specific detection is now handled via the plugin system (ReferenceDetector trait).

pub mod generic;

// Re-export key functions (crate-internal only)
pub(crate) use generic::{find_generic_affected_files, get_all_imported_files};
