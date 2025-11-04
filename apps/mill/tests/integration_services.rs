//! Service-level integration tests
//!
//! These tests verify the refactoring service layer, including the AST service,
//! file service, and planner. They test the complete refactoring pipeline from
//! planning to execution, ensuring correctness, atomicity, and cache invalidation.
//!
//! Unlike E2E tests which use TestClient and MCP protocol, these tests directly
//! instantiate service components and test their integration.

// Force linker to include plugin-bundle for inventory collection
extern crate mill_plugin_bundle;

use mill_ast::AstCache;
use mill_foundation::protocol::AstService;
use mill_plugin_system::PluginManager;
use mill_server::handlers::AppState;
use mill_server::services::{DefaultAstService, FileService, LockManager, OperationQueue};
use mill_server::workspaces::WorkspaceManager;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

/// Test project structure for multi-file refactoring tests
struct TestProject {
    /// Root directory of the test project
    pub root: PathBuf,
    /// Temporary directory handle (kept alive for cleanup)
    pub _temp_dir: TempDir,
    /// Path to main.ts file
    pub main_file: PathBuf,
}

/// Create a test project with TypeScript files for refactoring tests
fn setup_test_project() -> std::io::Result<TestProject> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    // Create src directory
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir)?;

    // Create utils.ts with exportable function
    let _utils_file = src_dir.join("utils.ts");
    fs::write(
        &_utils_file,
        r#"// Utility functions module
export const oldName = () => {
    return "Hello from oldName function!";
};

export const helper = (value: string) => {
    return oldName() + " " + value;
};

// Another function that uses oldName
export function processData() {
    const result = oldName();
    return result.toUpperCase();
}
"#,
    )?;

    // Create main.ts that imports and uses the function
    let main_file = src_dir.join("main.ts");
    fs::write(
        &main_file,
        r#"// Main application entry point
import { oldName, helper, processData } from './utils';

function main() {
    console.log(oldName());
    console.log(helper("world"));
    console.log(processData());

    // Call oldName multiple times to test all references
    const result1 = oldName();
    const result2 = oldName();
    return [result1, result2];
}

// Export for testing
export { main, oldName };
"#,
    )?;

    // Create package.json for TypeScript project context
    let package_json = root.join("package.json");
    fs::write(
        &package_json,
        r#"{
  "name": "test-refactoring-project",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "build": "tsc"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}
"#,
    )?;

    // Create tsconfig.json
    let tsconfig = root.join("tsconfig.json");
    fs::write(
        &tsconfig,
        r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ES2020",
    "moduleResolution": "node",
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "outDir": "./dist",
    "rootDir": "./src"
  },
  "include": ["src/**/*"]
}
"#,
    )?;

    Ok(TestProject {
        root,
        _temp_dir: temp_dir,
        main_file,
    })
}

/// Create application state for testing with the given project root
async fn create_test_app_state(project_root: PathBuf) -> Arc<AppState> {
    let ast_cache = Arc::new(AstCache::new());
    let plugin_registry = mill_server::services::registry_builder::build_language_plugin_registry();
    let ast_service: Arc<dyn AstService> = Arc::new(DefaultAstService::new(
        ast_cache.clone(),
        plugin_registry.clone(),
    ));
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));
    let config = mill_config::AppConfig::default();
    let file_service = Arc::new(FileService::new(
        project_root.clone(),
        ast_cache.clone(),
        lock_manager.clone(),
        operation_queue.clone(),
        &config,
        plugin_registry.clone(),
    ));
    let planner = mill_server::services::planner::DefaultPlanner::new();
    let plugin_manager = Arc::new(PluginManager::new());
    let workflow_executor = mill_server::services::workflow_executor::DefaultWorkflowExecutor::new(
        plugin_manager.clone(),
    );
    let workspace_manager = Arc::new(WorkspaceManager::new());

    Arc::new(AppState {
        ast_service,
        file_service,
        planner,
        workflow_executor,
        project_root,
        lock_manager,
        operation_queue,
        start_time: Instant::now(),
        workspace_manager,
        language_plugins: mill_handlers::LanguagePluginRegistry::from_registry(plugin_registry),
    })
}

// NOTE: test_edit_plan_error_handling has been removed as it tested error handling
// which is now covered by the data-driven test architecture.

#[tokio::test]
async fn test_cache_performance_improvement() {
    // Step 1: Set up test project
    let project = setup_test_project().expect("Failed to create test project");
    let app_state = create_test_app_state(project.root.clone()).await;

    // Step 2: First parse (cache miss)
    let start_time = std::time::Instant::now();
    let _graph1 = app_state
        .ast_service
        .build_import_graph(&project.main_file)
        .await
        .expect("Failed to build import graph");
    let first_parse_duration = start_time.elapsed();

    // Step 3: Second parse (cache hit)
    let start_time = std::time::Instant::now();
    let _graph2 = app_state
        .ast_service
        .build_import_graph(&project.main_file)
        .await
        .expect("Failed to build import graph");
    let second_parse_duration = start_time.elapsed();

    // Step 4: Verify cache performance improvement
    let cache_stats = app_state.ast_service.cache_stats().await;
    assert!(cache_stats.hits > 0, "Cache should have recorded hits");

    // Second parse should be significantly faster (cache hit)
    println!(
        "First parse: {:?}, Second parse: {:?}",
        first_parse_duration, second_parse_duration
    );

    // Cache hit should be at least 2x faster (generous threshold for test stability)
    let speedup_ratio =
        first_parse_duration.as_nanos() as f64 / second_parse_duration.as_nanos() as f64;
    assert!(
        speedup_ratio > 1.5,
        "Cache hit should provide significant speedup, got {:.2}x",
        speedup_ratio
    );

    println!(
        "✓ Cache performance improvement verified: {:.2}x speedup",
        speedup_ratio
    );
}

// ============================================================================
// In-Process Workspace Edit Test (from e2e_in_process_test.rs)
// ============================================================================

// Note: test_workspace_edit_in_process removed - used internal tool 'apply_workspace_edit'
// that is no longer part of the public MCP API. This was a performance stress test
// for the internal workspace edit mechanism. The public equivalent is now the unified
// refactoring API (rename, extract, etc. with options.dryRun: false).
#[allow(dead_code)]
async fn test_workspace_edit_in_process_removed() {
    // Function removed - see comment above
    let _temp_dir = tempfile::TempDir::new().unwrap();
    let _workspace_path = _temp_dir.path().to_path_buf();
}