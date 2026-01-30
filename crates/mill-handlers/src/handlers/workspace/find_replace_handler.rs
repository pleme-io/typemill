//! Find and replace service for workspace-wide search and replace operations
//!
//! This module provides a comprehensive find-replace tool that supports:
//! - Literal string matching with optional whole-word boundaries
//! - Regex pattern matching with capture group expansion
//! - Case-preserving replacements
//! - Configurable file scope (include/exclude patterns)
//! - Dry-run mode for safe previewing

use crate::handlers::workspace::{case_preserving, literal_matcher, regex_matcher};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::protocol::{EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit};
use regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

/// Parameters for find/replace operations
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindReplaceParams {
    /// Pattern to search for (literal or regex)
    pub pattern: String,

    /// Replacement text (may contain $1, $2 for regex mode)
    pub replacement: String,

    /// Search mode: "literal" or "regex"
    #[serde(default = "default_mode")]
    pub mode: SearchMode,

    /// For literal mode: match whole words only
    #[serde(default, alias = "wholeWord")]
    pub whole_word: bool,

    /// Preserve case style when replacing
    #[serde(default)]
    pub preserve_case: bool,

    /// Scope configuration
    #[serde(default)]
    pub scope: ScopeParam,

    /// Dry-run mode (default: true for safety)
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

/// Search mode enumeration
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    Literal,
    Regex,
}

fn default_mode() -> SearchMode {
    SearchMode::Literal
}

fn default_dry_run() -> bool {
    true
}

/// Scope parameter that accepts either a keyword string or a configuration object
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ScopeParam {
    Keyword(String),
    Config(ScopeConfig),
}

impl Default for ScopeParam {
    fn default() -> Self {
        ScopeParam::Config(ScopeConfig::default())
    }
}

/// Scope configuration for controlling which files to search
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScopeConfig {
    /// Glob patterns to include (e.g., ["**/*.rs", "**/*.toml"])
    #[serde(default)]
    pub include_patterns: Vec<String>,

    /// Glob patterns to exclude (e.g., ["**/target/**"])
    #[serde(default = "default_excludes")]
    pub exclude_patterns: Vec<String>,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            include_patterns: Vec::new(),
            exclude_patterns: default_excludes(),
        }
    }
}

fn default_excludes() -> Vec<String> {
    vec![
        "**/target/**".into(),
        "**/node_modules/**".into(),
        "**/.git/**".into(),
        "**/build/**".into(),
        "**/dist/**".into(),
    ]
}

/// Result of applying a find/replace operation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub success: bool,
    pub files_modified: Vec<String>,
    pub matches_found: usize,
    pub matches_replaced: usize,
}

/// Execute a workspace find/replace operation.
pub async fn handle_find_replace(
    context: &mill_handler_api::ToolHandlerContext,
    args: Value,
) -> ServerResult<Value> {
    let params: FindReplaceParams = serde_json::from_value(args).map_err(|e| {
        ServerError::invalid_request(format!("Failed to parse find_replace params: {}", e))
    })?;

    // Validate parameters
    if params.pattern.is_empty() {
        return Err(ServerError::invalid_request(
            "Pattern cannot be empty".to_string(),
        ));
    }

    // Validate regex pattern early (before processing any files)
    if params.mode == SearchMode::Regex {
        regex::Regex::new(&params.pattern)
            .map_err(|e| ServerError::invalid_request(format!("Invalid regex pattern: {}", e)))?;
    }

    info!(
        pattern = %params.pattern,
        mode = ?params.mode,
        dry_run = params.dry_run,
        "Starting find/replace operation"
    );

    // Get workspace root from app_state
    let workspace_root = &context.app_state.project_root;

    // Resolve scope parameter
    let scope_config = match &params.scope {
        ScopeParam::Keyword(k) => {
            if k == "workspace" {
                ScopeConfig::default()
            } else {
                return Err(ServerError::invalid_request(format!(
                    "Invalid scope keyword: '{}'. Use 'workspace' or a configuration object.",
                    k
                )));
            }
        }
        ScopeParam::Config(c) => c.clone(),
    };

    // 1. Discover files matching scope
    let files = discover_files(workspace_root, &scope_config).await?;
    debug!(files_count = files.len(), "Discovered files to search");

    // 2. Process each file
    let mut all_edits: Vec<FileEdits> = Vec::new();
    let mut total_matches = 0;

    for file_path in files {
        match process_file(&file_path, &params, context).await {
            Ok(file_edits) => {
                if !file_edits.edits.is_empty() {
                    total_matches += file_edits.edits.len();
                    all_edits.push(file_edits);
                }
            }
            Err(e) => {
                error!(
                    file_path = %file_path.display(),
                    error = %e,
                    "Failed to process file"
                );
                // Continue processing other files
            }
        }
    }

    info!(
        files_with_matches = all_edits.len(),
        total_matches = total_matches,
        "Find/replace scan complete"
    );

    // 3. Convert to EditPlan
    let plan = create_edit_plan(all_edits, &params);

    // 4. Return plan (for dry-run) or apply (for execution)
    if params.dry_run {
        info!("Dry-run mode: returning plan without applying changes");
        Ok(serde_json::to_value(plan)?)
    } else {
        info!("Executing find/replace (applying changes)");
        let files_modified = apply_plan(&plan, context).await?;

        let result = ApplyResult {
            success: true,
            files_modified: files_modified.clone(),
            matches_found: total_matches,
            matches_replaced: total_matches,
        };

        info!(
            files_modified = files_modified.len(),
            matches_replaced = total_matches,
            "Find/replace operation completed"
        );

        Ok(serde_json::to_value(result)?)
    }
}

