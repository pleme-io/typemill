//! Dependency analysis for pre-consolidation validation
//!
//! This module provides functionality to detect circular dependencies
//! that would be created by consolidating one crate into another.

use super::FileService;
use codebuddy_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

/// Result of circular dependency analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependencyAnalysis {
    /// Whether consolidation would create a circular dependency
    pub has_circular_dependency: bool,

    /// Source crate being consolidated
    pub source_crate: String,

    /// Target crate receiving the consolidation
    pub target_crate: String,

    /// The dependency chain that creates the cycle
    /// Example: ["cb-plugin-api", "codebuddy-foundation", "cb-plugin-api"]
    pub dependency_chain: Vec<String>,

    /// Modules in source crate that cause the circular dependency
    pub problematic_modules: Vec<ProblematicModule>,
}

/// A module that would create a circular dependency if moved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblematicModule {
    /// File path relative to source crate (e.g., "src/language.rs")
    pub file_path: String,

    /// The crate this module imports that creates the cycle
    pub imports_crate: String,

    /// Specific imports from the problematic crate
    pub imports: Vec<String>,
}

/// Dependency graph for workspace crates
pub struct DependencyGraph {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a crate to the graph
    fn add_crate(&mut self, crate_name: String) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(&crate_name) {
            return idx;
        }

        let idx = self.graph.add_node(crate_name.clone());
        self.node_map.insert(crate_name, idx);
        idx
    }

    /// Add a dependency edge: from depends on to
    fn add_dependency(&mut self, from: &str, to: &str) {
        let from_idx = self.add_crate(from.to_string());
        let to_idx = self.add_crate(to.to_string());
        self.graph.add_edge(from_idx, to_idx, ());
    }

    /// Check if there's a path from `from` to `to`
    fn has_path(&self, from: &str, to: &str) -> bool {
        let Some(&from_idx) = self.node_map.get(from) else {
            return false;
        };
        let Some(&to_idx) = self.node_map.get(to) else {
            return false;
        };

        let mut dfs = Dfs::new(&self.graph, from_idx);
        while let Some(node) = dfs.next(&self.graph) {
            if node == to_idx {
                return true;
            }
        }

        false
    }

    /// Find the shortest path from `from` to `to`
    fn find_path(&self, from: &str, to: &str) -> Vec<String> {
        let Some(&from_idx) = self.node_map.get(from) else {
            return vec![];
        };
        let Some(&to_idx) = self.node_map.get(to) else {
            return vec![];
        };

        // Use BFS to find shortest path
        use petgraph::algo::astar;

        if let Some((_, path)) = astar(
            &self.graph,
            from_idx,
            |finish| finish == to_idx,
            |_| 1,
            |_| 0,
        ) {
            path.iter()
                .map(|&idx| self.graph[idx].clone())
                .collect()
        } else {
            vec![]
        }
    }

    /// Check if a crate is involved in a cycle with another crate
    fn is_in_cycle_with(&self, crate1: &str, crate2: &str) -> bool {
        self.has_path(crate1, crate2) && self.has_path(crate2, crate1)
    }
}

