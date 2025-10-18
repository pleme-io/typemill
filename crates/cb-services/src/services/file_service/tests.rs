#[cfg(test)]
mod tests {
    use crate::services::file_service::FileService;
    use crate::services::lock_manager::LockManager;
    use crate::services::operation_queue::{OperationQueue, OperationType};
    use cb_ast::AstCache;
    use cb_protocol::{ApiError, DependencyUpdate, EditPlan, EditPlanMetadata, TextEdit};
    use std::path::Path;
    use std::sync::Arc;
    use tempfile::TempDir;

    // Helper to start a background worker for tests
    fn spawn_test_worker(queue: Arc<OperationQueue>) {
        use tokio::fs;

        tokio::spawn(async move {
            queue
                .process_with(|op, stats| async move {
                    let result: Result<(), ApiError> = match op.operation_type {
                        OperationType::CreateDir => {
                            fs::create_dir_all(&op.file_path).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to create directory: {}", e))
                            })
                        }
                        OperationType::CreateFile | OperationType::Write => {
                            let content = op
                                .params
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            fs::write(&op.file_path, content).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to write file: {}", e))
                            })
                        }
                        OperationType::Delete => {
                            if op.file_path.exists() {
                                fs::remove_file(&op.file_path).await.map_err(|e| {
                                    ApiError::Internal(format!("Failed to delete file: {}", e))
                                })
                            } else {
                                Ok(())
                            }
                        }
                        OperationType::Rename => {
                            let new_path_str = op
                                .params
                                .get("new_path")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                ApiError::Internal("Missing new_path".to_string())
                            })?;
                            fs::rename(&op.file_path, new_path_str).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to rename file: {}", e))
                            })
                        }
                        _ => Ok(()),
                    };

                    // Update stats after operation completes
                    let mut stats_guard = stats.lock().await;
                    match &result {
                        Ok(_) => {
                            stats_guard.completed_operations += 1;
                        }
                        Err(_) => {
                            stats_guard.failed_operations += 1;
                        }
                    }
                    drop(stats_guard);

                    result.map(|_| serde_json::Value::Null)
                })
                .await;
        });
    }

    pub(super) fn create_test_service(temp_dir: &TempDir) -> (FileService, Arc<OperationQueue>) {
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));
        let config = codebuddy_config::config::AppConfig::default();
        let plugin_registry = crate::services::build_language_plugin_registry();
        let service = FileService::new(
            temp_dir.path(),
            ast_cache,
            lock_manager,
            operation_queue.clone(),
            &config,
            plugin_registry,
        );

        // Spawn background worker to process queued operations
        spawn_test_worker(operation_queue.clone());

        (service, operation_queue)
    }

    #[tokio::test]
    async fn test_create_and_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let file_path = Path::new("test.txt");
        let content = "Hello, World!";

        // Create file
        service
            .create_file(file_path, Some(content), false, false)
            .await
            .unwrap();

        // Wait for queue to process operations
        queue.wait_until_idle().await;

        // Read file
        let read_content = service.read_file(file_path).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_rename_file() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create initial file
        let old_path = Path::new("old.txt");
        let new_path = Path::new("new.txt");
        service
            .create_file(old_path, Some("content"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Rename file
        let result = service
            .rename_file_with_imports(old_path, new_path, false, None)
            .await
            .unwrap();
        assert!(result.result["success"].as_bool().unwrap_or(false));
        queue.wait_until_idle().await;

        // Verify old file doesn't exist and new file does
        assert!(!temp_dir.path().join(old_path).exists());
        assert!(temp_dir.path().join(new_path).exists());
    }

    #[tokio::test]
    async fn test_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let file_path = Path::new("to_delete.txt");

        // Create and then delete file
        service
            .create_file(file_path, Some("temporary"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;
        assert!(temp_dir.path().join(file_path).exists());

        service.delete_file(file_path, false, false).await.unwrap();
        queue.wait_until_idle().await;
        assert!(!temp_dir.path().join(file_path).exists());
    }

    #[tokio::test]
    async fn test_atomic_edit_plan_success() {
        use cb_protocol::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let (service, _queue) = create_test_service(&temp_dir);

        // Create test files
        let main_file = "main.ts";
        let dep_file = "dependency.ts";

        service
            .create_file(
                Path::new(main_file),
                Some("import { foo } from './old';\nconst x = 1;"),
                false,
                false,
            )
            .await
            .unwrap();
        service
            .create_file(
                Path::new(dep_file),
                Some("import './old';\nconst y = 2;"),
                false,
                false,
            )
            .await
            .unwrap();

        // Create edit plan
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 1,
                    start_column: 0,
                    end_line: 1,
                    end_column: 12,
                },
                original_text: "const x = 1;".to_string(),
                new_text: "const x = 2;".to_string(),
                priority: 1,
                description: "Update value".to_string(),
            }],
            dependency_updates: vec![DependencyUpdate {
                target_file: dep_file.to_string(),
                update_type: DependencyUpdateType::ImportPath,
                old_reference: "./old".to_string(),
                new_reference: "./new".to_string(),
            }],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec!["test".to_string()],
            },
        };

        // Apply edit plan
        let result = service.apply_edit_plan(&plan).await.unwrap();

        // Verify success
        assert!(result.success);
        assert_eq!(result.modified_files.len(), 2);
        assert!(result.errors.is_none());

        // Verify file contents were updated
        let main_content = service.read_file(Path::new(main_file)).await.unwrap();
        assert!(main_content.contains("const x = 2;"));

        let dep_content = service.read_file(Path::new(dep_file)).await.unwrap();
        assert!(dep_content.contains("./new"));
    }

    #[tokio::test]
    async fn test_atomic_rollback_on_main_file_failure() {
        use cb_protocol::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create test files with specific content
        let main_file = "main.ts";
        let dep_file = "dependency.ts";

        let main_original = "import { foo } from './old';\nconst x = 1;";
        let dep_original = "import './old';\nconst y = 2;";

        service
            .create_file(Path::new(main_file), Some(main_original), false, false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file), Some(dep_original), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Create edit plan with invalid edit location that will fail
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 999, // Invalid line - will cause failure
                    start_column: 0,
                    end_line: 999,
                    end_column: 10,
                },
                original_text: "invalid".to_string(),
                new_text: "replacement".to_string(),
                priority: 1,
                description: "This should fail".to_string(),
            }],
            dependency_updates: vec![DependencyUpdate {
                target_file: dep_file.to_string(),
                update_type: DependencyUpdateType::ImportPath,
                old_reference: "./old".to_string(),
                new_reference: "./new".to_string(),
            }],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test_failure".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec!["test".to_string()],
            },
        };

        // Apply edit plan - should fail
        let result = service.apply_edit_plan(&plan).await;
        assert!(result.is_err());

        // Verify files were rolled back to original state
        let main_content = service.read_file(Path::new(main_file)).await.unwrap();
        assert_eq!(
            main_content, main_original,
            "Main file should be rolled back"
        );

        let dep_content = service.read_file(Path::new(dep_file)).await.unwrap();
        assert_eq!(
            dep_content, dep_original,
            "Dependency file should be rolled back"
        );
    }

    #[tokio::test]
    async fn test_atomic_rollback_on_dependency_failure() {
        use cb_protocol::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create main file
        let main_file = "main.ts";
        let main_original = "const x = 1;";

        service
            .create_file(Path::new(main_file), Some(main_original), false, false)
            .await
            .unwrap();

        // Create a dependency file with unparseable content that will cause AST failure
        let dep_file = "bad_syntax.ts";
        let dep_original = "<<<< this is invalid typescript syntax >>>>";

        service
            .create_file(Path::new(dep_file), Some(dep_original), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Create edit plan that will fail when trying to parse the bad dependency file
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 0,
                    start_column: 0,
                    end_line: 0,
                    end_column: 12,
                },
                original_text: "const x = 1;".to_string(),
                new_text: "const x = 2;".to_string(),
                priority: 1,
                description: "Update value".to_string(),
            }],
            dependency_updates: vec![DependencyUpdate {
                target_file: dep_file.to_string(),
                update_type: DependencyUpdateType::ImportPath,
                old_reference: "<<<<".to_string(),
                new_reference: "./new".to_string(),
            }],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test_dep_failure".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec!["test".to_string()],
            },
        };

        // Apply edit plan - should fail on dependency update due to parse error
        let result = service.apply_edit_plan(&plan).await;
        assert!(result.is_err());

        // Verify main file was rolled back to original state
        let main_content = service.read_file(Path::new(main_file)).await.unwrap();
        assert_eq!(
            main_content, main_original,
            "Main file should be rolled back after dependency failure"
        );

        // Verify bad dependency file was also rolled back
        let dep_content = service.read_file(Path::new(dep_file)).await.unwrap();
        assert_eq!(
            dep_content, dep_original,
            "Dependency file should be rolled back"
        );
    }

    #[tokio::test]
    async fn test_atomic_rollback_multiple_files() {
        use cb_protocol::{DependencyUpdateType, EditLocation, EditType};

        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        // Create multiple files
        let main_file = "main.ts";
        let dep_file1 = "dep1.ts";
        let dep_file2 = "dep2.ts";
        let dep_file3 = "dep3.ts";

        let main_original = "const x = 1;";
        let dep1_original = "import './old1';";
        let dep2_original = "import './old2';";
        let dep3_original = "import 'this_will_cause_parse_error'; <<<< invalid syntax >>>>";

        service
            .create_file(Path::new(main_file), Some(main_original), false, false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file1), Some(dep1_original), false, false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file2), Some(dep2_original), false, false)
            .await
            .unwrap();
        service
            .create_file(Path::new(dep_file3), Some(dep3_original), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Create edit plan that will fail on the last dependency due to parse error
        let plan = EditPlan {
            source_file: main_file.to_string(),
            edits: vec![TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: 0,
                    start_column: 0,
                    end_line: 0,
                    end_column: 12,
                },
                original_text: "const x = 1;".to_string(),
                new_text: "const x = 999;".to_string(),
                priority: 1,
                description: "Update value".to_string(),
            }],
            dependency_updates: vec![
                DependencyUpdate {
                    target_file: dep_file1.to_string(),
                    update_type: DependencyUpdateType::ImportPath,
                    old_reference: "./old1".to_string(),
                    new_reference: "./new1".to_string(),
                },
                DependencyUpdate {
                    target_file: dep_file2.to_string(),
                    update_type: DependencyUpdateType::ImportPath,
                    old_reference: "./old2".to_string(),
                    new_reference: "./new2".to_string(),
                },
                DependencyUpdate {
                    target_file: dep_file3.to_string(),
                    update_type: DependencyUpdateType::ImportPath,
                    old_reference: "this_will_cause_parse_error".to_string(),
                    new_reference: "./new3".to_string(),
                },
            ],
            validations: vec![],
            metadata: EditPlanMetadata {
                intent_name: "test_multi_rollback".to_string(),
                intent_arguments: serde_json::json!({}),
                created_at: chrono::Utc::now(),
                complexity: 3,
                impact_areas: vec!["test".to_string()],
            },
        };

        // Apply edit plan - should fail on third dependency due to parse error
        let result = service.apply_edit_plan(&plan).await;
        assert!(result.is_err());

        // Verify ALL files were rolled back to original state
        let main_content = service.read_file(Path::new(main_file)).await.unwrap();
        assert_eq!(
            main_content, main_original,
            "Main file should be rolled back"
        );

        let dep1_content = service.read_file(Path::new(dep_file1)).await.unwrap();
        assert_eq!(
            dep1_content, dep1_original,
            "First dependency file should be rolled back"
        );

        let dep2_content = service.read_file(Path::new(dep_file2)).await.unwrap();
        assert_eq!(
            dep2_content, dep2_original,
            "Second dependency file should be rolled back"
        );

        let dep3_content = service.read_file(Path::new(dep_file3)).await.unwrap();
        assert_eq!(
            dep3_content, dep3_original,
            "Third dependency file should remain unchanged"
        );
    }
}

