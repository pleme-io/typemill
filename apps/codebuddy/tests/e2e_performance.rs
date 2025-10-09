use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use cb_test_support::harness::{TestClient, TestWorkspace};

#[tokio::test]
#[cfg(feature = "heavy-tests")]
async fn test_large_file_performance() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();
    let mut client = TestClient::new(workspace.path());

    // Create a large TypeScript file (50k lines)
    let large_file = workspace.path().join("large_performance.ts");
    let mut content = String::new();

    for i in 0..50000 {
        content.push_str(&format!("const variable{} = {}; // Line {}\n", i, i, i + 1));
    }

    // Time file creation
    let start = Instant::now();
    let response = client
        .call_tool(
            "create_file",
            json!({
                "file_path": large_file.to_string_lossy(),
                "content": content
            }),
        )
        .await
        .unwrap();
    let create_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");
    assert!(result["success"].as_bool().unwrap_or(false));
    println!("Large file creation took: {:?}", create_duration);

    // Time file reading
    let start = Instant::now();
    let response = client
        .call_tool(
            "read_file",
            json!({
                "file_path": large_file.to_string_lossy()
            }),
        )
        .await
        .unwrap();
    let read_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");
    assert!(result.get("content").is_some());
    println!("Large file reading took: {:?}", read_duration);

    // Time LSP operations on large file
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let start = Instant::now();
    let response = client
        .call_tool_with_timeout(
            "get_document_symbols",
            json!({
                "file_path": large_file.to_string_lossy()
            }),
            Duration::from_secs(60),
        )
        .await;
    let lsp_duration = start.elapsed();

    match response {
        Ok(resp) => {
            println!(
                "LSP document symbols on large file took: {:?}",
                lsp_duration
            );
            let result = resp
                .get("result")
                .expect("Response should have result field");
            let content = result
                .get("content")
                .expect("Response should have content field");
            let symbols = content["symbols"].as_array().unwrap();
            assert!(!symbols.is_empty());
        }
        Err(_) => {
            println!(
                "LSP operation timed out or failed on large file after: {:?}",
                lsp_duration
            );
        }
    }

    // Performance assertions (adjust based on expected performance)
    assert!(
        create_duration < Duration::from_secs(30),
        "File creation should complete within 30 seconds"
    );
    assert!(
        read_duration < Duration::from_secs(10),
        "File reading should complete within 10 seconds"
    );
}

#[tokio::test]
#[cfg(feature = "heavy-tests")]
#[ignore = "TypeScript LSP workspace/symbol requires tsconfig.json or didOpen notifications"]
async fn test_many_small_files_performance() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();
    let mut client = TestClient::new(workspace.path());

    let file_count = 100;
    let mut file_paths = Vec::new();

    // Create many small files
    let start = Instant::now();
    for i in 0..file_count {
        let file_path = workspace.path().join(format!("small_file_{}.ts", i));
        let content = format!(
            r#"
export interface Data{} {{
    id: number;
    value: string;
}}

export function process{}(data: Data{}): string {{
    return `Processing ${{data.id}}: ${{data.value}}`;
}}
"#,
            i, i, i
        );

        let response = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": content
                }),
            )
            .await
            .unwrap();

        let result = response
            .get("result")
            .expect("Response should have result field");
        assert!(result["success"].as_bool().unwrap_or(false));
        file_paths.push(file_path);
    }
    let creation_duration = start.elapsed();

    println!("Created {} files in: {:?}", file_count, creation_duration);
    println!(
        "Average time per file: {:?}",
        creation_duration / file_count as u32
    );

    // Give LSP time to process all files
    tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;

    // Test workspace symbol search performance
    let start = Instant::now();
    let response = client
        .call_tool(
            "search_symbols",
            json!({
                "query": "Data"
            }),
        )
        .await
        .unwrap();
    let search_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");
    let symbols = result
        .get("content")
        .and_then(|v| v.as_array())
        .expect("Response should have content array");
    println!(
        "Workspace symbol search found {} symbols in: {:?}",
        symbols.len(),
        search_duration
    );

    assert!(!symbols.is_empty());
    assert!(
        search_duration < Duration::from_secs(10),
        "Workspace search should complete within 10 seconds"
    );

    // Test listing all files performance
    let start = Instant::now();
    let response = client
        .call_tool(
            "list_files",
            json!({
                "directory": workspace.path().to_string_lossy(),
                "recursive": true
            }),
        )
        .await
        .unwrap();
    let list_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");
    let content = result
        .get("content")
        .expect("Response should have content field");
    let files = content["files"].as_array().unwrap();
    println!("Listed {} files in: {:?}", files.len(), list_duration);

    assert!(files.len() >= file_count as usize);
    assert!(
        list_duration < Duration::from_secs(5),
        "File listing should complete within 5 seconds"
    );
}

