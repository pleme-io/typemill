// analysis/cb-analysis-common/src/graph.rs

//! A generic dependency graph for symbol analysis.

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a node in the symbol dependency graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolNode {
    pub id: String,   // A unique identifier, e.g., "file.rs::MyStruct::my_function"
    pub name: String, // The symbol name, e.g., "my_function"
    pub file_path: String,
    pub is_public: bool, // Is the symbol exported or part of a public API?
}

/// The dependency graph, mapping symbol relationships.
pub struct DependencyGraph {
    pub graph: DiGraph<SymbolNode, ()>,
    pub node_map: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    /// Creates a new, empty dependency graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Adds a symbol to the graph if it doesn't already exist.
    pub fn add_symbol(&mut self, symbol: SymbolNode) {
        if !self.node_map.contains_key(&symbol.id) {
            let index = self.graph.add_node(symbol.clone());
            self.node_map.insert(symbol.id, index);
        }
    }

    /// Adds a dependency relationship between two symbols.
    /// `from_id` is the symbol that depends on `to_id`.
    pub fn add_dependency(&mut self, from_id: &str, to_id: &str) {
        if let (Some(&from_index), Some(&to_index)) =
            (self.node_map.get(from_id), self.node_map.get(to_id))
        {
            self.graph.add_edge(from_index, to_index, ());
        }
    }

    /// Finds all symbols that are not referenced by any other symbol in the graph.
    /// This is a simple, naive dead code detection.
    pub fn find_unreferenced_nodes(&self) -> Vec<&SymbolNode> {
        self.graph
            .externals(petgraph::Direction::Incoming)
            .map(|index| &self.graph[index])
            .collect()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DependencyGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "DependencyGraph {{")?;
        writeln!(f, "  Nodes:")?;
        for node_index in self.graph.node_indices() {
            writeln!(f, "    {:?}: {:?}", node_index, &self.graph[node_index])?;
        }
        writeln!(f, "  Edges:")?;
        for edge in self.graph.edge_references() {
            writeln!(f, "    {:?} -> {:?}", edge.source(), edge.target())?;
        }
        writeln!(f, "}}")
    }
}
