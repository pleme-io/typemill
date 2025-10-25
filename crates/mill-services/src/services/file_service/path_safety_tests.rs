//! Path safety regression tests for directory traversal protection

#[cfg(test)]
mod tests {
    use crate::services::file_service::FileService;
    use crate::services::lock_manager::LockManager;
    use crate::services::operation_queue::OperationQueue;
    use mill_ast::AstCache;
    use mill_config::config::AppConfig;
    use mill_plugin_api::PluginRegistry;
    use std::path::Path;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_service() -> (FileService, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_root = temp_dir.path().to_path_buf();

        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(OperationQueue::new());
        let plugin_registry = Arc::new(PluginRegistry::new());
        let config = AppConfig::default();

        let service = FileService::new(
            &project_root,
            ast_cache,
            lock_manager,
            operation_queue,
            &config,
            plugin_registry,
        );

        (service, temp_dir)
    }

    #[test]
    fn test_traversal_attempt_parent_dirs() {
        let (service, _temp) = create_test_service();

        // Attempt to traverse outside project root using ../
        let result = service.to_absolute_path_checked(Path::new("../../../etc/passwd"));

        assert!(
            result.is_err(),
            "Should reject path traversal using ../"
        );

        if let Err(e) = result {
            assert!(
                e.to_string().contains("Path traversal detected")
                    || e.to_string().contains("Parent directory does not exist"),
                "Error should mention path traversal or missing parent: {}",
                e
            );
        }
    }

    #[test]
    fn test_absolute_path_outside_root() {
        let (service, _temp) = create_test_service();

        // Attempt to use absolute path outside project root
        let result = service.to_absolute_path_checked(Path::new("/etc/passwd"));

        assert!(
            result.is_err(),
            "Should reject absolute path outside project root"
        );

        if let Err(e) = result {
            assert!(
                e.to_string().contains("Path traversal detected"),
                "Error should mention path traversal: {}",
                e
            );
        }
    }

    #[test]
    fn test_safe_relative_path() {
        let (service, temp) = create_test_service();

        // Create a valid subdirectory
        let test_dir = temp.path().join("src");
        std::fs::create_dir(&test_dir).expect("Failed to create test dir");

        // Valid relative path within project
        let result = service.to_absolute_path_checked(Path::new("src"));

        assert!(result.is_ok(), "Should accept valid relative path");

        let canonical = result.unwrap();
        assert!(
            canonical.starts_with(temp.path()),
            "Canonicalized path should be within project root"
        );
    }

    #[test]
    fn test_non_existent_file_safe_parent() {
        let (service, temp) = create_test_service();

        // Create parent directory
        let src_dir = temp.path().join("src");
        std::fs::create_dir(&src_dir).expect("Failed to create src dir");

        // Non-existent file in existing parent (for file creation)
        let result = service.to_absolute_path_checked(Path::new("src/new_file.rs"));

        assert!(
            result.is_ok(),
            "Should accept non-existent file in existing parent"
        );

        let canonical = result.unwrap();
        assert!(
            canonical.starts_with(temp.path()),
            "Path should be within project root"
        );
        assert!(
            canonical.ends_with("new_file.rs"),
            "Should preserve filename"
        );
    }

    #[test]
    fn test_non_existent_parent() {
        let (service, _temp) = create_test_service();

        // Non-existent parent directory
        let result = service.to_absolute_path_checked(Path::new("nonexistent/file.rs"));

        assert!(
            result.is_err(),
            "Should reject file in non-existent parent"
        );

        if let Err(e) = result {
            assert!(
                e.to_string().contains("Parent directory does not exist"),
                "Error should mention non-existent parent: {}",
                e
            );
        }
    }

    #[test]
    fn test_symlink_escape_attempt() {
        let (service, temp) = create_test_service();

        // Create a symlink pointing outside project root
        let link_path = temp.path().join("escape_link");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            if symlink("/etc", &link_path).is_ok() {
                let result = service.to_absolute_path_checked(Path::new("escape_link/passwd"));

                assert!(
                    result.is_err(),
                    "Should reject symlink escape to /etc/passwd"
                );

                if let Err(e) = result {
                    assert!(
                        e.to_string().contains("Path traversal detected"),
                        "Error should mention path traversal: {}",
                        e
                    );
                }
            }
        }

        #[cfg(windows)]
        {
            // Windows symlink test (requires admin privileges, so may skip)
            use std::os::windows::fs::symlink_dir;
            if symlink_dir("C:\\Windows", &link_path).is_ok() {
                let result = service.to_absolute_path_checked(Path::new("escape_link\\system32"));

                assert!(
                    result.is_err(),
                    "Should reject symlink escape"
                );
            }
        }
    }

    #[test]
    fn test_complex_nested_path() {
        let (service, temp) = create_test_service();

        // Create nested directory structure
        let nested = temp.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).expect("Failed to create nested dirs");

        // Valid nested path
        let result = service.to_absolute_path_checked(Path::new("a/b/c"));

        assert!(result.is_ok(), "Should accept valid nested path");

        let canonical = result.unwrap();
        assert!(
            canonical.starts_with(temp.path()),
            "Nested path should be within project root"
        );
    }
}
