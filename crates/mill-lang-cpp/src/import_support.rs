use mill_plugin_api::import_support::{
    ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
    ImportRenameSupport,
};
use tree_sitter::{Parser, Query, QueryCursor};
use std::path::Path;

fn get_cpp_imports_query() -> &'static str {
    r#"
    (preproc_include
        path: (string_literal) @path)
    (preproc_include
        path: (system_lib_string) @path)
    "#
}

pub struct CppImportSupport;

impl ImportParser for CppImportSupport {
    fn parse_imports(&self, source: &str) -> Vec<String> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_cpp::language())
            .expect("Error loading C++ grammar");

        let tree = parser.parse(source, None).unwrap();
        let query = Query::new(tree_sitter_cpp::language(), get_cpp_imports_query()).unwrap();

        let mut query_cursor = QueryCursor::new();
        query_cursor
            .matches(&query, tree.root_node(), source.as_bytes())
            .flat_map(|m| {
                m.captures.iter().map(|c| {
                    let range = c.node.range();
                    let text = source[range.start_byte..range.end_byte].to_string();
                    // Trim quotes and angle brackets
                    text.trim_matches(|c| c == '"' || c == '<' || c == '>')
                        .to_string()
                })
            })
            .collect()
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        self.parse_imports(content).contains(&module.to_string())
    }
}

impl ImportRenameSupport for CppImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        source: &str,
        old_path: &str,
        new_path: &str,
    ) -> (String, usize) {
        // Local struct to hold edit information.
        struct Edit {
            range_to_replace: tree_sitter::Range,
            replacement: String,
        }

        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_cpp::language())
            .expect("Error loading C++ grammar");

        let tree = parser.parse(source, None).unwrap();
        let query_text = get_cpp_imports_query();
        let query = Query::new(tree_sitter_cpp::language(), query_text).unwrap();
        let path_capture_index = query.capture_index_for_name("path").unwrap();

        let mut query_cursor = QueryCursor::new();
        let mut edits = vec![];

        for match_ in query_cursor.matches(&query, tree.root_node(), source.as_bytes()) {
            let path_node = match_
                .nodes_for_capture_index(path_capture_index)
                .next()
                .unwrap();
            let import_path_text = path_node
                .utf8_text(source.as_bytes())
                .unwrap()
                .trim_matches(|c| c == '"' || c == '<' || c == '>');

            if import_path_text == old_path {
                let replacement = format!("\"{}\"", new_path);
                edits.push(Edit {
                    range_to_replace: path_node.range(),
                    replacement,
                });
            }
        }

        if !edits.is_empty() {
            let changes = edits.len();
            let mut new_source = source.to_string();
            // Sort edits by start byte in reverse order to apply them without invalidating ranges.
            edits.sort_by(|a, b| b.range_to_replace.start_byte.cmp(&a.range_to_replace.start_byte));

            for edit in edits {
                new_source.replace_range(
                    edit.range_to_replace.start_byte..edit.range_to_replace.end_byte,
                    &edit.replacement,
                );
            }
            (new_source, changes)
        } else {
            (source.to_string(), 0)
        }
    }
}

use path_clean::PathClean;

impl ImportMoveSupport for CppImportSupport {
    fn rewrite_imports_for_move(
        &self,
        source: &str,
        moved_from_path: &Path,
        moved_to_path: &Path,
    ) -> (String, usize) {
        struct Edit {
            range_to_replace: tree_sitter::Range,
            replacement: String,
        }

        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_cpp::language())
            .expect("Error loading C++ grammar");

        let tree = parser.parse(source, None).unwrap();
        let query_text = get_cpp_imports_query();
        let query = Query::new(tree_sitter_cpp::language(), query_text).unwrap();
        let path_capture_index = query.capture_index_for_name("path").unwrap();

        let mut query_cursor = QueryCursor::new();
        let mut edits = vec![];

        let moved_from_dir = match moved_from_path.parent() {
            Some(p) => p,
            None => return (source.to_string(), 0), // No parent, can't resolve relative paths.
        };

        let moved_to_dir = match moved_to_path.parent() {
            Some(p) => p,
            None => return (source.to_string(), 0), // Should not happen if from_dir exists.
        };