#[cfg(test)]
mod workspace_tests {
    use crate::services::file_service::FileService;
    use crate::services::lock_manager::LockManager;
    use crate::services::operation_queue::{OperationQueue, OperationType};
    use cb_ast::AstCache;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::fs;

    // Helper to start a background worker for tests
    fn spawn_test_worker(queue: Arc<OperationQueue>) {
        use cb_protocol::ApiError;

        tokio::spawn(async move {
            queue
                .process_with(|op, stats| async move {
                    let result: Result<(), ApiError> = match op.operation_type {
                        OperationType::CreateDir => {
                            fs::create_dir_all(&op.file_path).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to create directory: {}", e))
                            })
                        }
                        OperationType::CreateFile | OperationType::Write => {
                            let content = op
                                .params
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            fs::write(&op.file_path, content).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to write file: {}", e))
                            })
                        }
                        OperationType::Delete => {
                            if op.file_path.exists() {
                                fs::remove_file(&op.file_path).await.map_err(|e| {
                                    ApiError::Internal(format!("Failed to delete file: {}", e))
                                })
                            } else {
                                Ok(())
                            }
                        }
                        OperationType::Rename => {
                            let new_path_str = op
                                .params
                                .get("new_path")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                ApiError::Internal("Missing new_path".to_string())
                            })?;
                            fs::rename(&op.file_path, new_path_str).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to rename file: {}", e))
                            })
                        }
                        _ => Ok(()),
                    };

                    // Update stats after operation completes
                    let mut stats_guard = stats.lock().await;
                    match &result {
                        Ok(_) => {
                            stats_guard.completed_operations += 1;
                        }
                        Err(_) => {
                            stats_guard.failed_operations += 1;
                        }
                    }
                    drop(stats_guard);

                    result.map(|_| serde_json::Value::Null)
                })
                .await;
        });
    }

    fn create_test_service(temp_dir: &TempDir) -> (FileService, Arc<OperationQueue>) {
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));
        let config = codebuddy_config::config::AppConfig::default();
        let plugin_registry = crate::services::build_language_plugin_registry();
        let service = FileService::new(
            temp_dir.path(),
            ast_cache,
            lock_manager,
            operation_queue.clone(),
            &config,
            plugin_registry,
        );

        // Spawn background worker to process queued operations
        spawn_test_worker(operation_queue.clone());

        (service, operation_queue)
    }

    #[tokio::test]
    async fn test_update_workspace_manifests_simple_rename() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a workspace Cargo.toml
        let workspace_toml_content = r#"
