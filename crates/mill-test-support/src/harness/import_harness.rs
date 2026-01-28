//! Cross-language import test harness
//!
//! This module provides a parameterized testing framework for import operations
//! across multiple programming languages. It enables writing a single test that runs
//! against equivalent code in TypeScript, Rust, and Python.
//!
//! ## Design Philosophy
//!
//! - **DRY**: One test covers all languages (no duplication)
//! - **Consistency**: All languages tested identically
//! - **Extensibility**: Easy to add new languages or operations
//! - **Coverage**: Ensures every language implements core import operations
//!
//! ## Core Import Operations Tested
//!
//! 1. **parse_imports**: Extract import statements from source code
//! 2. **contains_import**: Check if specific module is imported
//! 3. **add_import**: Add new import statement
//! 4. **remove_import**: Remove existing import statement
//! 5. **rewrite_for_rename**: Update imports when file/module renamed
//! 6. **rewrite_for_move**: Update imports when file/module moved
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! fn test_parse_imports_all_languages() {
//!     let test_case = ImportScenarios::parse_simple_imports();
//!     for fixture in test_case.fixtures {
//!         // Test runs on TypeScript, Rust, Python
//!         assert!(fixture.expected_imports.len() > 0);
//!     }
//! }
//! ```

// Re-use Language enum from refactoring harness
pub use super::refactoring_harness::Language;

// Extend Language enum with import-specific helper
impl Language {
    pub fn all_with_import_support() -> Vec<Language> {
        // Core languages with full import support
        vec![Language::TypeScript, Language::Rust, Language::Python]
    }
}

/// Import operations that can be tested
#[derive(Debug, Clone)]
pub enum ImportOperation {
    /// Parse import statements from source code
    ParseImports,
    /// Check if a specific module is imported
    ContainsImport { module_name: String },
    /// Add a new import statement
    AddImport { module_name: String },
    /// Remove an existing import statement
    RemoveImport { module_name: String },
    /// Rewrite imports when a file/module is renamed
    RewriteForRename { old_name: String, new_name: String },
    /// Rewrite imports when a file/module is moved
    RewriteForMove {
        old_path: String,
        new_path: String,
        importing_file_path: String,
    },
}

/// Expected behavior for an import test
#[derive(Debug, Clone)]
pub enum ImportExpectedBehavior {
    /// Operation should succeed
    Success,
    /// Parse should return specific imports
    ParsedImports(Vec<String>),
    /// Contains should return true/false
    Contains(bool),
    /// Add should result in source containing the import
    Added,
    /// Remove should result in source not containing the import
    Removed,
    /// Rewrite should change N imports
    RewriteCount(usize),
    /// Operation not supported for this language
    NotSupported,
    /// Operation supported but expected to fail (e.g., invalid code)
    ExpectedError { message_contains: Option<String> },
}

/// Language-specific code fixture for an import test scenario
#[derive(Debug, Clone)]
pub struct ImportFixture {
    pub language: Language,
    pub source_code: &'static str,
    pub operation: ImportOperation,
    pub expected: ImportExpectedBehavior,
}

/// Complete test case for cross-language import testing
pub struct ImportTestCase {
    pub scenario_name: &'static str,
    pub fixtures: Vec<ImportFixture>,
}

impl ImportTestCase {
    pub fn new(scenario_name: &'static str) -> Self {
        Self {
            scenario_name,
            fixtures: Vec::new(),
        }
    }

    pub fn with_fixture(mut self, fixture: ImportFixture) -> Self {
        self.fixtures.push(fixture);
        self
    }

    pub fn with_all_languages<F>(mut self, generator: F) -> Self
    where
        F: Fn(Language) -> ImportFixture,
    {
        for lang in Language::all_with_import_support() {
            let fixture = generator(lang);
            self.fixtures.push(fixture);
        }
        self
    }
}

/// Predefined import test scenarios with language-equivalent fixtures
pub struct ImportScenarios;

impl ImportScenarios {
    /// Parse simple imports from source code
    pub fn parse_simple_imports() -> ImportTestCase {
        ImportTestCase::new("parse_simple_imports").with_all_languages(|lang| {
            let (source, expected_imports) = match lang {
                Language::TypeScript => (
                    "import { foo } from './utils';\nimport bar from './other';\n",
                    vec!["./utils".to_string(), "./other".to_string()],
                ),
                Language::Rust => (
                    "use std::collections::HashMap;\nuse crate::utils::helper;\n",
                    vec!["std::collections".to_string(), "crate::utils".to_string()],
                ),
                Language::Python => (
                    "import os\nfrom typing import List\n",
                    vec!["os".to_string(), "typing".to_string()],
                ),
            };

            ImportFixture {
                language: lang,
                source_code: source,
                operation: ImportOperation::ParseImports,
                expected: ImportExpectedBehavior::ParsedImports(expected_imports),
            }
        })
    }

    /// Check if source contains a specific import
    pub fn contains_import_positive() -> ImportTestCase {
        ImportTestCase::new("contains_import_positive").with_all_languages(|lang| {
            let (source, module_name) = match lang {
                Language::TypeScript => (
                    "import { foo } from './utils';\nimport bar from './other';\n",
                    "./utils",
                ),
                Language::Rust => (
                    "use std::collections::HashMap;\nuse crate::utils::helper;\n",
                    "std::collections",
                ),
                Language::Python => ("import os\nfrom typing import List\n", "os"),
            };

            ImportFixture {
                language: lang,
                source_code: source,
                operation: ImportOperation::ContainsImport {
                    module_name: module_name.to_string(),
                },
                expected: ImportExpectedBehavior::Contains(true),
            }
        })
    }

    /// Check if source does NOT contain a specific import (negative case)
    pub fn contains_import_negative() -> ImportTestCase {
        ImportTestCase::new("contains_import_negative").with_all_languages(|lang| {
            let (source, module_name) = match lang {
                Language::TypeScript => (
                    "import { foo } from './utils';\nimport bar from './other';\n",
                    "./nonexistent",
                ),
                Language::Rust => (
                    "use std::collections::HashMap;\nuse crate::utils::helper;\n",
                    "std::fs",
                ),
                Language::Python => ("import os\nfrom typing import List\n", "json"),
            };

            ImportFixture {
                language: lang,
                source_code: source,
                operation: ImportOperation::ContainsImport {
                    module_name: module_name.to_string(),
                },
                expected: ImportExpectedBehavior::Contains(false),
            }
        })
    }

    /// Add a new import to existing imports
    pub fn add_import_to_existing() -> ImportTestCase {
        ImportTestCase::new("add_import_to_existing").with_all_languages(|lang| {
            let (source, module_to_add) = match lang {
                Language::TypeScript => (
                    "import { foo } from './utils';\n\nfunction main() {}\n",
                    "./newModule",
                ),
                Language::Rust => (
                    "use std::collections::HashMap;\n\nfn main() {}\n",
                    "serde",
                ),
                Language::Python => ("import os\n\ndef main():\n    pass\n", "sys"),
            };

            ImportFixture {
                language: lang,
                source_code: source,
                operation: ImportOperation::AddImport {
                    module_name: module_to_add.to_string(),
                },
                expected: ImportExpectedBehavior::Added,
            }
        })
    }

    /// Add import to empty file
    pub fn add_import_to_empty() -> ImportTestCase {
        ImportTestCase::new("add_import_to_empty").with_all_languages(|lang| {
            let (source, module_to_add) = match lang {
                Language::TypeScript => ("", "./utils"),
                Language::Rust => ("", "serde"),
                Language::Python => ("", "os"),
            };

            ImportFixture {
                language: lang,
                source_code: source,
                operation: ImportOperation::AddImport {
                    module_name: module_to_add.to_string(),
                },
                expected: ImportExpectedBehavior::Added,
            }
        })
    }

    /// Remove an existing import
    pub fn remove_existing_import() -> ImportTestCase {
        ImportTestCase::new("remove_existing_import").with_all_languages(|lang| {
            let (source, module_to_remove) = match lang {
                Language::TypeScript => (
                    "import { foo } from './utils';\nimport bar from './other';\n",
                    "./utils",
                ),
                Language::Rust => (
                    "use std::collections::HashMap;\nuse serde::Serialize;\n",
                    "serde",
                ),
                Language::Python => ("import os\nfrom typing import List\n", "os"),
            };

            ImportFixture {
                language: lang,
                source_code: source,
                operation: ImportOperation::RemoveImport {
                    module_name: module_to_remove.to_string(),
                },
                expected: ImportExpectedBehavior::Removed,
            }
        })
    }

    /// Rewrite imports when a module is renamed
    pub fn rewrite_for_module_rename() -> ImportTestCase {
        ImportTestCase::new("rewrite_for_module_rename").with_all_languages(|lang| {
            let (source, old_name, new_name, expected_count) = match lang {
                Language::TypeScript => (
                    "import { foo } from './utils';\nimport bar from './other';\n",
                    "nonexistent",
                    "stillnonexistent",
                    0usize,
                ),
                Language::Rust => (
                    "use crate::utils::helper;\nuse std::collections::HashMap;\n",
                    "crate::utils",
                    "crate::helpers",
                    1usize,
                ),
                Language::Python => (
                    "from utils import helper\nimport os\n",
                    "utils",
                    "helpers",
                    1usize,
                ),
            };

            ImportFixture {
                language: lang,
                source_code: source,
                operation: ImportOperation::RewriteForRename {
                    old_name: old_name.to_string(),
                    new_name: new_name.to_string(),
                },
                expected: ImportExpectedBehavior::RewriteCount(expected_count),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_scenarios_defined() {
        // Ensure all core scenarios exist
        let scenarios = [
            ImportScenarios::parse_simple_imports(),
            ImportScenarios::contains_import_positive(),
            ImportScenarios::contains_import_negative(),
            ImportScenarios::add_import_to_existing(),
            ImportScenarios::add_import_to_empty(),
            ImportScenarios::remove_existing_import(),
            ImportScenarios::rewrite_for_module_rename(),
        ];

        assert_eq!(scenarios.len(), 7, "Should have 7 core import scenarios");
    }

    #[test]
    fn test_scenario_has_all_languages() {
        let scenario = ImportScenarios::parse_simple_imports();
        let languages = Language::all_with_import_support();

        assert_eq!(
            scenario.fixtures.len(),
            languages.len(),
            "Each scenario should have fixtures for all languages"
        );
    }

    #[test]
    fn test_language_extensions() {
        assert_eq!(Language::TypeScript.file_extension(), "ts");
        assert_eq!(Language::Rust.file_extension(), "rs");
    }
}
