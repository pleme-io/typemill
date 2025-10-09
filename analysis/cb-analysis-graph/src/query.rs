//! A high-level query API for analysis graphs.
//!
//! This trait provides a common interface for performing complex queries
//! on both dependency graphs and call graphs.

use crate::call::{CallSite, FunctionId};
use crate::dependency::{Dependency, DependencyGraph, ModuleNode, NodeId};
use petgraph::algo::{all_simple_paths, tarjan_scc};
use std::collections::HashSet;

/// A trait for querying analysis graphs.
pub trait GraphQuery {
    /// The type of node in the graph.
    type Node;
    /// The type of edge in the graph.
    type Edge;

    /// Finds all simple paths (no repeated nodes) from a source to a target.
    ///
    /// This is useful for detecting and diagnosing circular dependencies.
    fn find_all_simple_paths(&self, from: NodeId, to: NodeId) -> Vec<Vec<NodeId>>;

    /// Finds all strongly connected components (SCCs) in the graph.
    ///
    /// An SCC is a subgraph where every node is reachable from every other node.
    /// Any SCC with more than one node represents a cycle.
    fn strongly_connected_components(&self) -> Vec<Vec<NodeId>>;

    /// Gets all transitive dependencies of a given node.
    fn transitive_dependencies(&self, start_node: NodeId) -> HashSet<NodeId>;

    /// Gets all call sites for a given function.
    ///
    /// Note: This is only applicable to `CallGraph`.
    fn get_call_sites(&self, _function: FunctionId) -> Vec<CallSite> {
        // Default implementation returns an empty Vec.
        vec![]
    }

    /// Finds all functions reachable from a given entry point.
    ///
    /// Note: This is only applicable to `CallGraph`.
    fn reachable_functions(&self, _entry: FunctionId) -> HashSet<FunctionId> {
        // Default implementation returns an empty set.
        HashSet::new()
    }
}

impl GraphQuery for DependencyGraph {
    type Node = ModuleNode;
    type Edge = Dependency;

    fn find_all_simple_paths(&self, from: NodeId, to: NodeId) -> Vec<Vec<NodeId>> {
        all_simple_paths(&self.graph, from, to, 0, None)
            .collect()
    }

    fn strongly_connected_components(&self) -> Vec<Vec<NodeId>> {
        tarjan_scc(&self.graph)
    }

    fn transitive_dependencies(&self, start_node: NodeId) -> HashSet<NodeId> {
        // This is a direct call to the method on DependencyGraph.
        self.transitive_dependencies(start_node)
    }
}