[workspace]
members = [
    "crates/my-crate",
]
"#;
        fs::write(project_root.join("Cargo.toml"), workspace_toml_content)
            .await
            .unwrap();

        // Create the package directory and its Cargo.toml
        let old_crate_dir = project_root.join("crates/my-crate");
        fs::create_dir_all(&old_crate_dir).await.unwrap();
        fs::write(
            old_crate_dir.join("Cargo.toml"),
            "[package]\nname = \"my-crate\"",
        )
        .await
        .unwrap();

        let new_crate_dir = project_root.join("crates/my-renamed-crate");

        // Setup FileService
        let (service, _queue) = create_test_service(&temp_dir);

        // Run the update
        service
            .update_workspace_manifests(&old_crate_dir, &new_crate_dir)
            .await
            .unwrap();

        // Verify the workspace Cargo.toml was updated
        let updated_content = fs::read_to_string(project_root.join("Cargo.toml"))
            .await
            .unwrap();
        let doc = updated_content.parse::<toml_edit::DocumentMut>().unwrap();
        let members = doc["workspace"]["members"].as_array().unwrap();

        assert_eq!(members.len(), 1);
        assert_eq!(
            members.iter().next().unwrap().as_str(),
            Some("crates/my-renamed-crate")
        );
    }

    #[test]
    fn test_adjust_relative_path_logic() {
        let temp_dir = TempDir::new().unwrap();
        // This test doesn't need async operations, so create service directly
        let ast_cache = Arc::new(AstCache::new());
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));
        let config = codebuddy_config::config::AppConfig::default();
        let plugin_registry = crate::services::build_language_plugin_registry();
        let service = FileService::new(
            temp_dir.path(),
            ast_cache,
            lock_manager,
            operation_queue,
            &config,
            plugin_registry,
        );

        // Moved deeper: 1 level
        assert_eq!(
            service.adjust_relative_path("../sibling", 1, 2),
            "../../sibling"
        );
        // Moved deeper: 2 levels
        assert_eq!(
            service.adjust_relative_path("../sibling", 1, 3),
            "../../../sibling"
        );
        // Moved shallower: 1 level
        assert_eq!(
            service.adjust_relative_path("../../sibling", 2, 1),
            "../sibling"
        );
        // Moved shallower: 2 levels
        assert_eq!(
            service.adjust_relative_path("../../../sibling", 3, 1),
            "../sibling"
        );
        // No change
        assert_eq!(
            service.adjust_relative_path("../sibling", 2, 2),
            "../sibling"
        );
        // Path with no up-levels
        assert_eq!(service.adjust_relative_path("sibling", 2, 1), "sibling");
    }
}

