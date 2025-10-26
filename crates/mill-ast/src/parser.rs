//! AST parsing functionality
use crate::error::AstResult;
use mill_foundation::protocol::{
    ImportGraph, ImportGraphMetadata, ImportInfo, ImportType, NamedImport, SourceLocation,
};
use petgraph::graph::NodeIndex;
use petgraph::{Direction, Graph};
use std::collections::{HashMap, HashSet};
use std::path::Path;
/// Build import graph for a source file
pub fn build_import_graph(source: &str, path: &Path) -> AstResult<ImportGraph> {
    // Note: Only Rust and TypeScript supported after language reduction
    let language = match path.extension().and_then(|ext| ext.to_str()) {
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("rs") => "rust",
        _ => "unknown",
    };
    let imports = match language {
        "typescript" | "javascript" => {
            // TypeScript/JavaScript parsing is handled by mill-lang-typescript plugin
            // Cannot be called here due to circular dependency (mill-lang-typescript depends on mill-ast)
            tracing::debug!(
                file_path = % path.display(),
                "TypeScript/JavaScript import parsing should use mill-lang-typescript plugin directly"
            );
            Vec::new()
        }
        "rust" => {
            // Rust import parsing is handled by mill-lang-rust plugin
            // Cannot be called here due to circular dependency (mill-lang-rust depends on mill-ast)
            // Use mill_lang_rust::parse_imports() directly when needed
            tracing::debug!("Rust import parsing should use mill-lang-rust plugin directly");
            Vec::new()
        }
        _ => parse_imports_basic(source)?,
    };
    let external_dependencies = imports
        .iter()
        .filter_map(|imp| {
            if is_external_dependency(&imp.module_path) {
                Some(imp.module_path.clone())
            } else {
                None
            }
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    Ok(ImportGraph {
        source_file: path.to_string_lossy().to_string(),
        imports,
        importers: Vec::new(),
        metadata: ImportGraphMetadata {
            language: language.to_string(),
            parsed_at: chrono::Utc::now(),
            parser_version: "0.3.0-swc".to_string(),
            circular_dependencies: Vec::new(),
            external_dependencies,
        },
    })
}
// TypeScript/JavaScript import parsing has been moved to mill-lang-typescript plugin
// The functions parse_js_ts_imports_swc() and parse_js_ts_imports_enhanced() have been removed
// Use mill_lang_typescript::parser::analyze_imports() instead
/// Basic import parsing (simplified for foundation)
fn parse_imports_basic(source: &str) -> AstResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.starts_with("import ") && line.contains(" from ") {
            if let Some(import_info) = parse_es_import(line, line_num as u32)? {
                imports.push(import_info);
            }
        } else if line.contains("require(") {
            if let Some(import_info) = parse_commonjs_require(line, line_num as u32)? {
                imports.push(import_info);
            }
        } else if line.contains("import(") {
            if let Some(import_info) = parse_dynamic_import(line, line_num as u32)? {
                imports.push(import_info);
            }
        }
    }
    Ok(imports)
}
/// Parse ES module import statement (simplified)
fn parse_es_import(line: &str, line_num: u32) -> AstResult<Option<ImportInfo>> {
    if let Some(from_pos) = line.find(" from ") {
        let import_part = &line[6..from_pos].trim();
        let module_part = &line[from_pos + 6..].trim();
        let module_path = module_part
            .trim_matches('"')
            .trim_matches('\'')
            .trim_end_matches(';');
        let type_only = line.contains("import type");
        let (default_import, named_imports, namespace_import) =
            parse_import_specifiers(import_part)?;
        return Ok(Some(ImportInfo {
            module_path: module_path.to_string(),
            import_type: if type_only {
                ImportType::TypeOnly
            } else {
                ImportType::EsModule
            },
            named_imports,
            default_import,
            namespace_import,
            type_only,
            location: SourceLocation {
                start_line: line_num,
                start_column: 0,
                end_line: line_num,
                end_column: line.len() as u32,
            },
        }));
    }
    Ok(None)
}
/// Parse CommonJS require (simplified)
fn parse_commonjs_require(line: &str, line_num: u32) -> AstResult<Option<ImportInfo>> {
    if let Some(require_start) = line.find("require(") {
        let require_part = &line[require_start + 8..];
        if let Some(end_paren) = require_part.find(')') {
            let module_path = &require_part[..end_paren]
                .trim_matches('"')
                .trim_matches('\'');
            return Ok(Some(ImportInfo {
                module_path: module_path.to_string(),
                import_type: ImportType::CommonJs,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: require_start as u32,
                    end_line: line_num,
                    end_column: (require_start + 8 + end_paren + 1) as u32,
                },
            }));
        }
    }
    Ok(None)
}
/// Parse dynamic import (simplified)
fn parse_dynamic_import(line: &str, line_num: u32) -> AstResult<Option<ImportInfo>> {
    if let Some(import_start) = line.find("import(") {
        let import_part = &line[import_start + 7..];
        if let Some(end_paren) = import_part.find(')') {
            let module_path = &import_part[..end_paren]
                .trim_matches('"')
                .trim_matches('\'');
            return Ok(Some(ImportInfo {
                module_path: module_path.to_string(),
                import_type: ImportType::Dynamic,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: import_start as u32,
                    end_line: line_num,
                    end_column: (import_start + 7 + end_paren + 1) as u32,
                },
            }));
        }
    }
    Ok(None)
}
/// Parse import specifiers (simplified)
fn parse_import_specifiers(
    import_part: &str,
) -> AstResult<(Option<String>, Vec<NamedImport>, Option<String>)> {
    let import_part = import_part.trim();
    if let Some(stripped) = import_part.strip_prefix("* as ") {
        let namespace = stripped.trim().to_string();
        return Ok((None, Vec::new(), Some(namespace)));
    }
    if import_part.starts_with('{') && import_part.ends_with('}') {
        let inner = &import_part[1..import_part.len() - 1];
        let named_imports = parse_named_imports(inner)?;
        return Ok((None, named_imports, None));
    }
    if let Some(comma_pos) = import_part.find(',') {
        let default_part = import_part[..comma_pos].trim();
        let rest_part = import_part[comma_pos + 1..].trim();
        let default_import = if !default_part.is_empty() {
            Some(default_part.to_string())
        } else {
            None
        };
        let named_imports = if rest_part.starts_with('{') && rest_part.ends_with('}') {
            let inner = &rest_part[1..rest_part.len() - 1];
            parse_named_imports(inner)?
        } else {
            Vec::new()
        };
        return Ok((default_import, named_imports, None));
    }
    if !import_part.is_empty() && !import_part.starts_with('{') {
        return Ok((Some(import_part.to_string()), Vec::new(), None));
    }
    Ok((None, Vec::new(), None))
}
/// Parse named imports from braces content
fn parse_named_imports(inner: &str) -> AstResult<Vec<NamedImport>> {
    let mut named_imports = Vec::new();
    for item in inner.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let type_only = item.starts_with("type ");
        let item = if type_only { &item[5..] } else { item };
        if let Some(as_pos) = item.find(" as ") {
            let name = item[..as_pos].trim().to_string();
            let alias = item[as_pos + 4..].trim().to_string();
            named_imports.push(NamedImport {
                name,
                alias: Some(alias),
                type_only,
            });
        } else {
            named_imports.push(NamedImport {
                name: item.to_string(),
                alias: None,
                type_only,
            });
        }
    }
    Ok(named_imports)
}