impl FileService {
    /// Validate that consolidation won't create circular dependencies
    ///
    /// This function analyzes the workspace dependency graph and checks if
    /// consolidating the source crate into the target crate would create
    /// a circular dependency.
    ///
    /// # Arguments
    ///
    /// * `source_crate_path` - Path to the source crate directory
    /// * `target_crate_path` - Path to the target crate directory
    ///
    /// # Returns
    ///
    /// Analysis results including whether a cycle would be created and
    /// which modules are problematic.
    pub async fn validate_no_circular_dependencies(
        &self,
        source_crate_path: &Path,
        target_crate_path: &Path,
    ) -> ServerResult<CircularDependencyAnalysis> {
        info!(
            source = %source_crate_path.display(),
            target = %target_crate_path.display(),
            "Validating consolidation for circular dependencies"
        );

        // 1. Get crate names from Cargo.toml files
        let source_crate_name = self.get_crate_name(source_crate_path).await?;
        let target_crate_name = self.get_crate_name(target_crate_path).await?;

        debug!(
            source_crate = %source_crate_name,
            target_crate = %target_crate_name,
            "Extracted crate names"
        );

        // 2. Build workspace dependency graph
        let dep_graph = self.build_workspace_dependency_graph().await?;

        // 3. Check if target depends on source (which would create a cycle)
        let would_create_cycle = dep_graph.has_path(&target_crate_name, &source_crate_name);

        if !would_create_cycle {
            info!(
                source_crate = %source_crate_name,
                target_crate = %target_crate_name,
                "No circular dependency detected"
            );

            return Ok(CircularDependencyAnalysis {
                has_circular_dependency: false,
                source_crate: source_crate_name,
                target_crate: target_crate_name,
                dependency_chain: vec![],
                problematic_modules: vec![],
            });
        }

        // 4. Cycle detected - find the dependency chain
        let dependency_chain = dep_graph.find_path(&target_crate_name, &source_crate_name);

        warn!(
            source_crate = %source_crate_name,
            target_crate = %target_crate_name,
            chain = ?dependency_chain,
            "Circular dependency detected"
        );

        // 5. Find problematic modules in source crate
        let problematic_modules = self
            .find_problematic_modules(source_crate_path, &source_crate_name, &dependency_chain)
            .await?;

        warn!(
            problematic_count = problematic_modules.len(),
            "Found problematic modules"
        );

        Ok(CircularDependencyAnalysis {
            has_circular_dependency: true,
            source_crate: source_crate_name,
            target_crate: target_crate_name,
            dependency_chain,
            problematic_modules,
        })
    }

    /// Build a dependency graph for the entire workspace
    async fn build_workspace_dependency_graph(&self) -> ServerResult<DependencyGraph> {
        debug!(workspace_root = %self.project_root.display(), "Building workspace dependency graph");

        // Use cargo metadata to get accurate dependency information
        let metadata = cargo_metadata::MetadataCommand::new()
            .current_dir(&self.project_root)
            .exec()
            .map_err(|e| {
                ServerError::Internal(format!("Failed to run cargo metadata: {}", e))
            })?;

        let mut graph = DependencyGraph::new();

        // Add all workspace members and their dependencies
        for package in &metadata.workspace_packages() {
            let package_name = package.name.clone();

            // Add dependencies
            for dependency in &package.dependencies {
                // Only track workspace dependencies
                if let Some(_) = metadata
                    .workspace_packages()
                    .iter()
                    .find(|p| p.name == dependency.name)
                {
                    graph.add_dependency(&package_name, &dependency.name);
                    debug!(
                        from = %package_name,
                        to = %dependency.name,
                        "Added dependency edge"
                    );
                }
            }
        }

        info!(
            crates = metadata.workspace_packages().len(),
            "Built workspace dependency graph"
        );

        Ok(graph)
    }

