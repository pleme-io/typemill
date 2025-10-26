//! AST transformation functionality

use crate::error::{AstError, AstResult};
use mill_foundation::protocol::{EditPlan, TextEdit};
use serde::{Deserialize, Serialize};

/// Transformation result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransformResult {
    /// Transformed source code
    pub transformed_source: String,
    /// Applied edits
    pub applied_edits: Vec<TextEdit>,
    /// Skipped edits (due to conflicts or errors)
    pub skipped_edits: Vec<SkippedEdit>,
    /// Transformation statistics
    pub statistics: TransformStatistics,
}

/// Information about a skipped edit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkippedEdit {
    /// The edit that was skipped
    pub edit: TextEdit,
    /// Reason it was skipped
    pub reason: String,
    /// Suggestion for manual resolution
    pub suggestion: Option<String>,
}

/// Transformation statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransformStatistics {
    /// Total number of edits in the plan
    pub total_edits: usize,
    /// Number of successfully applied edits
    pub applied_count: usize,
    /// Number of skipped edits
    pub skipped_count: usize,
    /// Lines added
    pub lines_added: i32,
    /// Lines removed
    pub lines_removed: i32,
    /// Characters added
    pub characters_added: i32,
    /// Characters removed
    pub characters_removed: i32,
}

/// Deduplicate overlapping edits by removing redundant edits
///
/// When multiple edits target overlapping text regions, keep only the most specific/largest edit
/// to prevent conflicts where one edit modifies text that another edit expects to be unchanged.
fn deduplicate_overlapping_edits(edits: &[TextEdit]) -> Vec<TextEdit> {
    if edits.is_empty() {
        return Vec::new();
    }

    tracing::debug!(total_edits = edits.len(), "Starting edit deduplication");

    let mut unique_edits = Vec::new();

    for (idx, new_edit) in edits.iter().enumerate() {
        tracing::debug!(
            edit_index = idx,
            edit_description = %new_edit.description,
            start_line = new_edit.location.start_line,
            start_col = new_edit.location.start_column,
            end_line = new_edit.location.end_line,
            end_col = new_edit.location.end_column,
            original_text = %new_edit.original_text,
            new_text = %new_edit.new_text,
            "Processing edit for deduplication"
        );

        let mut is_redundant = false;

        // Check if this edit overlaps with any existing edit
        #[allow(clippy::needless_range_loop)]
        for i in 0..unique_edits.len() {
            let existing_edit = &unique_edits[i];

            if edits_overlap(new_edit, existing_edit) {
                // Determine which edit to keep (the larger/more specific one)
                let new_size = edit_text_length(new_edit);
                let existing_size = edit_text_length(existing_edit);

                tracing::warn!(
                    new_edit_desc = %new_edit.description,
                    existing_edit_desc = %existing_edit.description,
                    new_size = new_size,
                    existing_size = existing_size,
                    "Found overlapping edits"
                );

                if new_size > existing_size {
                    // New edit is larger - replace the existing one
                    tracing::warn!(
                        keeping = "new",
                        new_desc = %new_edit.description,
                        replacing_desc = %existing_edit.description,
                        "Keeping larger edit"
                    );
                    unique_edits[i] = new_edit.clone();
                    is_redundant = false;
                } else {
                    // Existing edit is larger - skip the new one
                    tracing::warn!(
                        keeping = "existing",
                        existing_desc = %existing_edit.description,
                        skipping_desc = %new_edit.description,
                        "Skipping smaller/equal edit"
                    );
                    is_redundant = true;
                }
                break;
            }
        }

        if !is_redundant {
            // Check if we already added this edit (exact duplicate)
            if !unique_edits
                .iter()
                .any(|e| edits_are_identical(e, new_edit))
            {
                tracing::debug!(
                    edit_desc = %new_edit.description,
                    "Adding unique edit to list"
                );
                unique_edits.push(new_edit.clone());
            } else {
                tracing::debug!(
                    edit_desc = %new_edit.description,
                    "Skipping exact duplicate edit"
                );
            }
        }
    }

    tracing::debug!(
        original_count = edits.len(),
        unique_count = unique_edits.len(),
        removed_count = edits.len() - unique_edits.len(),
        "Deduplication complete"
    );

    unique_edits
}

