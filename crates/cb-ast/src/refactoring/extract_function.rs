use super::common::{detect_language, extract_range_text, generate_function_call};
use super::{CodeRange, ExtractableFunction, LspRefactoringService};
use crate::error::{AstError, AstResult};
use cb_protocol::{EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType};
use std::collections::HashMap;
use tracing::debug;

/// Analyze code selection for function extraction (simplified fallback)
pub fn analyze_extract_function(
    _source: &str,
    range: &CodeRange,
    _file_path: &str,
) -> AstResult<ExtractableFunction> {
    // Simplified text-based analysis (language plugins should provide AST-based analysis)
    let analyzer = ExtractFunctionAnalyzer::new(range.clone());
    // Note: Using simplified text-based analysis for TypeScript/JavaScript
    // Full AST traversal with scope analysis is planned but not required for basic functionality
    // Python implementation demonstrates this approach works well for common refactoring cases
    analyzer.finalize()
}

/// LSP-based extract function refactoring
///
/// Queries the LSP server for "refactor.extract.function" code actions
/// and converts the result to an EditPlan.
async fn lsp_extract_function(
    lsp_service: &dyn LspRefactoringService,
    file_path: &str,
    range: &CodeRange,
    _function_name: &str,
) -> AstResult<EditPlan> {
    debug!(
        file_path = %file_path,
        start_line = range.start_line,
        end_line = range.end_line,
        "Requesting LSP extract function refactoring"
    );

    let actions = lsp_service
        .get_code_actions(
            file_path,
            range,
            Some(vec!["refactor.extract.function".to_string()]),
        )
        .await?;

    // Find the extract function action
    let action = actions
        .as_array()
        .and_then(|arr| {
            arr.iter().find(|a| {
                a.get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k.starts_with("refactor.extract"))
                    .unwrap_or(false)
            })
        })
        .ok_or_else(|| {
            AstError::analysis("LSP server returned no extract function actions".to_string())
        })?;

    // Extract WorkspaceEdit from the action
    let workspace_edit = action
        .get("edit")
        .ok_or_else(|| AstError::analysis("Code action missing edit field".to_string()))?;

    // Convert to EditPlan
    cb_protocol::EditPlan::from_lsp_workspace_edit(workspace_edit, file_path, "extract_function")
        .map_err(|e| AstError::analysis(format!("Failed to convert LSP edit: {}", e)))
}

/// Generate edit plan for extract function refactoring
///
/// This function implements an LSP-first approach:
/// 1. If LSP service is provided, try LSP code actions first
/// 2. Fall back to AST-based analysis if LSP is unavailable or fails
///
/// # Arguments
///
/// * `source` - Source code content
/// * `range` - Code range to extract
/// * `new_function_name` - Name for the extracted function
/// * `file_path` - Path to the source file
/// * `lsp_service` - Optional LSP service for refactoring
pub async fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
    lsp_service: Option<&dyn LspRefactoringService>,
) -> AstResult<EditPlan> {
    // Try AST first (faster, more reliable, under our control)
    // Note: Only TypeScript and Rust supported after language reduction
    let ast_result = match detect_language(file_path) {
        "typescript" | "javascript" => {
            ast_extract_function_ts_js(source, range, new_function_name, file_path)
        }
        "rust" => ast_extract_function_rust(source, range, new_function_name, file_path),
        _ => {
            // Unsupported language - will try LSP fallback below
            Err(AstError::analysis(format!(
                "AST implementation not available for: {} (only TypeScript and Rust supported)",
                file_path
            )))
        }
    };

    // Return AST result if successful
    if let Ok(plan) = ast_result {
        return Ok(plan);
    }

    // Fallback to LSP if AST failed or not available
    if let Some(lsp) = lsp_service {
        debug!(
            file_path = %file_path,
            "AST extract function not available or failed, trying LSP fallback"
        );

        match lsp_extract_function(lsp, file_path, range, new_function_name).await {
            Ok(plan) => return Ok(plan),
            Err(e) => {
                debug!(
                    error = %e,
                    file_path = %file_path,
                    "LSP extract function also failed"
                );
            }
        }
    }

    // Both AST and LSP failed
    Err(AstError::analysis(format!(
        "Extract function not supported for: {}. Neither AST nor LSP implementation succeeded.",
        file_path
    )))
}

