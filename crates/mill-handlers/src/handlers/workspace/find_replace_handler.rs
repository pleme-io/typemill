//! Find and replace service for workspace-wide search and replace operations
//!
//! This module provides a comprehensive find-replace tool that supports:
//! - Literal string matching with optional whole-word boundaries
//! - Regex pattern matching with capture group expansion
//! - Case-preserving replacements
//! - Configurable file scope (include/exclude patterns)
//! - Dry-run mode for safe previewing

use crate::handlers::workspace::{case_preserving, literal_matcher, regex_matcher};
use memchr::memmem;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::protocol::{EditLocation, EditPlan, EditPlanMetadata, EditType, TextEdit};
use regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info};

const STREAM_SCAN_THRESHOLD_BYTES: u64 = 1_000_000;
const STREAM_SCAN_CHUNK_BYTES: usize = 64 * 1024;

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
        // Build and cache directories
        "**/target/**".into(),
        "**/node_modules/**".into(),
        "**/.git/**".into(),
        "**/build/**".into(),
        "**/dist/**".into(),
        "**/.svelte-kit/**".into(),
        "**/.next/**".into(),
        "**/.output/**".into(),
        "**/coverage/**".into(),
        "**/.turbo/**".into(),
        // Binary image files
        "**/*.png".into(),
        "**/*.jpg".into(),
        "**/*.jpeg".into(),
        "**/*.gif".into(),
        "**/*.ico".into(),
        "**/*.webp".into(),
        "**/*.avif".into(),
        "**/*.svg".into(),
        "**/*.bmp".into(),
        // Font files
        "**/*.woff".into(),
        "**/*.woff2".into(),
        "**/*.ttf".into(),
        "**/*.otf".into(),
        "**/*.eot".into(),
        // Archive files
        "**/*.zip".into(),
        "**/*.tar".into(),
        "**/*.gz".into(),
        "**/*.rar".into(),
        "**/*.7z".into(),
        // Media files
        "**/*.mp3".into(),
        "**/*.mp4".into(),
        "**/*.webm".into(),
        "**/*.wav".into(),
        "**/*.avi".into(),
        "**/*.mov".into(),
        // Other binary files
        "**/*.glb".into(),
        "**/*.pdf".into(),
        "**/*.exe".into(),
        "**/*.dll".into(),
        "**/*.so".into(),
        "**/*.dylib".into(),
        "**/*.wasm".into(),
        // Lock files (shouldn't be modified by find/replace)
        "**/package-lock.json".into(),
        "**/yarn.lock".into(),
        "**/pnpm-lock.yaml".into(),
        "**/Cargo.lock".into(),
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

fn is_likely_binary_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(
        ext.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "ico"
            | "webp"
            | "avif"
            | "svg"
            | "bmp"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "eot"
            | "zip"
            | "tar"
            | "gz"
            | "rar"
            | "7z"
            | "mp3"
            | "mp4"
            | "webm"
            | "wav"
            | "avi"
            | "mov"
            | "glb"
            | "pdf"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "wasm"
    )
}

async fn file_may_contain_literal(path: &Path, pattern: &[u8]) -> Result<bool, ServerError> {
    if pattern.is_empty() {
        return Ok(true);
    }

    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to read metadata: {}", e)))?;

    // Small files: single read is faster than chunk orchestration.
    if metadata.len() <= STREAM_SCAN_THRESHOLD_BYTES {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;
        return Ok(memmem::find(&bytes, pattern).is_some());
    }

    // Large files: stream in chunks to avoid loading full content in memory.
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to open file: {}", e)))?;
    let finder = memmem::Finder::new(pattern);
    let overlap = pattern.len().saturating_sub(1);

    let mut carry: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; STREAM_SCAN_CHUNK_BYTES];

    loop {
        let read = file
            .read(&mut buf)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to stream file: {}", e)))?;

        if read == 0 {
            return Ok(false);
        }

        let chunk = &buf[..read];
        let mut scan_buf = Vec::with_capacity(carry.len() + chunk.len());
        scan_buf.extend_from_slice(&carry);
        scan_buf.extend_from_slice(chunk);

        if finder.find(&scan_buf).is_some() {
            return Ok(true);
        }

        if overlap == 0 {
            carry.clear();
        } else if scan_buf.len() > overlap {
            carry = scan_buf[scan_buf.len() - overlap..].to_vec();
        } else {
            carry = scan_buf;
        }
    }
}

