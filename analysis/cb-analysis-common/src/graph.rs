// analysis/cb-analysis-common/src/graph.rs

//! A generic dependency graph for symbol analysis.

use lsp_types::{Range, SymbolKind as LspSymbolKind};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Describes how a symbol is used.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UsageContext {
    TypeAnnotation,
    GenericParameter,
    Implements,
    Extends,
    Import,
    FunctionCall,
    /// The context could not be determined.
    Unknown,
}

/// A detailed categorization of a symbol.
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    // High-level categories from the proposal
    Type,
    Interface,
    Trait,
    Constant,
    // AST-based categories
    Struct,
    Enum,
    Function,
    Module,
    TypeAlias,
    // LSP kinds for more specific details
    Lsp(LspSymbolKind),
    // Fallback for unknown kinds
    Unknown,
}

impl Hash for SymbolKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        if let SymbolKind::Lsp(lsp_kind) = self {
            // LspSymbolKind doesn't implement Hash, so we serialize it to get its numeric value.
            if let Ok(value) = serde_json::to_value(lsp_kind) {
                if let Some(num) = value.as_u64() {
                    (num as u8).hash(state);
                }
            }
        }
    }
}

impl From<LspSymbolKind> for SymbolKind {
    fn from(lsp_kind: LspSymbolKind) -> Self {
        match lsp_kind {
            LspSymbolKind::CLASS | LspSymbolKind::STRUCT | LspSymbolKind::ENUM | LspSymbolKind::TYPE_PARAMETER => Self::Type,
            LspSymbolKind::INTERFACE => Self::Interface,
            LspSymbolKind::CONSTANT => Self::Constant,
            other => Self::Lsp(other),
        }
    }
}

/// Represents a node in the symbol dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub id: String,   // A unique identifier, e.g., "file.rs::MyStruct::my_function"
    pub name: String, // The symbol name, e.g., "my_function"
    pub kind: SymbolKind, // The detailed kind of the symbol
    pub file_path: String,
    pub is_public: bool, // Is the symbol exported or part of a public API?
    #[serde(skip, default = "default_range")]
    pub range: Range, // The location in the file, crucial for find_references
}

fn default_range() -> Range {
    Range::default()
}

impl PartialEq for SymbolNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SymbolNode {}

impl Hash for SymbolNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// The dependency graph, mapping symbol relationships.
pub struct DependencyGraph {
    pub graph: DiGraph<SymbolNode, UsageContext>,
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
    pub fn add_dependency(&mut self, from_id: &str, to_id: &str, context: UsageContext) {
        if let (Some(&from_index), Some(&to_index)) =
            (self.node_map.get(from_id), self.node_map.get(to_id))
        {
            self.graph.add_edge(from_index, to_index, context);
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
