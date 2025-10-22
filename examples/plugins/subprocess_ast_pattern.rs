// Example: Subprocess AST Pattern (Python, Node, Go, Java)
// Best for: High accuracy, language-native parsers

use cb_lang_common::{SubprocessAstTool, run_ast_tool, parse_with_fallback, ImportGraphBuilder};
use mill_plugin_api::{ PluginResult , Symbol , SymbolKind };
use mill_foundation::protocol::{ ImportGraph , ImportInfo };

/// Analyze imports using AST parser with regex fallback
pub fn analyze_imports(source: &str, file_path: Option<&Path>) -> PluginResult<ImportGraph> {
    let imports = parse_with_fallback(
        || parse_imports_ast(source),      // Primary: accurate AST
        || parse_imports_regex(source),    // Fallback: regex
        "Import parsing"
    )?;

    Ok(ImportGraphBuilder::new("mylang")
        .with_source_file(file_path)
        .with_imports(imports)
        .extract_external_dependencies(|path| !path.starts_with('.'))
        .build())
}

/// Primary parser: spawn subprocess with embedded AST tool
fn parse_imports_ast(source: &str) -> Result<Vec<ImportInfo>, PluginError> {
    const AST_TOOL: &str = include_str!("../resources/ast_tool.py");

    let tool = SubprocessAstTool::new("python3")
        .with_embedded_str(AST_TOOL)
        .with_temp_filename("ast_tool.py")
        .with_args(vec!["analyze-imports".to_string()]);

    run_ast_tool(tool, source)  // Returns deserialized JSON
}

/// Fallback parser: regex-based (when runtime unavailable)
fn parse_imports_regex(source: &str) -> PluginResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    let import_re = regex::Regex::new(r#"^import\s+([^\s]+)"#)?;

    for (line_num, line) in source.lines().enumerate() {
        if let Some(caps) = import_re.captures(line) {
            imports.push(ImportInfo {
                module_path: caps[1].to_string(),
                line: line_num + 1,
                // ... other fields
            });
        }
    }

    Ok(imports)
}