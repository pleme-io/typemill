//! AST transformation functionality

use crate::error::AstResult;
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

    // Sort edits by location (ascending / top-down) for O(N) application
    let mut sorted_edits = deduplicated_edits;
    sorted_edits.sort_by(|a, b| {
        a.location
            .start_line
            .cmp(&b.location.start_line)
            .then_with(|| a.location.start_column.cmp(&b.location.start_column))
    });

    // Calculate approximate capacity
    let total_new_text_len: usize = sorted_edits.iter().map(|e| e.new_text.len()).sum();
    let total_old_text_len: usize = sorted_edits.iter().map(|e| e.original_text.len()).sum();
    let estimated_capacity = source
        .len()
        .saturating_add(total_new_text_len)
        .saturating_sub(total_old_text_len);

    let mut result_source = String::with_capacity(estimated_capacity);
    let mut last_byte_idx = 0;
    let mut current_line = 0;
    let mut current_byte_idx = 0; // Tracks start of current_line

    for edit in sorted_edits {
        let start_line = edit.location.start_line as usize;
        let start_col = edit.location.start_column as usize;
        let end_line = edit.location.end_line as usize;
        let end_col = edit.location.end_column as usize;

        // --- Find Start Position ---

        // Advance lines until start_line
        let mut advance_failed = false;
        while current_line < start_line {
            match source[current_byte_idx..].find('\n') {
                Some(idx) => {
                    current_byte_idx += idx + 1; // Skip past \n
                    current_line += 1;
                }
                None => {
                    // Check if we are appending at EOF (allowed if start_line == num_lines)
                    // But we don't know num_lines exactly without full scan?
                    // Actually, if we hit None, current_byte_idx is start of last line (or empty string).
                    // If source ends with newline, we are at EOF? No.
                    // `find` failing means no more newlines.
                    // We might still have text until EOF.
                    // If start_line > current_line, and no more newlines, then start_line is out of bounds
                    // UNLESS we are targeting the line *after* the last line (append at EOF).
                    // But `current_line` counts from 0.
                    // If source has 3 lines (0, 1, 2). `find` will find 2 newlines (if ending with newline).

                    // Let's rely on indices being valid or we error.
                    advance_failed = true;
                    break;
                }
            }
        }

        if advance_failed {
            skipped_edits.push(SkippedEdit {
                edit: edit.clone(),
                reason: format!(
                    "Edit start line {} is out of bounds (reached line {})",
                    start_line, current_line
                ),
                suggestion: None,
            });
            continue;
        }

        // Advance columns to start_col
        let mut start_offset_from_line = 0;
        let mut col_advance_failed = false;

        // We are at start of `current_line`.
        // We need to find `start_col` chars.
        let mut chars = source[current_byte_idx..].chars();
        let mut current_col = 0;

        while current_col < start_col {
            match chars.next() {
                Some(ch) => {
                    if ch == '\n' {
                        col_advance_failed = true;
                        break;
                    }
                    start_offset_from_line += ch.len_utf8();
                    current_col += 1;
                }
                None => {
                    // EOF reached
                    if current_col == start_col {
                        break; // We are exactly at EOF which is allowed for append
                    }
                    col_advance_failed = true;
                    break;
                }
            }
        }

        if col_advance_failed {
            skipped_edits.push(SkippedEdit {
                edit: edit.clone(),
                reason: format!(
                    "Edit start position {}:{} is out of bounds",
                    start_line, start_col
                ),
                suggestion: None,
            });
            continue;
        }

        let start_byte_idx = current_byte_idx + start_offset_from_line;

        // Check overlap with previous edit
        if start_byte_idx < last_byte_idx {
            skipped_edits.push(SkippedEdit {
                edit: edit.clone(),
                reason: format!(
                    "Edit overlaps with previous edit (start byte {} < last byte {})",
                    start_byte_idx, last_byte_idx
                ),
                suggestion: None,
            });
            continue;
        }

        // --- Find End Position ---

        // We continue from start position? No, we scan from current_byte_idx/current_line
        // But `end_line` might be further.

        // We can reuse current_byte_idx and current_line if we update them?
        // But we need to keep them for the *next* edit search?
        // No, next edit is >= this edit.

        // So let's advance our "cursor" (current_line, current_byte_idx) to end position?
        // Wait, if we move cursor to end position, we lose the start of the line for the next edit?
        // No, next edit start >= this edit start.
        // So we can update current_line/byte_idx to at least start_line/start_byte_idx?
        // Actually, better:
        // Use temporary vars for end search.

        let mut end_cursor_byte_idx = current_byte_idx; // Start of start_line
        let mut end_cursor_line = current_line;

        let mut end_advance_failed = false;
        while end_cursor_line < end_line {
            match source[end_cursor_byte_idx..].find('\n') {
                Some(idx) => {
                    end_cursor_byte_idx += idx + 1;
                    end_cursor_line += 1;
                }
                None => {
                    end_advance_failed = true;
                    break;
                }
            }
        }

        if end_advance_failed {
            skipped_edits.push(SkippedEdit {
                edit: edit.clone(),
                reason: format!("Edit end line {} is out of bounds", end_line),
                suggestion: None,
            });
            continue;
        }

        // Advance columns to end_col
        let mut end_offset_from_line = 0;
        let mut end_col_advance_failed = false;
        let mut chars_end = source[end_cursor_byte_idx..].chars();
        let mut current_end_col = 0;

        while current_end_col < end_col {
            match chars_end.next() {
                Some(ch) => {
                    if ch == '\n' {
                        end_col_advance_failed = true;
                        break;
                    }
                    end_offset_from_line += ch.len_utf8();
                    current_end_col += 1;
                }
                None => {
                    if current_end_col == end_col {
                        break;
                    }
                    end_col_advance_failed = true;
                    break;
                }
            }
        }

        if end_col_advance_failed {
            skipped_edits.push(SkippedEdit {
                edit: edit.clone(),
                reason: format!(
                    "Edit end position {}:{} is out of bounds",
                    end_line, end_col
                ),
                suggestion: None,
            });
            continue;
        }

        let end_byte_idx = end_cursor_byte_idx + end_offset_from_line;

        // Validation: Verify original text
        if !edit.original_text.is_empty() {
            let actual_text = &source[start_byte_idx..end_byte_idx];
            if actual_text != edit.original_text {
                skipped_edits.push(SkippedEdit {
                    edit: edit.clone(),
                    reason: format!(
                        "Expected text '{}' but found '{}'",
                        edit.original_text, actual_text
                    ),
                    suggestion: None,
                });
                continue;
            }
        }

        // --- Apply Edit ---

        // Append text before edit
        result_source.push_str(&source[last_byte_idx..start_byte_idx]);

        // Append new text
        result_source.push_str(&edit.new_text);

        // Update stats
        let is_multiline = start_line != end_line;
        let l_removed = if is_multiline {
            (end_line - start_line + 1) as i32
        } else {
            0
        };
        let l_added = if is_multiline {
            edit.new_text.matches('\n').count() as i32 + 1
        } else {
            edit.new_text.matches('\n').count() as i32
        };

        lines_removed += l_removed;
        lines_added += l_added;
        characters_removed += (end_byte_idx - start_byte_idx) as i32;
        characters_added += edit.new_text.len() as i32;

        applied_edits.push(edit.clone());
        last_byte_idx = end_byte_idx;

        // Update our main cursor to the line/byte of the END of this edit,
        // because next edit must be after this one.
        // Actually, we can update `current_line` and `current_byte_idx` to `end_cursor_line` and `end_cursor_byte_idx`.
        current_line = end_cursor_line;
        current_byte_idx = end_cursor_byte_idx;
    }

    // Append remaining text
    if last_byte_idx < source.len() {
        result_source.push_str(&source[last_byte_idx..]);
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

#[cfg(test)]
mod tests {
    use super::*;
    use mill_foundation::protocol::{EditLocation, EditType};

    #[test]
    fn test_apply_single_line_edit() {
        let source = "let oldName = 42;".to_string();
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

        let plan = EditPlan {
            source_file: "test.rs".to_string(),
            edits: vec![edit],
            dependency_updates: vec![],
            validations: vec![],
            metadata: mill_foundation::protocol::EditPlanMetadata {
                intent_name: "test".to_string(),
                intent_arguments: serde_json::Value::Null,
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec![],
                consolidation: None,
            },
        };

        let result = apply_edit_plan(&source, &plan).unwrap();
        assert_eq!(result.transformed_source, "let newName = 42;");
        assert_eq!(result.statistics.characters_added, 7);
        assert_eq!(result.statistics.characters_removed, 7);
    }

    #[test]
    fn test_apply_insert_edit() {
        let source = "console.log('hello');".to_string();
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

        let plan = EditPlan {
            source_file: "test.rs".to_string(),
            edits: vec![edit],
            dependency_updates: vec![],
            validations: vec![],
            metadata: mill_foundation::protocol::EditPlanMetadata {
                intent_name: "test".to_string(),
                intent_arguments: serde_json::Value::Null,
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec![],
                consolidation: None,
            },
        };

        let result = apply_edit_plan(&source, &plan).unwrap();
        assert_eq!(
            result.transformed_source,
            "// Added comment\nconsole.log('hello');"
        );
        assert_eq!(result.statistics.lines_added, 1);
        assert_eq!(result.statistics.characters_added, 17);
    }
}

#[cfg(test)]
mod benchmarks {
    use super::*;
    use mill_foundation::protocol::{EditLocation, EditType};
    use std::time::Instant;

    #[test]
    fn benchmark_apply_edit_plan_performance() {
        // Create a large source file (10,000 lines)
        let mut source = String::new();
        for i in 0..10000 {
            source.push_str(&format!(
                "let variable_{} = {}; // Some content to make the line longer\n",
                i, i
            ));
        }

        // Create 1000 edits (modifying every 10th line)
        let mut edits = Vec::new();
        for i in (0..10000).step_by(10) {
            let line_idx = i;
            let original_line = format!(
                "let variable_{} = {}; // Some content to make the line longer\n",
                i, i
            );

            edits.push(TextEdit {
                file_path: None,
                edit_type: EditType::Replace,
                location: EditLocation {
                    start_line: line_idx,
                    start_column: 0,
                    end_line: line_idx,
                    end_column: original_line.len() as u32 - 1, // Keep the newline
                },
                original_text: original_line.trim_end().to_string(),
                new_text: format!("let optimized_{} = {};", i, i),
                priority: 0,
                description: "Benchmark edit".to_string(),
            });
        }

        let plan = EditPlan {
            source_file: "benchmark.rs".to_string(),
            edits,
            dependency_updates: vec![],
            validations: vec![],
            metadata: mill_foundation::protocol::EditPlanMetadata {
                intent_name: "benchmark".to_string(),
                intent_arguments: serde_json::Value::Null,
                created_at: chrono::Utc::now(),
                complexity: 1,
                impact_areas: vec![],
                consolidation: None,
            },
        };

        println!(
            "Benchmarking apply_edit_plan with {} edits on {} lines...",
            plan.edits.len(),
            10000
        );

        let start = Instant::now();
        let result = apply_edit_plan(&source, &plan).unwrap();
        let duration = start.elapsed();

        println!("Execution time: {:?}", duration);

        // Verify result validity briefly
        assert_eq!(result.applied_edits.len(), 1000);

        // With optimization, this should be very fast (< 50ms)
        assert!(
            duration.as_millis() < 50,
            "Optimization should be faster than 50ms"
        );
    }
}
