//! Language metadata definitions
//!
//! Provides static metadata for language plugins, eliminating the need for
//! multiple trait methods returning fixed values.

use crate::ProjectLanguage;

/// Static metadata about a programming language
///
/// This struct consolidates all language-specific constants in one place.
/// Each language plugin provides a pre-defined constant (e.g., LanguageMetadata::RUST).
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

    /// Language enum variant
    pub language: ProjectLanguage,
}

impl LanguageMetadata {
    /// Rust language metadata constant
    pub const RUST: Self = Self {
        name: "Rust",
        extensions: &["rs"],
        manifest_filename: "Cargo.toml",
        source_dir: "src",
        entry_point: "lib.rs",
        module_separator: "::",
        language: ProjectLanguage::Rust,
    };

    /// TypeScript language metadata constant
    pub const TYPESCRIPT: Self = Self {
        name: "TypeScript",
        extensions: &["ts", "tsx", "js", "jsx"],
        manifest_filename: "package.json",
        source_dir: "src",
        entry_point: "index.ts",
        module_separator: ".",
        language: ProjectLanguage::TypeScript,
    };

    /// Go language metadata constant
    pub const GO: Self = Self {
        name: "Go",
        extensions: &["go"],
        manifest_filename: "go.mod",
        source_dir: ".",
        entry_point: "main.go",
        module_separator: ".",
        language: ProjectLanguage::Go,
    };

    /// Python language metadata constant
    pub const PYTHON: Self = Self {
        name: "Python",
        extensions: &["py"],
        manifest_filename: "pyproject.toml",
        source_dir: ".",
        entry_point: "__init__.py",
        module_separator: ".",
        language: ProjectLanguage::Python,
    };

    /// Java language metadata constant
    pub const JAVA: Self = Self {
        name: "Java",
        extensions: &["java"],
        manifest_filename: "pom.xml",
        source_dir: "src/main/java",
        entry_point: "",  // No single entry point
        module_separator: ".",
        language: ProjectLanguage::Java,
    };
}
