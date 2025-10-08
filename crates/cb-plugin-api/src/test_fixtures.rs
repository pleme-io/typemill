//! Test fixtures that language plugins can optionally provide
//!
//! This module enables plugins to self-provide test scenarios for
//! integration testing. When a new language plugin is added, it can
//! define its own test contracts without modifying the test framework.

use serde::{Deserialize, Serialize};

/// Collection of test fixtures a language plugin can provide
#[derive(Debug, Clone, Default)]
pub struct LanguageTestFixtures {
    /// Complexity analysis test scenarios
    pub complexity_scenarios: Vec<ComplexityFixture>,

    /// Refactoring operation test scenarios
    pub refactoring_scenarios: Vec<RefactoringFixture>,
}

/// A complexity analysis test scenario
#[derive(Debug, Clone)]
pub struct ComplexityFixture {
    /// Scenario identifier (e.g., "simple_function", "nested_complexity")
    pub scenario_name: &'static str,

    /// Source code for this scenario
    pub source_code: &'static str,

    /// Filename with correct extension (e.g., "simple.py")
    pub file_name: &'static str,

    /// Expected cyclomatic complexity range
    pub expected_cyclomatic_min: u32,
    pub expected_cyclomatic_max: u32,

    /// Expected cognitive complexity range
    pub expected_cognitive_min: u32,
    pub expected_cognitive_max: u32,

    /// Expected nesting depth
    pub expected_nesting_depth_min: u32,
}

/// A refactoring test scenario
#[derive(Debug, Clone)]
pub struct RefactoringFixture {
    /// Scenario identifier (e.g., "extract_simple_expression")
    pub scenario_name: &'static str,

    /// Source code for this scenario
    pub source_code: &'static str,

    /// Filename with correct extension
    pub file_name: &'static str,

    /// Refactoring operation to perform
    pub operation: RefactoringOperation,
}

/// Refactoring operation definition
#[derive(Debug, Clone)]
pub enum RefactoringOperation {
    ExtractFunction {
        new_name: String,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    },
    InlineVariable {
        line: u32,
        character: u32,
    },
    ExtractVariable {
        variable_name: String,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    },
}

impl RefactoringOperation {
    /// Convert to MCP tool name
    pub fn to_mcp_tool_name(&self) -> &'static str {
        match self {
            RefactoringOperation::ExtractFunction { .. } => "extract_function",
            RefactoringOperation::InlineVariable { .. } => "inline_variable",
            RefactoringOperation::ExtractVariable { .. } => "extract_variable",
        }
    }

    /// Convert to JSON parameters for MCP call
    pub fn to_json_params(&self, file_path: &str) -> serde_json::Value {
        match self {
            RefactoringOperation::ExtractFunction {
                new_name,
                start_line,
                start_char: _,
                end_line,
                end_char: _,
            } => serde_json::json!({
                "file_path": file_path,
                "start_line": start_line,
                "end_line": end_line,
                "function_name": new_name
            }),
            RefactoringOperation::InlineVariable { line, character } => serde_json::json!({
                "file_path": file_path,
                "line": line,
                "character": character
            }),
            RefactoringOperation::ExtractVariable {
                variable_name,
                start_line,
                start_char,
                end_line,
                end_char,
            } => serde_json::json!({
                "file_path": file_path,
                "start_line": start_line,
                "start_character": start_char,
                "end_line": end_line,
                "end_character": end_char,
                "variable_name": variable_name
            }),
        }
    }
}