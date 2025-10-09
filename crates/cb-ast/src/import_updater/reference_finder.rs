use cb_plugin_api::ModuleReference;
use std::path::Path;
use tracing::debug;

/// Helper function to create TextEdits from ModuleReferences for import path updates
pub(crate) fn create_text_edits_from_references(
    references: &[cb_plugin_api::ModuleReference],
    file_path: &Path,
    old_module_name: &str,
    new_module_name: &str,
) -> Vec<cb_protocol::TextEdit> {
    use cb_protocol::{EditLocation, EditType, TextEdit};

    references
        .iter()
        .map(|refer| TextEdit {
            file_path: Some(file_path.to_string_lossy().to_string()),
            edit_type: EditType::UpdateImport,
            location: EditLocation {
                start_line: (refer.line.saturating_sub(1)) as u32, // Convert to 0-based
                start_column: refer.column as u32,
                end_line: (refer.line.saturating_sub(1)) as u32,
                end_column: (refer.column + refer.length) as u32,
            },
            original_text: refer.text.clone(),
            new_text: refer.text.replace(old_module_name, new_module_name),
            priority: 1,
            description: format!(
                "Update {} reference from '{}' to '{}'",
                match refer.kind {
                    cb_plugin_api::ReferenceKind::Declaration => "import",
                    cb_plugin_api::ReferenceKind::QualifiedPath => "qualified path",
                    cb_plugin_api::ReferenceKind::StringLiteral => "string literal",
                },
                old_module_name,
                new_module_name
            ),
        })
        .collect()
}

/// Find inline fully-qualified crate references in code
///
/// This finds patterns like `old_crate::module::function()` that appear
/// outside of `use` import statements.
///
/// # Arguments
///
/// * `content` - The file content to scan
/// * `file_path` - Path to the file being scanned
/// * `crate_name` - Name of the crate to search for (e.g., "cb_ast")
///
/// # Returns
///
/// Vec of ModuleReference for each inline occurrence found
pub(crate) fn find_inline_crate_references(
    content: &str,
    file_path: &Path,
    crate_name: &str,
) -> Vec<cb_plugin_api::ModuleReference> {
    use cb_plugin_api::ReferenceKind;

    let mut references = Vec::new();

    // Pattern to match: `crate_name::` followed by identifiers
    // Regex: \bcrate_name::[\w:]+
    // But we'll use simple string matching for robustness

    for (line_num, line) in content.lines().enumerate() {
        // Skip lines that are import statements (already handled)
        if line.trim_start().starts_with("use ") || line.trim_start().starts_with("pub use ") {
            continue;
        }

        // Skip comment lines
        if line.trim_start().starts_with("//") || line.trim_start().starts_with("/*") {
            continue;
        }

        // Find all occurrences of `crate_name::` in this line
        let search_pattern = format!("{}::", crate_name);
        let mut search_start = 0;

        while let Some(pos) = line[search_start..].find(&search_pattern) {
            let absolute_pos = search_start + pos;

            // Ensure it's a word boundary (not part of a larger identifier)
            let is_word_boundary = if absolute_pos == 0 {
                true
            } else {
                let prev_char = line.chars().nth(absolute_pos - 1).unwrap_or(' ');
                !prev_char.is_alphanumeric() && prev_char != '_'
            };

            if is_word_boundary {
                // Extract the full qualified path (including trailing :: and identifiers)
                let remaining = &line[absolute_pos..];
                let mut path_end = search_pattern.len();

                // Continue while we see `identifier::` or `identifier`
                for ch in remaining[search_pattern.len()..].chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == ':' {
                        path_end += ch.len_utf8();
                    } else {
                        break;
                    }
                }

                // Trim trailing `::`
                while line[absolute_pos..absolute_pos + path_end].ends_with("::") {
                    path_end -= 2;
                }

                let reference_text = &line[absolute_pos..absolute_pos + path_end];

                debug!(
                    file = ?file_path,
                    line = line_num + 1,
                    column = absolute_pos,
                    text = %reference_text,
                    "Found inline fully-qualified path reference"
                );

                references.push(ModuleReference {
                    line: line_num + 1, // 1-based line numbers
                    column: absolute_pos,
                    length: reference_text.len(),
                    text: reference_text.to_string(),
                    kind: ReferenceKind::QualifiedPath,
                });
            }

            search_start = absolute_pos + search_pattern.len();
        }
    }

    debug!(
        file = ?file_path,
        crate_name = %crate_name,
        references_found = references.len(),
        "Inline crate reference scan complete"
    );

    references
}