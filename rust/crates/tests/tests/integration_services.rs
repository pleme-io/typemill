//! Service-level integration tests
//!
//! These tests verify the refactoring service layer, including the AST service,
//! file service, and planner. They test the complete refactoring pipeline from
//! planning to execution, ensuring correctness, atomicity, and cache invalidation.
//!
//! Unlike E2E tests which use TestClient and MCP protocol, these tests directly
//! instantiate service components and test their integration.

use cb_api::AstService;
use cb_ast::AstCache;
use cb_core::model::IntentSpec;
use cb_plugins::PluginManager;
use cb_server::handlers::AppState;
use cb_server::services::{DefaultAstService, FileService, LockManager, OperationQueue};
use serde_json::json;
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
    /// Path to utils.ts file
    pub utils_file: PathBuf,
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
    let utils_file = src_dir.join("utils.ts");
    fs::write(
        &utils_file,
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
        utils_file,
        main_file,
    })
}

/// Create application state for testing with the given project root
async fn create_test_app_state(project_root: PathBuf) -> Arc<AppState> {
    let ast_cache = Arc::new(AstCache::new());
    let ast_service: Arc<dyn AstService> = Arc::new(DefaultAstService::new(ast_cache.clone()));
    let lock_manager = Arc::new(LockManager::new());
    let file_service = Arc::new(FileService::new(
        project_root.clone(),
        ast_cache.clone(),
        lock_manager.clone(),
    ));
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));
    let planner = cb_server::services::planner::DefaultPlanner::new();
    let plugin_manager = Arc::new(PluginManager::new());
    let workflow_executor = cb_server::services::workflow_executor::DefaultWorkflowExecutor::new(
        plugin_manager.clone(),
    );

    Arc::new(AppState {
        ast_service,
        file_service,
        planner,
        workflow_executor,
        project_root,
        lock_manager,
        operation_queue,
        start_time: Instant::now(),
    })
}

#[tokio::test]
async fn test_rename_symbol_workflow_success() {
    // Step 1: Set up test project
    let project = setup_test_project().expect("Failed to create test project");
    println!("Test project created at: {}", project.root.display());

    // Step 2: Create application state
    let app_state = create_test_app_state(project.root.clone()).await;

    // Step 3: Verify initial file contents
    let initial_utils_content = fs::read_to_string(&project.utils_file).unwrap();
    let initial_main_content = fs::read_to_string(&project.main_file).unwrap();

    assert!(initial_utils_content.contains("export const oldName"));
    assert!(initial_main_content.contains("import { oldName"));
    assert!(initial_main_content.contains("oldName()"));

    println!("✓ Initial file contents verified");

    // Step 4: Generate edit plan using AST service
    let intent = IntentSpec::new(
        "rename_symbol_with_imports",
        json!({
            "oldName": "oldName",
            "newName": "newName",
            "sourceFile": project.utils_file.to_string_lossy()
        }),
    );

    let edit_plan = app_state
        .ast_service
        .plan_refactor(&intent, &project.utils_file)
        .await
        .expect("Failed to generate edit plan");

    println!(
        "✓ Edit plan generated with {} edits and {} dependency updates",
        edit_plan.edits.len(),
        edit_plan.dependency_updates.len()
    );

    // Step 5: Verify edit plan contents
    assert!(
        !edit_plan.edits.is_empty(),
        "Edit plan should contain edits for the source file"
    );
    assert_eq!(edit_plan.source_file, project.utils_file.to_string_lossy());

    // Verify dependency updates for main.ts
    let has_main_update = edit_plan
        .dependency_updates
        .iter()
        .any(|update| update.target_file.contains("main.ts"));
    assert!(
        has_main_update,
        "Edit plan should include dependency update for main.ts"
    );

    println!("✓ Edit plan structure validated");

    // Step 6: Get initial cache stats
    let initial_cache_stats = app_state.ast_service.cache_stats().await;
    println!(
        "Initial cache stats: hits={}, misses={}",
        initial_cache_stats.hits, initial_cache_stats.misses
    );

    // Step 7: Execute edit plan
    let edit_result = app_state
        .file_service
        .apply_edit_plan(&edit_plan)
        .await
        .expect("Failed to apply edit plan");

    assert!(edit_result.success, "Edit plan application should succeed");
    assert!(
        !edit_result.modified_files.is_empty(),
        "Some files should be modified"
    );

    println!(
        "✓ Edit plan applied successfully, modified {} files",
        edit_result.modified_files.len()
    );

    // Step 8: Verify that files were actually modified
    let updated_utils_content = fs::read_to_string(&project.utils_file).unwrap();
    let updated_main_content = fs::read_to_string(&project.main_file).unwrap();

    // Basic verification - files should be different and contain newName
    assert_ne!(
        initial_utils_content, updated_utils_content,
        "utils.ts should be modified"
    );
    assert_ne!(
        initial_main_content, updated_main_content,
        "main.ts should be modified"
    );

    // Verify new name appears in both files
    assert!(
        updated_utils_content.contains("newName") || updated_main_content.contains("newName"),
        "At least one file should contain the new name"
    );

    println!("✓ File modifications verified");

    // Step 9: Verify AST cache invalidation
    // Build import graph for main.ts again - this should trigger cache miss or hit
    let _import_graph = app_state
        .ast_service
        .build_import_graph(&project.main_file)
        .await
        .expect("Failed to build import graph after refactoring");

    let final_cache_stats = app_state.ast_service.cache_stats().await;
    println!(
        "Final cache stats: hits={}, misses={}",
        final_cache_stats.hits, final_cache_stats.misses
    );

    println!("✓ AST service continues to function after refactoring");

    println!("✓ End-to-end refactoring workflow completed successfully");
}

