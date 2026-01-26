//! Cross-file reference enhancement for find_references
//!
//! The LSP's textDocument/references only finds references in files that have been opened.
//! This module enhances find_references by:
//! 1. Detecting when LSP only returned same-file references
//! 2. Using grep to discover files that import the source file
//! 3. Querying LSP for references in those importing files
//! 4. Merging results into a comprehensive cross-file reference list
//!
//! This follows the hybrid grep+LSP approach for reliable cross-file reference discovery.

use futures::stream::{self, StreamExt};
use ignore::WalkBuilder;
use mill_foundation::errors::MillResult as ServerResult;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

/// Default patterns to exclude from file discovery
const DEFAULT_EXCLUDES: &[&str] = &[
    "**/node_modules/**",
    "**/target/**",
    "**/.git/**",
    "**/dist/**",
    "**/build/**",
    "**/.next/**",
    "**/coverage/**",
];

/// File extensions to search for imports
const SEARCHABLE_EXTENSIONS: &[&str] = &[
    "js", "jsx", "ts", "tsx", "mjs", "mts", "cjs", "cts",   // JavaScript/TypeScript
    "rs",    // Rust
    "py",    // Python
    "go",    // Go
    "java",  // Java
    "swift", // Swift
    "cs",    // C#
];

/// Location structure matching LSP Location format
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    pub uri: String,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

impl Location {
    fn from_json(value: &Value) -> Option<Self> {
        let uri = value.get("uri")?.as_str()?.to_string();
        let range = value.get("range")?;
        let start = range.get("start")?;
        let end = range.get("end")?;

        Some(Location {
            uri,
            start_line: start.get("line")?.as_u64()? as u32,
            start_character: start.get("character")?.as_u64()? as u32,
            end_line: end.get("line")?.as_u64()? as u32,
            end_character: end.get("character")?.as_u64()? as u32,
        })
    }

    fn to_json(&self) -> Value {
        json!({
            "uri": self.uri,
            "range": {
                "start": {
                    "line": self.start_line,
                    "character": self.start_character
                },
                "end": {
                    "line": self.end_line,
                    "character": self.end_character
                }
            }
        })
    }
}

/// Check if all locations are from the same file
fn is_same_file_only(locations: &[Location], source_uri: &str) -> bool {
    locations.iter().all(|loc| loc.uri == source_uri)
}

/// Extract locations from LSP response
fn extract_locations(response: &Value) -> Vec<Location> {
    let content = response.get("content").unwrap_or(response);
    let locations_array = content.get("locations").and_then(|v| v.as_array());

    match locations_array {
        Some(arr) => arr.iter().filter_map(Location::from_json).collect(),
        None => Vec::new(),
    }
}

/// Search parameters for finding importing files
struct ImportSearchPatterns {
    source_name: String,
    source_filename: String,
    source_path_str: String,
    source_without_ext: String,
    parent_filename: String,
}

