//! Module path computation for Rust files
//!
//! Computes fully-qualified module paths from file paths within a Rust project.

use std::path::Path;

/// Compute the full module path from a file path
///
/// # Examples
/// - `common/src/utils.rs` → `common::utils`
/// - `common/src/utils/mod.rs` → `common::utils` (mod.rs represents the parent directory)
/// - `common/src/foo/bar/mod.rs` → `common::foo::bar`
/// - `new_utils/src/lib.rs` → `new_utils` (lib.rs is the crate root)
/// - `common/src/main.rs` → `common` (main.rs is the crate root)
/// - `common/src/foo/bar.rs` → `common::foo::bar`
pub fn compute_module_path_from_file(
    file_path: &Path,
    crate_name: &str,
    project_root: &Path,
) -> String {
    // Get the file path relative to project root
    let rel_path = file_path.strip_prefix(project_root).unwrap_or(file_path);

    // Get components after the crate name
    let mut components: Vec<&str> = rel_path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Remove the crate name (first component)
    if !components.is_empty() {
        components.remove(0);
    }

    // Remove "src" if present
    if components.first().copied() == Some("src") {
        components.remove(0);
    }

    // Special handling for mod.rs files
    // mod.rs represents the parent directory's module, not a module named "mod"
    // Example: common/src/utils/mod.rs → common::utils (not common::utils::mod)
    if components.last().copied() == Some("mod.rs") {
        components.pop(); // Remove "mod.rs"
                          // The parent directory name is now the last component (the module name)
    }

    // If the file is lib.rs or main.rs, it's the crate root
    if components.last().copied() == Some("lib.rs") || components.last().copied() == Some("main.rs")
    {
        return crate_name.to_string();
    }

    // Remove the .rs extension from the last component
    if let Some(last) = components.last_mut() {
        if let Some(stripped) = last.strip_suffix(".rs") {
            *last = stripped;
        }
    }

    // Build the module path: crate_name::module1::module2...
    let mut module_path = crate_name.to_string();
    for component in components {
        if !component.is_empty() {
            module_path.push_str("::");
            module_path.push_str(component);
        }
    }

    module_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_module_path_from_file_simple() {
        let project_root = Path::new("/workspace");

        // Test simple file: common/src/utils.rs → common::utils
        let file_path = Path::new("/workspace/common/src/utils.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common::utils");
    }

    #[test]
    fn test_compute_module_path_from_file_mod_rs() {
        let project_root = Path::new("/workspace");

        // Test mod.rs: common/src/utils/mod.rs → common::utils (NOT common::utils::mod)
        let file_path = Path::new("/workspace/common/src/utils/mod.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common::utils");
    }

    #[test]
    fn test_compute_module_path_from_file_nested_mod_rs() {
        let project_root = Path::new("/workspace");

        // Test nested mod.rs: common/src/foo/bar/mod.rs → common::foo::bar
        let file_path = Path::new("/workspace/common/src/foo/bar/mod.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common::foo::bar");
    }

    #[test]
    fn test_compute_module_path_from_file_lib_rs() {
        let project_root = Path::new("/workspace");

        // Test lib.rs (crate root): common/src/lib.rs → common
        let file_path = Path::new("/workspace/common/src/lib.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common");
    }

    #[test]
    fn test_compute_module_path_from_file_main_rs() {
        let project_root = Path::new("/workspace");

        // Test main.rs (crate root): common/src/main.rs → common
        let file_path = Path::new("/workspace/common/src/main.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common");
    }

    #[test]
    fn test_compute_module_path_from_file_nested() {
        let project_root = Path::new("/workspace");

        // Test nested file: common/src/foo/bar.rs → common::foo::bar
        let file_path = Path::new("/workspace/common/src/foo/bar.rs");
        let result = compute_module_path_from_file(file_path, "common", project_root);
        assert_eq!(result, "common::foo::bar");
    }
}