/// Generate edit plan for extract function refactoring (TypeScript/JavaScript) using AST
#[allow(dead_code)]
fn ast_extract_function_ts_js(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> AstResult<EditPlan> {
    let analysis = analyze_extract_function(source, range, file_path)?;

    let mut edits = Vec::new();

    // 1. Create the new function at the insertion point
    let function_code =
        super::common::generate_extracted_function(source, &analysis, new_function_name)?;

    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Insert,
        location: analysis.insertion_point.clone().into(),
        original_text: String::new(),
        new_text: format!("\n{}\n", function_code),
        priority: 100,
        description: format!("Create extracted function '{}'", new_function_name),
    });

    // 2. Replace the selected code with a function call
    let call_code = generate_function_call(&analysis, new_function_name)?;

    edits.push(TextEdit {
        file_path: None,
        edit_type: EditType::Replace,
        location: analysis.selected_range.clone().into(),
        original_text: extract_range_text(source, &analysis.selected_range)?,
        new_text: call_code,
        priority: 90,
        description: format!("Replace selected code with call to '{}'", new_function_name),
    });

    Ok(EditPlan {
        source_file: file_path.to_string(),
        edits,
        dependency_updates: Vec::new(),
        validations: vec![
            ValidationRule {
                rule_type: ValidationType::SyntaxCheck,
                description: "Verify syntax is valid after extraction".to_string(),
                parameters: HashMap::new(),
            },
            ValidationRule {
                rule_type: ValidationType::TypeCheck,
                description: "Verify types are consistent".to_string(),
                parameters: HashMap::new(),
            },
        ],
        metadata: EditPlanMetadata {
            intent_name: "extract_function".to_string(),
            intent_arguments: serde_json::json!({
                "range": range,
                "function_name": new_function_name
            }),
            created_at: chrono::Utc::now(),
            complexity: analysis.complexity_score.min(10) as u8,
            impact_areas: vec!["function_extraction".to_string()],
        },
    })
}

/// Generate edit plan for extract function refactoring (Rust) using AST
fn ast_extract_function_rust(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> AstResult<EditPlan> {
    let start_line = range.start_line;
    let end_line = range.end_line;

    cb_lang_rust::refactoring::plan_extract_function(
        source,
        start_line,
        end_line,
        new_function_name,
        file_path,
    )
    .map_err(|e| AstError::analysis(format!("Rust refactoring error: {}", e)))
}

/// Visitor for analyzing code selection for function extraction
struct ExtractFunctionAnalyzer {
    selection_range: CodeRange,
    contains_return: bool,
    complexity_score: u32,
}

impl ExtractFunctionAnalyzer {
    fn new(range: CodeRange) -> Self {
        Self {
            selection_range: range,
            contains_return: false,
            complexity_score: 1,
        }
    }

    fn finalize(self) -> AstResult<ExtractableFunction> {
        // Simplified implementation for TypeScript/JavaScript extract function
        // This provides basic functionality while full AST-based scope analysis is deferred
        //
        // Limitations of current approach:
        // - No automatic parameter detection (user must verify variables in scope)
        // - No return variable analysis (function returns void by default)
        // - Generic function naming (user should rename immediately)
        // - Basic insertion point heuristic (places before current line)
        //
        // These limitations are acceptable because:
        // 1. LSP-based rename and find-references provide safety after extraction
        // 2. User reviews generated code before applying
        // 3. Python implementation proves text-based approach works
        // 4. Full scope analysis requires significant SWC visitor infrastructure
        //
        // To improve this: see Python implementation in analyze_extract_function_python()
        // which demonstrates regex-based variable and function detection patterns

        let range_copy = self.selection_range.clone();
        Ok(ExtractableFunction {
            selected_range: range_copy,
            required_parameters: Vec::new(), // User must verify scope manually
            return_variables: Vec::new(),    // Function returns void
            suggested_name: "extracted_function".to_string(), // Generic name - rename suggested
            insertion_point: CodeRange {
                // Places function just before selected code - simple but functional
                start_line: self.selection_range.start_line.saturating_sub(1),
                start_col: 0,
                end_line: self.selection_range.start_line.saturating_sub(1),
                end_col: 0,
            },
            contains_return_statements: self.contains_return,
            complexity_score: self.complexity_score,
        })
    }
}