/// Discover files that might import the source file
pub async fn discover_importing_files(
    workspace_root: &Path,
    source_file: &Path,
    _context: &mill_handler_api::ToolHandlerContext,
) -> ServerResult<Vec<PathBuf>> {
    use globset::{Glob, GlobSetBuilder};

    // Build exclude matcher
    let mut exclude_builder = GlobSetBuilder::new();
    for pattern in DEFAULT_EXCLUDES {
        if let Ok(glob) = Glob::new(pattern) {
            exclude_builder.add(glob);
        }
    }
    let exclude_matcher = exclude_builder.build().unwrap_or_default();

    // Get the source file name/path for pattern matching
    let source_name = source_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    // Also get full filename with extension (for import path matching)
    let source_filename = source_file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    // Get parent directory name (for relative import matching like '../galactic/')
    let source_parent = source_file
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("");

    // Get relative path from workspace root for import matching
    let relative_source = source_file
        .strip_prefix(workspace_root)
        .unwrap_or(source_file);

    debug!(
        source_file = %source_file.display(),
        source_name = %source_name,
        source_filename = %source_filename,
        source_parent = %source_parent,
        "Import matching patterns"
    );

    // Prepare search patterns
    let source_path_str = relative_source.to_string_lossy().to_string();
    let source_without_ext = relative_source
        .with_extension("")
        .to_string_lossy()
        .to_string();

    // Build pattern for relative import: "parent/filename" (e.g., "galactic/Galactic.is.js")
    let parent_filename = if !source_parent.is_empty() && !source_filename.is_empty() {
        format!("{}/{}", source_parent, source_filename)
    } else {
        String::new()
    };

    let search_patterns = Arc::new(ImportSearchPatterns {
        source_name: source_name.clone(),
        source_filename,
        source_path_str,
        source_without_ext,
        parent_filename,
    });

    // Walk workspace in a blocking task to avoid blocking the executor
    let workspace_root_owned = workspace_root.to_path_buf();
    let source_file_owned = source_file.to_path_buf();
    // GlobSet is cheap to clone (Arc internals) and Send/Sync
    let exclude_matcher_owned = exclude_matcher.clone();

    let candidate_files = tokio::task::spawn_blocking(move || {
        WalkBuilder::new(&workspace_root_owned)
            .hidden(false)
            .git_ignore(true)
            .build()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|path| {
                // Skip non-files
                if !path.is_file() {
                    return false;
                }

                // Skip excluded paths
                if exclude_matcher_owned.is_match(path) {
                    return false;
                }

                // Skip non-searchable extensions
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !SEARCHABLE_EXTENSIONS.contains(&ext) {
                    return false;
                }

                // Skip the source file itself
                if path == &source_file_owned {
                    return false;
                }

                true
            })
            .collect::<Vec<PathBuf>>()
    })
    .await
    .map_err(|e| mill_foundation::errors::MillError::internal(format!("Task join error: {}", e)))?;

    // Process files in parallel
    let mut importing_files: Vec<PathBuf> = stream::iter(candidate_files)
        .map(|path| {
            let patterns = search_patterns.clone();
            async move {
                // Read file asynchronously
                // Use tokio::fs to avoid blocking the executor
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    // Check various import patterns - use word boundaries to avoid false positives
                    // e.g., "is" shouldn't match inside "promise"
                    let has_import = contains_word(&content, &patterns.source_name)
                        && (
                            // ES6 import
                            content.contains("import ")
                            // CommonJS require
                            || content.contains("require(")
                            // Rust use
                            || content.contains("use ")
                            // Python import
                            || content.contains("from ")
                            // Go import
                            || content.contains("import \"")
                        );

                    if has_import {
                        // More specific check: does it reference the source file?
                        // Path-based checks can use contains() since paths are specific
                        // Word boundary check for source_name to avoid false positives
                        let matches = content.contains(&patterns.source_path_str)  // Full relative path from workspace
                            || content.contains(&patterns.source_without_ext)      // Without extension
                            || content.contains(&patterns.source_filename)         // Just filename with ext
                            || (!patterns.parent_filename.is_empty() && content.contains(&patterns.parent_filename))  // parent/file pattern
                            || contains_word(&content, &patterns.source_name); // Just file stem (with word boundary)

                        if matches {
                            debug!(
                                file = %path.display(),
                                "Found importing file"
                            );
                            return Some(path);
                        }
                    }
                }
                None
            }
        })
        .buffer_unordered(50) // Process up to 50 files concurrently
        .filter_map(|opt| async { opt })
        .collect()
        .await;

    // Sort to ensure deterministic order
    importing_files.sort();

    debug!(
        source = %source_file.display(),
        importing_count = importing_files.len(),
        "Discovered importing files"
    );

    Ok(importing_files)
}

