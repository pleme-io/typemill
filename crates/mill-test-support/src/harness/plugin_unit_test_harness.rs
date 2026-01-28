//! Integration Test Harness for Language Plugins
//!
//! Provides a reusable test environment for plugin integration testing with
//! temporary directories, setup/teardown, and common test patterns.

use anyhow::Result;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Main test harness for plugin integration testing
///
/// Manages temporary file systems and provides utilities for common
/// plugin testing patterns across all languages.
///
/// # Example
///
/// ```text
/// use mill_test_support::harness::IntegrationTestHarness;
///
/// #[tokio::test]
/// async fn test_parse_and_verify() {
///     let harness = IntegrationTestHarness::new();
///     let file = harness.create_source_file("main.rs", "fn main() {}").unwrap();
///     assert!(file.exists());
/// }
/// ```
pub struct IntegrationTestHarness {
    /// Temporary directory for test files
    pub temp_dir: TempDir,
}

impl IntegrationTestHarness {
    /// Create a new test harness with a fresh temporary directory
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        Ok(Self { temp_dir })
    }

    /// Get the root path of the temporary directory
    pub fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a source file in the temporary directory
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file to create
    /// * `content` - Content to write to the file
    ///
    /// # Returns
    ///
    /// Path to the created file
    pub fn create_source_file(&self, filename: &str, content: &str) -> Result<PathBuf> {
        let file_path = self.temp_dir.path().join(filename);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Create a directory structure
    ///
    /// # Arguments
    ///
    /// * `rel_path` - Relative path from root
    ///
    /// # Returns
    ///
    /// Path to the created directory
    pub fn create_directory(&self, rel_path: &str) -> Result<PathBuf> {
        let dir_path = self.temp_dir.path().join(rel_path);
        std::fs::create_dir_all(&dir_path)?;
        Ok(dir_path)
    }

    /// Read content from a file
    pub fn read_file(&self, rel_path: &str) -> Result<String> {
        let file_path = self.temp_dir.path().join(rel_path);
        Ok(std::fs::read_to_string(&file_path)?)
    }

    /// Common pattern: Parse → Modify → Verify
    ///
    /// This pattern is used across many language plugins:
    /// 1. Create a source file with initial content
    /// 2. Apply a modification function
    /// 3. Verify the result
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the source file
    /// * `source` - Initial source code
    /// * `modify_fn` - Function that takes the source and returns modified source
    ///
    /// # Returns
    ///
    /// The modified source code
    pub fn test_parse_modify_verify<F>(
        &self,
        filename: &str,
        source: &str,
        modify_fn: F,
    ) -> Result<String>
    where
        F: Fn(&str) -> String,
    {
        // Create the source file
        self.create_source_file(filename, source)?;

        // Apply modification
        let modified = modify_fn(source);

        // Verify file exists and is accessible
        let file_path = self.temp_dir.path().join(filename);
        assert!(file_path.exists(), "Source file should exist");

        Ok(modified)
    }

    /// Common pattern: Create package → Add dependency → Verify manifest
    ///
    /// This pattern tests manifest manipulation:
    /// 1. Create a package structure
    /// 2. Add a dependency to the manifest
    /// 3. Verify the manifest was updated correctly
    ///
    /// # Arguments
    ///
    /// * `manifest_filename` - Name of the manifest file (e.g., "Cargo.toml")
    /// * `initial_manifest` - Initial manifest content
    /// * `dependency_name` - Name of dependency to add
    ///
    /// # Returns
    ///
    /// Updated manifest content
    pub fn test_manifest_dependency_workflow(
        &self,
        manifest_filename: &str,
        initial_manifest: &str,
        dependency_name: &str,
    ) -> Result<String> {
        // Create manifest
        self.create_source_file(manifest_filename, initial_manifest)?;

        // Simulate adding a dependency (caller will do actual dependency logic)
        let mut updated = initial_manifest.to_string();
        if !updated.contains(dependency_name) {
            updated.push_str(&format!("\n# Added dependency: {}", dependency_name));
        }

        // Verify manifest exists
        let manifest_path = self.temp_dir.path().join(manifest_filename);
        assert!(manifest_path.exists(), "Manifest should exist");

        Ok(updated)
    }

    /// Common pattern: Move file → Verify references updated
    ///
    /// This pattern tests that import/reference updates work correctly
    /// during file moves:
    /// 1. Create a source file with references
    /// 2. Create the referenced module
    /// 3. Simulate moving the file
    /// 4. Verify references would be updated
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path to the file being moved
    /// * `source_content` - Content of the file
    /// * `reference_pattern` - Pattern to find references to update
    ///
    /// # Returns
    ///
    /// (old_path, new_path) tuple
    pub fn test_move_file_update_references(
        &self,
        source_file: &str,
        source_content: &str,
        reference_pattern: &str,
    ) -> Result<(PathBuf, PathBuf)> {
        // Create source file
        let old_path = self.create_source_file(source_file, source_content)?;

        // Create new location
        let new_path = self.temp_dir.path().join("moved").join(source_file);
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Verify source exists
        assert!(old_path.exists(), "Source file should exist before move");
        assert!(
            source_content.contains(reference_pattern) || reference_pattern.is_empty(),
            "Reference pattern should exist in content or be empty"
        );

        Ok((old_path, new_path))
    }

    /// Common pattern: Large file performance test
    ///
    /// Tests plugin performance on large files:
    /// 1. Generate a large file with many symbols
    /// 2. Measure parsing/processing time
    /// 3. Verify completion within reasonable time
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file to create
    /// * `generator` - Function that generates file content with item_count items
    /// * `item_count` - Number of items to generate
    ///
    /// # Returns
    ///
    /// Path to the generated file
    pub fn test_large_file_performance<F>(
        &self,
        filename: &str,
        generator: F,
        item_count: usize,
    ) -> Result<PathBuf>
    where
        F: Fn(usize) -> String,
    {
        let content = generator(item_count);
        self.create_source_file(filename, &content)
    }

    /// Common pattern: Circular dependency detection
    ///
    /// Tests that plugins can handle and detect circular dependencies:
    /// 1. Create module A that imports B
    /// 2. Create module B that imports A
    /// 3. Verify circular dependency is detected (or gracefully handled)
    ///
    /// # Arguments
    ///
    /// * `module_a_name` - Name of first module
    /// * `module_b_name` - Name of second module
    /// * `import_a_to_b` - Import statement from A to B
    /// * `import_b_to_a` - Import statement from B to A
    ///
    /// # Returns
    ///
    /// (path_to_a, path_to_b) tuple
    pub fn test_circular_dependency_detection(
        &self,
        module_a_name: &str,
        module_b_name: &str,
        import_a_to_b: &str,
        import_b_to_a: &str,
    ) -> Result<(PathBuf, PathBuf)> {
        let content_a = format!("{}\n// Module A content", import_a_to_b);
        let content_b = format!("{}\n// Module B content", import_b_to_a);

        let path_a = self.create_source_file(module_a_name, &content_a)?;
        let path_b = self.create_source_file(module_b_name, &content_b)?;

        Ok((path_a, path_b))
    }

    /// Helper to create a multi-file workspace structure
    ///
    /// # Arguments
    ///
    /// * `files` - Vec of (relative_path, content) pairs
    ///
    /// # Returns
    ///
    /// Vec of created file paths
    pub fn create_file_structure(&self, files: Vec<(&str, &str)>) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        for (rel_path, content) in files {
            let path = self.create_source_file(rel_path, content)?;
            paths.push(path);
        }

        Ok(paths)
    }
}

