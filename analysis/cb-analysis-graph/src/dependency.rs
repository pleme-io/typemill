//! A module-level dependency graph.
//!
//! This data structure represents the relationships between files/modules
//! in a codebase, tracking imports, exports, and re-exports.

use mill_plugin_api::Symbol;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A unique identifier for a node in the dependency graph.
pub type NodeId = NodeIndex;

/// Represents a single file or module in the codebase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleNode {
    /// The absolute path to the file.
    pub path: PathBuf,
    /// The programming language of the module (e.g., "rust", "typescript").
    pub language: String,
    /// A list of symbols exported by this module.
    pub exports: Vec<Symbol>,
}

/// The type of dependency between two modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind {
    /// A direct import (e.g., `import { X } from './foo'`).
    Import,
    /// An export from another module (e.g., `export { X } from './foo'`).
    Export,
    /// A re-export that also makes the symbol available in the current module.
    ReExport,
}

/// Represents a dependency relationship (an edge in the graph).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    /// The kind of dependency.
    pub kind: DependencyKind,
    /// The specific symbols being imported/exported, if applicable.
    /// An empty vector implies importing the entire module (e.g., `import * as foo from './foo'`).
    pub symbols: Vec<String>,
}

/// The primary data structure for the module dependency graph.
#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// The underlying directed graph from petgraph.
    pub graph: DiGraph<ModuleNode, Dependency>,
    /// A map from file paths to their corresponding node indices in the graph.
    /// This provides fast lookups for existing modules.
    #[serde(skip)]
    pub nodes: HashMap<PathBuf, NodeId>,
}

impl DependencyGraph {
    /// Creates a new, empty dependency graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            nodes: HashMap::new(),
        }
    }

    /// Adds a module to the graph if it doesn't already exist.
    ///
    /// Returns the `NodeId` of the new or existing module.
    pub fn add_module(&mut self, node: ModuleNode) -> NodeId {
        // Use the entry API to avoid a double lookup.
        *self
            .nodes
            .entry(node.path.clone())
            .or_insert_with(|| self.graph.add_node(node))
    }

    /// Adds a dependency between two modules, identified by their paths.
    ///
    /// If the modules do not exist in the graph, they will be created with
    /// default `ModuleNode` values.
    pub fn add_dependency(
        &mut self,
        from: &Path,
        to: &Path,
        dependency: Dependency,
        default_lang: &str,
    ) {
        let from_node = ModuleNode {
            path: from.to_path_buf(),
            language: default_lang.to_string(),
            exports: vec![],
        };
        let to_node = ModuleNode {
            path: to.to_path_buf(),
            language: default_lang.to_string(),
            exports: vec![],
        };

        let from_id = self.add_module(from_node);
        let to_id = self.add_module(to_node);

        self.graph.add_edge(from_id, to_id, dependency);
    }

    /// Finds a module by its path.
    ///
    /// Returns the `NodeId` if the module is found.
    pub fn find_node_by_path(&self, path: &Path) -> Option<NodeId> {
        self.nodes.get(path).copied()
    }

    /// Retrieves all direct dependencies of a given module.
    pub fn direct_dependencies(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.graph.neighbors(id)
    }

    /// Retrieves all modules that directly depend on the given module.
    pub fn direct_dependents(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.graph
            .neighbors_directed(id, petgraph::Direction::Incoming)
    }

    /// Calculates the set of all modules that a given module transitively depends on.
    ///
    /// This includes direct dependencies, their dependencies, and so on.
    /// The starting node itself is not included in the result.
    pub fn transitive_dependencies(&self, start_node: NodeId) -> std::collections::HashSet<NodeId> {
        // A Dfs visitor explores nodes in depth-first order.
        let mut dfs = petgraph::visit::Dfs::new(&self.graph, start_node);

        // The first node visited is the starting node itself. We skip it.
        dfs.next(&self.graph);

        // Collect all other reachable nodes.
        let mut dependencies = std::collections::HashSet::new();
        while let Some(nx) = dfs.next(&self.graph) {
            dependencies.insert(nx);
        }
        dependencies
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashSet;

    fn create_test_graph() -> (DependencyGraph, PathBuf, PathBuf, PathBuf, PathBuf) {
        let mut graph = DependencyGraph::new();

        let path_a = PathBuf::from("/test/a.rs");
        let path_b = PathBuf::from("/test/b.rs");
        let path_c = PathBuf::from("/test/c.rs");
        let path_d = PathBuf::from("/test/d.rs");

        // A -> B -> C
        // A -> D
        graph.add_dependency(
            &path_a,
            &path_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
            "rust",
        );
        graph.add_dependency(
            &path_b,
            &path_c,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
            "rust",
        );
        graph.add_dependency(
            &path_a,
            &path_d,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
            "rust",
        );

        (graph, path_a, path_b, path_c, path_d)
    }

    #[test]
    fn test_add_module_and_dependency() {
        let (graph, path_a, path_b, _, _) = create_test_graph();

        assert_eq!(graph.graph.node_count(), 4);
        assert_eq!(graph.graph.edge_count(), 3);

        let id_a = graph.find_node_by_path(&path_a).unwrap();
        let id_b = graph.find_node_by_path(&path_b).unwrap();

        assert!(graph.graph.contains_edge(id_a, id_b));
    }

    #[test]
    fn test_direct_dependencies() {
        let (graph, path_a, path_b, path_c, path_d) = create_test_graph();
        let id_a = graph.find_node_by_path(&path_a).unwrap();
        let id_b = graph.find_node_by_path(&path_b).unwrap();
        let id_c = graph.find_node_by_path(&path_c).unwrap();

        let deps_a: HashSet<_> = graph.direct_dependencies(id_a).collect();
        let expected_deps_a: HashSet<_> = [
            graph.find_node_by_path(&path_b).unwrap(),
            graph.find_node_by_path(&path_d).unwrap(),
        ]
        .iter()
        .cloned()
        .collect();
        assert_eq!(deps_a, expected_deps_a);

        let deps_b: HashSet<_> = graph.direct_dependencies(id_b).collect();
        assert_eq!(deps_b, [id_c].iter().cloned().collect());
    }

    #[test]
    fn test_transitive_dependencies() {
        let (graph, path_a, path_b, path_c, path_d) = create_test_graph();
        let id_a = graph.find_node_by_path(&path_a).unwrap();

        let transitive_deps: HashSet<_> = graph.transitive_dependencies(id_a);

        let expected_deps: HashSet<_> = [
            graph.find_node_by_path(&path_b).unwrap(),
            graph.find_node_by_path(&path_c).unwrap(),
            graph.find_node_by_path(&path_d).unwrap(),
        ]
        .iter()
        .cloned()
        .collect();

        assert_eq!(transitive_deps, expected_deps);
    }

    #[test]
    fn test_direct_dependents() {
        let (graph, path_a, path_b, path_c, _) = create_test_graph();
        let id_a = graph.find_node_by_path(&path_a).unwrap();
        let id_b = graph.find_node_by_path(&path_b).unwrap();
        let id_c = graph.find_node_by_path(&path_c).unwrap();

        let dependents_b: HashSet<_> = graph.direct_dependents(id_b).collect();
        assert_eq!(dependents_b, [id_a].iter().cloned().collect());

        let dependents_c: HashSet<_> = graph.direct_dependents(id_c).collect();
        assert_eq!(dependents_c, [id_b].iter().cloned().collect());
    }
}