/// Enhance find_references with cross-file discovery
///
/// This function takes the original LSP response and enhances it by:
/// 1. Checking if results are same-file only
/// 2. Discovering files that import the source
/// 3. Querying each importing file for references
/// 4. Merging all results
pub async fn enhance_find_references(
    original_response: Value,
    source_file: &Path,
    line: u32,
    character: u32,
    context: &mill_handler_api::ToolHandlerContext,
) -> ServerResult<Value> {
    // Extract original locations
    let mut locations: HashSet<Location> =
        extract_locations(&original_response).into_iter().collect();

    let source_uri = format!("file://{}", source_file.display());

    // Check if we only have same-file references
    let locations_vec: Vec<_> = locations.iter().cloned().collect();
    if !locations_vec.is_empty() && !is_same_file_only(&locations_vec, &source_uri) {
        // Already have cross-file references, return as-is
        debug!("LSP already returned cross-file references, no enhancement needed");
        return Ok(original_response);
    }

    debug!(
        source = %source_file.display(),
        original_count = locations.len(),
        "Enhancing find_references with cross-file discovery"
    );

    // Find workspace root (walk up to find Cargo.toml, package.json, or .git)
    let workspace_root = find_workspace_root(source_file)
        .unwrap_or_else(|| source_file.parent().unwrap_or(source_file).to_path_buf());

    // Discover importing files
    let importing_files = discover_importing_files(&workspace_root, source_file, context).await?;

    if importing_files.is_empty() {
        debug!("No importing files found");
        return Ok(original_response);
    }

    // For each importing file, query LSP for references
    // This is where we'd ideally open the file in LSP and query
    // For now, we'll do a simpler grep-based approach

    // Read source content once
    let source_content = tokio::fs::read_to_string(source_file).await.ok();
    let symbol_name = source_content
        .as_deref()
        .and_then(|c| extract_symbol_at_position(c, line, character));

    if let Some(symbol_name) = symbol_name {
        let symbol_name = Arc::new(symbol_name);

        // Process files in parallel
        let results = stream::iter(importing_files)
            .map(|importing_file| {
                let symbol_name = symbol_name.clone();
                async move {
                    // Read file content using tokio::fs to avoid blocking
                    if let Ok(content) = tokio::fs::read_to_string(&importing_file).await {
                        // Find occurrences of the symbol in the importing file
                        let file_uri = format!("file://{}", importing_file.display());
                        let matches = find_symbol_occurrences(&content, &symbol_name);

                        if !matches.is_empty() {
                            let locs: Vec<Location> = matches
                                .into_iter()
                                .map(|(match_line, match_char, match_len)| Location {
                                    uri: file_uri.clone(),
                                    start_line: match_line,
                                    start_character: match_char,
                                    end_line: match_line,
                                    end_character: match_char + match_len,
                                })
                                .collect();
                            return Some(locs);
                        }
                    }
                    None
                }
            })
            .buffer_unordered(50) // Process up to 50 files concurrently
            .filter_map(|opt| async { opt })
            .collect::<Vec<Vec<Location>>>()
            .await;

        for locs in results {
            locations.extend(locs);
        }
    }

    // Convert back to JSON response format
    let locations_json: Vec<Value> = locations.iter().map(|loc| loc.to_json()).collect();

    // Preserve original response metadata
    let plugin = original_response
        .get("plugin")
        .cloned()
        .unwrap_or(json!("enhanced"));
    let processing_time = original_response
        .get("processing_time_ms")
        .cloned()
        .unwrap_or(json!(0));
    let cached = original_response
        .get("cached")
        .cloned()
        .unwrap_or(json!(false));

    debug!(
        original_count = extract_locations(&original_response).len(),
        enhanced_count = locations.len(),
        "Cross-file reference enhancement complete"
    );

    Ok(json!({
        "content": {
            "locations": locations_json
        },
        "plugin": plugin,
        "processing_time_ms": processing_time,
        "cached": cached,
        "enhanced": true,
        "enhancement_source": "cross_file_grep"
    }))
}

/// Find workspace root by looking for marker files
fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let markers = [
        "Cargo.toml",
        "package.json",
        ".git",
        "go.mod",
        "pyproject.toml",
    ];

    let mut current = start.parent()?;
    while current.parent().is_some() {
        for marker in &markers {
            if current.join(marker).exists() {
                return Some(current.to_path_buf());
            }
        }
        current = current.parent()?;
    }
    None
}

/// Extract the symbol name at a given position in source code (public wrapper)
pub fn extract_symbol_at_position_public(
    content: &str,
    line: u32,
    character: u32,
) -> Option<String> {
    extract_symbol_at_position(content, line, character)
}