/// Check if two edits overlap in their text regions
fn edits_overlap(edit1: &TextEdit, edit2: &TextEdit) -> bool {
    // Two edits overlap if their line/column ranges intersect

    // Check if they're on completely different lines
    if edit1.location.end_line < edit2.location.start_line
        || edit2.location.end_line < edit1.location.start_line
    {
        return false; // No overlap
    }

    // If they share any lines, check column ranges
    // Simplified: if their original text contains each other, they overlap
    if !edit1.original_text.is_empty() && !edit2.original_text.is_empty() {
        // Check if one is a substring of the other
        if edit1.original_text.contains(&edit2.original_text)
            || edit2.original_text.contains(&edit1.original_text)
        {
            return true;
        }
    }

    // Check precise position overlap for same line
    if edit1.location.start_line == edit2.location.start_line
        && edit1.location.end_line == edit2.location.end_line
    {
        // Same line - check column overlap
        let e1_start = edit1.location.start_column;
        let e1_end = edit1.location.end_column;
        let e2_start = edit2.location.start_column;
        let e2_end = edit2.location.end_column;

        // Ranges overlap if: start1 <= end2 AND start2 <= end1
        return e1_start <= e2_end && e2_start <= e1_end;
    }

    // Multi-line overlap: they share at least one line
    true
}

/// Check if two edits are identical
fn edits_are_identical(edit1: &TextEdit, edit2: &TextEdit) -> bool {
    edit1.location.start_line == edit2.location.start_line
        && edit1.location.start_column == edit2.location.start_column
        && edit1.location.end_line == edit2.location.end_line
        && edit1.location.end_column == edit2.location.end_column
        && edit1.original_text == edit2.original_text
        && edit1.new_text == edit2.new_text
}

/// Calculate the text length of an edit (for determining which is more specific)
fn edit_text_length(edit: &TextEdit) -> usize {
    if !edit.original_text.is_empty() {
        edit.original_text.len()
    } else {
        // For inserts, use new_text length
        edit.new_text.len()
    }
}

/// Apply an edit plan to source code
pub fn apply_edit_plan(source: &str, plan: &EditPlan) -> AstResult<TransformResult> {
    let mut result_source = source.to_string();
    let mut applied_edits = Vec::new();
    let mut skipped_edits = Vec::new();
    let mut lines_added = 0;
    let mut lines_removed = 0;
    let mut characters_added = 0;
    let mut characters_removed = 0;

    // Deduplicate overlapping edits before applying (prevents conflicts)
    let deduplicated_edits = deduplicate_overlapping_edits(&plan.edits);
    let removed_count = plan.edits.len() - deduplicated_edits.len();
    if removed_count > 0 {
        tracing::warn!(
            removed_count = removed_count,
            original_count = plan.edits.len(),
            "Removed redundant overlapping edits to prevent conflicts"
        );
    }

    // Sort edits by priority (highest first) and then by location (reverse order to avoid offset issues)
    let mut sorted_edits = deduplicated_edits;
    sorted_edits.sort_by(|a, b| {
        match b.priority.cmp(&a.priority) {
            std::cmp::Ordering::Equal => {
                // If same priority, sort by location (reverse order)
                match b.location.start_line.cmp(&a.location.start_line) {
                    std::cmp::Ordering::Equal => {
                        b.location.start_column.cmp(&a.location.start_column)
                    }
                    other => other,
                }
            }
            other => other,
        }
    });

    for edit in sorted_edits {
        match apply_single_edit(&mut result_source, &edit) {
            Ok(edit_stats) => {
                applied_edits.push(edit);
                lines_added += edit_stats.lines_added;
                lines_removed += edit_stats.lines_removed;
                characters_added += edit_stats.characters_added;
                characters_removed += edit_stats.characters_removed;
            }
            Err(err) => {
                skipped_edits.push(SkippedEdit {
                    edit,
                    reason: err.to_string(),
                    suggestion: None, // Could provide helpful suggestions based on error type
                });
            }
        }
    }

    let applied_count = applied_edits.len();
    let skipped_count = skipped_edits.len();

    Ok(TransformResult {
        transformed_source: result_source,
        applied_edits,
        skipped_edits,
        statistics: TransformStatistics {
            total_edits: plan.edits.len(),
            applied_count,
            skipped_count,
            lines_added,
            lines_removed,
            characters_added,
            characters_removed,
        },
    })
}

