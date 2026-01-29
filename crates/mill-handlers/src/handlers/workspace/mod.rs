//! Workspace-level operations module
//!
//! Contains utilities for workspace-wide operations like find/replace.

pub mod case_preserving;
pub mod find_replace_handler;
pub mod literal_matcher;
pub mod regex_matcher;

pub use case_preserving::{
    apply_case_style, detect_case_style, replace_preserving_case, split_into_words, CaseStyle,
};
pub use find_replace_handler::handle_find_replace;
pub use literal_matcher::{find_literal_matches, Match};
pub use regex_matcher::{find_regex_matches, RegexError, RegexMatch};