/// Extract the symbol name at a given position in source code
fn extract_symbol_at_position(content: &str, line: u32, character: u32) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let target_line = lines.get(line as usize)?;

    // Find word boundaries around the character position
    let chars: Vec<char> = target_line.chars().collect();
    let char_pos = character as usize;

    if char_pos >= chars.len() {
        return None;
    }

    // Find start of identifier
    let mut start = char_pos;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }

    // Find end of identifier
    let mut end = char_pos;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    if start == end {
        return None;
    }

    let symbol: String = chars[start..end].iter().collect();

    // Validate it's a reasonable identifier
    if symbol.is_empty() || symbol.chars().next()?.is_numeric() {
        return None;
    }

    Some(symbol)
}

/// Check if content contains a word with proper word boundaries
/// This prevents "is" from matching inside "promise"
fn contains_word(content: &str, word: &str) -> bool {
    let mut search_start = 0;
    while let Some(pos) = content[search_start..].find(word) {
        let absolute_pos = search_start + pos;

        // Check word boundaries
        let before_ok = absolute_pos == 0
            || !content
                .chars()
                .nth(absolute_pos - 1)
                .map(|c| c.is_alphanumeric() || c == '_')
                .unwrap_or(false);

        let after_pos = absolute_pos + word.len();
        let after_ok = after_pos >= content.len()
            || !content
                .chars()
                .nth(after_pos)
                .map(|c| c.is_alphanumeric() || c == '_')
                .unwrap_or(false);

        if before_ok && after_ok {
            return true;
        }

        search_start = absolute_pos + 1;
    }
    false
}

/// Find all occurrences of a symbol in content
/// Returns Vec<(line, character, length)> - all 0-indexed
fn find_symbol_occurrences(content: &str, symbol: &str) -> Vec<(u32, u32, u32)> {
    let mut occurrences = Vec::new();
    let symbol_len = symbol.chars().count();
    let symbol_chars: Vec<char> = symbol.chars().collect();

    if symbol_chars.is_empty() {
        return occurrences;
    }

    for (line_idx, line) in content.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let mut in_string = false;
        let mut quote_char = '\0';
        let mut escaped = false;

        while i < chars.len() {
            let c = chars[i];

            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == quote_char {
                    in_string = false;
                }
            } else {
                // Not in string
                if c == '"' || c == '\'' || c == '`' {
                    in_string = true;
                    quote_char = c;
                } else if c == symbol_chars[0] {
                    // Possible match
                    if i + symbol_len <= chars.len()
                        && &chars[i..i + symbol_len] == symbol_chars.as_slice()
                    {
                        // Check word boundaries
                        let before_ok =
                            i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');

                        let after_idx = i + symbol_len;
                        let after_ok = after_idx >= chars.len()
                            || (!chars[after_idx].is_alphanumeric() && chars[after_idx] != '_');

                        if before_ok && after_ok {
                            occurrences.push((line_idx as u32, i as u32, symbol_len as u32));
                        }
                    }
                }
            }
            i += 1;
        }
    }

    occurrences
}