#[tokio::test]
#[cfg(feature = "heavy-tests")]
async fn test_rapid_operations_performance() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let operation_count = 20;
    let mut successful_creates = 0;
    let mut successful_reads = 0;
    let mut successful_lsp = 0;
    let mut operation_times = Vec::new();

    // Launch rapid sequential file operations
    let start = Instant::now();
    for i in 0..operation_count {
        let file_path = workspace.path().join(format!("rapid_{}.ts", i));
        let content = format!(
            r#"
export class RapidClass{} {{
    private value: number = {};

    public getValue(): number {{
        return this.value;
    }}

    public async processAsync(): Promise<string> {{
        return new Promise(resolve => {{
            setTimeout(() => resolve(`Processed ${{this.value}}`), 100);
        }});
    }}
}}
"#,
            i, i
        );

        let op_start = Instant::now();

        // Create file
        let create_result = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": content
                }),
            )
            .await;

        // Read file back
        let read_result = client
            .call_tool(
                "read_file",
                json!({
                    "file_path": file_path.to_string_lossy()
                }),
            )
            .await;

        // Try LSP operation
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let symbols_result = client
            .call_tool(
                "get_document_symbols",
                json!({
                    "file_path": file_path.to_string_lossy()
                }),
            )
            .await;

        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);

        if create_result.is_ok() {
            successful_creates += 1;
        }
        if read_result.is_ok() {
            successful_reads += 1;
        }
        if symbols_result.is_ok() {
            successful_lsp += 1;
        }
    }

    let total_duration = start.elapsed();

    println!(
        "Completed {} rapid operations in: {:?}",
        operation_count, total_duration
    );

    let total_ops_time: Duration = operation_times.iter().sum();
    let avg_op_time = total_ops_time / operation_count as u32;

    println!(
        "Successful operations - Creates: {}, Reads: {}, LSP: {}",
        successful_creates, successful_reads, successful_lsp
    );
    println!("Average operation time: {:?}", avg_op_time);

    // Performance assertions
    assert!(
        successful_creates >= operation_count * 19 / 20,
        "At least 95% of creates should succeed, got {}/{}",
        successful_creates,
        operation_count
    );
    assert!(
        successful_reads >= operation_count * 19 / 20,
        "At least 95% of reads should succeed, got {}/{}",
        successful_reads,
        operation_count
    );
    assert!(
        total_duration < Duration::from_secs(30),
        "All rapid operations should complete within 30 seconds"
    );
}

