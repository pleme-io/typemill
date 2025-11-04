//! Integration tests for cross-language import harness
//!
//! These tests verify that the import harness scenarios work with actual language plugins.
//! Each test runs against TypeScript, Rust, and Python plugins to ensure consistent behavior.

use mill_test_support::harness::{
    get_test_registry, ImportExpectedBehavior, ImportOperation, ImportScenarios,
};

// Force linker to include plugin-bundle for inventory collection in tests
extern crate mill_plugin_bundle;

// Force linker to include each language plugin crate
// This is required for inventory system - without these declarations,
// the linker performs dead code elimination and plugins aren't discovered
// Note: These are unconditional because mill-plugin-bundle enables these features
extern crate mill_lang_c;
extern crate mill_lang_cpp;
extern crate mill_lang_go;
extern crate mill_lang_java;
extern crate mill_lang_python;
extern crate mill_lang_rust;
extern crate mill_lang_swift;
extern crate mill_lang_typescript;

#[cfg(test)]
mod import_harness_tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_imports_all_languages() {
        let registry = get_test_registry();
        let scenario = ImportScenarios::parse_simple_imports();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .expect(&format!("Plugin not found for {:?}", fixture.language));

            let parser = plugin
                .import_parser()
                .expect(&format!("{:?} should have import parser", fixture.language));

            let imports = parser.parse_imports(fixture.source_code);

