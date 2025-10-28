use mill_plugin_api::import_support::ImportParser;
use tree_sitter::{Parser, Query, QueryCursor};

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
                    text.trim_matches(|c| c == '"' || c == '<' || c == '>').to_string()
                })
            })
            .collect()
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        self.parse_imports(content).contains(&module.to_string())
    }
}