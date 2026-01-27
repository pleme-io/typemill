//! Public types for dead code analysis.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for dead code analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Entry points that are considered "alive" roots.
    pub entry_points: EntryPoints,

    /// Minimum confidence to report (0.0 - 1.0).
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,

    /// File extensions to analyze (e.g., ["rs", "ts"]).
    /// If None, uses defaults based on workspace detection.
    pub file_extensions: Option<Vec<String>>,

    /// Maximum number of symbols to analyze (for large codebases).
    pub max_symbols: Option<usize>,
}

fn default_min_confidence() -> f32 {
    0.7
}

impl Default for Config {
    fn default() -> Self {
        Self {
            entry_points: EntryPoints::default(),
            min_confidence: default_min_confidence(),
            file_extensions: None,
            max_symbols: None,
        }
    }
}

/// Defines what counts as an entry point (roots for reachability).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoints {
    /// Include `fn main()` as entry point.
    #[serde(default = "default_true")]
    pub include_main: bool,

    /// Include test functions (`#[test]`, `test_*`, etc.).
    #[serde(default = "default_true")]
    pub include_tests: bool,

    /// Include public exports from lib.rs / mod.rs.
    #[serde(default = "default_true")]
    pub include_pub_exports: bool,

    /// Additional entry point patterns (symbol names).
    #[serde(default)]
    pub custom: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for EntryPoints {
    fn default() -> Self {
        Self {
            include_main: true,
            include_tests: true,
            include_pub_exports: true,
            custom: vec![],
        }
    }
}

/// The analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// All dead code found.
    pub dead_code: Vec<DeadCode>,

    /// Analysis statistics.
    pub stats: Stats,
}

/// A single piece of dead code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCode {
    /// The symbol name.
    pub name: String,

    /// The kind of symbol.
    pub kind: Kind,

    /// Where it's located.
    pub location: Location,

    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,

    /// Why it's considered dead.
    pub reason: Reason,
}

/// The kind of dead code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    Static,
    TypeAlias,
    Import,
    Module,
    Interface,
    Class,
    Variable,
    Unknown,
}

impl Kind {
    /// Convert from LSP SymbolKind number.
    pub fn from_lsp(kind: u64) -> Self {
        match kind {
            1 => Kind::Module,      // File
            2 => Kind::Module,      // Module
            3 => Kind::Module,      // Namespace
            4 => Kind::Module,      // Package
            5 => Kind::Class,       // Class
            6 => Kind::Method,      // Method
            7 => Kind::Variable,    // Property
            8 => Kind::Variable,    // Field
            9 => Kind::Function,    // Constructor
            10 => Kind::Enum,       // Enum
            11 => Kind::Interface,  // Interface
            12 => Kind::Function,   // Function
            13 => Kind::Variable,   // Variable
            14 => Kind::Const,      // Constant
            15 => Kind::Variable,   // String
            16 => Kind::Variable,   // Number
            17 => Kind::Variable,   // Boolean
            18 => Kind::Variable,   // Array
            19 => Kind::Variable,   // Object
            20 => Kind::Variable,   // Key
            21 => Kind::Variable,   // Null
            22 => Kind::Variable,   // EnumMember
            23 => Kind::Struct,     // Struct
            24 => Kind::Variable,   // Event
            25 => Kind::Function,   // Operator
            26 => Kind::TypeAlias,  // TypeParameter
            _ => Kind::Unknown,
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Function => write!(f, "function"),
            Kind::Method => write!(f, "method"),
            Kind::Struct => write!(f, "struct"),
            Kind::Enum => write!(f, "enum"),
            Kind::Trait => write!(f, "trait"),
            Kind::Impl => write!(f, "impl"),
            Kind::Const => write!(f, "const"),
            Kind::Static => write!(f, "static"),
            Kind::TypeAlias => write!(f, "type"),
            Kind::Import => write!(f, "import"),
            Kind::Module => write!(f, "module"),
            Kind::Interface => write!(f, "interface"),
            Kind::Class => write!(f, "class"),
            Kind::Variable => write!(f, "variable"),
            Kind::Unknown => write!(f, "symbol"),
        }
    }
}

/// Why the code is considered dead.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Reason {
    /// No references found anywhere.
    NoReferences,

    /// Only referenced by other dead code.
    OnlyDeadReferences {
        /// Names of the dead code that references this.
        from: Vec<String>,
    },

    /// Cannot reach from any entry point.
    UnreachableFromEntryPoints,
}

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reason::NoReferences => write!(f, "no references found"),
            Reason::OnlyDeadReferences { from } => {
                write!(f, "only referenced by dead code: {}", from.join(", "))
            }
            Reason::UnreachableFromEntryPoints => {
                write!(f, "unreachable from entry points")
            }
        }
    }
}

/// Location of dead code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// File path.
    pub file: PathBuf,

    /// Line number (1-indexed).
    pub line: u32,

    /// Column number (0-indexed).
    pub column: u32,
}

/// Analysis statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    /// Number of files analyzed.
    pub files_analyzed: usize,

    /// Number of symbols analyzed.
    pub symbols_analyzed: usize,

    /// Number of dead code items found.
    pub dead_found: usize,

    /// Analysis duration in milliseconds.
    pub duration_ms: u64,
}

/// Visibility level of a symbol.
///
/// This enum captures the full range of Rust visibility modifiers.
/// Currently only `Public` and `Private` are used for determining entry points,
/// but the other variants are prepared for future crate-level analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants prepared for future crate-level visibility analysis
pub(crate) enum SymbolVisibility {
    /// Fully public (`pub`)
    Public,
    /// Public within crate (`pub(crate)`)
    Crate,
    /// Public to parent module (`pub(super)`)
    Super,
    /// Public to a specific path (`pub(in path)`)
    Restricted,
    /// Private (no visibility modifier)
    Private,
}

impl SymbolVisibility {
    /// Returns true if this is an external API symbol (fully public).
    pub fn is_api_public(&self) -> bool {
        matches!(self, SymbolVisibility::Public)
    }
}

/// Internal symbol representation used during analysis.
#[derive(Debug, Clone)]
pub(crate) struct Symbol {
    /// Unique identifier (e.g., "file.rs::MyStruct::my_function").
    pub id: String,

    /// The symbol name.
    pub name: String,

    /// The kind of symbol.
    pub kind: Kind,

    /// File path (relative to workspace).
    pub file_path: String,

    /// File URI for LSP queries.
    pub uri: String,

    /// Start line number (0-indexed for LSP).
    pub line: u32,

    /// Start column number (0-indexed).
    pub column: u32,

    /// End line number (0-indexed for LSP).
    pub end_line: u32,

    /// End column number (0-indexed).
    pub end_column: u32,

    /// Visibility level of this symbol.
    pub visibility: SymbolVisibility,
}

/// A reference from one symbol to another.
#[derive(Debug, Clone)]
pub(crate) struct Reference {
    /// The symbol that contains the reference.
    pub from_id: String,

    /// The symbol being referenced.
    pub to_id: String,
}
