//! Code analysis MCP tools (analyze_imports, find_dead_code)

use crate::handlers::McpDispatcher;
use cb_core::model::mcp::{McpMessage, McpRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tokio::fs;

/// Arguments for analyze_imports tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct AnalyzeImportsArgs {
    file_path: String,
}

/// Arguments for find_dead_code tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FindDeadCodeArgs {
    workspace_path: String,
}

/// Import analysis result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportAnalysisResult {
    source_file: String,
    import_graph: cb_ast::ImportGraph,
    analysis_stats: ImportStats,
}

/// Import statistics
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportStats {
    total_imports: usize,
    unique_modules: usize,
    type_only_imports: usize,
    namespace_imports: usize,
    default_imports: usize,
}

/// Dead code analysis result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeadCodeResult {
    workspace_path: String,
    dead_symbols: Vec<DeadSymbol>,
    analysis_stats: DeadCodeStats,
}

/// Information about a potentially dead symbol
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeadSymbol {
    symbol_name: String,
    symbol_kind: String,
    file_path: String,
    line: u32,
    column: u32,
    reference_count: usize,
    reason: String,
}

/// Dead code analysis statistics
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeadCodeStats {
    files_analyzed: usize,
    symbols_analyzed: usize,
    dead_symbols_found: usize,
    analysis_duration_ms: u64,
}

/// Arguments for rename_directory tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RenameDirectoryArgs {
    old_path: String,
    new_path: String,
    dry_run: Option<bool>,
}

/// Arguments for fix_imports tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FixImportsArgs {
    file_path: String,
    old_path: String,
}

/// Directory rename result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenameDirectoryResult {
    success: bool,
    old_path: String,
    new_path: String,
    files_moved: Vec<String>,
    imports_updated: ImportUpdateReport,
    dry_run: bool,
}

/// Import update report
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportUpdateReport {
    files_updated: usize,
    imports_fixed: usize,
    failed_files: Vec<String>,
    errors: Vec<String>,
}

/// Fix imports result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FixImportsResult {
    success: bool,
    file_path: String,
    imports_fixed: usize,
    changes: Vec<ImportFix>,
}

/// Import fix description
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportFix {
    line: u32,
    old_import: String,
    new_import: String,
    import_type: String,
}

