//! Utility functions for source_crate
//!
//! This module provides helper utilities that will be consolidated
//! into the target crate. The file header comments should be preserved
//! during consolidation (Bug #6 regression test).

/// Format a greeting message with custom text
pub fn format_greeting(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// Utility constant
pub const MAX_RETRIES: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_greeting() {
        assert_eq!(format_greeting("World"), "Hello, World!");
    }
}
