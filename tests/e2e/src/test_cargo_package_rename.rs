//! Integration tests for complete Cargo package rename coverage (V2 - CONSOLIDATED)
//!
//! Migrated to use closure-based helper pattern for reduced boilerplate.

use crate::test_helpers::*;
use anyhow::Result;

#[tokio::test]
async fn test_complete_cargo_package_rename() -> Result<()> {
    run_tool_test(
        &[
            (
                "Cargo.toml",
                r#"
[workspace]
members = [
    "integration-tests",
    "app",
]
"#,
            ),
            (
                "integration-tests/Cargo.toml",
                r#"
[package]
name = "integration-tests"
version = "0.1.0"
edition = "2021"

[features]
test-feature = []
"#,
            ),
            ("integration-tests/src/lib.rs", "pub fn test_helper() {}"),
            (
                "integration-tests/src/main.rs",
                r#"
use integration_tests::test_helper;

fn main() {
    test_helper();
}
"#,
            ),
            (
                "app/Cargo.toml",
                r#"
[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dev-dependencies]
integration-tests = { path = "../integration-tests" }

[features]
# Bug 1: Feature flag references - should be updated when integration-tests is renamed
testing = ["integration-tests/test-feature"]
"#,
            ),
            ("app/src/lib.rs", "pub fn app_fn() {}"),
        ],
        "rename_all",
        |ws| build_rename_params(ws, "integration-tests", "tests", "directory"),
        |ws| {
            // VERIFICATION 1: Root workspace Cargo.toml members list updated
            let root_cargo = ws.read_file("Cargo.toml");
            assert!(
                root_cargo.contains(r#""tests""#) || root_cargo.contains("tests"),
                "Root Cargo.toml should reference 'tests'\nActual:\n{}",
                root_cargo
            );
            assert!(
                !root_cargo.contains("integration-tests"),
                "Root Cargo.toml should not reference 'integration-tests'\nActual:\n{}",
                root_cargo
            );

            // VERIFICATION 2: Package name in moved Cargo.toml updated
            let package_cargo = ws.read_file("tests/Cargo.toml");
            assert!(
                package_cargo.contains(r#"name = "tests""#),
                "Package Cargo.toml should have name = 'tests'\nActual:\n{}",
                package_cargo
            );

            // VERIFICATION 3: Dev-dependency references updated
            let app_cargo = ws.read_file("app/Cargo.toml");
            assert!(
                app_cargo.contains(r#"tests = { path = "../tests" }"#)
                    || (app_cargo.contains("tests") && app_cargo.contains("../tests")),
                "App Cargo.toml should reference 'tests' with correct path\nActual:\n{}",
                app_cargo
            );
            assert!(
                !app_cargo.contains("integration-tests = {")
                    && !app_cargo.contains("integration-tests/"),
                "App Cargo.toml should not reference 'integration-tests'\nActual:\n{}",
                app_cargo
            );

            // VERIFICATION 4: Feature flag references updated (Bug 1)
            assert!(
                app_cargo.contains(r#"["tests/test-feature"]"#)
                    || app_cargo.contains("[\"tests/test-feature\"]"),
                "Feature flags should be updated to 'tests/test-feature'\nActual:\n{}",
                app_cargo
            );

            // VERIFICATION 5: Self-referencing imports updated (Bug 2)
            let main_rs = ws.read_file("tests/src/main.rs");
            assert!(
                main_rs.contains("use tests::test_helper;"),
                "Self-import should be updated to 'tests'\nActual:\n{}",
                main_rs
            );
            assert!(
                !main_rs.contains("integration_tests"),
                "Should not contain old crate name 'integration_tests'\nActual:\n{}",
                main_rs
            );
            Ok(())
        },
    )
    .await
}