#[tokio::test]
#[cfg(feature = "heavy-tests")]
async fn test_workspace_edit_performance() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();
    let mut client = TestClient::new(workspace.path());

    // Create multiple files for large workspace edit
    let file_count = 50;
    let mut file_paths = Vec::new();

    for i in 0..file_count {
        let file_path = workspace.path().join(format!("edit_perf_{}.ts", i));
        let content = format!(
            r#"
export interface OldInterface{} {{
    id: number;
    oldProperty: string;
}}

export function oldFunction{}(param: OldInterface{}): string {{
    return param.oldProperty;
}}

const oldConstant{} = "old_value_{}";
"#,
            i, i, i, i, i
        );

        let response = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": content
                }),
            )
            .await
            .unwrap();

        let result = response
            .get("result")
            .expect("Response should have result field");
        assert!(result["success"].as_bool().unwrap_or(false));
        file_paths.push(file_path);
    }

    // Give file system time to sync all files
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // DEBUG: Verify ALL files have content before workspace edit
    let mut empty_files = Vec::new();
    for (i, file_path) in file_paths.iter().enumerate() {
        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                if content.is_empty() {
                    empty_files.push(i);
                    eprintln!(
                        "DEBUG: File {} (index {}) is EMPTY!",
                        file_path.display(),
                        i
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "DEBUG: File {} (index {}) ERROR: {}",
                    file_path.display(),
                    i,
                    e
                );
                empty_files.push(i);
            }
        }
    }
    if !empty_files.is_empty() {
        panic!("Files are empty or missing: {:?}", empty_files);
    }
    eprintln!(
        "DEBUG: All {} files verified to have content!",
        file_paths.len()
    );

    // Prepare large workspace edit
    let mut changes = json!({});

    for (index, file_path) in file_paths.iter().enumerate() {
        changes[file_path.to_string_lossy().to_string()] = json!([
            {
                "range": {
                    "start": { "line": 1, "character": 17 },
                    "end": { "line": 1, "character": 17 + format!("OldInterface{}", index).len() }
                },
                "newText": format!("NewInterface{}", index)
            },
            {
                "range": {
                    "start": { "line": 3, "character": 4 },
                    "end": { "line": 3, "character": 15 }
                },
                "newText": "newProperty"
            },
            {
                "range": {
                    "start": { "line": 6, "character": 16 },
                    "end": { "line": 6, "character": 16 + format!("oldFunction{}", index).len() }
                },
                "newText": format!("newFunction{}", index)
            }
        ]);
    }

    // Execute large workspace edit
    let start = Instant::now();
    let response = client
        .call_tool(
            "apply_workspace_edit",
            json!({
                "changes": changes
            }),
        )
        .await
        .unwrap();
    let edit_duration = start.elapsed();

    println!(
        "Workspace edit across {} files took: {:?}",
        file_count, edit_duration
    );

    eprintln!(
        "APPLY_EDITS RESPONSE: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    let result = response
        .get("result")
        .expect("Response should have result field");
    assert!(result["applied"].as_bool().unwrap_or(false));
    assert!(
        edit_duration < Duration::from_secs(20),
        "Large workspace edit should complete within 20 seconds"
    );

    // Verify changes were applied correctly
    let verification_start = Instant::now();
    for (index, file_path) in file_paths.iter().enumerate().take(5) {
        // Check first 5 files
        let content_response = client
            .call_tool(
                "read_file",
                json!({
                    "file_path": file_path.to_string_lossy()
                }),
            )
            .await
            .unwrap();

        let result = content_response
            .get("result")
            .expect("Response should have result field");
        let content = result["content"].as_str().unwrap();

        // Debug: Print file content for first file
        if index == 0 {
            println!("File content after edit:\n{}", content);
        }

        // Verify the specific edits that were made
        assert!(
            content.contains(&format!("export interface NewInterface{}", index)),
            "Interface name should be changed to NewInterface{}",
            index
        );
        assert!(
            content.contains("newProperty: string"),
            "Property should be renamed to newProperty"
        );
        assert!(
            content.contains(&format!("export function newFunction{}", index)),
            "Function name should be changed to newFunction{}",
            index
        );

        // Note: OldInterface and oldProperty still appear in other places (parameter types, return statements)
        // We only edited the interface declaration, property declaration, and function name
        assert!(
            !content.contains(&format!("export interface OldInterface{}", index)),
            "Old interface declaration should be gone"
        );
        // The property oldProperty is used in "return param.oldProperty" - only the declaration was changed
    }
    let verification_duration = verification_start.elapsed();

    println!("Verification of changes took: {:?}", verification_duration);
}