/// Edits for a single file
struct FileEdits {
    file_path: PathBuf,
    edits: Vec<TextEdit>,
}

/// Discover files matching the scope configuration
async fn discover_files(
    workspace_root: &Path,
    scope: &ScopeConfig,
) -> Result<Vec<PathBuf>, ServerError> {
    use globset::{Glob, GlobSetBuilder};
    use ignore::WalkBuilder;

    // Build exclude matcher
    let mut exclude_builder = GlobSetBuilder::new();
    for pattern in &scope.exclude_patterns {
        let glob = Glob::new(pattern).map_err(|e| {
            ServerError::invalid_request(format!("Invalid exclude pattern '{}': {}", pattern, e))
        })?;
        exclude_builder.add(glob);
    }
    let exclude_matcher = exclude_builder
        .build()
        .map_err(|e| ServerError::internal(format!("Failed to build exclude matcher: {}", e)))?;

    // Build include matcher (if specified)
    let include_matcher = if scope.include_patterns.is_empty() {
        None
    } else {
        let mut include_builder = GlobSetBuilder::new();
        for pattern in &scope.include_patterns {
            let glob = Glob::new(pattern).map_err(|e| {
                ServerError::invalid_request(format!(
                    "Invalid include pattern '{}': {}",
                    pattern, e
                ))
            })?;
            include_builder.add(glob);
        }
        Some(include_builder.build().map_err(|e| {
            ServerError::internal(format!("Failed to build include matcher: {}", e))
        })?)
    };

    // Walk the workspace using ignore crate (respects .gitignore)
    let mut files = Vec::new();
    for entry in WalkBuilder::new(workspace_root)
        .hidden(false) // Don't skip hidden files by default
        .git_ignore(true) // Respect .gitignore
        .build()
    {
        let entry = entry.map_err(|e| ServerError::internal(format!("Walk error: {}", e)))?;
        let path = entry.path();

        // Only include files, not directories
        if !path.is_file() {
            continue;
        }

        // Get relative path for glob matching (globs work on relative paths)
        let relative_path = path.strip_prefix(workspace_root).unwrap_or(path);

        // Check exclude patterns
        if exclude_matcher.is_match(relative_path) {
            continue;
        }

        // Check include patterns (if specified)
        if let Some(ref matcher) = include_matcher {
            if !matcher.is_match(relative_path) {
                continue;
            }
        }

        files.push(path.to_path_buf());
    }

    Ok(files)
}

/// Process a single file and find all matches
async fn process_file(
    file_path: &Path,
    params: &FindReplaceParams,
    context: &mill_handler_api::ToolHandlerContext,
) -> Result<FileEdits, ServerError> {
    // Read file content
    let content = context
        .app_state
        .file_service
        .read_file(file_path)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

    // Find matches based on mode
    let edits = match params.mode {
        SearchMode::Literal => {
            let matches =
                literal_matcher::find_literal_matches(&content, &params.pattern, params.whole_word);
            convert_literal_matches_to_edits(matches, &params.replacement, params.preserve_case)?
        }
        SearchMode::Regex => {
            let matches =
                regex_matcher::find_regex_matches(&content, &params.pattern, &params.replacement)
                    .map_err(|e| ServerError::invalid_request(format!("Regex error: {}", e)))?;
            convert_regex_matches_to_edits(matches)?
        }
    };

    Ok(FileEdits {
        file_path: file_path.to_path_buf(),
        edits,
    })
}