/// Process a single file and find all matches
async fn process_file(
    file_path: &Path,
    params: &FindReplaceParams,
    _context: &mill_handler_api::ToolHandlerContext,
) -> Result<FileEdits, ServerError> {
    if is_likely_binary_file(file_path) {
        return Ok(FileEdits {
            file_path: file_path.to_path_buf(),
            edits: Vec::new(),
        });
    }

    // Fast literal prefilter. For large files we use chunked streaming scan to avoid
    // loading whole files when there is no candidate byte match.
    if params.mode == SearchMode::Literal
        && !params.pattern.is_empty()
        && !file_may_contain_literal(file_path, params.pattern.as_bytes()).await?
    {
        return Ok(FileEdits {
            file_path: file_path.to_path_buf(),
            edits: Vec::new(),
        });
    }

    // Read file content only after passing prefilters.
    let content = _context
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
        let content = context
            .app_state
            .file_service
            .read_file(&path)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

        // Apply edits
        let new_content = apply_edits(&content, edits)?;

        // Write updated content
        context
            .app_state
            .file_service
            .write_file(&path, &new_content, false) // false = not dry run
            .await
            .map_err(|e| ServerError::internal(format!("Failed to write file: {}", e)))?;

        modified_files.push(file_path);
    }

    Ok(modified_files)
}

