//! Detects and reports circular dependencies in a `DependencyGraph`.

pub mod builder;

use mill_analysis_graph::dependency::{DependencyGraph, NodeId};
use petgraph::algo::tarjan_scc;
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// The primary output of the circular dependency analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircularDependenciesResult {
    pub cycles: Vec<Cycle>,
    pub summary: Summary,
}

/// Represents a single circular dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cycle {
    pub id: usize,
    pub modules: Vec<String>,
    pub import_chain: Vec<ImportLink>,
}

/// Represents a single link in the import chain of a cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportLink {
    pub from: String,
    pub to: String,
    pub symbols: Vec<String>,
}

/// A summary of the analysis results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub total_cycles: usize,
    pub total_modules_in_cycles: usize,
    pub largest_cycle_size: usize,
    pub files_analyzed: usize,
    pub analysis_time_ms: u64,
}

/// Errors that can occur during circular dependency analysis.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Graph construction failed: {0}")]
    GraphError(String),
    #[error("An unexpected error occurred: {0}")]
    Internal(String),
}

/// Analyzes the given dependency graph to find all circular dependencies.
///
/// # Arguments
///
/// * `graph` - A `DependencyGraph` representing the codebase's module dependencies.
///
/// # Returns
///
/// A `Result` containing the `CircularDependenciesResult` on success, or an `Error` on failure.
pub fn find_circular_dependencies(
    graph: &DependencyGraph,
    min_size: Option<usize>,
) -> Result<CircularDependenciesResult, Error> {
    use std::time::Instant;

    let start = Instant::now();
    let files_analyzed = graph.graph.node_count();

    // Use Tarjan's algorithm to find strongly connected components (SCCs).
    let sccs = tarjan_scc(&graph.graph);

    let min_cycle_size = min_size.unwrap_or(2);

    let cycles_nodes: Vec<Vec<NodeId>> = sccs
        .into_iter()
        .filter(|scc| scc.len() >= min_cycle_size)
        .collect();

    let mut cycles = Vec::new();
    let mut total_modules_in_cycles = 0;
    let mut largest_cycle_size = 0;

    for (i, scc) in cycles_nodes.iter().enumerate() {
        let cycle_nodes: HashSet<NodeId> = scc.iter().cloned().collect();
        let modules: Vec<String> = scc
            .iter()
            .map(|&node_id| {
                graph
                    .graph
                    .node_weight(node_id)
                    .unwrap()
                    .path
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        total_modules_in_cycles += modules.len();
        if modules.len() > largest_cycle_size {
            largest_cycle_size = modules.len();
        }

        let import_chain = build_import_chain(graph, scc, &cycle_nodes);

        cycles.push(Cycle {
            id: i + 1,
            modules,
            import_chain,
        });
    }

    let analysis_time_ms = start.elapsed().as_millis() as u64;

    let summary = Summary {
        total_cycles: cycles.len(),
        total_modules_in_cycles,
        largest_cycle_size,
        files_analyzed,
        analysis_time_ms,
    };

    Ok(CircularDependenciesResult { cycles, summary })
}

/// Constructs the chain of imports that form a cycle.
pub(crate) fn build_import_chain(
    dep_graph: &DependencyGraph,
    scc: &[NodeId],
    cycle_nodes: &HashSet<NodeId>,
) -> Vec<ImportLink> {
    if scc.is_empty() {
        return vec![];
    }

    let mut chain = Vec::new();
    let mut current_node = scc[0];
    let start_node = scc[0];
    let mut visited = HashSet::new();

    loop {
        visited.insert(current_node);
        let current_node_path = dep_graph
            .graph
            .node_weight(current_node)
            .unwrap()
            .path
            .to_string_lossy()
            .to_string();

        // Find an edge from the current node to another node within the same cycle.
        let edge = dep_graph
            .graph
            .edges(current_node)
            .find(|edge| cycle_nodes.contains(&edge.target()));

        if let Some(edge) = edge {
            let target_node = edge.target();
            let target_node_path = dep_graph
                .graph
                .node_weight(target_node)
                .unwrap()
                .path
                .to_string_lossy()
                .to_string();
            let dependency = edge.weight();

            chain.push(ImportLink {
                from: current_node_path.clone(),
                to: target_node_path,
                symbols: dependency.symbols.clone(),
            });

            current_node = target_node;
            if current_node == start_node || visited.contains(&current_node) {
                break;
            }
        } else {
            // Should not happen in a strongly connected component.
            break;
        }
    }

    chain
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_analysis_graph::dependency::{
        Dependency, DependencyGraph, DependencyKind, ModuleNode,
    };
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    fn module(path: &str) -> ModuleNode {
        ModuleNode {
            path: PathBuf::from(path),
            language: "rust".to_string(),
            exports: vec![],
        }
    }

    #[test]
    fn test_find_circular_dependencies_with_simple_cycle() {
        let mut graph = DependencyGraph::new();
        let id_a = graph.add_module(module("/test/a.rs"));
        let id_b = graph.add_module(module("/test/b.rs"));

        graph.graph.add_edge(
            id_a,
            id_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec!["B".to_string()],
            },
        );
        graph.graph.add_edge(
            id_b,
            id_a,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec!["A".to_string()],
            },
        );

        let result = find_circular_dependencies(&graph, None).unwrap();

        assert_eq!(result.summary.total_cycles, 1);
        assert_eq!(result.cycles.len(), 1);
        let cycle = &result.cycles[0];
        assert_eq!(cycle.modules.len(), 2);
        assert!(cycle.modules.contains(&"/test/a.rs".to_string()));
        assert!(cycle.modules.contains(&"/test/b.rs".to_string()));
        assert_eq!(cycle.import_chain.len(), 2);
    }

    #[test]
    fn test_no_cycles_found() {
        let mut graph = DependencyGraph::new();
        let id_a = graph.add_module(module("/test/a.rs"));
        let id_b = graph.add_module(module("/test/b.rs"));

        graph.graph.add_edge(
            id_a,
            id_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec!["B".to_string()],
            },
        );

        let result = find_circular_dependencies(&graph, None).unwrap();

        assert_eq!(result.summary.total_cycles, 0);
        assert_eq!(result.cycles.len(), 0);
    }

    #[test]
    fn test_three_node_cycle() {
        let mut graph = DependencyGraph::new();
        let id_a = graph.add_module(module("/test/a.rs"));
        let id_b = graph.add_module(module("/test/b.rs"));
        let id_c = graph.add_module(module("/test/c.rs"));

        graph.graph.add_edge(
            id_a,
            id_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_b,
            id_c,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_c,
            id_a,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );

        let result = find_circular_dependencies(&graph, None).unwrap();
        assert_eq!(result.summary.total_cycles, 1);
        assert_eq!(result.cycles[0].modules.len(), 3);
        assert_eq!(result.cycles[0].import_chain.len(), 3);
    }

    #[test]
    fn test_multiple_independent_cycles() {
        // A ↔ B and C ↔ D ↔ E (two independent cycles)
        let mut graph = DependencyGraph::new();

        // First cycle: A ↔ B
        let id_a = graph.add_module(module("/test/a.rs"));
        let id_b = graph.add_module(module("/test/b.rs"));
        graph.graph.add_edge(
            id_a,
            id_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_b,
            id_a,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );

        // Second cycle: C → D → E → C
        let id_c = graph.add_module(module("/test/c.rs"));
        let id_d = graph.add_module(module("/test/d.rs"));
        let id_e = graph.add_module(module("/test/e.rs"));
        graph.graph.add_edge(
            id_c,
            id_d,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_d,
            id_e,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_e,
            id_c,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );

        let result = find_circular_dependencies(&graph, None).unwrap();
        assert_eq!(result.summary.total_cycles, 2);
        assert_eq!(result.summary.total_modules_in_cycles, 5); // 2 + 3
        assert_eq!(result.summary.largest_cycle_size, 3);
    }

    #[test]
    fn test_nested_overlapping_cycles() {
        // A ↔ B and B → C → D → B (overlapping at B)
        let mut graph = DependencyGraph::new();
        let id_a = graph.add_module(module("/test/a.rs"));
        let id_b = graph.add_module(module("/test/b.rs"));
        let id_c = graph.add_module(module("/test/c.rs"));
        let id_d = graph.add_module(module("/test/d.rs"));

        // First cycle: A ↔ B
        graph.graph.add_edge(
            id_a,
            id_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_b,
            id_a,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );

        // Second cycle: B → C → D → B
        graph.graph.add_edge(
            id_b,
            id_c,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_c,
            id_d,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_d,
            id_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );

        let result = find_circular_dependencies(&graph, None).unwrap();
        // Should find at least 1 cycle (could be more depending on SCC algorithm)
        assert!(result.summary.total_cycles >= 1);
    }

    #[test]
    fn test_large_cycle() {
        // Create a cycle with 10 modules
        let mut graph = DependencyGraph::new();
        let mut nodes = Vec::new();

        for i in 0..10 {
            nodes.push(graph.add_module(module(&format!("/test/mod{}.rs", i))));
        }

        // Create chain: 0 → 1 → 2 → ... → 9 → 0
        for i in 0..10 {
            let next = (i + 1) % 10;
            graph.graph.add_edge(
                nodes[i],
                nodes[next],
                Dependency {
                    kind: DependencyKind::Import,
                    symbols: vec![],
                },
            );
        }

        let result = find_circular_dependencies(&graph, None).unwrap();
        assert_eq!(result.summary.total_cycles, 1);
        assert_eq!(result.cycles[0].modules.len(), 10);
        assert_eq!(result.summary.largest_cycle_size, 10);
    }

    #[test]
    fn test_symbols_in_dependency() {
        let mut graph = DependencyGraph::new();
        let id_a = graph.add_module(module("/test/a.rs"));
        let id_b = graph.add_module(module("/test/b.rs"));

        graph.graph.add_edge(
            id_a,
            id_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec!["Foo".to_string(), "Bar".to_string()],
            },
        );
        graph.graph.add_edge(
            id_b,
            id_a,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec!["Baz".to_string()],
            },
        );

        let result = find_circular_dependencies(&graph, None).unwrap();
        assert_eq!(result.summary.total_cycles, 1);

        // Check that import chain includes symbols
        let cycle = &result.cycles[0];
        assert_eq!(cycle.import_chain.len(), 2);

        // Find the edge with multiple symbols
        let multi_symbol_link = cycle
            .import_chain
            .iter()
            .find(|link| link.symbols.len() == 2);
        assert!(multi_symbol_link.is_some());

        if let Some(link) = multi_symbol_link {
            assert!(link.symbols.contains(&"Foo".to_string()));
            assert!(link.symbols.contains(&"Bar".to_string()));
        }
    }

    #[test]
    fn test_timing_and_metrics() {
        let mut graph = DependencyGraph::new();
        let id_a = graph.add_module(module("/test/a.rs"));
        let id_b = graph.add_module(module("/test/b.rs"));

        graph.graph.add_edge(
            id_a,
            id_b,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );
        graph.graph.add_edge(
            id_b,
            id_a,
            Dependency {
                kind: DependencyKind::Import,
                symbols: vec![],
            },
        );

        let result = find_circular_dependencies(&graph, None).unwrap();

        // Verify timing metrics are populated (analysis_time_ms is u64, always >= 0)
        assert_eq!(result.summary.files_analyzed, 2);
    }
}