/// Convert literal matches to TextEdit objects
fn convert_literal_matches_to_edits(
    matches: Vec<literal_matcher::Match>,
    replacement: &str,
    preserve_case: bool,
) -> Result<Vec<TextEdit>, ServerError> {
    matches
        .into_iter()
        .map(|m| {
            let replacement_text = if preserve_case {
                case_preserving::replace_preserving_case(&m.matched_text, replacement)
            } else {
                replacement.to_string()
            };

            Ok(TextEdit {
                file_path: None, // Will be set later
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: m.line - 1, // Convert from 1-indexed to 0-indexed
                    start_column: m.column - 1,
                    end_line: m.line - 1,
                    end_column: m.column - 1 + m.matched_text.chars().count() as u32,
                },
                original_text: m.matched_text.clone(),
                new_text: replacement_text,
                priority: 0,
                description: format!(
                    "Replace '{}' with '{}' (literal)",
                    m.matched_text, replacement
                ),
            })
        })
        .collect()
}

/// Convert regex matches to TextEdit objects
fn convert_regex_matches_to_edits(
    matches: Vec<regex_matcher::RegexMatch>,
) -> Result<Vec<TextEdit>, ServerError> {
    matches
        .into_iter()
        .map(|m| {
            Ok(TextEdit {
                file_path: None, // Will be set later
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: m.line - 1, // Convert from 1-indexed to 0-indexed
                    start_column: m.column,
                    end_line: m.line - 1,
                    end_column: m.column + m.matched_text.chars().count() as u32,
                },
                original_text: m.matched_text.clone(),
                new_text: m.replacement_text.clone(),
                priority: 0,
                description: format!(
                    "Replace '{}' with '{}' (regex)",
                    m.matched_text, m.replacement_text
                ),
            })
        })
        .collect()
}

/// Create an EditPlan from all file edits
fn create_edit_plan(all_edits: Vec<FileEdits>, params: &FindReplaceParams) -> EditPlan {
    let total_files = all_edits.len();
    let _total_edits: usize = all_edits.iter().map(|fe| fe.edits.len()).sum();

    // Flatten all edits into a single list, setting file_path on each edit
    let mut flattened_edits = Vec::new();
    for file_edits in all_edits {
        let file_path_str = file_edits.file_path.display().to_string();
        for mut edit in file_edits.edits {
            edit.file_path = Some(file_path_str.clone());
            flattened_edits.push(edit);
        }
    }

    EditPlan {
        source_file: "workspace".to_string(), // No single source file for workspace operations
        edits: flattened_edits,
        dependency_updates: vec![],
        validations: vec![],
        metadata: EditPlanMetadata {
            intent_name: "find_replace".to_string(),
            intent_arguments: serde_json::json!({
                "pattern": params.pattern,
                "replacement": params.replacement,
                "mode": match params.mode {
                    SearchMode::Literal => "literal",
                    SearchMode::Regex => "regex",
                }
            }),
            created_at: chrono::Utc::now(),
            complexity: total_files.min(10) as u8,
            impact_areas: vec!["workspace".to_string()],
            consolidation: None,
        },
    }
}

/// Apply an EditPlan using the file service
async fn apply_plan(
    plan: &EditPlan,
    context: &mill_handler_api::ToolHandlerContext,
) -> Result<Vec<String>, ServerError> {
    // Group edits by file
    let mut edits_by_file: HashMap<String, Vec<TextEdit>> = HashMap::new();

    for edit in &plan.edits {
        if let Some(ref file_path) = edit.file_path {
            edits_by_file
                .entry(file_path.clone())
                .or_default()
                .push(edit.clone());
        }
    }

    // Apply edits to each file
    let mut modified_files = Vec::new();
    for (file_path, edits) in edits_by_file {
        let path = PathBuf::from(&file_path);

        // Read current content
        let mut content = context
            .app_state
            .file_service
            .read_file(&path)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

        // Apply edits in reverse order (to preserve positions)
        let mut sorted_edits = edits;
        sorted_edits.sort_by(|a, b| {
            b.location
                .start_line
                .cmp(&a.location.start_line)
                .then_with(|| b.location.start_column.cmp(&a.location.start_column))
        });

        for edit in sorted_edits {
            content = apply_single_edit(&content, &edit)?;
        }

        // Write updated content
        context
            .app_state
            .file_service
            .write_file(&path, &content, false) // false = not dry run
            .await
            .map_err(|e| ServerError::internal(format!("Failed to write file: {}", e)))?;

        modified_files.push(file_path);
    }

    Ok(modified_files)
}

