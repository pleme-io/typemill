//! Entry point detection and reachability analysis.

use crate::graph::CallGraph;
use crate::types::{EntryPoints, Symbol};
use petgraph::graph::NodeIndex;
use std::collections::HashSet;

/// Find all entry points in the symbol list.
pub(crate) fn find_entry_points(symbols: &[Symbol], config: &EntryPoints) -> Vec<String> {
    let mut entry_points = Vec::new();

    for symbol in symbols {
        if is_entry_point(symbol, config) {
            entry_points.push(symbol.id.clone());
        }
    }

    entry_points
}

/// Check if a symbol is an entry point.
fn is_entry_point(symbol: &Symbol, config: &EntryPoints) -> bool {
    // Check main function
    if config.include_main && is_main_function(symbol) {
        return true;
    }

    // Check test functions
    if config.include_tests && is_test_function(symbol) {
        return true;
    }

    // Check public exports (only fully `pub` symbols, not pub(crate) or pub(super))
    if config.include_pub_exports && symbol.visibility.is_api_public() {
        return true;
    }

    // Check custom patterns
    for pattern in &config.custom {
        if symbol.name.contains(pattern) || symbol.id.contains(pattern) {
            return true;
        }
    }

    false
}

/// Check if symbol is a main function.
fn is_main_function(symbol: &Symbol) -> bool {
    symbol.name == "main"
        && (symbol.file_path.ends_with("main.rs")
            || symbol.file_path.ends_with("main.ts")
            || symbol.file_path.ends_with("main.py")
            || symbol.file_path.ends_with("main.go")
            || symbol.file_path.contains("/bin/"))
}

/// Check if symbol is a test function.
fn is_test_function(symbol: &Symbol) -> bool {
    // Rust: fn test_* or #[test]
    // Python: def test_*
    // JS/TS: test(, it(, describe(
    // Go: func Test*
    let name = &symbol.name;

    name.starts_with("test_")
        || name.starts_with("Test")
        || name == "tests"
        || name.ends_with("_test")
        || symbol.file_path.contains("test")
        || symbol.file_path.contains("spec")
}

/// Perform reachability analysis from entry points.
///
/// Returns the set of symbol IDs that are reachable from any entry point.
pub(crate) fn analyze(graph: &CallGraph, entry_point_ids: &[String]) -> HashSet<String> {
    let mut reachable = HashSet::new();
    let mut worklist: Vec<NodeIndex> = Vec::new();

    // Start with all entry points
    for id in entry_point_ids {
        if let Some(node) = graph.get_node(id) {
            worklist.push(node);
        }
    }

    // BFS/DFS to find all reachable nodes
    while let Some(node) = worklist.pop() {
        let id = match graph.get_id(node) {
            Some(id) => id.clone(),
            None => continue,
        };

        // Skip if already visited
        if !reachable.insert(id) {
            continue;
        }

        // Add all neighbors to worklist
        for neighbor in graph.neighbors(node) {
            if let Some(neighbor_id) = graph.get_id(neighbor) {
                if !reachable.contains(neighbor_id) {
                    worklist.push(neighbor);
                }
            }
        }
    }

    reachable
}