#[cfg(test)]
mod move_tests {
    use std::path::Path;

    use tempfile::TempDir;

    use cb_protocol::ApiError;

    use super::tests::create_test_service;

    #[tokio::test]
    async fn test_move_file_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let source_path = Path::new("source.txt");
        let dest_path = Path::new("dest.txt");
        let content = "move content";

        service
            .create_file(source_path, Some(content), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Perform a dry-run move
        let result = service
            .rename_file_with_imports(source_path, dest_path, true, None)
            .await
            .unwrap();

        // A successful dry run should not error, but it does not have a "success" field.
        // The unwrap() call above is sufficient to check for errors.
        assert!(result.dry_run);
        // In dry run, no operations should be queued.
        assert_eq!(queue.queue_size().await, 0);

        // Verify that the source file still exists and the destination does not.
        assert!(temp_dir.path().join(source_path).exists());
        assert!(!temp_dir.path().join(dest_path).exists());
    }

    #[tokio::test]
    async fn test_move_file_collision_detection() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let source_path = Path::new("source.txt");
        let dest_path = Path::new("dest.txt");

        service
            .create_file(source_path, Some("source"), false, false)
            .await
            .unwrap();
        service
            .create_file(dest_path, Some("destination"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Attempt to move, expecting a collision error.
        let result = service
            .rename_file_with_imports(source_path, dest_path, false, None)
            .await;

        assert!(result.is_err());
        if let Err(ApiError::AlreadyExists(path_str)) = result {
            // The error message contains the full absolute path.
            // We'll just check that it contains the destination path we expect.
            assert!(path_str.contains(dest_path.to_str().unwrap()));
        } else {
            panic!("Expected AlreadyExists error, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_move_file_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let source_path = Path::new("source.txt");
        let dest_path = Path::new("new/nested/dir/dest.txt");
        let content = "move content";

        service
            .create_file(source_path, Some(content), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Perform move, which should create parent directories.
        service
            .rename_file_with_imports(source_path, dest_path, false, None)
            .await
            .unwrap();

        queue.wait_until_idle().await;

        assert!(!temp_dir.path().join(source_path).exists());
        assert!(temp_dir.path().join(dest_path).exists());
        assert!(temp_dir.path().join("new/nested/dir").is_dir());
    }

    #[tokio::test]
    async fn test_move_directory_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let source_dir = Path::new("source_dir");
        let dest_dir = Path::new("dest_dir");
        let file_in_dir = source_dir.join("file.txt");

        // Create the directory by creating a file inside it.
        service
            .create_file(&file_in_dir, Some("content"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Perform a dry-run move
        service
            .rename_directory_with_imports(source_dir, dest_dir, true, false, None, false)
            .await
            .unwrap();

        assert_eq!(queue.queue_size().await, 0);

        // Verify that the source directory still exists and the destination does not.
        assert!(temp_dir.path().join(source_dir).exists());
        assert!(temp_dir.path().join(file_in_dir).exists());
        assert!(!temp_dir.path().join(dest_dir).exists());
    }

    #[tokio::test]
    async fn test_move_directory_execution() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let source_dir = Path::new("source_dir");
        let dest_dir = Path::new("dest_dir");
        let file_in_source = source_dir.join("file.txt");
        let file_in_dest = dest_dir.join("file.txt");

        service
            .create_file(&file_in_source, Some("content"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Perform a real move
        service
            .rename_directory_with_imports(source_dir, dest_dir, false, false, None, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Verify that the source is gone and the destination exists.
        assert!(!temp_dir.path().join(source_dir).exists());
        assert!(!temp_dir.path().join(file_in_source).exists());
        assert!(temp_dir.path().join(dest_dir).exists());
        assert!(temp_dir.path().join(file_in_dest).exists());
    }

    #[tokio::test]
    async fn test_move_directory_collision_detection() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let source_dir = Path::new("source_dir");
        let dest_dir = Path::new("dest_dir");

        // Create directories by creating files within them
        service
            .create_file(&source_dir.join("dummy.txt"), Some(""), false, false)
            .await
            .unwrap();
        service
            .create_file(&dest_dir.join("dummy.txt"), Some(""), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Attempt to move, expecting a collision error.
        let result = service
            .rename_directory_with_imports(source_dir, dest_dir, false, false, None, false)
            .await;

        assert!(result.is_err());
        if let Err(ApiError::AlreadyExists(path_str)) = result {
            // The error message contains the full absolute path.
            // We'll just check that it contains the destination path we expect.
            assert!(path_str.contains(dest_dir.to_str().unwrap()));
        } else {
            panic!("Expected AlreadyExists error, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_move_directory_with_nested_contents() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let source_dir = Path::new("source_dir");
        let dest_dir = Path::new("dest_dir");
        let file1_in_source = source_dir.join("file1.txt");
        let nested_dir_in_source = source_dir.join("nested");
        let file2_in_source = nested_dir_in_source.join("file2.txt");

        let file1_in_dest = dest_dir.join("file1.txt");
        let nested_dir_in_dest = dest_dir.join("nested");
        let file2_in_dest = nested_dir_in_dest.join("file2.txt");

        // Create nested structure
        service
            .create_file(&file1_in_source, Some("content1"), false, false)
            .await
            .unwrap();
        service
            .create_file(&file2_in_source, Some("content2"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Perform a real move
        service
            .rename_directory_with_imports(source_dir, dest_dir, false, false, None, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // Verify that the source is gone and the destination exists with all content.
        assert!(!temp_dir.path().join(source_dir).exists());
        assert!(!temp_dir.path().join(file1_in_source).exists());
        assert!(!temp_dir.path().join(nested_dir_in_source).exists());
        assert!(!temp_dir.path().join(file2_in_source).exists());

        assert!(temp_dir.path().join(dest_dir).exists());
        assert!(temp_dir.path().join(&file1_in_dest).exists());
        assert!(temp_dir.path().join(nested_dir_in_dest).is_dir());
        assert!(temp_dir.path().join(&file2_in_dest).exists());

        // Also check content
        let content1 = service.read_file(&file1_in_dest).await.unwrap();
        assert_eq!(content1, "content1");
        let content2 = service.read_file(&file2_in_dest).await.unwrap();
        assert_eq!(content2, "content2");
    }

    // A simple case-only rename test. The exact behavior can be filesystem-dependent.
    #[tokio::test]
    async fn test_case_only_rename() {
        let temp_dir = TempDir::new().unwrap();
        let (service, queue) = create_test_service(&temp_dir);

        let source_path = Path::new("file.txt");
        let dest_path = Path::new("File.txt");

        service
            .create_file(source_path, Some("content"), false, false)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        service
            .rename_file_with_imports(source_path, dest_path, false, None)
            .await
            .unwrap();
        queue.wait_until_idle().await;

        // On case-insensitive filesystems, old path might still appear to exist.
        // The most reliable check is that the new path exists and its content is correct.
        assert!(temp_dir.path().join(dest_path).exists());
        let content = service.read_file(dest_path).await.unwrap();
        assert_eq!(content, "content");
    }
}