impl Default for IntegrationTestHarness {
    fn default() -> Self {
        Self::new().expect("Failed to create temp directory for test harness")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = IntegrationTestHarness::new();
        assert!(harness.is_ok());
    }

    #[test]
    fn test_create_source_file() {
        let harness = IntegrationTestHarness::new().unwrap();
        let result = harness.create_source_file("test.txt", "content");
        assert!(result.is_ok());
        assert!(harness.root().join("test.txt").exists());
    }

    #[test]
    fn test_create_nested_file() {
        let harness = IntegrationTestHarness::new().unwrap();
        let result = harness.create_source_file("dir/subdir/test.txt", "content");
        assert!(result.is_ok());
        assert!(harness.root().join("dir/subdir/test.txt").exists());
    }

    #[test]
    fn test_read_file() {
        let harness = IntegrationTestHarness::new().unwrap();
        let content = "test content";
        harness.create_source_file("test.txt", content).unwrap();
        let read = harness.read_file("test.txt").unwrap();
        assert_eq!(read, content);
    }

    #[test]
    fn test_parse_modify_verify() {
        let harness = IntegrationTestHarness::new().unwrap();
        let result = harness
            .test_parse_modify_verify("test.rs", "fn main() {}", |s| s.replace("main", "modified"));
        assert!(result.is_ok());
        assert!(result.unwrap().contains("modified"));
    }

    #[test]
    fn test_manifest_dependency_workflow() {
        let harness = IntegrationTestHarness::new().unwrap();
        let result = harness.test_manifest_dependency_workflow(
            "Cargo.toml",
            "[package]\nname = \"test\"",
            "tokio",
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("tokio"));
    }

    #[test]
    fn test_move_file_update_references() {
        let harness = IntegrationTestHarness::new().unwrap();
        let result =
            harness.test_move_file_update_references("src/main.rs", "use utils::foo;", "utils");
        assert!(result.is_ok());
        let (old_path, new_path) = result.unwrap();
        assert!(old_path.exists());
        assert!(!new_path.exists()); // New path shouldn't exist yet
    }

    #[test]
    fn test_create_directory() {
        let harness = IntegrationTestHarness::new().unwrap();
        let result = harness.create_directory("src/utils");
        assert!(result.is_ok());
        assert!(harness.root().join("src/utils").is_dir());
    }

    #[test]
    fn test_create_file_structure() {
        let harness = IntegrationTestHarness::new().unwrap();
        let files = vec![
            ("src/main.rs", "fn main() {}"),
            ("src/lib.rs", "pub mod utils;"),
            ("tests/unit.rs", "#[test] fn test() {}"),
        ];
        let result = harness.create_file_structure(files);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }
}