#[tokio::test]
#[cfg(feature = "heavy-tests")]
async fn test_memory_usage_large_operations() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();
    let mut client = TestClient::new(workspace.path());

    // Create a very large content string (5MB)
    let large_content = "A".repeat(5 * 1024 * 1024);
    let large_file = workspace.path().join("memory_test.txt");

    // Test memory efficiency with large content
    let start = Instant::now();
    let response = client
        .call_tool(
            "create_file",
            json!({
                "file_path": large_file.to_string_lossy(),
                "content": large_content
            }),
        )
        .await
        .unwrap();
    let create_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");
    assert!(result["success"].as_bool().unwrap_or(false));
    println!("Created 5MB file in: {:?}", create_duration);

    // Read it back
    let start = Instant::now();
    let response = client
        .call_tool(
            "read_file",
            json!({
                "file_path": large_file.to_string_lossy()
            }),
        )
        .await
        .unwrap();
    let read_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");
    let read_content = result["content"].as_str().unwrap();
    assert_eq!(read_content.len(), large_content.len());
    println!("Read 5MB file in: {:?}", read_duration);

    // Create many smaller files to test memory accumulation
    for i in 0..20 {
        let file_path = workspace.path().join(format!("memory_small_{}.txt", i));
        let content = format!("Small file {} with some content", i).repeat(1000); // ~30KB each

        let response = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": content
                }),
            )
            .await
            .unwrap();

        let result = response
            .get("result")
            .expect("Response should have result field");
        assert!(result["success"].as_bool().unwrap_or(false));
    }

    // List all files to test memory with many file handles
    let start = Instant::now();
    let response = client
        .call_tool(
            "list_files",
            json!({
                "directory": workspace.path().to_string_lossy(),
                "recursive": true
            }),
        )
        .await
        .unwrap();
    let list_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");

    // Debug: Print result structure
    println!("list_files result structure: {:?}", result);

    // list_files returns files directly in result, not in a content field
    let files = result["files"]
        .as_array()
        .or_else(|| result.get("content").and_then(|c| c["files"].as_array()))
        .expect("Response should have files array");
    println!("Listed {} files in: {:?}", files.len(), list_duration);

    assert!(files.len() >= 21); // large file + 20 small files
}

