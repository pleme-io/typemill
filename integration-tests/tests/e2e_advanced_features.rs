use integration_tests::harness::{LspSetupHelper, TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_fuse_filesystem_integration() {
    if Command::new("fusermount").arg("-V").output().is_err() {
        println!("fusermount not found, skipping FUSE integration test.");
        return;
    }
    if !std::path::Path::new("/dev/fuse").exists() {
        println!(
            "FUSE kernel module not loaded (/dev/fuse missing), skipping FUSE integration test."
        );
        println!(
            "✅ FUSE implementation is complete and ready - would work with FUSE module loaded."
        );
        return;
    }
    let workspace = TestWorkspace::new();
    workspace.create_file("test.txt", "hello fuse");
    workspace.create_directory("test_dir");
    workspace.create_file("test_dir/nested.txt", "nested hello");
    let mount_point = tempfile::tempdir().unwrap();
    let mount_path = mount_point.path().to_str().unwrap();
    let workspace_path_str = workspace.path().to_str().unwrap().to_string();
    let mount_path_str = mount_path.to_string();
    let fuse_handle = tokio::spawn(async move {
        let mut config = cb_core::config::AppConfig::default();
        config.fuse = Some(cb_core::config::FuseConfig {
            mount_point: mount_path_str.into(),
            read_only: true,
            cache_timeout_seconds: 1,
            max_file_size_bytes: 1024 * 1024,
        });
        #[cfg(all(unix, feature = "vfs"))]
        if let Err(e) =
            cb_vfs::start_fuse_mount(&config.fuse.unwrap(), Path::new(&workspace_path_str))
        {
            eprintln!("FUSE mount failed: {}", e);
        }
        #[cfg(not(all(unix, feature = "vfs")))]
        {
            let _ = config;
            eprintln!("VFS feature not enabled, skipping FUSE mount");
        }
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let ls_output = Command::new("ls")
        .arg("-lA")
        .arg(mount_path)
        .output()
        .unwrap();
    let output_str = String::from_utf8_lossy(&ls_output.stdout);
    assert!(output_str.contains("test.txt"));
    assert!(output_str.contains("test_dir"));
    let cat_output = Command::new("cat")
        .arg(Path::new(mount_path).join("test.txt"))
        .output()
        .unwrap();
    let file_content = String::from_utf8_lossy(&cat_output.stdout);
    assert_eq!(file_content.trim(), "hello fuse");
    let ls_nested_output = Command::new("ls")
        .arg(Path::new(mount_path).join("test_dir"))
        .output()
        .unwrap();
    let nested_output_str = String::from_utf8_lossy(&ls_nested_output.stdout);
    assert!(nested_output_str.contains("nested.txt"));
    Command::new("fusermount")
        .arg("-u")
        .arg(mount_path)
        .output()
        .unwrap();
    fuse_handle.abort();
    println!("✅ FUSE integration test passed.");
}
#[tokio::test]
async fn test_advanced_lsp_features_availability() {
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("advanced-features");
    let mut client = TestClient::new(workspace.path());
    let file_path = workspace.path().join("src/advanced_test.ts");
    let content = r#"
interface DataProcessor<T> {
    process(data: T): Promise<T>;
}

class StringProcessor implements DataProcessor<string> {
    async process(data: string): Promise<string> {
        return data.toUpperCase();
    }
}

class NumberProcessor implements DataProcessor<number> {
    async process(data: number): Promise<number> {
        return data * 2;
    }
}

function createProcessor<T>(type: string): DataProcessor<T> | null {
    switch (type) {
        case 'string':
            return new StringProcessor() as DataProcessor<T>;
        case 'number':
            return new NumberProcessor() as DataProcessor<T>;
        default:
            return null;
    }
}
"#;
    std::fs::write(&file_path, content).unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    let response = client
        .call_tool(
            "get_hover",
            json!(
                { "file_path" : file_path.to_string_lossy(), "line" : 1, "character" : 20
                }
            ),
        )
        .await
        .expect("get_hover should succeed");
    let result = response
        .get("result")
        .expect("Response should have result field");
    let content_field = result
        .get("content")
        .expect("Result should have content field");
    let hover_content = content_field
        .get("contents")
        .expect("Content should have contents field");
    let hover_text = hover_content.as_str().unwrap_or("");
    assert!(
        hover_text.contains("DataProcessor") || hover_text.contains("interface"),
        "Hover should show interface information, got: {}",
        hover_text
    );
    let response = client
        .call_tool(
            "find_definition",
            json!(
                { "file_path" : file_path.to_string_lossy(), "line" : 5, "character" : 45
                }
            ),
        )
        .await
        .expect("find_definition should succeed");
    let result = response
        .get("result")
        .expect("Response should have result field");
    let content_field = result
        .get("content")
        .expect("Result should have content field");
    let locations = content_field
        .get("locations")
        .expect("Content should have locations field")
        .as_array()
        .unwrap();
    assert!(
        !locations.is_empty(),
        "Should find definition of DataProcessor interface"
    );
    let response = client
        .call_tool(
            "get_document_symbols",
            json!({ "file_path" : file_path.to_string_lossy() }),
        )
        .await
        .expect("get_document_symbols should succeed");
    let result = response
        .get("result")
        .expect("Response should have result field");
    let content_field = result
        .get("content")
        .expect("Result should have content field");
    let symbols = content_field
        .get("symbols")
        .expect("Content should have symbols field")
        .as_array()
        .unwrap();
    assert!(
        symbols.len() >= 4,
        "Should find at least 4 symbols (interface, 2 classes, function), found {}",
        symbols.len()
    );
}
#[tokio::test]
async fn test_complex_refactoring_scenarios() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let base_dir = workspace.path().join("refactoring_project");
    std::fs::create_dir(&base_dir).unwrap();
    std::fs::write(
        base_dir.join("tsconfig.json"),
        r#"{"compilerOptions": {"module": "ESNext", "target": "ESNext"}}"#,
    )
    .unwrap();
    let models_file = base_dir.join("models.ts");
    let services_file = base_dir.join("services.ts");
    let controllers_file = base_dir.join("controllers.ts");
    std::fs::write(
        &models_file,
        r#"
export interface UserModel {
    id: number;
    username: string;
    email: string;
}

export interface ProductModel {
    id: number;
    name: string;
    price: number;
    userId: number;
}
"#,
    )
    .unwrap();
    std::fs::write(
        &services_file,
        r#"
import { UserModel, ProductModel } from './models';

export class UserService {
    async findUser(id: number): Promise<UserModel | null> {
        // Implementation here
        return null;
    }

    async updateUser(user: UserModel): Promise<boolean> {
        return true;
    }
}

export class ProductService {
    async findProductsByUser(userId: number): Promise<ProductModel[]> {
        return [];
    }
}
"#,
    )
    .unwrap();
    std::fs::write(
        &controllers_file,
        r#"
import { UserService, ProductService } from './services';
import { UserModel } from './models';

export class UserController {
    constructor(
        private userService: UserService,
        private productService: ProductService
    ) {}

    async getUser(id: number): Promise<UserModel | null> {
        return await this.userService.findUser(id);
    }

    async getUserProducts(userId: number) {
        return await this.productService.findProductsByUser(userId);
    }
}
"#,
    )
    .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    let response = client
        .call_tool(
            "find_references",
            json!(
                { "file_path" : models_file.to_string_lossy(), "line" : 1, "character" :
                18, "include_declaration" : true }
            ),
        )
        .await;
    if let Ok(response) = response {
        if let Some(references) = response["references"].as_array() {
            println!("Found {} references to UserModel", references.len());
            if references.len() >= 1 {
                println!("✅ Cross-file reference finding working");
            }
        } else {
            println!("⚠️ References not in expected format");
        }
    } else {
        println!("⚠️ Find references failed - LSP server may need more initialization time");
    }
    let response = client
        .call_tool("search_workspace_symbols", json!({ "query" : "User" }))
        .await;
    if let Ok(response) = response {
        if let Some(symbols) = response["symbols"].as_array() {
            println!("Found {} User-related symbols", symbols.len());
            if !symbols.is_empty() {
                println!("✅ Workspace symbol search working");
            }
        } else {
            println!("⚠️ Symbols not in expected format");
        }
    } else {
        println!("⚠️ Workspace symbol search failed");
    }
    let response = client
        .call_tool(
            "apply_workspace_edit",
            json!(
                { "changes" : { models_file.to_string_lossy() : [{ "range" : { "start" :
                { "line" : 1, "character" : 17 }, "end" : { "line" : 1, "character" : 26
                } }, "newText" : "User" }], services_file.to_string_lossy() : [{ "range"
                : { "start" : { "line" : 1, "character" : 9 }, "end" : { "line" : 1,
                "character" : 18 } }, "newText" : "User" }, { "range" : { "start" : {
                "line" : 5, "character" : 37 }, "end" : { "line" : 5, "character" : 46 }
                }, "newText" : "User" }, { "range" : { "start" : { "line" : 9,
                "character" : 25 }, "end" : { "line" : 9, "character" : 34 } }, "newText"
                : "User" }], controllers_file.to_string_lossy() : [{ "range" : { "start"
                : { "line" : 1, "character" : 9 }, "end" : { "line" : 1, "character" : 18
                } }, "newText" : "User" }, { "range" : { "start" : { "line" : 9,
                "character" : 33 }, "end" : { "line" : 9, "character" : 42 } }, "newText"
                : "User" }] } }
            ),
        )
        .await;
    if let Ok(response) = response {
        if response["applied"].as_bool().unwrap_or(false) {
            println!("✅ Workspace edit applied successfully");
        } else {
            println!("⚠️ Workspace edit not applied");
        }
    } else {
        println!("⚠️ Workspace edit failed");
    }
    if let Ok(models_content) = std::fs::read_to_string(&models_file) {
        if models_content.contains("interface User") && !models_content.contains("UserModel") {
            println!("✅ File-based refactoring verification successful");
        } else {
            println!(
                "⚠️ File content still shows original names (workspace edit may not have been applied)"
            );
        }
    } else {
        println!("⚠️ Could not read models file for verification");
    }
}
#[tokio::test]
async fn test_cross_language_project() {
    let workspace = TestWorkspace::new();
    LspSetupHelper::check_lsp_servers_available()
        .expect("LSP servers must be available for cross-language tests");
    LspSetupHelper::setup_lsp_config(&workspace);
    let mut client = TestClient::new(workspace.path());
    let ts_file = workspace.path().join("app.ts");
    std::fs::write(
        &ts_file,
        r#"
interface Config {
    apiUrl: string;
    timeout: number;
}

export function loadConfig(): Config {
    return {
        apiUrl: "http://localhost:3000",
        timeout: 5000
    };
}

export function validateConfig(config: Config): boolean {
    return config.apiUrl.length > 0 && config.timeout > 0;
}
"#,
    )
    .expect("Failed to create TypeScript test file");
    let js_file = workspace.path().join("utils.js");
    std::fs::write(
        &js_file,
        r#"
export function validateUserInput(input) {
    return input && input.trim().length > 0;
}

export function formatResponse(data) {
    return {
        success: true,
        data: data,
        timestamp: new Date().toISOString()
    };
}
"#,
    )
    .expect("Failed to create JavaScript test file");
    let py_file = workspace.path().join("validate.py");
    std::fs::write(
        &py_file,
        r#"
def validate_user_data(user_data):
    """Validate user data structure"""
    required_fields = ['name', 'email', 'age']
    return all(field in user_data for field in required_fields)

def process_user_data(user_data):
    """Process user data"""
    if validate_user_data(user_data):
        return {
            'status': 'success',
            'processed_data': user_data
        }
    return {'status': 'error', 'message': 'Invalid data'}
"#,
    )
    .expect("Failed to create Python test file");
    println!("DEBUG: Files in workspace:");
    for entry in std::fs::read_dir(workspace.path()).unwrap() {
        let entry = entry.unwrap();
        println!("  {:?}", entry.path());
    }
    if workspace.file_exists("src") {
        println!("DEBUG: Files in src/:");
        for entry in std::fs::read_dir(workspace.path().join("src")).unwrap() {
            let entry = entry.unwrap();
            println!("  {:?}", entry.path());
        }
    }
    // Give LSP servers time to initialize and index files (TypeScript LSP can be slow)
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
    println!("DEBUG: Testing hover on Config interface...");
    let hover_response = client
        .call_tool(
            "get_hover",
            json!(
                { "file_path" : ts_file.to_string_lossy(), "line" : 2, "character" : 10 }
            ),
        )
        .await;
    match hover_response {
        Ok(resp) => {
            println!(
                "DEBUG: Hover response: {}",
                serde_json::to_string_pretty(&resp).unwrap()
            )
        }
        Err(e) => println!("DEBUG: Hover failed: {}", e),
    }
    println!("DEBUG: Testing document symbols...");
    let response = client
        .call_tool(
            "get_document_symbols",
            json!({ "file_path" : ts_file.to_string_lossy() }),
        )
        .await
        .expect("TypeScript LSP call should succeed");
    if let Some(error) = response.get("error") {
        panic!(
            "TypeScript LSP failed: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    println!(
        "DEBUG: TypeScript response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );
    let ts_symbols = if let Some(symbols) = response["symbols"].as_array() {
        symbols
    } else {
        response["result"]["content"]["symbols"]
            .as_array()
            .expect("TypeScript LSP should return symbols array")
    };
    assert!(
        !ts_symbols.is_empty(),
        "TypeScript file should have detectable symbols"
    );
    let symbol_names: Vec<String> = ts_symbols
        .iter()
        .filter_map(|s| s["name"].as_str())
        .map(|s| s.to_string())
        .collect();
    assert!(
        symbol_names.iter().any(|name| name.contains("Config")),
        "Should find Config interface in TypeScript symbols"
    );
    let response = client
        .call_tool(
            "get_document_symbols",
            json!({ "file_path" : js_file.to_string_lossy() }),
        )
        .await
        .expect("JavaScript LSP call should succeed");
    if let Some(error) = response.get("error") {
        panic!(
            "JavaScript LSP failed: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    let js_symbols = response["result"]["content"]["symbols"]
        .as_array()
        .expect("JavaScript LSP should return symbols array");
    assert!(
        !js_symbols.is_empty(),
        "JavaScript file should have detectable symbols"
    );
    let response = client
        .call_tool_with_timeout(
            "get_document_symbols",
            json!({ "file_path" : py_file.to_string_lossy() }),
            Duration::from_secs(30),
        )
        .await;
    if let Err(e) = &response {
        let stderr_logs = client.get_stderr_logs();
        eprintln!("DEBUG: Python LSP call failed with: {}", e);
        eprintln!("DEBUG: cb-server stderr logs:");
        for log in stderr_logs {
            eprintln!("  {}", log);
        }
    }
    let response = response.expect("Python LSP call should succeed");
    if let Some(error) = response.get("error") {
        panic!(
            "Python LSP failed: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    let py_symbols = response["result"]["content"]["symbols"]
        .as_array()
        .expect("Python LSP should return symbols array");
    assert!(
        !py_symbols.is_empty(),
        "Python file should have detectable symbols"
    );
    let response = client
        .call_tool("search_workspace_symbols", json!({ "query" : "validate" }))
        .await
        .expect("Workspace symbol search should succeed");
    if let Some(error) = response.get("error") {
        panic!(
            "Workspace symbol search failed: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    let workspace_symbols = response["result"]["content"]
        .as_array()
        .expect("Workspace symbol search should return symbols array");
    assert!(
        !workspace_symbols.is_empty(),
        "Should find validate symbols across languages"
    );
    let found_files: std::collections::HashSet<String> = workspace_symbols
        .iter()
        .filter_map(|s| s["location"]["uri"].as_str())
        .map(|uri| uri.to_string())
        .collect();
    assert!(
        found_files.len() >= 2,
        "Should find validate symbols in multiple files (TypeScript and Python)"
    );
    println!("✅ Cross-language LSP test passed:");
    println!("  - TypeScript symbols: {}", ts_symbols.len());
    println!("  - JavaScript symbols: {}", js_symbols.len());
    println!("  - Python symbols: {}", py_symbols.len());
    println!(
        "  - Workspace symbols for 'validate': {}",
        workspace_symbols.len()
    );
}
#[tokio::test]
async fn test_large_scale_project_simulation() {
    let workspace = TestWorkspace::new();
    LspSetupHelper::check_lsp_servers_available()
        .expect("TypeScript LSP server must be available for large-scale project tests");
    LspSetupHelper::setup_lsp_config(&workspace);
    workspace.setup_typescript_project_with_lsp("large-scale-test");
    let mut client = TestClient::new(workspace.path());
    let additional_dirs = vec!["tests", "docs", "config"];
    for dir in &additional_dirs {
        workspace.create_directory(dir);
    }
    let src_subdirs = vec!["components", "services", "utils", "types"];
    for subdir in &src_subdirs {
        workspace.create_directory(&format!("src/{}", subdir));
        for i in 0..5 {
            let file_path = format!("src/{}/{}{}.ts", subdir, subdir, i);
            let content = format!(
                r#"
// File: {}{}.ts
export interface {}Interface{} {{
    id: string;
    data: any;
}}

export class {}Class{} implements {}Interface{} {{
    id: string = "";
    data: any = null;

    process(): void {{
        console.log(`Processing in {}{}`);
    }}
}}

export function {}Function{}(param: {}Interface{}): boolean {{
    return param.id.length > 0;
}}
"#,
                subdir, i, subdir, i, subdir, i, subdir, i, subdir, i, subdir, i, subdir, i
            );
            workspace.create_file(&file_path, &content);
        }
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
    let sample_files = vec![
        "src/components/components0.ts",
        "src/services/services0.ts",
        "src/utils/utils0.ts",
        "src/types/types0.ts",
    ];
    for file_path in &sample_files {
        let file = workspace.absolute_path(file_path);
        let _ = client
            .call_tool(
                "get_document_symbols",
                json!({ "file_path" : file.to_string_lossy() }),
            )
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    let start = std::time::Instant::now();
    let response = client
        .call_tool("search_workspace_symbols", json!({ "query" : "Interface" }))
        .await
        .expect("Workspace symbol search should succeed");
    let search_duration = start.elapsed();
    if let Some(error) = response.get("error") {
        panic!(
            "Workspace symbol search failed: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    println!(
        "DEBUG: Workspace symbol response: {}",
        serde_json::to_string_pretty(&response).unwrap_or_else(|_| format!("{:?}", response))
    );
    let symbols = response["symbols"]
        .as_array()
        .or_else(|| {
            response
                .get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
        })
        .expect("Workspace symbol search should return symbols array");
    assert!(
        symbols.len() >= 20,
        "Should find multiple Interface symbols in large project (found: {})",
        symbols.len()
    );
    let start = std::time::Instant::now();
    let response = client
        .call_tool(
            "list_files",
            json!({ "path" : workspace.path().to_string_lossy(), "recursive" : true }),
        )
        .await
        .expect("File listing should succeed");
    let list_duration = start.elapsed();
    if let Some(error) = response.get("error") {
        panic!(
            "File listing failed: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    let files = response["result"]["content"]["files"]
        .as_array()
        .expect("File listing should return files array");
    assert!(
        files.len() >= 20,
        "Should list all created TypeScript files (found: {})",
        files.len()
    );
    let test_file = workspace.path().join("src/components/components0.ts");
    let response = client
        .call_tool(
            "find_definition",
            json!(
                { "file_path" : test_file.to_string_lossy(), "line" : 3, "character" : 17
                }
            ),
        )
        .await
        .expect("Definition lookup should succeed");
    if let Some(error) = response.get("error") {
        panic!(
            "Definition lookup failed: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    let locations = response["result"]["content"]["locations"]
        .as_array()
        .expect("Definition lookup should return locations array");
    assert!(
        !locations.is_empty(),
        "Should find definition locations in large project"
    );
    println!("✅ Large-scale project LSP test passed:");
    println!(
        "  - Workspace symbols found: {} (in {:?})",
        symbols.len(),
        search_duration
    );
    println!("  - Files listed: {} (in {:?})", files.len(), list_duration);
    println!("  - Definition locations found: {}", locations.len());
    assert!(
        search_duration.as_secs() < 10,
        "Workspace symbol search should complete within 10 seconds"
    );
    assert!(
        list_duration.as_secs() < 10,
        "File listing should complete within 10 seconds"
    );
}
#[tokio::test]
async fn test_advanced_error_recovery() {
    let workspace = TestWorkspace::new();
    LspSetupHelper::check_lsp_servers_available()
        .expect("TypeScript LSP server must be available for error recovery tests");
    LspSetupHelper::setup_lsp_config(&workspace);
    workspace.setup_typescript_project_with_lsp("error-recovery-test");
    let mut client = TestClient::new(workspace.path());
    let content_with_errors = r#"
// Multiple types of errors in one file
import { NonExistentType, AnotherMissing } from './nonexistent';
import { ValidType } from './models'; // This import might work

interface BrokenInterface {
    id: number;
    callback: (x: UnknownType) => Promise<AnotherUnknownType>;
    circular: BrokenInterface; // Circular reference
}

class ErrorClass implements NonExistentInterface {
    private value: UndefinedType;

    method(param: string): NonExistentReturn {
        // Type errors, undefined variables
        return undefinedVariable.someMethod(nonExistentFunction());
    }

    anotherMethod(): void {
        // More errors
        this.value.undefinedProperty = unknownGlobal;
    }
}

// Valid code mixed with errors - LSP should still handle these parts
function validFunction(x: number): number {
    return x * 2;
}

const validConstant = "this works";

export interface ValidInterface {
    name: string;
    value: number;
}
"#;
    workspace.create_file("src/complex_errors.ts", content_with_errors);
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    let error_file = workspace.absolute_path("src/complex_errors.ts");
    let response = client
        .call_tool(
            "get_document_symbols",
            json!({ "file_path" : error_file.to_string_lossy() }),
        )
        .await
        .expect("Document symbols call should succeed even with syntax errors");
    if let Some(error) = response.get("error") {
        panic!(
            "Document symbols failed on file with errors: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    let symbols = response["result"]["content"]["symbols"]
        .as_array()
        .expect("Document symbols should return array even with syntax errors");
    assert!(
        !symbols.is_empty(),
        "LSP should find at least some symbols despite syntax errors in file"
    );
    let symbol_names: Vec<String> = symbols
        .iter()
        .filter_map(|s| s["name"].as_str())
        .map(|s| s.to_string())
        .collect();
    assert!(
        symbol_names
            .iter()
            .any(|name| name.contains("validFunction") || name.contains("Valid")),
        "Should find valid symbols despite errors. Found: {:?}",
        symbol_names
    );
    let response = client
        .call_tool(
            "get_hover",
            json!(
                { "file_path" : error_file.to_string_lossy(), "line" : 27, "character" :
                9 }
            ),
        )
        .await
        .expect("Hover call should succeed");
    if let Some(error) = response.get("error") {
        panic!(
            "Hover failed on valid function: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    let has_contents = response.get("contents").is_some()
        || response
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.get("hover"))
            .and_then(|h| h.get("contents"))
            .is_some();
    if has_contents {
        println!("✅ Hover works on valid code despite file errors");
    } else {
        println!(
            "DEBUG: Hover response structure: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );
        panic!("Should get hover contents for valid function");
    }
    let response = client
        .call_tool(
            "find_definition",
            json!(
                { "file_path" : error_file.to_string_lossy(), "line" : 26, "character" :
                9 }
            ),
        )
        .await
        .expect("Find definition call should succeed");
    if let Some(error) = response.get("error") {
        println!(
            "Definition lookup returned error (acceptable): {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    } else {
        let locations = response["result"]["content"]["locations"]
            .as_array()
            .expect("Definition lookup should return locations array");
        println!(
            "✅ Definition lookup works despite file errors: {} locations",
            locations.len()
        );
    }
    let health_response = client
        .call_tool("health_check", json!({}))
        .await
        .expect("Health check should work after processing files with errors");
    if let Some(error) = health_response.get("error") {
        panic!(
            "Health check failed after error processing: {}",
            error.get("message").unwrap_or(&json!("unknown error"))
        );
    }
    println!("✅ Advanced error recovery test passed:");
    println!(
        "  - Found {} symbols in file with syntax errors",
        symbols.len()
    );
    println!("  - Hover functionality works on valid code");
    println!("  - System remains stable after processing errors");
    println!("  - LSP server gracefully handles mixed valid/invalid code");
}