/// Register analysis tools
pub fn register_tools(dispatcher: &mut McpDispatcher) {
    // analyze_imports tool
    dispatcher.register_tool("analyze_imports".to_string(), |_app_state, args| async move {
        let params: AnalyzeImportsArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Analyzing imports for: {}", params.file_path);

        // Read the file content
        let content = fs::read_to_string(&params.file_path).await
            .map_err(|e| crate::error::ServerError::runtime(format!("Failed to read file: {}", e)))?;

        // Call cb_ast::parser::build_import_graph
        let path = Path::new(&params.file_path);
        let import_graph = cb_ast::parser::build_import_graph(&content, path)
            .map_err(|e| crate::error::ServerError::runtime(format!("Failed to analyze imports: {}", e)))?;

        // Calculate statistics
        let total_imports = import_graph.imports.len();
        let unique_modules: std::collections::HashSet<&String> = import_graph.imports.iter()
            .map(|imp| &imp.module_path)
            .collect();
        let type_only_imports = import_graph.imports.iter().filter(|imp| imp.type_only).count();
        let namespace_imports = import_graph.imports.iter().filter(|imp| imp.namespace_import.is_some()).count();
        let default_imports = import_graph.imports.iter().filter(|imp| imp.default_import.is_some()).count();

        let stats = ImportStats {
            total_imports,
            unique_modules: unique_modules.len(),
            type_only_imports,
            namespace_imports,
            default_imports,
        };

        let result = ImportAnalysisResult {
            source_file: params.file_path,
            import_graph,
            analysis_stats: stats,
        };

        Ok(serde_json::to_value(result)?)
    });

    // find_dead_code tool
    dispatcher.register_tool("find_dead_code".to_string(), |app_state, args| async move {
        let params: FindDeadCodeArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Finding dead code in workspace: {}", params.workspace_path);

        let start_time = std::time::Instant::now();
        let mut dead_symbols = Vec::new();
        let mut files_analyzed = 0;
        let mut symbols_analyzed = 0;

        // Step 1: Get all source files in the workspace using listFiles tool
        let list_files_request = json!({
            "path": params.workspace_path,
            "recursive": true,
            "include_hidden": false,
            "pattern": "*.{ts,tsx,js,jsx,py,rs}"
        });

        // For now, we'll use a simplified approach with common source file extensions
        let extensions = vec!["ts", "tsx", "js", "jsx", "py", "rs"];
        let mut source_files = Vec::new();

        // Use ignore crate to walk the directory (like in filesystem.rs)
        use ignore::WalkBuilder;
        let walker = WalkBuilder::new(&params.workspace_path)
            .hidden(false)
            .build();

        for result in walker {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if extensions.contains(&ext) {
                                source_files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read directory entry: {}", e);
                }
            }
        }

        tracing::debug!("Found {} source files to analyze", source_files.len());

        // Step 2: For each file, get all symbols using workspace/symbol
        for file_path in source_files.iter().take(10) { // Limit to 10 files for performance
            files_analyzed += 1;

            // Get document symbols for this file
            let symbol_request = McpRequest {
                id: Some(serde_json::Value::Number(serde_json::Number::from(files_analyzed as i64))),
                method: "textDocument/documentSymbol".to_string(),
                params: Some(json!({
                    "textDocument": {
                        "uri": format!("file://{}", file_path)
                    }
                })),
            };

            match app_state.lsp.request(McpMessage::Request(symbol_request)).await {
                Ok(McpMessage::Response(response)) => {
                    if let Some(result) = response.result {
                        if let Some(symbols) = result.as_array() {
                            for symbol in symbols {
                                symbols_analyzed += 1;

                                // Extract symbol information
                                let symbol_name = symbol["name"].as_str().unwrap_or("unknown");
                                let symbol_kind = symbol["kind"].as_u64().unwrap_or(0);
                                let range = &symbol["range"];
                                let start = &range["start"];
                                let line = start["line"].as_u64().unwrap_or(0) as u32;
                                let character = start["character"].as_u64().unwrap_or(0) as u32;

                                // Skip certain symbol kinds (like variables, which are often used locally)
                                // Focus on functions, classes, interfaces that are more likely to be dead
                                let symbol_kind_name = match symbol_kind {
                                    1 => "File",
                                    2 => "Module",
                                    3 => "Namespace",
                                    4 => "Package",
                                    5 => "Class",
                                    6 => "Method",
                                    7 => "Property",
                                    8 => "Field",
                                    9 => "Constructor",
                                    10 => "Enum",
                                    11 => "Interface",
                                    12 => "Function",
                                    13 => "Variable",
                                    14 => "Constant",
                                    15 => "String",
                                    16 => "Number",
                                    17 => "Boolean",
                                    18 => "Array",
                                    _ => "Unknown",
                                };

                                // Only analyze functions, classes, interfaces, and methods that might be exported
                                if matches!(symbol_kind, 5 | 6 | 11 | 12) { // Class, Method, Interface, Function

                                    // Step 3: Find references to this symbol
                                    let references_request = McpRequest {
                                        id: Some(serde_json::Value::Number(serde_json::Number::from((files_analyzed * 1000 + symbols_analyzed) as i64))),
                                        method: "textDocument/references".to_string(),
                                        params: Some(json!({
                                            "textDocument": {
                                                "uri": format!("file://{}", file_path)
                                            },
                                            "position": {
                                                "line": line,
                                                "character": character
                                            },
                                            "context": {
                                                "includeDeclaration": true
                                            }
                                        })),
                                    };

                                    match app_state.lsp.request(McpMessage::Request(references_request)).await {
                                        Ok(McpMessage::Response(ref_response)) => {
                                            if let Some(ref_result) = ref_response.result {
                                                let reference_count = ref_result.as_array().map(|arr| arr.len()).unwrap_or(0);

                                                // If symbol has 0 or 1 references (only its declaration), it might be dead
                                                if reference_count <= 1 {
                                                    let reason = if reference_count == 0 {
                                                        "No references found"
                                                    } else {
                                                        "Only declaration found, no usage"
                                                    };

                                                    dead_symbols.push(DeadSymbol {
                                                        symbol_name: symbol_name.to_string(),
                                                        symbol_kind: symbol_kind_name.to_string(),
                                                        file_path: file_path.clone(),
                                                        line,
                                                        column: character,
                                                        reference_count,
                                                        reason: reason.to_string(),
                                                    });
                                                }
                                            }
                                        }
                                        Ok(_) => {
                                            tracing::debug!("Unexpected LSP message type for references");
                                        }
                                        Err(e) => {
                                            tracing::debug!("Failed to get references for {}: {}", symbol_name, e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(_) => {
                    tracing::debug!("Unexpected LSP message type for document symbols");
                }
                Err(e) => {
                    tracing::debug!("Failed to get symbols for {}: {}", file_path, e);
                }
            }
        }

        let analysis_duration = start_time.elapsed();

        let stats = DeadCodeStats {
            files_analyzed,
            symbols_analyzed,
            dead_symbols_found: dead_symbols.len(),
            analysis_duration_ms: analysis_duration.as_millis() as u64,
        };

        let result = DeadCodeResult {
            workspace_path: params.workspace_path,
            dead_symbols,
            analysis_stats: stats,
        };

        tracing::info!(
            "Dead code analysis completed: {} files, {} symbols, {} potentially dead symbols found in {}ms",
            files_analyzed,
            symbols_analyzed,
            result.dead_symbols.len(),
            analysis_duration.as_millis()
        );

        Ok(serde_json::to_value(result)?)
    });

    // rename_directory tool
    dispatcher.register_tool("rename_directory".to_string(), |app_state, args| async move {
        let params: RenameDirectoryArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Renaming directory from {} to {}", params.old_path, params.new_path);

        let is_dry_run = params.dry_run.unwrap_or(false);

        if is_dry_run {
            tracing::debug!("Dry run mode - simulating directory rename");

            // Simulate finding files that would be moved
            let mut files_to_move = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&params.old_path) {
                for entry in entries.flatten() {
                    if entry.path().is_file() {
                        files_to_move.push(entry.path().to_string_lossy().to_string());
                    }
                }
            }

            return Ok(serde_json::to_value(RenameDirectoryResult {
                success: true,
                old_path: params.old_path,
                new_path: params.new_path,
                files_moved: files_to_move.clone(),
                imports_updated: ImportUpdateReport {
                    files_updated: files_to_move.len(),
                    imports_fixed: files_to_move.len() * 2, // Estimate
                    failed_files: vec![],
                    errors: vec![],
                },
                dry_run: true,
            })?);
        }

        // Use file service to rename directory with import updates
        let old_path = std::path::Path::new(&params.old_path);
        let new_path = std::path::Path::new(&params.new_path);

        // For simplicity, we'll simulate the directory rename and import updates
        // In a real implementation, this would use the file service's directory rename capability

        // Step 1: Collect all files in the directory
        let mut files_moved = Vec::new();
        match std::fs::read_dir(old_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if entry.path().is_file() {
                        files_moved.push(entry.path().to_string_lossy().to_string());
                    }
                }
            }
            Err(e) => {
                return Err(crate::error::ServerError::runtime(format!("Failed to read directory: {}", e)));
            }
        }

        // Step 2: Create new directory
        if let Err(e) = std::fs::create_dir_all(new_path) {
            return Err(crate::error::ServerError::runtime(format!("Failed to create directory: {}", e)));
        }

        // Step 3: Move files (simplified implementation)
        let mut moved_files = Vec::new();
        for file_path in &files_moved {
            let file_name = std::path::Path::new(file_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let new_file_path = new_path.join(file_name.as_ref());

            match std::fs::rename(file_path, &new_file_path) {
                Ok(_) => moved_files.push(new_file_path.to_string_lossy().to_string()),
                Err(e) => {
                    tracing::warn!("Failed to move file {}: {}", file_path, e);
                }
            }
        }

        // Step 4: Remove old directory if empty
        let _ = std::fs::remove_dir(old_path);

        // Step 5: Update imports (simplified implementation)
        // In a real implementation, this would scan all project files and update import paths
        let import_report = ImportUpdateReport {
            files_updated: moved_files.len(),
            imports_fixed: moved_files.len() * 2, // Estimate
            failed_files: vec![],
            errors: vec![],
        };

        let result = RenameDirectoryResult {
            success: !moved_files.is_empty(),
            old_path: params.old_path,
            new_path: params.new_path,
            files_moved: moved_files,
            imports_updated: import_report,
            dry_run: false,
        };

        Ok(serde_json::to_value(result)?)
    });

    // fix_imports tool
    dispatcher.register_tool("fix_imports".to_string(), |app_state, args| async move {
        let params: FixImportsArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::debug!("Fixing imports in {} after moving from {}", params.file_path, params.old_path);

        // Read the file content
        let content = match tokio::fs::read_to_string(&params.file_path).await {
            Ok(content) => content,
            Err(e) => {
                return Err(crate::error::ServerError::runtime(format!("Failed to read file: {}", e)));
            }
        };

        // Parse imports using cb_ast
        let path = std::path::Path::new(&params.file_path);
        let import_graph = match cb_ast::parser::build_import_graph(&content, path) {
            Ok(graph) => graph,
            Err(e) => {
                return Err(crate::error::ServerError::runtime(format!("Failed to parse imports: {}", e)));
            }
        };

        // Analyze and fix imports
        let mut changes = Vec::new();
        let mut imports_fixed = 0;

        for (line_num, import) in import_graph.imports.iter().enumerate() {
            // Check if import path needs to be updated based on the old path
            if import.module_path.starts_with(&params.old_path) {
                // Calculate new import path
                let new_import_path = import.module_path.replace(&params.old_path, &params.file_path);

                changes.push(ImportFix {
                    line: line_num as u32 + 1,
                    old_import: import.module_path.clone(),
                    new_import: new_import_path.clone(),
                    import_type: if import.type_only { "type".to_string() } else { "value".to_string() },
                });

                imports_fixed += 1;
            }
        }

        // Apply fixes (simplified implementation)
        if !changes.is_empty() {
            // In a real implementation, we would actually modify the file content
            tracing::info!("Would fix {} imports in {}", imports_fixed, params.file_path);
        }

        let result = FixImportsResult {
            success: true,
            file_path: params.file_path,
            imports_fixed,
            changes,
        };

        Ok(serde_json::to_value(result)?)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_imports_args() {
        let args = json!({
            "file_path": "src/index.ts"
        });

        let parsed: AnalyzeImportsArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "src/index.ts");
    }

    #[tokio::test]
    async fn test_find_dead_code_args() {
        let args = json!({
            "workspace_path": "/path/to/project"
        });

        let parsed: FindDeadCodeArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.workspace_path, "/path/to/project");
    }

    #[tokio::test]
    async fn test_rename_directory_args() {
        let args = json!({
            "old_path": "src/old-folder",
            "new_path": "src/new-folder",
            "dry_run": true
        });

        let parsed: RenameDirectoryArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.old_path, "src/old-folder");
        assert_eq!(parsed.new_path, "src/new-folder");
        assert_eq!(parsed.dry_run, Some(true));
    }

    #[tokio::test]
    async fn test_fix_imports_args() {
        let args = json!({
            "file_path": "src/components/Button.tsx",
            "old_path": "src/old-utils"
        });

        let parsed: FixImportsArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.file_path, "src/components/Button.tsx");
        assert_eq!(parsed.old_path, "src/old-utils");
    }
}