//! Language metadata definitions
//!
//! Provides static metadata for language plugins.

/// Static metadata about a programming language.
///
/// This struct consolidates all language-specific constants in one place.
/// This metadata is provided by a `LanguagePlugin` implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageMetadata {
    /// Human-readable language name (e.g., "Rust", "TypeScript")
    pub name: &'static str,

    /// File extensions this language handles (e.g., ["rs"], ["ts", "tsx"])
    pub extensions: &'static [&'static str],

    /// Primary manifest filename (e.g., "Cargo.toml", "package.json")
    pub manifest_filename: &'static str,

    /// Default source directory (e.g., "src", "lib")
    pub source_dir: &'static str,

    /// Entry point filename (e.g., "lib.rs", "index.ts")
    pub entry_point: &'static str,

    /// Module path separator (e.g., "::" for Rust, "." for TypeScript)
    pub module_separator: &'static str,
}