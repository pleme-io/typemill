/// Target crate - will receive consolidated code from source_crate
pub fn target_function() -> &'static str {
    "I'm in the target crate"
}

// After consolidation, this file should have:
// pub mod source;