            match &fixture.expected {
                ImportExpectedBehavior::ParsedImports(expected) => {
                    assert_eq!(
                        imports, *expected,
                        "parse_imports failed for {:?}\nSource: {}\nExpected: {:?}\nGot: {:?}",
                        fixture.language, fixture.source_code, expected, imports
                    );
                }
                _ => panic!("Wrong expected behavior for parse_imports test"),
            }
        }
    }

    #[tokio::test]
    async fn test_contains_import_positive_all_languages() {
        let registry = get_test_registry();
        let scenario = ImportScenarios::contains_import_positive();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .expect(&format!("Plugin not found for {:?}", fixture.language));

            let parser = plugin
                .import_parser()
                .expect(&format!("{:?} should have import parser", fixture.language));

            if let ImportOperation::ContainsImport { module_name } = &fixture.operation {
                let contains = parser.contains_import(fixture.source_code, module_name);

                match &fixture.expected {
                    ImportExpectedBehavior::Contains(expected) => {
                        assert_eq!(
                            contains, *expected,
                            "contains_import failed for {:?}\nSource: {}\nModule: {}\nExpected: {}\nGot: {}",
                            fixture.language, fixture.source_code, module_name, expected, contains
                        );
                    }
                    _ => panic!("Wrong expected behavior for contains_import test"),
                }
            } else {
                panic!("Wrong operation for contains_import test");
            }
        }
    }

    #[tokio::test]
    async fn test_contains_import_negative_all_languages() {
        let registry = get_test_registry();
        let scenario = ImportScenarios::contains_import_negative();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .expect(&format!("Plugin not found for {:?}", fixture.language));

            let parser = plugin
                .import_parser()
                .expect(&format!("{:?} should have import parser", fixture.language));

            if let ImportOperation::ContainsImport { module_name } = &fixture.operation {
                let contains = parser.contains_import(fixture.source_code, module_name);

                match &fixture.expected {
                    ImportExpectedBehavior::Contains(expected) => {
                        assert_eq!(
                            contains, *expected,
                            "contains_import (negative) failed for {:?}\nSource: {}\nModule: {}\nExpected: {}\nGot: {}",
                            fixture.language, fixture.source_code, module_name, expected, contains
                        );
                    }
                    _ => panic!("Wrong expected behavior for contains_import test"),
                }
            } else {
                panic!("Wrong operation for contains_import test");
            }
        }
    }

    #[tokio::test]
    async fn test_add_import_to_existing_all_languages() {
        let registry = get_test_registry();
        let scenario = ImportScenarios::add_import_to_existing();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .expect(&format!("Plugin not found for {:?}", fixture.language));

            let mutation = plugin.import_mutation_support().expect(&format!(
                "{:?} should have import mutation support",
                fixture.language
            ));

            if let ImportOperation::AddImport { module_name } = &fixture.operation {
                let result = mutation.add_import(fixture.source_code, module_name);

                // Verify the import was added by checking if it now contains the module
                let parser = plugin.import_parser().unwrap();
                let contains = parser.contains_import(&result, module_name);

                assert!(
                    contains,
                    "add_import failed for {:?}\nSource: {}\nModule: {}\nResult: {}\nShould contain the module",
                    fixture.language, fixture.source_code, module_name, result
                );
            } else {
                panic!("Wrong operation for add_import test");
            }
        }
    }

    #[tokio::test]
    async fn test_add_import_to_empty_all_languages() {
        let registry = get_test_registry();
        let scenario = ImportScenarios::add_import_to_empty();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .expect(&format!("Plugin not found for {:?}", fixture.language));

            let mutation = plugin.import_mutation_support().expect(&format!(
                "{:?} should have import mutation support",
                fixture.language
            ));

            if let ImportOperation::AddImport { module_name } = &fixture.operation {
                let result = mutation.add_import(fixture.source_code, module_name);

                // Verify the import was added
                let parser = plugin.import_parser().unwrap();
                let contains = parser.contains_import(&result, module_name);

                assert!(
                    contains,
                    "add_import to empty failed for {:?}\nModule: {}\nResult: {}\nShould contain the module",
                    fixture.language, module_name, result
                );
            } else {
                panic!("Wrong operation for add_import test");
            }
        }
    }

    #[tokio::test]
    async fn test_remove_import_all_languages() {
        let registry = get_test_registry();
        let scenario = ImportScenarios::remove_existing_import();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .expect(&format!("Plugin not found for {:?}", fixture.language));

            let mutation = plugin.import_mutation_support().expect(&format!(
                "{:?} should have import mutation support",
                fixture.language
            ));

            if let ImportOperation::RemoveImport { module_name } = &fixture.operation {
                let result = mutation.remove_import(fixture.source_code, module_name);

                // Verify the import was removed by checking it no longer contains the module
                let parser = plugin.import_parser().unwrap();
                let contains = parser.contains_import(&result, module_name);

                assert!(
                    !contains,
                    "remove_import failed for {:?}\nSource: {}\nModule: {}\nResult: {}\nShould NOT contain the module",
                    fixture.language, fixture.source_code, module_name, result
                );
            } else {
                panic!("Wrong operation for remove_import test");
            }
        }
    }

    #[tokio::test]
    async fn test_rewrite_for_rename_all_languages() {
        let registry = get_test_registry();
        let scenario = ImportScenarios::rewrite_for_module_rename();

        for fixture in scenario.fixtures {
            let plugin = registry
                .find_by_extension(fixture.language.file_extension())
                .expect(&format!("Plugin not found for {:?}", fixture.language));

            let rename_support = plugin.import_rename_support().expect(&format!(
                "{:?} should have import rename support",
                fixture.language
            ));

            if let ImportOperation::RewriteForRename { old_name, new_name } = &fixture.operation {
                let (result, changes) = rename_support.rewrite_imports_for_rename(
                    fixture.source_code,
                    old_name,
                    new_name,
                );

                match &fixture.expected {
                    ImportExpectedBehavior::RewriteCount(expected_count) => {
                        assert_eq!(
                            changes, *expected_count,
                            "rewrite_for_rename failed for {:?}\nSource: {}\nOld: {}\nNew: {}\nExpected changes: {}\nGot: {}",
                            fixture.language, fixture.source_code, old_name, new_name, expected_count, changes
                        );

                        // Verify the old import is gone and new one is present
                        let parser = plugin.import_parser().unwrap();
                        let contains_old = parser.contains_import(&result, old_name);
                        let contains_new = parser.contains_import(&result, new_name);

                        if *expected_count > 0 {
                            assert!(
                                !contains_old,
                                "Should not contain old import after rename for {:?}",
                                fixture.language
                            );
                            assert!(
                                contains_new,
                                "Should contain new import after rename for {:?}",
                                fixture.language
                            );
                        }
                    }
                    _ => panic!("Wrong expected behavior for rewrite_for_rename test"),
                }
            } else {
                panic!("Wrong operation for rewrite_for_rename test");
            }
        }
    }
}