/// Edit statistics for a single edit
#[derive(Debug, Default)]
struct EditStats {
    lines_added: i32,
    lines_removed: i32,
    characters_added: i32,
    characters_removed: i32,
}

/// Apply a single text edit to source code
fn apply_single_edit(source: &mut String, edit: &TextEdit) -> AstResult<EditStats> {
    let lines: Vec<&str> = source.lines().collect();

    // Validate edit location
    if edit.location.start_line as usize >= lines.len() {
        return Err(AstError::transformation(format!(
            "Edit start line {} is beyond source length {}",
            edit.location.start_line,
            lines.len()
        )));
    }

    if edit.location.end_line as usize >= lines.len() {
        return Err(AstError::transformation(format!(
            "Edit end line {} is beyond source length {}",
            edit.location.end_line,
            lines.len()
        )));
    }

    // Handle single-line edits
    if edit.location.start_line == edit.location.end_line {
        return apply_single_line_edit(source, edit);
    }

    // Handle multi-line edits
    apply_multi_line_edit(source, edit)
}

/// Apply edit within a single line
fn apply_single_line_edit(source: &mut String, edit: &TextEdit) -> AstResult<EditStats> {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = edit.location.start_line as usize;
    let line = lines[line_idx];

    // Validate column positions
    if edit.location.start_column as usize > line.len() {
        return Err(AstError::transformation(format!(
            "Edit start column {} is beyond line length {}",
            edit.location.start_column,
            line.len()
        )));
    }

    if edit.location.end_column as usize > line.len() {
        return Err(AstError::transformation(format!(
            "Edit end column {} is beyond line length {}",
            edit.location.end_column,
            line.len()
        )));
    }

    let start_col = edit.location.start_column as usize;
    let end_col = edit.location.end_column as usize;

    // Extract the text being replaced for validation
    let actual_text = &line[start_col..end_col];
    if !edit.original_text.is_empty() && actual_text != edit.original_text {
        return Err(AstError::transformation(format!(
            "Expected text '{}' but found '{}'",
            edit.original_text, actual_text
        )));
    }

    // Build the new line
    let new_line = format!(
        "{}{}{}",
        &line[..start_col],
        edit.new_text,
        &line[end_col..]
    );

    // Rebuild the source
    let mut new_lines = lines.clone();
    new_lines[line_idx] = &new_line;

    // We need to handle this more carefully to avoid lifetime issues
    *source = new_lines.join("\n");

    // Calculate statistics
    let mut stats = EditStats {
        characters_added: edit.new_text.len() as i32,
        characters_removed: (end_col - start_col) as i32,
        ..Default::default()
    };

    // Count newlines in the new text
    let new_newlines = edit.new_text.matches('\n').count() as i32;
    stats.lines_added = new_newlines;

    Ok(stats)
}