/// Parse Rust imports using AST (syn crate)
///
/// This provides accurate parsing of complex Rust import statements including:
/// - Nested module paths: `use std::collections::HashMap;`
/// - Grouped imports: `use std::{sync::Arc, collections::HashMap};`
/// - Glob imports: `use module::*;`
/// - Aliased imports: `use std::collections::HashMap as Map;`
/// - Nested groups: `use std::{io::{self, Read}, collections::*};`
///
/// Check if a module path represents an external dependency
fn is_external_dependency(module_path: &str) -> bool {
    if module_path.starts_with("./") || module_path.starts_with("../") {
        return false;
    }
    if module_path.starts_with("/") || module_path.starts_with("src/") {
        return false;
    }
    if module_path.starts_with("@") {
        return true;
    }
    !module_path.contains("/")
        || module_path.contains("node_modules")
        || !module_path.starts_with(".")
}
/// Build a dependency graph for a collection of files
pub fn build_dependency_graph(import_graphs: &[ImportGraph]) -> DependencyGraph {
    let mut graph = Graph::new();
    let mut file_nodes = HashMap::new();
    let mut path_to_node = HashMap::new();
    for import_graph in import_graphs {
        let node = graph.add_node(import_graph.source_file.clone());
        file_nodes.insert(import_graph.source_file.clone(), node);
        path_to_node.insert(import_graph.source_file.clone(), node);
    }
    for import_graph in import_graphs {
        if let Some(&source_node) = file_nodes.get(&import_graph.source_file) {
            for import in &import_graph.imports {
                if let Some(target_file) = resolve_import_path(
                    &import.module_path,
                    &import_graph.source_file,
                    import_graphs,
                ) {
                    if let Some(&target_node) = file_nodes.get(&target_file) {
                        graph.add_edge(source_node, target_node, import.clone());
                    }
                }
            }
        }
    }
    let circular_dependencies = detect_cycles(&graph, &path_to_node);
    DependencyGraph {
        graph,
        file_nodes,
        circular_dependencies,
    }
}
/// Dependency graph structure
pub struct DependencyGraph {
    pub graph: Graph<String, ImportInfo>,
    pub file_nodes: HashMap<String, NodeIndex>,
    pub circular_dependencies: Vec<Vec<String>>,
}
impl DependencyGraph {
    /// Get all files that import the given file
    pub fn get_importers(&self, file_path: &str) -> Vec<String> {
        if let Some(&node) = self.file_nodes.get(file_path) {
            self.graph
                .neighbors_directed(node, Direction::Incoming)
                .map(|n| self.graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
    /// Get all files imported by the given file
    pub fn get_imports(&self, file_path: &str) -> Vec<String> {
        if let Some(&node) = self.file_nodes.get(file_path) {
            self.graph
                .neighbors_directed(node, Direction::Outgoing)
                .map(|n| self.graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
    /// Check if there's a dependency path between two files
    pub fn has_dependency_path(&self, from: &str, to: &str) -> bool {
        if let (Some(&from_node), Some(&to_node)) =
            (self.file_nodes.get(from), self.file_nodes.get(to))
        {
            petgraph::algo::has_path_connecting(&self.graph, from_node, to_node, None)
        } else {
            false
        }
    }
}
/// Resolve an import path to an actual file path
fn resolve_import_path(
    import_path: &str,
    source_file: &str,
    graphs: &[ImportGraph],
) -> Option<String> {
    if import_path.starts_with("./") || import_path.starts_with("../") {
        let source_dir = Path::new(source_file).parent()?;
        let resolved = source_dir.join(import_path);
        for ext in &["", ".ts", ".tsx", ".js", ".jsx", ".json"] {
            let with_ext = format!("{}{}", resolved.to_string_lossy(), ext);
            if graphs.iter().any(|g| g.source_file == with_ext) {
                return Some(with_ext);
            }
        }
    }
    for graph in graphs {
        if graph.source_file.ends_with(import_path)
            || graph.source_file.contains(&format!("/{}", import_path))
        {
            return Some(graph.source_file.clone());
        }
    }
    None
}
/// Detect circular dependencies in the graph
fn detect_cycles(
    graph: &Graph<String, ImportInfo>,
    path_to_node: &HashMap<String, NodeIndex>,
) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();
    for &node in path_to_node.values() {
        if !visited.contains(&node) {
            find_cycles_dfs(
                graph,
                node,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }
    cycles
}
/// DFS helper for cycle detection
fn find_cycles_dfs(
    graph: &Graph<String, ImportInfo>,
    node: NodeIndex,
    visited: &mut HashSet<NodeIndex>,
    rec_stack: &mut HashSet<NodeIndex>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node);
    rec_stack.insert(node);
    path.push(graph[node].clone());
    for neighbor in graph.neighbors(node) {
        if !visited.contains(&neighbor) {
            find_cycles_dfs(graph, neighbor, visited, rec_stack, path, cycles);
        } else if rec_stack.contains(&neighbor) {
            let cycle_start = path.iter().position(|p| p == &graph[neighbor]).unwrap_or(0);
            let cycle = path[cycle_start..].to_vec();
            cycles.push(cycle);
        }
    }
    path.pop();
    rec_stack.remove(&node);
}
#[cfg(test)]
mod tests {
    use super::*;
    // TypeScript/JavaScript import parsing tests have been moved to mill-lang-typescript plugin tests
    // Python import parsing tests have been moved to mill-lang-python plugin tests
    #[test]
    fn test_is_external_dependency() {
        assert!(is_external_dependency("react"));
        assert!(is_external_dependency("@types/node"));
        assert!(is_external_dependency("lodash"));
        assert!(!is_external_dependency("./component"));
        assert!(!is_external_dependency("../utils"));
        assert!(!is_external_dependency("src/types"));
    }
    #[test]
    fn test_dependency_graph() {
        let graphs = vec![
            ImportGraph {
                source_file: "a.ts".to_string(),
                imports: vec![ImportInfo {
                    module_path: "./b".to_string(),
                    import_type: ImportType::EsModule,
                    named_imports: vec![],
                    default_import: None,
                    namespace_import: None,
                    type_only: false,
                    location: SourceLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 20,
                    },
                }],
                importers: vec![],
                metadata: ImportGraphMetadata {
                    language: "typescript".to_string(),
                    parsed_at: chrono::Utc::now(),
                    parser_version: "0.2.0".to_string(),
                    circular_dependencies: vec![],
                    external_dependencies: vec![],
                },
            },
            ImportGraph {
                source_file: "b.ts".to_string(),
                imports: vec![],
                importers: vec![],
                metadata: ImportGraphMetadata {
                    language: "typescript".to_string(),
                    parsed_at: chrono::Utc::now(),
                    parser_version: "0.2.0".to_string(),
                    circular_dependencies: vec![],
                    external_dependencies: vec![],
                },
            },
        ];
        let dep_graph = build_dependency_graph(&graphs);
        assert!(dep_graph.file_nodes.contains_key("a.ts"));
        assert!(dep_graph.file_nodes.contains_key("b.ts"));
        let imports = dep_graph.get_imports("a.ts");
        assert_eq!(imports.len(), 0);
        let importers = dep_graph.get_importers("b.ts");
        assert_eq!(importers.len(), 0);
    }
    // test_parse_named_imports_enhanced removed - see mill-lang-typescript plugin tests
}
