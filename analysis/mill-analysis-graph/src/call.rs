//! A whole-program call graph.
//!
//! This data structure represents the relationships between functions and methods
//! in a codebase, tracking definitions and call sites.

use mill_plugin_api::SourceLocation;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A unique identifier for a function in the call graph.
pub type FunctionId = NodeIndex;

/// The visibility of a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    /// Accessible from anywhere.
    Public,
    /// Accessible only within the same module/crate.
    Internal,
    /// Accessible only within the same file or class.
    Private,
}

/// The type of a function call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallType {
    /// A direct, statically dispatched call.
    Direct,
    /// An indirect call via a function pointer or trait object.
    Indirect,
    /// A virtual call that is dispatched at runtime.
    Virtual,
}

/// Represents a function signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub params: Vec<(String, String)>, // (name, type)
    pub return_type: Option<String>,
}

/// Represents a single function definition (a node in the graph).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionNode {
    /// The fully qualified name of the function.
    pub name: String,
    /// The location of the function definition in the source code.
    pub location: SourceLocation,
    /// The function's signature.
    pub signature: FunctionSignature,
    /// The visibility of the function.
    pub visibility: Visibility,
}

/// Represents a call from one function to another (an edge in the graph).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    /// The location of the call in the source code.
    pub location: SourceLocation,
    /// The type of the call.
    pub call_type: CallType,
}

/// The primary data structure for the program call graph.
#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraph {
    /// The underlying directed graph from petgraph.
    pub graph: DiGraph<FunctionNode, CallSite>,
    /// A map from function names to their corresponding node indices.
    #[serde(skip)]
    pub nodes: HashMap<String, FunctionId>,
}

impl CallGraph {
    /// Creates a new, empty call graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            nodes: HashMap::new(),
        }
    }

    /// Adds a function to the graph if it doesn't already exist.
    ///
    /// Returns the `FunctionId` of the new or existing function.
    pub fn add_function(&mut self, node: FunctionNode) -> FunctionId {
        *self
            .nodes
            .entry(node.name.clone())
            .or_insert_with(|| self.graph.add_node(node))
    }

    /// Adds a call relationship between two functions.
    pub fn add_call(&mut self, from: FunctionId, to: FunctionId, call_site: CallSite) {
        self.graph.add_edge(from, to, call_site);
    }
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_plugin_api::SourceLocation;
    use pretty_assertions::assert_eq;

    fn create_test_call_graph() -> (CallGraph, FunctionId, FunctionId, FunctionId) {
        let mut graph = CallGraph::new();

        let func_a_node = FunctionNode {
            name: "a".to_string(),
            location: SourceLocation { line: 1, column: 0 },
            signature: FunctionSignature {
                params: vec![],
                return_type: None,
            },
            visibility: Visibility::Public,
        };
        let func_b_node = FunctionNode {
            name: "b".to_string(),
            location: SourceLocation { line: 5, column: 0 },
            signature: FunctionSignature {
                params: vec![],
                return_type: None,
            },
            visibility: Visibility::Public,
        };
        let func_c_node = FunctionNode {
            name: "c".to_string(),
            location: SourceLocation {
                line: 10,
                column: 0,
            },
            signature: FunctionSignature {
                params: vec![],
                return_type: None,
            },
            visibility: Visibility::Private,
        };

        let id_a = graph.add_function(func_a_node);
        let id_b = graph.add_function(func_b_node);
        let id_c = graph.add_function(func_c_node);

        // a() -> b()
        graph.add_call(
            id_a,
            id_b,
            CallSite {
                location: SourceLocation { line: 2, column: 4 },
                call_type: CallType::Direct,
            },
        );

        // b() -> c()
        graph.add_call(
            id_b,
            id_c,
            CallSite {
                location: SourceLocation { line: 6, column: 4 },
                call_type: CallType::Direct,
            },
        );

        (graph, id_a, id_b, id_c)
    }

    #[test]
    fn test_add_function_and_call() {
        let (graph, id_a, id_b, _) = create_test_call_graph();

        assert_eq!(graph.graph.node_count(), 3);
        assert_eq!(graph.graph.edge_count(), 2);
        assert!(graph.graph.contains_edge(id_a, id_b));
    }

    #[test]
    fn test_callers_and_callees() {
        let (graph, id_a, id_b, id_c) = create_test_call_graph();

        // Test callees of a
        let callees_a: Vec<_> = graph.graph.neighbors(id_a).collect();
        assert_eq!(callees_a, vec![id_b]);

        // Test callers of b
        let callers_b: Vec<_> = graph
            .graph
            .neighbors_directed(id_b, petgraph::Direction::Incoming)
            .collect();
        assert_eq!(callers_b, vec![id_a]);

        // Test callees of b
        let callees_b: Vec<_> = graph.graph.neighbors(id_b).collect();
        assert_eq!(callees_b, vec![id_c]);
    }
}