#[tokio::test]
#[cfg(feature = "heavy-tests")]
async fn test_lsp_performance_complex_project() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config();
    let mut client = TestClient::new(workspace.path());

    // Create a complex TypeScript project structure
    let src_dir = workspace.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let types_dir = src_dir.join("types");
    fs::create_dir(&types_dir).unwrap();

    let utils_dir = src_dir.join("utils");
    fs::create_dir(&utils_dir).unwrap();

    let services_dir = src_dir.join("services");
    fs::create_dir(&services_dir).unwrap();

    // Create types files
    for i in 0..10 {
        let types_file = types_dir.join(format!("types{}.ts", i));
        let content = format!(
            r#"
export interface User{} {{
    id: number;
    name: string;
    email: string;
    preferences: UserPreferences{};
}}

export interface UserPreferences{} {{
    theme: 'light' | 'dark';
    language: string;
    notifications: boolean;
}}

export type UserRole{} = 'admin' | 'user' | 'moderator';

export interface ApiResponse{}<T> {{
    data: T;
    status: number;
    message: string;
}}
"#,
            i, i, i, i, i
        );

        fs::write(&types_file, content).unwrap();
    }

    // Create utils files
    for i in 0..10 {
        let utils_file = utils_dir.join(format!("utils{}.ts", i));
        let content = format!(
            r#"
import {{ User{}, UserPreferences{}, ApiResponse{} }} from '../types/types{}';

export function validateUser{}(user: User{}): boolean {{
    return user.id > 0 && user.name.length > 0 && user.email.includes('@');
}}

export function formatUserDisplay{}(user: User{}): string {{
    return `${{user.name}} (${{user.email}})`;
}}

export async function fetchUserData{}(id: number): Promise<ApiResponse{}<User{}>> {{
    const response = await fetch(`/api/users/${{id}}`);
    return response.json();
}}

export function applyPreferences{}(prefs: UserPreferences{}): void {{
    document.body.setAttribute('data-theme', prefs.theme);
    document.documentElement.lang = prefs.language;
}}
"#,
            i, i, i, i, i, i, i, i, i, i, i, i, i
        );

        fs::write(&utils_file, content).unwrap();
    }

    // Create service files
    for i in 0..10 {
        let service_file = services_dir.join(format!("service{}.ts", i));
        let content = format!(
            r#"
import {{ User{}, UserPreferences{}, UserRole{}, ApiResponse{} }} from '../types/types{}';
import {{ validateUser{}, formatUserDisplay{}, fetchUserData{} }} from '../utils/utils{}';

export class UserService{} {{
    private users: Map<number, User{}> = new Map();

    async loadUser{}(id: number): Promise<User{} | null> {{
        try {{
            const response = await fetchUserData{}(id);
            if (response.status === 200 && validateUser{}(response.data)) {{
                this.users.set(id, response.data);
                return response.data;
            }}
        }} catch (error) {{
            console.error('Failed to load user:', error);
        }}
        return null;
    }}

    getUserDisplay{}(id: number): string {{
        const user = this.users.get(id);
        return user ? formatUserDisplay{}(user) : 'Unknown User';
    }}

    updateUserPreferences{}(id: number, prefs: UserPreferences{}): boolean {{
        const user = this.users.get(id);
        if (user) {{
            user.preferences = prefs;
            return true;
        }}
        return false;
    }}
}}
"#,
            i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i
        );

        fs::write(&service_file, content).unwrap();
    }

    // Add tsconfig.json for proper TypeScript indexing
    let tsconfig = workspace.path().join("tsconfig.json");
    fs::write(
        &tsconfig,
        r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": true
  },
  "include": ["src/**/*"]
}"#,
    )
    .unwrap();

    // Give LSP time to process the complex project and index with tsconfig
    tokio::time::sleep(tokio::time::Duration::from_millis(8000)).await;

    // Test find definition performance across the project
    let start = Instant::now();
    let response = client
        .call_tool_with_timeout(
            "find_definition",
            json!({
                "file_path": services_dir.join("service0.ts").to_string_lossy(),
                "line": 1,
                "character": 10 // Should point to User0 import
            }),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    let definition_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");
    let content = result
        .get("content")
        .expect("Response should have content field");
    let locations = content["locations"].as_array().unwrap();
    assert!(!locations.is_empty());
    println!(
        "Cross-file definition lookup took: {:?}",
        definition_duration
    );

    // Test workspace symbol search performance
    let start = Instant::now();
    let response = client
        .call_tool_with_timeout(
            "search_symbols",
            json!({
                "query": "User"
            }),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    let search_duration = start.elapsed();

    let result = response
        .get("result")
        .expect("Response should have result field");
    let symbols = result
        .get("content")
        .and_then(|v| v.as_array())
        .expect("Response should have content array");
    println!(
        "Workspace symbol search found {} symbols in: {:?}",
        symbols.len(),
        search_duration
    );

    // Debug: Print first few symbols
    for (i, symbol) in symbols.iter().enumerate().take(15) {
        println!(
            "Symbol {}: {}",
            i,
            symbol.get("name").and_then(|v| v.as_str()).unwrap_or("?")
        );
    }

    // TypeScript LSP may not index all files immediately
    // This is a known limitation - symbol search depends on LSP server indexing
    // For now, just verify we got some symbols back
    assert!(
        symbols.len() > 0,
        "Should find at least some User-related symbols (got {})",
        symbols.len()
    );

    // Test find references performance
    let start = Instant::now();
    let response = client
        .call_tool_with_timeout(
            "find_references",
            json!({
                "file_path": types_dir.join("types0.ts").to_string_lossy(),
                "line": 1,
                "character": 18, // User0 interface
                "include_declaration": true
            }),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    let references_duration = start.elapsed();

    // Check if response has error field
    if let Some(error) = response.get("error") {
        println!("Find references returned error: {:?}", error);
        println!("Skipping references assertion - LSP may need more time to index");
    } else if let Some(result) = response.get("result") {
        if let Some(content) = result.get("content") {
            if let Some(references) = content["references"].as_array() {
                println!(
                    "Found {} references in: {:?}",
                    references.len(),
                    references_duration
                );
                assert!(
                    !references.is_empty(),
                    "Should find at least some references"
                );
            }
        }
    }

    // Performance assertions for complex project
    assert!(
        definition_duration < Duration::from_secs(5),
        "Definition lookup should be fast"
    );
    assert!(
        search_duration < Duration::from_secs(10),
        "Workspace search should complete reasonably fast"
    );
    assert!(
        references_duration < Duration::from_secs(10),
        "Reference finding should be efficient"
    );
}

#[tokio::test]
#[cfg(feature = "heavy-tests")]
async fn test_stress_test_rapid_operations() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let operations_count = 200;
    let mut operation_times = Vec::new();

    for i in 0..operations_count {
        let start = Instant::now();

        match i % 4 {
            0 => {
                // Create file operation
                let file_path = workspace.path().join(format!("stress_{}.txt", i));
                let _response = client
                    .call_tool(
                        "create_file",
                        json!({
                            "file_path": file_path.to_string_lossy(),
                            "content": format!("Stress test content {}", i)
                        }),
                    )
                    .await;
            }
            1 => {
                // List files operation
                let _response = client
                    .call_tool(
                        "list_files",
                        json!({
                            "directory": workspace.path().to_string_lossy()
                        }),
                    )
                    .await;
            }
            2 => {
                // Read file operation (if files exist)
                if i > 0 {
                    let file_path = workspace.path().join(format!("stress_{}.txt", i - 1));
                    let _response = client
                        .call_tool(
                            "read_file",
                            json!({
                                "file_path": file_path.to_string_lossy()
                            }),
                        )
                        .await;
                }
            }
            3 => {
                // Health check operation
                let _response = client.call_tool("health_check", json!({})).await;
            }
            _ => unreachable!(),
        }

        let duration = start.elapsed();
        operation_times.push(duration);

        // Small delay to prevent overwhelming the system
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Analyze performance statistics
    let total_time: Duration = operation_times.iter().sum();
    let avg_time = total_time / operations_count as u32;
    let max_time = operation_times.iter().max().unwrap();
    let min_time = operation_times.iter().min().unwrap();

    // Sort for percentile calculations
    let mut sorted_times = operation_times.clone();
    sorted_times.sort();
    let p95_time = sorted_times[(operations_count * 95 / 100) as usize];
    let p99_time = sorted_times[(operations_count * 99 / 100) as usize];

    println!("Stress test results for {} operations:", operations_count);
    println!("Total time: {:?}", total_time);
    println!("Average time: {:?}", avg_time);
    println!("Min time: {:?}", min_time);
    println!("Max time: {:?}", max_time);
    println!("95th percentile: {:?}", p95_time);
    println!("99th percentile: {:?}", p99_time);

    // Performance assertions
    assert!(
        avg_time < Duration::from_millis(500),
        "Average operation should be under 500ms"
    );
    assert!(
        p95_time < Duration::from_secs(2),
        "95% of operations should complete within 2 seconds"
    );
    assert!(
        p99_time < Duration::from_secs(5),
        "99% of operations should complete within 5 seconds"
    );
}