/// Apply multiple TextEdits to content in a single pass (optimized O(N + M) where N is content size, M is edits count)
fn apply_edits(content: &str, edits: Vec<TextEdit>) -> Result<String, ServerError> {
    if edits.is_empty() {
        return Ok(content.to_string());
    }

    // Sort edits by start position (ascending)
    let mut sorted_edits = edits;
    sorted_edits.sort_by(|a, b| {
        a.location
            .start_line
            .cmp(&b.location.start_line)
            .then_with(|| a.location.start_column.cmp(&b.location.start_column))
    });

    // Calculate approximate capacity to reduce reallocations
    // Improved estimation: content + new - old (saturated to avoid underflow)
    let total_new_text_len: usize = sorted_edits.iter().map(|e| e.new_text.len()).sum();
    let total_old_text_len: usize = sorted_edits.iter().map(|e| e.original_text.len()).sum();
    let estimated_capacity = content
        .len()
        .saturating_add(total_new_text_len)
        .saturating_sub(total_old_text_len);

    let mut result = String::with_capacity(estimated_capacity);

    let mut last_byte_idx = 0;

    // Iterator state
    let mut current_line = 0;
    let mut current_col = 0;
    let mut current_byte_idx = 0;

    for edit in sorted_edits {
        let start_line = edit.location.start_line as usize;
        let start_col = edit.location.start_column as usize;
        let end_line = edit.location.end_line as usize;
        let end_col = edit.location.end_column as usize;

        // 1. Advance to start position
        // Skip full lines if possible using memchr logic (via str::find)
        while current_line < start_line {
            // If we are at the end of content, this will return None
            match content[current_byte_idx..].find('\n') {
                Some(idx) => {
                    current_byte_idx += idx + 1; // Skip past \n
                    current_line += 1;
                    current_col = 0;
                }
                None => {
                    return Err(ServerError::internal(format!(
                        "Edit start position {}:{} is out of bounds (end at line {})",
                        start_line, start_col, current_line
                    )));
                }
            }
        }

        // Now we are on the correct line, advance to start_col
        // We must iterate chars here because column is in chars
        let mut chars_iter = content[current_byte_idx..].char_indices();
        let mut additional_bytes = 0;

        if current_col != start_col {
            loop {
                match chars_iter.next() {
                    Some((idx, ch)) => {
                        if ch == '\n' {
                            return Err(ServerError::internal(format!(
                                "Edit start position {}:{} is out of bounds (end of line at column {})",
                                start_line, start_col, current_col
                            )));
                        }
                        current_col += 1;
                        additional_bytes = idx + ch.len_utf8();
                        if current_col == start_col {
                            break;
                        }
                    }
                    None => {
                        // Check if we reached the column exactly at EOF
                        if current_col == start_col {
                            break;
                        }
                        return Err(ServerError::internal(format!(
                            "Edit start position {}:{} is out of bounds (end of content)",
                            start_line, start_col
                        )));
                    }
                }
            }
        }

        let start_byte_idx = current_byte_idx + additional_bytes;

        // Validate overlap
        if start_byte_idx < last_byte_idx {
            return Err(ServerError::internal(format!(
                "Overlapping edit detected at byte {} (last was {})",
                start_byte_idx, last_byte_idx
            )));
        }

        // Append text from last_byte_idx up to start_byte_idx
        result.push_str(&content[last_byte_idx..start_byte_idx]);

        // Append replacement text
        result.push_str(&edit.new_text);

        // Update current position to start_byte_idx for end position search
        current_byte_idx = start_byte_idx;
        // current_line is still start_line
        // current_col is start_col

        // 2. Advance to end position
        // Similar logic: skip lines if needed
        while current_line < end_line {
            match content[current_byte_idx..].find('\n') {
                Some(idx) => {
                    current_byte_idx += idx + 1;
                    current_line += 1;
                    current_col = 0;
                }
                None => {
                    return Err(ServerError::internal(format!(
                        "Edit end position {}:{} is out of bounds (end at line {})",
                        end_line, end_col, current_line
                    )));
                }
            }
        }

        // Advance to end_col
        chars_iter = content[current_byte_idx..].char_indices();
        additional_bytes = 0;

        if current_col != end_col {
            loop {
                match chars_iter.next() {
                    Some((idx, ch)) => {
                        if ch == '\n' {
                            return Err(ServerError::internal(format!(
                                "Edit end position {}:{} is out of bounds (end of line at column {})",
                                end_line, end_col, current_col
                            )));
                        }
                        current_col += 1;
                        additional_bytes = idx + ch.len_utf8();
                        if current_col == end_col {
                            break;
                        }
                    }
                    None => {
                        if current_col == end_col {
                            break;
                        }
                        return Err(ServerError::internal(format!(
                            "Edit end position {}:{} is out of bounds (end of content)",
                            end_line, end_col
                        )));
                    }
                }
            }
        }

        // Check original text if provided (Safety Check)
        let end_byte_idx = current_byte_idx + additional_bytes;
        if !edit.original_text.is_empty() {
            let actual_text = &content[start_byte_idx..end_byte_idx];
            if actual_text != edit.original_text {
                return Err(ServerError::internal(format!(
                    "Edit conflict: Expected original text '{}' but found '{}' at {}:{}",
                    edit.original_text, actual_text, start_line, start_col
                )));
            }
        }

        // Update last_byte_idx to point to the end of the replaced range
        last_byte_idx = end_byte_idx;

        // Prepare current_byte_idx for next edit
        current_byte_idx = last_byte_idx;
        // current_line is already end_line
        // current_col is end_col
    }

    // Append remaining text
    if last_byte_idx < content.len() {
        result.push_str(&content[last_byte_idx..]);
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

        let result = apply_edits(content, vec![edit]).unwrap();
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

        let result = apply_edits(content, vec![edit]).unwrap();
        assert_eq!(result, "line1\nreplaced");
    }

    #[test]
    fn test_apply_edits_safety_check() {
        let content = "hello world";
        let edit = TextEdit {
            file_path: None,
            edit_type: EditType::Replace,
            location: EditLocation {
                start_line: 0,
                start_column: 6,
                end_line: 0,
                end_column: 11,
            },
            original_text: "universe".to_string(), // Mismatch! Content has "world"
            new_text: "galaxy".to_string(),
            priority: 0,
            description: "test".to_string(),
        };

        let result = apply_edits(content, vec![edit]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Edit conflict"));
    }
}

#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_apply_edits_performance() {
        // Create a large content
        let mut content = String::new();
        for i in 0..10000 {
            content.push_str(&format!("line {} with some content to replace\n", i));
        }

        // Create many edits
        let mut edits = Vec::new();
        for i in (0..10000).step_by(10) {
            let original = format!("{}", i);
            let len = original.len() as u32;
            edits.push(TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: i,
                    start_column: 5,
                    end_line: i,
                    end_column: 5 + len,
                },
                original_text: original,
                new_text: "REPLACED".to_string(),
                priority: 0,
                description: "benchmark".to_string(),
            });
        }

        println!(
            "Benchmarking apply_edits with {} edits on {} lines...",
            edits.len(),
            10000
        );

        // Run Optimized
        let start_opt = Instant::now();
        let _ = apply_edits(&content, edits).unwrap();
        let duration_opt = start_opt.elapsed();
        println!("Optimized implementation: {:?}", duration_opt);

        // Assert performance is reasonable (< 100ms)
        assert!(
            duration_opt.as_millis() < 100,
            "Optimized implementation should be faster than 100ms"
        );
    }
}
