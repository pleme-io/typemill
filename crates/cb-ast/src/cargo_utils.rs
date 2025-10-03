//! Cargo.toml manipulation utilities for Rust workspace refactoring

use crate::error::{AstError, AstResult};
use toml_edit::{value, DocumentMut, Item};

/// Modifies a Cargo.toml content to update a dependency.
/// Can be used to rename a dependency or change its path.
///
/// # Arguments
///
/// * `cargo_content` - The current content of the Cargo.toml file
/// * `old_dep_name` - The current name of the dependency (e.g., "cb-mcp-proxy")
/// * `new_dep_name` - The new name for the dependency (e.g., "cb-plugins")
/// * `new_path` - The new relative path to the dependency (e.g., "../cb-plugins")
///
/// # Returns
///
/// The updated Cargo.toml content as a string
///
/// # Examples
///
/// ```rust
/// use cb_ast::cargo_utils::update_dependency;
///
/// let cargo_toml = r#"
/// [dependencies]
/// cb-mcp-proxy = { path = "../cb-mcp-proxy" }
/// "#;
///
/// let updated = update_dependency(
///     cargo_toml,
///     "cb-mcp-proxy",
///     "cb-plugins",
///     "../cb-plugins"
/// ).unwrap();
///
/// assert!(updated.contains("cb-plugins"));
/// ```
pub fn update_dependency(
    cargo_content: &str,
    old_dep_name: &str,
    new_dep_name: &str,
    new_path: &str,
) -> AstResult<String> {
    let mut doc = cargo_content
        .parse::<DocumentMut>()
        .map_err(|e| AstError::Analysis {
            message: format!("Failed to parse Cargo.toml: {}", e),
        })?;

    // Try to update in [dependencies]
    if let Some(deps) = doc
        .get_mut("dependencies")
        .and_then(Item::as_table_like_mut)
    {
        if deps.contains_key(old_dep_name) {
            // Create a new table for the updated dependency
            let mut new_dep_table = toml_edit::InlineTable::new();
            new_dep_table.insert("path", new_path.into());

            // Remove the old key and insert the new one
            deps.remove(old_dep_name);
            deps.insert(new_dep_name, value(new_dep_table));
        }
    }

    // Also try to update in [dev-dependencies]
    if let Some(dev_deps) = doc
        .get_mut("dev-dependencies")
        .and_then(Item::as_table_like_mut)
    {
        if dev_deps.contains_key(old_dep_name) {
            // Create a new table for the updated dependency
            let mut new_dep_table = toml_edit::InlineTable::new();
            new_dep_table.insert("path", new_path.into());

            // Remove the old key and insert the new one
            dev_deps.remove(old_dep_name);
            dev_deps.insert(new_dep_name, value(new_dep_table));
        }
    }

    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_dependency_basic() {
        let cargo_toml = r#"
[package]
name = "test-crate"

[dependencies]
cb-mcp-proxy = { path = "../cb-mcp-proxy" }
other-dep = "1.0"
"#;

        let updated =
            update_dependency(cargo_toml, "cb-mcp-proxy", "cb-plugins", "../cb-plugins").unwrap();

        assert!(updated.contains("cb-plugins"));
        assert!(!updated.contains("cb-mcp-proxy"));
        assert!(updated.contains("../cb-plugins"));
    }

    #[test]
    fn test_update_dev_dependency() {
        let cargo_toml = r#"
[package]
name = "test-crate"

[dev-dependencies]
cb-mcp-proxy = { path = "../cb-mcp-proxy" }
"#;

        let updated =
            update_dependency(cargo_toml, "cb-mcp-proxy", "cb-plugins", "../cb-plugins").unwrap();

        assert!(updated.contains("cb-plugins"));
        assert!(!updated.contains("cb-mcp-proxy"));
    }

    #[test]
    fn test_update_preserves_other_deps() {
        let cargo_toml = r#"
[dependencies]
cb-mcp-proxy = { path = "../cb-mcp-proxy" }
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
"#;

        let updated =
            update_dependency(cargo_toml, "cb-mcp-proxy", "cb-plugins", "../cb-plugins").unwrap();

        assert!(updated.contains("serde"));
        assert!(updated.contains("tokio"));
    }
}