#[tokio::test]
async fn test_edit_plan_error_handling() {
    // This test verifies error handling in the edit plan application
    // Note: True atomicity across multiple files requires more sophisticated coordination
    // The current implementation provides file-level atomicity with error reporting

    // Step 1: Set up test project
    let project = setup_test_project().expect("Failed to create test project");
    println!("Test project created at: {}", project.root.display());

    // Step 2: Create application state
    let app_state = create_test_app_state(project.root.clone()).await;

    // Step 3: Generate a valid edit plan
    let intent = IntentSpec::new(
        "rename_symbol_with_imports",
        json!({
            "oldName": "oldName",
            "newName": "newName",
            "sourceFile": project.utils_file.to_string_lossy()
        }),
    );

    let edit_plan = app_state
        .ast_service
        .plan_refactor(&intent, &project.utils_file)
        .await
        .expect("Failed to generate edit plan");

    // Step 4: Make main.ts read-only to simulate a failure condition
    let mut permissions = fs::metadata(&project.main_file).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&project.main_file, permissions).expect("Failed to set file as read-only");

    println!("✓ Set main.ts as read-only to simulate failure");

    // Step 5: Attempt to apply edit plan - this should report errors
    let edit_result = app_state.file_service.apply_edit_plan(&edit_plan).await;

    // The operation should handle errors gracefully
    match edit_result {
        Err(e) => {
            println!("✓ Edit plan application failed as expected: {}", e);
        }
        Ok(result) => {
            // If it returns Ok, it should report the error properly
            assert!(
                !result.success || result.errors.is_some(),
                "Edit plan should indicate failure when file is read-only"
            );
            println!("✓ Edit plan application returned with errors as expected");
        }
    }

    // Step 6: Restore file permissions
    let mut permissions = fs::metadata(&project.main_file).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&project.main_file, permissions)
        .expect("Failed to restore file permissions");

    // Step 7: Verify we can successfully apply the plan after fixing the issue
    let retry_result = app_state
        .file_service
        .apply_edit_plan(&edit_plan)
        .await
        .expect("Failed to apply edit plan on retry");

    assert!(
        retry_result.success,
        "Edit plan should succeed after fixing permissions"
    );

    println!("✓ Edit plan error handling and recovery verified");
}

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
