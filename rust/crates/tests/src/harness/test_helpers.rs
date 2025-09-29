use crate::harness::{TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// Helper for timing operations
pub struct PerformanceTimer {
    start: Instant,
    operation: String,
}

impl PerformanceTimer {
    pub fn new(operation: &str) -> Self {
        Self {
            start: Instant::now(),
            operation: operation.to_string(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn finish(self) -> Duration {
        let duration = self.elapsed();
        println!("{} took: {:?}", self.operation, duration);
        duration
    }
}

/// Helper for creating test files with specific content patterns
pub struct TestFileBuilder {
    workspace: std::path::PathBuf,
}

impl TestFileBuilder {
    pub fn new(workspace: &TestWorkspace) -> Self {
        Self {
            workspace: workspace.path().to_path_buf(),
        }
    }

    pub async fn create_typescript_class(
        &mut self,
        client: &mut TestClient,
        name: &str,
        methods: &[&str],
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let file_path = self.workspace.join(format!("{}.ts", name.to_lowercase()));

        let mut content = format!("export class {} {{\n", name);

        for method in methods {
            content.push_str(&format!(
                "    public {}(): void {{\n        console.log('{}');\n    }}\n\n",
                method, method
            ));
        }

        content.push_str("}\n");

        let response = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": content
                }),
            )
            .await?;

        if !response["success"].as_bool().unwrap_or(false) {
            return Err("Failed to create TypeScript class file".into());
        }

        Ok(file_path)
    }

    pub async fn create_interface_file(
        &mut self,
        client: &mut TestClient,
        name: &str,
        properties: &[(&str, &str)],
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let file_path = self.workspace.join(format!("{}.ts", name.to_lowercase()));

        let mut content = format!("export interface {} {{\n", name);

        for (prop_name, prop_type) in properties {
            content.push_str(&format!("    {}: {};\n", prop_name, prop_type));
        }

        content.push_str("}\n");

        let response = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": content
                }),
            )
            .await?;

        if !response["success"].as_bool().unwrap_or(false) {
            return Err("Failed to create interface file".into());
        }

        Ok(file_path)
    }

    pub async fn create_import_chain(
        &mut self,
        client: &mut TestClient,
        chain_length: usize,
    ) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();

        for i in 0..chain_length {
            let file_name = format!("chain_{}.ts", i);
            let file_path = self.workspace.join(&file_name);

            let content = if i == 0 {
                // Base file
                r#"
export interface BaseType {
    id: number;
    value: string;
}

export function baseFunction(param: BaseType): string {
    return `Base: ${param.value}`;
}
"#.to_string()
            } else {
                // Import from previous file
                let prev_file = format!("chain_{}", i - 1);
                format!(
                    r#"
import {{ BaseType, baseFunction }} from './{prev_file}';

export interface ExtendedType{i} extends BaseType {{
    additional{i}: string;
}}

export function extendedFunction{i}(param: ExtendedType{i}): string {{
    return `Extended {i}: ${{baseFunction(param)}} + ${{param.additional{i}}}`;
}}
"#,
                    prev_file = prev_file,
                    i = i
                )
            };

            let response = client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path.to_string_lossy(),
                        "content": content
                    }),
                )
                .await?;

            if !response["success"].as_bool().unwrap_or(false) {
                return Err(format!("Failed to create chain file {}", i).into());
            }

            files.push(file_path);
        }

        Ok(files)
    }
}

/// Helper for batch operations
pub struct BatchOperationHelper {
    client: TestClient,
}

impl BatchOperationHelper {
    pub fn new(client: TestClient) -> Self {
        Self { client }
    }

    pub async fn create_multiple_files(
        &mut self,
        files: &[(&str, &str)],
    ) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for (file_path, content) in files {
            let response = self
                .client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path,
                        "content": content
                    }),
                )
                .await?;

            results.push(response);
        }

        Ok(results)
    }

    pub async fn read_multiple_files(
        &mut self,
        file_paths: &[&str],
    ) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for file_path in file_paths {
            let response = self
                .client
                .call_tool(
                    "read_file",
                    json!({
                        "file_path": file_path
                    }),
                )
                .await?;

            results.push(response);
        }

        Ok(results)
    }

    pub async fn find_definitions_batch(
        &mut self,
        queries: &[(&str, usize, usize)],
    ) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for (file_path, line, character) in queries {
            let response = self
                .client
                .call_tool(
                    "find_definition",
                    json!({
                        "file_path": file_path,
                        "line": line,
                        "character": character
                    }),
                )
                .await?;

            results.push(response);
        }

        Ok(results)
    }
}

/// Helper for testing LSP intelligence features
pub struct LspTestHelper {
    client: TestClient,
}

impl LspTestHelper {
    pub fn new(client: TestClient) -> Self {
        Self { client }
    }

