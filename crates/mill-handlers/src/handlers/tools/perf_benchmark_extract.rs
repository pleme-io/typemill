#[cfg(test)]
mod tests {
    use crate::handlers::tools::workspace_extract::handle_extract_dependencies;
    use async_trait::async_trait;
    use chrono::Utc;
    use mill_foundation::core::dry_run::DryRunnable;
    use mill_foundation::errors::MillError;
    use mill_handler_api::{AppState, FileService, LanguagePluginRegistry, ToolHandlerContext};
    use mill_plugin_system::PluginManager;
    use serde_json::Value;
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Mutex;

    // --- Mocks ---

    struct DummyFileService;
    #[async_trait]
    impl FileService for DummyFileService {
        async fn read_file(&self, _: &Path) -> Result<String, MillError> {
            Ok(String::new())
        }
        async fn list_files(&self, _: &Path, _: bool) -> Result<Vec<String>, MillError> {
            Ok(vec![])
        }
        async fn write_file(
            &self,
            _: &Path,
            _: &str,
            _: bool,
        ) -> Result<DryRunnable<Value>, MillError> {
            Ok(DryRunnable::new(false, Value::Null))
        }
        async fn delete_file(
            &self,
            _: &Path,
            _: bool,
            _: bool,
        ) -> Result<DryRunnable<Value>, MillError> {
            Ok(DryRunnable::new(false, Value::Null))
        }
        async fn create_file(
            &self,
            _: &Path,
            _: Option<&str>,
            _: bool,
            _: bool,
        ) -> Result<DryRunnable<Value>, MillError> {
            Ok(DryRunnable::new(false, Value::Null))
        }
        async fn rename_file_with_imports(
            &self,
            _: &Path,
            _: &Path,
            _: bool,
            _: Option<mill_plugin_api::ScanScope>,
        ) -> Result<DryRunnable<Value>, MillError> {
            Ok(DryRunnable::new(false, Value::Null))
        }
        async fn rename_directory_with_imports(
            &self,
            _: &Path,
            _: &Path,
            _: bool,
            _: Option<mill_plugin_api::ScanScope>,
            _: bool,
        ) -> Result<DryRunnable<Value>, MillError> {
            Ok(DryRunnable::new(false, Value::Null))
        }
        async fn list_files_with_pattern(
            &self,
            _: &Path,
            _: bool,
            _: Option<&str>,
        ) -> Result<Vec<String>, MillError> {
            Ok(vec![])
        }
        fn to_absolute_path_checked(&self, path: &Path) -> Result<PathBuf, MillError> {
            Ok(path.to_path_buf())
        }
        async fn apply_edit_plan(
            &self,
            _: &mill_foundation::protocol::EditPlan,
        ) -> Result<mill_foundation::protocol::EditPlanResult, MillError> {
            Ok(mill_foundation::protocol::EditPlanResult {
                success: true,
                modified_files: vec![],
                errors: None,
                plan_metadata: mill_foundation::planning::EditPlanMetadata {
                    intent_name: "dummy".to_string(),
                    intent_arguments: Value::Null,
                    created_at: Utc::now(),
                    complexity: 0,
                    impact_areas: vec![],
                    consolidation: None,
                },
            })
        }
    }

    struct DummyPluginRegistry;
    impl LanguagePluginRegistry for DummyPluginRegistry {
        fn get_plugin(&self, _: &str) -> Option<&dyn mill_plugin_api::LanguagePlugin> {
            None
        }
        fn supported_extensions(&self) -> Vec<String> {
            vec![]
        }
        fn get_plugin_for_manifest(
            &self,
            _: &Path,
        ) -> Option<&dyn mill_plugin_api::LanguagePlugin> {
            None
        }
        fn inner(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn create_dummy_context(root: PathBuf) -> ToolHandlerContext {
        ToolHandlerContext {
            user_id: None,
            app_state: Arc::new(AppState {
                file_service: Arc::new(DummyFileService),
                language_plugins: Arc::new(DummyPluginRegistry),
                project_root: root,
                extensions: None,
            }),
            plugin_manager: Arc::new(PluginManager::new()),
            lsp_adapter: Arc::new(Mutex::new(None)),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn benchmark_extract_dependencies_blocking() {
        // Create a temporary directory
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create a LARGE source package.json (to cause slow I/O)
        let source_file = root.join("source_package.json");
        {
            let mut f = File::create(&source_file).unwrap();
            f.write_all(b"{\n  \"dependencies\": {\n").unwrap();
            for i in 0..500000 {
                f.write_all(format!("    \"dep-{}\": \"1.0.0\",\n", i).as_bytes())
                    .unwrap();
            }
            f.write_all(b"    \"final-dep\": \"1.0.0\"\n  }\n}").unwrap();
        }

        // Create target package.json
        let target_file = root.join("target_package.json");
        {
            let mut f = File::create(&target_file).unwrap();
            f.write_all(b"{\n  \"dependencies\": {}\n}").unwrap();
        }

        let context = create_dummy_context(root.clone());

        // Prepare arguments
        let args = serde_json::json!({
            "sourceManifest": source_file.to_str().unwrap(),
            "targetManifest": target_file.to_str().unwrap(),
            "dependencies": ["dep-1", "dep-100", "dep-1000"],
            "options": {
                "dryRun": false
            }
        });

        println!("Starting benchmark with large file...");
        let start = Instant::now();

        // Spawn a heartbeat task to detect blocking
        let heartbeat_handle = tokio::spawn(async move {
            let mut intervals = Vec::new();
            let mut last_tick = Instant::now();
            // Run for up to 2 seconds
            let end_time = Instant::now() + Duration::from_secs(2);

            while Instant::now() < end_time {
                // Sleep for a short interval
                tokio::time::sleep(Duration::from_millis(10)).await;
                let now = Instant::now();
                let elapsed = now.duration_since(last_tick);
                intervals.push(elapsed);
                last_tick = now;
            }
            intervals
        });

        // Run the extract operation
        let _result = handle_extract_dependencies(&context, args)
            .await
            .expect("Extraction failed");

        let duration = start.elapsed();
        println!("Operation time taken: {:.2?}", duration);

        // Analyze heartbeat
        let intervals = heartbeat_handle.await.unwrap();
        let max_interval = intervals.iter().max().unwrap();
        println!("Max heartbeat interval: {:.2?}", max_interval);

        // If blocking occurred, max_interval should be significantly larger than 10ms
        // e.g. > 50ms implies significant blocking of the runtime
    }
}