/// Apply edit across multiple lines
fn apply_multi_line_edit(source: &mut String, edit: &TextEdit) -> AstResult<EditStats> {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = edit.location.start_line as usize;
    let end_line = edit.location.end_line as usize;
    let start_col = edit.location.start_column as usize;
    let end_col = edit.location.end_column as usize;

    // Validate positions
    if start_col > lines[start_line].len() {
        return Err(AstError::transformation(format!(
            "Start column {} beyond line {} length",
            start_col, start_line
        )));
    }

    if end_col > lines[end_line].len() {
        return Err(AstError::transformation(format!(
            "End column {} beyond line {} length",
            end_col, end_line
        )));
    }

    // Extract the original text for validation (if specified)
    if !edit.original_text.is_empty() {
        let mut original_text = String::new();

        // First line
        original_text.push_str(&lines[start_line][start_col..]);
        if start_line < end_line {
            original_text.push('\n');
        }

        // Middle lines
        for line in lines.iter().take(end_line).skip(start_line + 1) {
            original_text.push_str(line);
            original_text.push('\n');
        }

        // Last line (if different from first)
        if start_line < end_line {
            original_text.push_str(&lines[end_line][..end_col]);
        } else {
            // Same line - already handled above, just need to adjust
            original_text = lines[start_line][start_col..end_col].to_string();
        }

        if original_text != edit.original_text {
            return Err(AstError::transformation(
                "Expected original text doesn't match actual text".to_string(),
            ));
        }
    }

    // Build new content
    let mut new_lines: Vec<String> = Vec::new();

    // Lines before the edit
    for line in &lines[..start_line] {
        new_lines.push(line.to_string());
    }

    // The edited content
    let prefix = &lines[start_line][..start_col];
    let suffix = &lines[end_line][end_col..];
    let combined = format!("{}{}{}", prefix, edit.new_text, suffix);

    // Split the combined text by newlines and add to new_lines
    for line in combined.split('\n') {
        new_lines.push(line.to_string());
    }

    // Lines after the edit
    if end_line + 1 < lines.len() {
        for line in &lines[end_line + 1..] {
            new_lines.push(line.to_string());
        }
    }

    // Rebuild source
    *source = new_lines.join("\n");

    // Calculate statistics
    let mut stats = EditStats::default();
    let lines_removed = (end_line - start_line + 1) as i32;
    let lines_added = edit.new_text.matches('\n').count() as i32 + 1; // +1 for the line itself

    stats.lines_added = lines_added;
    stats.lines_removed = lines_removed;
    stats.characters_added = edit.new_text.len() as i32;
    stats.characters_removed = edit.original_text.len() as i32;

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_foundation::protocol::{EditLocation, EditType};

    #[test]
    fn test_apply_single_line_edit() {
        let mut source = "let oldName = 42;".to_string();
        let edit = TextEdit {
            file_path: None,
            edit_type: EditType::Rename,
            location: EditLocation {
                start_line: 0,
                start_column: 4,
                end_line: 0,
                end_column: 11,
            },
            original_text: "oldName".to_string(),
            new_text: "newName".to_string(),
            priority: 100,
            description: "Rename variable".to_string(),
        };

        let result = apply_single_edit(&mut source, &edit).unwrap();
        assert_eq!(source, "let newName = 42;");
        assert_eq!(result.characters_added, 7);
        assert_eq!(result.characters_removed, 7);
    }

    #[test]
    fn test_apply_insert_edit() {
        let mut source = "console.log('hello');".to_string();
        let edit = TextEdit {
            file_path: None,
            edit_type: EditType::Insert,
            location: EditLocation {
                start_line: 0,
                start_column: 0,
                end_line: 0,
                end_column: 0,
            },
            original_text: "".to_string(),
            new_text: "// Added comment\n".to_string(),
            priority: 100,
            description: "Add comment".to_string(),
        };

        let result = apply_single_edit(&mut source, &edit).unwrap();
        assert_eq!(source, "// Added comment\nconsole.log('hello');");
        assert_eq!(result.lines_added, 1);
        assert_eq!(result.characters_added, 17);
    }
}