        for match_ in query_cursor.matches(&query, tree.root_node(), source.as_bytes()) {
            let path_node = match_
                .nodes_for_capture_index(path_capture_index)
                .next()
                .unwrap();
            let import_path_text = path_node
                .utf8_text(source.as_bytes())
                .unwrap()
                .trim_matches(|c| c == '"' || c == '<' || c == '>');

            let import_path = Path::new(import_path_text);

            if import_path.is_relative() {
                let absolute_import_path = moved_from_dir.join(import_path).clean();
                if let Some(new_relative_path) =
                    pathdiff::diff_paths(&absolute_import_path, moved_to_dir)
                {
                    let replacement = format!("\"{}\"", new_relative_path.to_string_lossy());
                    edits.push(Edit {
                        range_to_replace: path_node.range(),
                        replacement,
                    });
                }
            }
        }

        if !edits.is_empty() {
            let changes = edits.len();
            let mut new_source = source.to_string();
            edits.sort_by(|a, b| b.range_to_replace.start_byte.cmp(&a.range_to_replace.start_byte));

            for edit in edits {
                new_source.replace_range(
                    edit.range_to_replace.start_byte..edit.range_to_replace.end_byte,
                    &edit.replacement,
                );
            }
            (new_source, changes)
        } else {
            (source.to_string(), 0)
        }
    }
}

impl ImportMutationSupport for CppImportSupport {
    fn add_import(&self, source: &str, module_to_add: &str) -> String {
        if self.contains_import(source, module_to_add) {
            return source.to_string();
        }

        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_cpp::language())
            .expect("Error loading C++ grammar");
        let tree = parser.parse(source, None).unwrap();
        let root_node = tree.root_node();

        let new_import_statement = format!("#include \"{}\"", module_to_add);
        let mut lines: Vec<String> = source.lines().map(String::from).collect();

        let last_include_node = root_node
            .children(&mut root_node.walk())
            .filter(|n| n.kind() == "preproc_include")
            .last();

        if let Some(node) = last_include_node {
            let insertion_line = node.end_position().row;
            lines.insert(insertion_line + 1, new_import_statement);
        } else {
            lines.insert(0, new_import_statement);
        }

        lines.join("\n")
    }

    fn remove_import(&self, source: &str, module_to_remove: &str) -> String {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_cpp::language())
            .expect("Error loading C++ grammar");

        let tree = parser.parse(source, None).unwrap();
        let query_text = get_cpp_imports_query();
        let query = Query::new(tree_sitter_cpp::language(), query_text).unwrap();
        let path_capture_index = query.capture_index_for_name("path").unwrap();

        let mut query_cursor = QueryCursor::new();

        let node_to_remove_range = query_cursor
            .matches(&query, tree.root_node(), source.as_bytes())
            .find_map(|match_| {
                let path_node = match_
                    .nodes_for_capture_index(path_capture_index)
                    .next()?;

                let import_path = path_node
                    .utf8_text(source.as_bytes())
                    .ok()?
                    .trim_matches(|c| c == '"' || c == '<' || c == '>');

                if import_path == module_to_remove {
                    // The parent of the path node is the preproc_include node
                    let include_node = path_node.parent()?;
                    if include_node.kind() == "preproc_include" {
                        return Some(include_node.range());
                    }
                }
                None
            });

        if let Some(range) = node_to_remove_range {
            let mut lines: Vec<&str> = source.lines().collect();
            let start_line = range.start_point.row;
            if start_line < lines.len() {
                lines.remove(start_line);
            }
            return lines.join("\n");
        }

        source.to_string()
    }
}

use mill_foundation::protocol::{DependencyUpdate, DependencyUpdateType};
use mill_plugin_api::{PluginError, PluginResult};

impl ImportAdvancedSupport for CppImportSupport {
    fn update_import_reference(
        &self,
        _file_path: &Path,
        content: &str,
        update: &DependencyUpdate,
    ) -> PluginResult<String> {
        match update.update_type {
            DependencyUpdateType::ImportPath => {
                let (new_content, changes) =
                    self.rewrite_imports_for_rename(content, &update.old_reference, &update.new_reference);
                if changes > 0 {
                    Ok(new_content)
                } else {
                    Ok(content.to_string())
                }
            }
            _ => Err(PluginError::not_supported(
                "Only ImportPath updates are supported for C++ includes.",
            )),
        }
    }
}
