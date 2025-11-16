//! Common utilities shared across refactoring handlers
//!
//! This module provides shared functionality used by rename, move, and other
//! refactoring operations to avoid code duplication.

pub mod checksums;

pub(crate) use checksums::{
    calculate_checksums_for_directory_rename, calculate_checksums_for_edits,
};