/// Enhance a symbol rename WorkspaceEdit with cross-file edits
///
/// The LSP's textDocument/rename only returns edits for opened files.
/// This function discovers additional files that use the symbol and adds
/// edits for those files to the WorkspaceEdit.
#[allow(clippy::mutable_key_type)] // lsp_types::Uri has interior mutability but is used as a key by the LSP spec
pub async fn enhance_symbol_rename(
    mut workspace_edit: lsp_types::WorkspaceEdit,
    source_file: &Path,
    _line: u32,
    _character: u32,
    old_name: &str,
    new_name: &str,
    context: &mill_handler_api::ToolHandlerContext,
) -> ServerResult<lsp_types::WorkspaceEdit> {
    use lsp_types::{TextEdit, Uri};

    let source_uri = format!("file://{}", source_file.display());

    // Check if we already have cross-file edits
    let has_cross_file = workspace_edit
        .changes
        .as_ref()
        .map(|changes| changes.keys().any(|uri| uri.as_str() != source_uri))
        .unwrap_or(false);

    if has_cross_file {
        debug!("LSP already returned cross-file rename edits, no enhancement needed");
        return Ok(workspace_edit);
    }

    debug!(
        source = %source_file.display(),
        old_name = %old_name,
        new_name = %new_name,
        "Enhancing symbol rename with cross-file discovery"
    );

    // Find workspace root
    let workspace_root = find_workspace_root(source_file)
        .unwrap_or_else(|| source_file.parent().unwrap_or(source_file).to_path_buf());

    // Discover importing files
    let importing_files = discover_importing_files(&workspace_root, source_file, context).await?;

    if importing_files.is_empty() {
        debug!("No importing files found");
        return Ok(workspace_edit);
    }

    // Ensure changes map exists
    let changes = workspace_edit.changes.get_or_insert_with(Default::default);

    let mut added_files = 0;
    let mut added_edits = 0;

    // For each importing file, find occurrences of the old symbol name
    let old_name = Arc::new(old_name.to_string());
    let new_name = Arc::new(new_name.to_string());

    let results = stream::iter(importing_files)
        .map(|importing_file| {
            let old_name = old_name.clone();
            let new_name = new_name.clone();
            async move {
                // Read file content using tokio::fs to avoid blocking
                if let Ok(content) = tokio::fs::read_to_string(&importing_file).await {
                    // Find occurrences of the old symbol name
                    let occurrences = find_symbol_occurrences(&content, &old_name);

                    if !occurrences.is_empty() {
                        // Create file URI
                        let file_uri_str = format!("file://{}", importing_file.display());
                        if let Ok(file_uri) = file_uri_str.parse::<Uri>() {
                            // Create text edits for each occurrence
                            let edits: Vec<TextEdit> = occurrences
                                .into_iter()
                                .map(|(line, char, len)| TextEdit {
                                    range: lsp_types::Range {
                                        start: lsp_types::Position {
                                            line,
                                            character: char,
                                        },
                                        end: lsp_types::Position {
                                            line,
                                            character: char + len,
                                        },
                                    },
                                    new_text: new_name.to_string(),
                                })
                                .collect();
                            return Some((file_uri, edits));
                        }
                    }
                }
                None
            }
        })
        .buffer_unordered(50)
        .filter_map(|opt| async { opt })
        .collect::<Vec<(Uri, Vec<TextEdit>)>>()
        .await;

    for (file_uri, edits) in results {
        // Skip if we already have edits for this file (from LSP)
        if !changes.contains_key(&file_uri) {
            added_edits += edits.len();
            added_files += 1;
            changes.insert(file_uri, edits);
        }
    }

    debug!(
        added_files = added_files,
        added_edits = added_edits,
        "Cross-file symbol rename enhancement complete"
    );

    Ok(workspace_edit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_symbol_at_position() {
        let content = "const myFunction = () => {}";
        assert_eq!(
            extract_symbol_at_position(content, 0, 6),
            Some("myFunction".to_string())
        );
        assert_eq!(
            extract_symbol_at_position(content, 0, 10),
            Some("myFunction".to_string())
        );
        assert_eq!(
            extract_symbol_at_position(content, 0, 0),
            Some("const".to_string())
        );
    }

    #[test]
    fn test_find_symbol_occurrences() {
        let content = "import { is } from './is'\nconst result = is(value)\nis.type(x)";
        let occurrences = find_symbol_occurrences(content, "is");
        // Only matches outside strings (ignoring './is')
        assert_eq!(occurrences.len(), 3);
        assert_eq!(occurrences[0], (0, 9, 2)); // import { is }
        assert_eq!(occurrences[1], (1, 15, 2)); // = is(
        assert_eq!(occurrences[2], (2, 0, 2)); // is.type
    }

    #[test]
    fn test_is_same_file_only() {
        let source_uri = "file:///test.js";
        let same_file = vec![
            Location {
                uri: source_uri.to_string(),
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 5,
            },
            Location {
                uri: source_uri.to_string(),
                start_line: 10,
                start_character: 0,
                end_line: 10,
                end_character: 5,
            },
        ];
        assert!(is_same_file_only(&same_file, source_uri));

        let cross_file = vec![
            Location {
                uri: source_uri.to_string(),
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 5,
            },
            Location {
                uri: "file:///other.js".to_string(),
                start_line: 5,
                start_character: 0,
                end_line: 5,
                end_character: 5,
            },
        ];
        assert!(!is_same_file_only(&cross_file, source_uri));
    }
}