/// Convert a character index to a byte index in a UTF-8 string
fn char_index_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}

/// Apply a single TextEdit to content
fn apply_single_edit(content: &str, edit: &TextEdit) -> Result<String, ServerError> {
    let lines: Vec<&str> = content.lines().collect();

    let start_line = edit.location.start_line as usize;
    let end_line = edit.location.end_line as usize;

    if start_line >= lines.len() || end_line >= lines.len() {
        return Err(ServerError::internal(format!(
            "Edit range out of bounds: line {} to {}, content has {} lines",
            start_line,
            end_line,
            lines.len()
        )));
    }

    // Build new content
    let mut result = String::new();

    // Lines before the edit
    for (i, line) in lines.iter().enumerate() {
        if i < start_line {
            result.push_str(line);
            result.push('\n');
        }
    }

    // The edited line(s)
    if start_line == end_line {
        // Single line edit
        let line = lines[start_line];
        let start_char_idx = edit.location.start_column as usize;
        let end_char_idx = edit.location.end_column as usize;

        // Convert character indices to byte indices for UTF-8 safety
        let start_byte = char_index_to_byte_index(line, start_char_idx);
        let end_byte = char_index_to_byte_index(line, end_char_idx);

        if start_byte > line.len() || end_byte > line.len() {
            return Err(ServerError::internal(format!(
                "Edit byte range out of bounds: {} to {}, line length {}",
                start_byte,
                end_byte,
                line.len()
            )));
        }

        result.push_str(&line[..start_byte]);
        result.push_str(&edit.new_text);
        result.push_str(&line[end_byte..]);
        result.push('\n');
    } else {
        // Multi-line edit (rare for find/replace)
        let first_line = lines[start_line];
        let last_line = lines[end_line];

        let start_byte = char_index_to_byte_index(first_line, edit.location.start_column as usize);
        let end_byte = char_index_to_byte_index(last_line, edit.location.end_column as usize);

        result.push_str(&first_line[..start_byte]);
        result.push_str(&edit.new_text);
        result.push_str(&last_line[end_byte..]);
        result.push('\n');
    }

    // Lines after the edit
    for (i, line) in lines.iter().enumerate() {
        if i > end_line {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Remove trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode() {
        assert_eq!(default_mode(), SearchMode::Literal);
    }

    #[test]
    fn test_default_dry_run() {
        assert!(default_dry_run());
    }

    #[test]
    fn test_default_excludes() {
        let excludes = default_excludes();
        assert!(excludes.contains(&"**/target/**".to_string()));
        assert!(excludes.contains(&"**/node_modules/**".to_string()));
        assert!(excludes.contains(&"**/.git/**".to_string()));
    }

    #[test]
    fn test_apply_single_edit_basic() {
        let content = "hello world\ngoodbye world\n";
        let edit = TextEdit {
            file_path: None,
            edit_type: EditType::Replace,
            location: EditLocation {
                start_line: 0,
                start_column: 6,
                end_line: 0,
                end_column: 11,
            },
            original_text: "world".to_string(),
            new_text: "universe".to_string(),
            priority: 0,
            description: "test".to_string(),
        };

        let result = apply_single_edit(content, &edit).unwrap();
        assert_eq!(result, "hello universe\ngoodbye world\n");
    }

    #[test]
    fn test_apply_single_edit_last_line() {
        let content = "line1\nline2";
        let edit = TextEdit {
            file_path: None,
            edit_type: EditType::Replace,
            location: EditLocation {
                start_line: 1,
                start_column: 0,
                end_line: 1,
                end_column: 5,
            },
            original_text: "line2".to_string(),
            new_text: "replaced".to_string(),
            priority: 0,
            description: "test".to_string(),
        };

        let result = apply_single_edit(content, &edit).unwrap();
        assert_eq!(result, "line1\nreplaced");
    }
}
