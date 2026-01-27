//! Call graph construction.

use crate::types::{Reference, Symbol};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// A call graph representing dependencies between symbols.
pub(crate) struct CallGraph {
    /// The underlying directed graph.
    graph: DiGraph<String, ()>,

    /// Map from symbol ID to node index.
    id_to_node: HashMap<String, NodeIndex>,

    /// Map from node index to symbol ID.
    node_to_id: HashMap<NodeIndex, String>,
}

impl CallGraph {
    /// Build a call graph from symbols and their references.
    pub fn new(symbols: &[Symbol], references: &[Reference]) -> Self {
        let mut graph = DiGraph::new();
        let mut id_to_node = HashMap::new();
        let mut node_to_id = HashMap::new();

        // Add all symbols as nodes
        for symbol in symbols {
            let node = graph.add_node(symbol.id.clone());
            id_to_node.insert(symbol.id.clone(), node);
            node_to_id.insert(node, symbol.id.clone());
        }

        // Add edges for references (from -> to means "from references to")
        for reference in references {
            if let (Some(&from_node), Some(&to_node)) = (
                id_to_node.get(&reference.from_id),
                id_to_node.get(&reference.to_id),
            ) {
                // Only add if not already present
                if !graph.contains_edge(from_node, to_node) {
                    graph.add_edge(from_node, to_node, ());
                }
            }
        }

        Self {
            graph,
            id_to_node,
            node_to_id,
        }
    }

    /// Get the node index for a symbol ID.
    pub fn get_node(&self, id: &str) -> Option<NodeIndex> {
        self.id_to_node.get(id).copied()
    }

    /// Get the symbol ID for a node index.
    pub fn get_id(&self, node: NodeIndex) -> Option<&String> {
        self.node_to_id.get(&node)
    }

    /// Get all node indices.
    #[allow(dead_code)] // May be used for future analysis extensions
    pub fn nodes(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.node_indices()
    }

    /// Get neighbors (symbols this one references).
    pub fn neighbors(&self, node: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.neighbors(node)
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

/// Build a call graph from symbols and references.
pub(crate) fn build(symbols: &[Symbol], references: &[Reference]) -> CallGraph {
    CallGraph::new(symbols, references)
}