    /// Find modules in source crate that import crates in the dependency chain
    async fn find_problematic_modules(
        &self,
        source_crate_path: &Path,
        source_crate_name: &str,
        dependency_chain: &[String],
    ) -> ServerResult<Vec<ProblematicModule>> {
        debug!(
            source_crate = %source_crate_name,
            chain = ?dependency_chain,
            "Finding problematic modules"
        );

        let mut problematic = Vec::new();
        let src_dir = source_crate_path.join("src");

        if !src_dir.exists() {
            return Ok(problematic);
        }

        // Walk all .rs files in source crate
        use ignore::WalkBuilder;
        let walker = WalkBuilder::new(&src_dir)
            .hidden(false)
            .git_ignore(false)
            .build();

        for entry in walker {
            let entry = entry.map_err(|e| {
                ServerError::Internal(format!("Failed to walk directory: {}", e))
            })?;

            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }

            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            // Read file content
            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

            // Extract imports
            let imports = self.extract_rust_imports(&content);

            // Check if any import is to a crate in the dependency chain
            for import in imports {
                // Convert "use foo_bar::" to "foo-bar"
                let imported_crate_ident = import
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .trim_start_matches("use ")
                    .trim();

                let imported_crate = imported_crate_ident.replace('_', "-");

                // Check if this crate is in the dependency chain
                if dependency_chain.contains(&imported_crate) && imported_crate != source_crate_name {
                    let relative_path = path
                        .strip_prefix(source_crate_path)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| path.display().to_string());

                    // Check if we already have this file
                    if let Some(existing) = problematic
                        .iter_mut()
                        .find(|m| m.file_path == relative_path && m.imports_crate == imported_crate)
                    {
                        existing.imports.push(import.clone());
                    } else {
                        problematic.push(ProblematicModule {
                            file_path: relative_path,
                            imports_crate: imported_crate.clone(),
                            imports: vec![import.clone()],
                        });
                    }

                    debug!(
                        file = %path.display(),
                        imports_crate = %imported_crate,
                        import = %import,
                        "Found problematic import"
                    );
                }
            }
        }

        Ok(problematic)
    }

    /// Extract Rust imports from source code
    ///
    /// Returns a list of import statements (e.g., "use foo::bar;")
    fn extract_rust_imports(&self, content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Match "use foo::bar" or "pub use foo::bar"
            if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
                // Extract the import path (everything between "use" and ";")
                if let Some(import_part) = trimmed.strip_prefix("pub use ").or_else(|| trimmed.strip_prefix("use ")) {
                    if let Some(import_end) = import_part.find(';') {
                        let import_path = import_part[..import_end].trim();
                        imports.push(import_path.to_string());
                    }
                }
            }
        }

        imports
    }

    /// Get the crate name from a Cargo.toml file
    async fn get_crate_name(&self, crate_path: &Path) -> ServerResult<String> {
        let cargo_toml = crate_path.join("Cargo.toml");

        let content = tokio::fs::read_to_string(&cargo_toml).await.map_err(|e| {
            ServerError::Internal(format!("Failed to read Cargo.toml: {}", e))
        })?;

        let doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| {
            ServerError::Internal(format!("Failed to parse Cargo.toml: {}", e))
        })?;

        let name = doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .ok_or_else(|| {
                ServerError::Internal("Cargo.toml missing package.name".to_string())
            })?;

        Ok(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_path_detection() {
        let mut graph = DependencyGraph::new();

        // Create a dependency chain: A -> B -> C
        graph.add_dependency("crate-a", "crate-b");
        graph.add_dependency("crate-b", "crate-c");

        // Test path detection
        assert!(graph.has_path("crate-a", "crate-b"));
        assert!(graph.has_path("crate-a", "crate-c"));
        assert!(graph.has_path("crate-b", "crate-c"));

        // Test no reverse path
        assert!(!graph.has_path("crate-c", "crate-a"));
        assert!(!graph.has_path("crate-b", "crate-a"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();

        // Create a cycle: A -> B -> C -> A
        graph.add_dependency("crate-a", "crate-b");
        graph.add_dependency("crate-b", "crate-c");
        graph.add_dependency("crate-c", "crate-a");

        // Test cycle detection
        assert!(graph.is_in_cycle_with("crate-a", "crate-b"));
        assert!(graph.is_in_cycle_with("crate-b", "crate-c"));
        assert!(graph.is_in_cycle_with("crate-c", "crate-a"));
    }

    #[test]
    fn test_find_path() {
        let mut graph = DependencyGraph::new();

        graph.add_dependency("crate-a", "crate-b");
        graph.add_dependency("crate-b", "crate-c");

        let path = graph.find_path("crate-a", "crate-c");
        assert_eq!(path, vec!["crate-a", "crate-b", "crate-c"]);
    }

    #[test]
    fn test_extract_rust_imports() {
        let service = FileService {
            project_root: std::path::PathBuf::from("/test"),
        };

        let content = r#"
            use std::path::Path;
            pub use crate::foo::Bar;
            use cb_plugin_api::iter_plugins;
            use codebuddy_core::utils;
        "#;

        let imports = service.extract_rust_imports(content);

        assert_eq!(imports.len(), 4);
        assert!(imports.contains(&"std::path::Path".to_string()));
        assert!(imports.contains(&"crate::foo::Bar".to_string()));
        assert!(imports.contains(&"cb_plugin_api::iter_plugins".to_string()));
        assert!(imports.contains(&"codebuddy_core::utils".to_string()));
    }
}