    pub async fn wait_for_lsp_ready(&mut self, timeout: Duration) -> bool {
        let start = Instant::now();

        while start.elapsed() < timeout {
            // Try a simple health check to see if LSP is responsive
            if let Ok(_) = self.client.call_tool("health_check", json!({})).await {
                return true;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        false
    }

    pub async fn verify_symbol_exists(
        &mut self,
        file_path: &str,
        symbol_name: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let response = self
            .client
            .call_tool(
                "get_document_symbols",
                json!({
                    "file_path": file_path
                }),
            )
            .await?;

        let symbols = response["symbols"].as_array().unwrap();

        let exists = symbols
            .iter()
            .any(|symbol| symbol["name"].as_str().unwrap_or("").contains(symbol_name));

        Ok(exists)
    }

    pub async fn find_symbol_location(
        &mut self,
        file_path: &str,
        symbol_name: &str,
    ) -> Result<Option<(usize, usize)>, Box<dyn std::error::Error>> {
        let response = self
            .client
            .call_tool(
                "get_document_symbols",
                json!({
                    "file_path": file_path
                }),
            )
            .await?;

        let symbols = response["symbols"].as_array().unwrap();

        for symbol in symbols {
            if symbol["name"].as_str().unwrap_or("").contains(symbol_name) {
                if let (Some(_range), Some(start)) =
                    (symbol.get("range"), symbol["range"].get("start"))
                {
                    let line = start["line"].as_u64().unwrap_or(0) as usize;
                    let character = start["character"].as_u64().unwrap_or(0) as usize;
                    return Ok(Some((line, character)));
                }
            }
        }

        Ok(None)
    }

    pub async fn test_cross_file_navigation(
        &mut self,
        from_file: &str,
        to_file: &str,
        line: usize,
        character: usize,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let response = self
            .client
            .call_tool(
                "find_definition",
                json!({
                    "file_path": from_file,
                    "line": line,
                    "character": character
                }),
            )
            .await?;

        let locations = response["locations"].as_array().unwrap();

        for location in locations {
            let uri = location["uri"].as_str().unwrap_or("");
            if uri.contains(to_file) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Helper for error simulation and testing
pub struct ErrorTestHelper {
    client: TestClient,
}

impl ErrorTestHelper {
    pub fn new(client: TestClient) -> Self {
        Self { client }
    }

    pub async fn test_operation_with_timeout(
        &mut self,
        operation: impl std::future::Future<Output = Result<Value, Box<dyn std::error::Error>>>,
        timeout: Duration,
    ) -> Result<Value, String> {
        match tokio::time::timeout(timeout, operation).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(format!("Operation failed: {}", e)),
            Err(_) => Err("Operation timed out".to_string()),
        }
    }

    pub async fn test_invalid_file_path(&mut self) -> bool {
        let invalid_paths = vec![
            "/absolutely/nonexistent/path/file.txt",
            "",
            "not_a_real_file.xyz",
            "\0invalid\0path\0",
        ];

        for invalid_path in invalid_paths {
            let response = self
                .client
                .call_tool(
                    "read_file",
                    json!({
                        "file_path": invalid_path
                    }),
                )
                .await;

            if response.is_ok() {
                return false; // Should have failed
            }
        }

        true
    }

    pub async fn test_malformed_parameters(&mut self) -> bool {
        let malformed_calls = vec![
            ("read_file", json!({})),                             // Missing file_path
            ("find_definition", json!({"file_path": "test.ts"})), // Missing line/character
            (
                "find_definition",
                json!({"file_path": "test.ts", "line": "not_a_number", "character": 0}),
            ), // Invalid type
        ];

        for (tool, params) in malformed_calls {
            let response = self.client.call_tool(tool, params).await;

            if response.is_ok() {
                return false; // Should have failed
            }
        }

        true
    }
}

/// Helper for performance testing
pub struct PerformanceTestHelper {
    client: TestClient,
}

impl PerformanceTestHelper {
    pub fn new(client: TestClient) -> Self {
        Self { client }
    }

    pub async fn measure_operation_latency(
        &mut self,
        operation: impl std::future::Future<Output = Result<Value, Box<dyn std::error::Error>>>,
    ) -> Result<(Value, Duration), Box<dyn std::error::Error>> {
        let start = Instant::now();
        let result = operation.await?;
        let duration = start.elapsed();
        Ok((result, duration))
    }

    pub async fn stress_test_operation(
        &mut self,
        operation_factory: impl Fn() -> Box<
            dyn std::future::Future<Output = Result<Value, Box<dyn std::error::Error>>> + Unpin,
        >,
        iterations: usize,
    ) -> (usize, Duration, Duration, Duration) {
        let mut successful = 0;
        let mut durations = Vec::new();

        for _ in 0..iterations {
            let start = Instant::now();
            let operation = operation_factory();

            match operation.await {
                Ok(_) => {
                    successful += 1;
                    durations.push(start.elapsed());
                }
                Err(_) => {
                    // Count failures but don't include in timing
                }
            }

            // Small delay to prevent overwhelming the system
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        if durations.is_empty() {
            return (successful, Duration::ZERO, Duration::ZERO, Duration::ZERO);
        }

        let total_time: Duration = durations.iter().sum();
        let avg_time = total_time / durations.len() as u32;
        let min_time = *durations.iter().min().unwrap();
        let max_time = *durations.iter().max().unwrap();

        (successful, avg_time, min_time, max_time)
    }

    pub async fn concurrent_operation_test(
        &mut self,
        operation_count: usize,
        operation_factory: impl Fn(
            usize,
        ) -> Box<
            dyn std::future::Future<Output = Result<Value, Box<dyn std::error::Error + Send>>>
                + Unpin
                + Send,
        >,
    ) -> (usize, Duration) {
        let start = Instant::now();
        let mut handles = Vec::new();

        for i in 0..operation_count {
            let operation = operation_factory(i);
            let handle = tokio::spawn(operation);
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        let total_duration = start.elapsed();

        let successful = results
            .iter()
            .filter(|r| match r {
                Ok(Ok(_)) => true,
                _ => false,
            })
            .count();

        (successful, total_duration)
    }
}

/// Helper for verifying test results
pub struct ResultVerifier;

impl ResultVerifier {
    pub fn verify_file_content(actual: &str, expected: &str) -> bool {
        actual.trim() == expected.trim()
    }

    pub fn verify_symbol_list(symbols: &[Value], expected_names: &[&str]) -> bool {
        let symbol_names: Vec<String> = symbols
            .iter()
            .map(|s| s["name"].as_str().unwrap_or("").to_string())
            .collect();

        for expected in expected_names {
            if !symbol_names.iter().any(|name| name.contains(expected)) {
                return false;
            }
        }

        true
    }

    pub fn verify_location_in_file(location: &Value, expected_file: &str) -> bool {
        if let Some(uri) = location.get("uri") {
            if let Some(uri_str) = uri.as_str() {
                return uri_str.contains(expected_file);
            }
        }
        false
    }

    pub fn verify_range_valid(range: &Value) -> bool {
        if let (Some(start), Some(end)) = (range.get("start"), range.get("end")) {
            if let (Some(start_line), Some(start_char), Some(end_line), Some(end_char)) = (
                start.get("line").and_then(|l| l.as_u64()),
                start.get("character").and_then(|c| c.as_u64()),
                end.get("line").and_then(|l| l.as_u64()),
                end.get("character").and_then(|c| c.as_u64()),
            ) {
                return start_line <= end_line && (start_line < end_line || start_char <= end_char);
            }
        }
        false
    }

    pub fn verify_performance_threshold(
        duration: Duration,
        threshold: Duration,
        operation: &str,
    ) -> bool {
        if duration > threshold {
            eprintln!(
                "Performance warning: {} took {:?}, expected < {:?}",
                operation, duration, threshold
            );
            false
        } else {
            true
        }
    }
}

/// Utility functions for common test patterns
pub mod utils {
    use super::*;

    pub async fn create_simple_typescript_project(
        workspace: &TestWorkspace,
        client: &mut TestClient,
    ) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
        let files = vec![
            (
                "types.ts",
                r#"
export interface User {
    id: number;
    name: string;
}

export type UserRole = 'admin' | 'user';
"#,
            ),
            (
                "utils.ts",
                r#"
import { User, UserRole } from './types';

export function createUser(name: string): User {
    return { id: Math.random(), name };
}

export function checkRole(role: UserRole): boolean {
    return ['admin', 'user'].includes(role);
}
"#,
            ),
            (
                "main.ts",
                r#"
import { createUser, checkRole } from './utils';
import { UserRole } from './types';

const user = createUser('John');
const role: UserRole = 'admin';
console.log(user, checkRole(role));
"#,
            ),
        ];

        let mut file_paths = Vec::new();

        for (filename, content) in files {
            let file_path = workspace.path().join(filename);

            let response = client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path.to_string_lossy(),
                        "content": content
                    }),
                )
                .await?;

            if !response["success"].as_bool().unwrap_or(false) {
                return Err(format!("Failed to create file: {}", filename).into());
            }

            file_paths.push(file_path);
        }

        // Give LSP time to process
        tokio::time::sleep(Duration::from_millis(1000)).await;

        Ok(file_paths)
    }

    pub async fn wait_for_lsp_processing(duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    pub fn extract_error_message(error: &Box<dyn std::error::Error>) -> String {
        error.to_string()
    }

    pub fn is_timeout_error(error: &Box<dyn std::error::Error>) -> bool {
        error.to_string().to_lowercase().contains("timeout")
    }
}
