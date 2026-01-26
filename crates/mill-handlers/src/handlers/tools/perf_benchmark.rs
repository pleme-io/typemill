#[cfg(test)]
mod tests {
    use crate::handlers::tools::cross_file_references::discover_importing_files;
    use async_trait::async_trait;
    use chrono::Utc;
    use mill_foundation::core::dry_run::DryRunnable;
    use mill_foundation::errors::MillError;
    use mill_handler_api::{
        AnalysisConfigTrait, AppState, FileService, LanguagePluginRegistry,
        ToolHandlerContext,
    };
    use mill_plugin_system::PluginManager;
    use serde_json::Value;
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Instant;
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

    struct DummyAnalysisConfig;
    impl AnalysisConfigTrait for DummyAnalysisConfig {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn create_dummy_context() -> ToolHandlerContext {
        ToolHandlerContext {
            user_id: None,
            app_state: Arc::new(AppState {
                file_service: Arc::new(DummyFileService),
                language_plugins: Arc::new(DummyPluginRegistry),
                project_root: PathBuf::from("/"),
                extensions: None,
            }),
            plugin_manager: Arc::new(PluginManager::new()),
            lsp_adapter: Arc::new(Mutex::new(None)),
            analysis_config: Arc::new(DummyAnalysisConfig),
        }
    }

    // --- Benchmark ---

    #[tokio::test(flavor = "multi_thread")]
    async fn benchmark_discover_importing_files() {
        // Create a temporary directory
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();

        // Create a source file
        let source_file = root.join("source.ts");
        {
            let mut f = File::create(&source_file).unwrap();
            f.write_all(b"export const myVar = 42;").unwrap();
        }

        // Create 2000 files, some importing the source
        let num_files = 2000;
        let num_importers = 100;

        for i in 0..num_files {
            let p = root.join(format!("file_{}.ts", i));
            let mut f = File::create(&p).unwrap();

            if i < num_importers {
                // Importing file
                let content = "import { myVar } from './source';";
                f.write_all(content.as_bytes()).unwrap();
            } else {
                // Non-importing file
                let content = "const x = 10;";
                f.write_all(content.as_bytes()).unwrap();
            }
        }

        let context = create_dummy_context();

        println!("Starting benchmark with {} files...", num_files);
        let start = Instant::now();

        let result = discover_importing_files(root, &source_file, &context)
            .await
            .expect("Discovery failed");

        let duration = start.elapsed();
        println!("Time taken: {:.2?}", duration);
        println!("Found {} importing files", result.len());

        // We expect at least the importers to be found
        // Note: The logic might match heuristicly on filenames too, but we just want to measure performance.
        assert!(result.len() >= num_importers);
    }
}
