// analysis/mill-analysis-deep-dead-code/src/dead_code_finder.rs

use crate::DeepDeadCodeConfig;
use mill_analysis_common::graph::{DependencyGraph, SymbolNode};
use std::collections::HashSet;
use std::path::Path;
use tracing::info;

pub struct DeadCodeFinder<'a> {
    graph: &'a DependencyGraph,
}

impl<'a> DeadCodeFinder<'a> {
    pub fn new(graph: &'a DependencyGraph) -> Self {
        Self { graph }
    }

    /// Finds all symbols that are considered "dead" by performing a reachability
    /// analysis from the public API surface.
    pub fn find(&self, config: &DeepDeadCodeConfig) -> Vec<SymbolNode> {
        if self.graph.node_map.is_empty() {
            return vec![];
        }

        info!("Analyzing dependency graph to find dead symbols...");

        let mut live_symbols = HashSet::new();

        let mut entry_points: Vec<_> = self
            .graph
            .graph
            .node_indices()
            .filter(|&i| {
                let node = &self.graph.graph[i];
                // A simple heuristic to identify the main function: it must be named "main"
                // and be in a file named "main.rs". This is more robust than just checking the name.
                node.name == "main" && Path::new(&node.file_path).ends_with("main.rs")
            })
            .collect();

        if !config.check_public_exports {
            info!("Default mode: Using 'main' and all other public symbols as entry points.");
            entry_points.extend(
                self.graph.graph.node_indices().filter(|&i| {
                    self.graph.graph[i].is_public && self.graph.graph[i].name != "main"
                }),
            );
        } else {
            info!("Aggressive mode enabled: public exports will not be considered entry points unless they are 'main'.");
        }

        info!(
            "Found {} entry points for graph traversal.",
            entry_points.len()
        );

        let mut worklist = entry_points;
        while let Some(node_index) = worklist.pop() {
            if live_symbols.insert(node_index) {
                for neighbor in self.graph.graph.neighbors(node_index) {
                    if !live_symbols.contains(&neighbor) {
                        worklist.push(neighbor);
                    }
                }
            }
        }

        info!(
            "Found {} live symbols through graph traversal.",
            live_symbols.len()
        );

        let mut dead_symbols = Vec::new();
        for &node_index in self.graph.node_map.values() {
            if !live_symbols.contains(&node_index) {
                dead_symbols.push(self.graph.graph[node_index].clone());
            }
        }

        info!("Found {} potentially dead symbols.", dead_symbols.len());
        dead_symbols
    }
}
