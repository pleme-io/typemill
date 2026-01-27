//! Report building from analysis results.

use crate::types::{Config, DeadCode, Kind, Location, Reason, Reference, Symbol};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Build the dead code report from analysis results.
pub(crate) fn build(
    symbols: &[Symbol],
    reachable: &HashSet<String>,
    references: &[Reference],
    config: &Config,
) -> Vec<DeadCode> {
    // Build a map of who references each symbol
    let reference_map = build_reference_map(references);

    // Build a name map for creating human-readable reasons
    let name_map: HashMap<&str, &str> = symbols
        .iter()
        .map(|s| (s.id.as_str(), s.name.as_str()))
        .collect();

    let mut dead_code = Vec::new();

    for symbol in symbols {
        // Skip if reachable
        if reachable.contains(&symbol.id) {
            continue;
        }

        // Determine confidence and reason
        let (confidence, reason) = determine_reason(symbol, &reference_map, reachable, &name_map);

        // Skip if below confidence threshold
        if confidence < config.min_confidence {
            continue;
        }

        dead_code.push(DeadCode {
            name: symbol.name.clone(),
            kind: symbol.kind,
            location: Location {
                file: PathBuf::from(&symbol.file_path),
                line: symbol.line + 1, // Convert to 1-indexed
                column: symbol.column,
            },
            confidence,
            reason,
        });
    }

    // Sort by file, then line
    dead_code.sort_by(|a, b| {
        a.location
            .file
            .cmp(&b.location.file)
            .then(a.location.line.cmp(&b.location.line))
    });

    dead_code
}

/// Build a map from symbol ID to list of IDs that reference it.
fn build_reference_map(references: &[Reference]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for reference in references {
        map.entry(reference.to_id.clone())
            .or_default()
            .push(reference.from_id.clone());
    }

    map
}

/// Determine confidence and reason for dead code.
fn determine_reason(
    symbol: &Symbol,
    reference_map: &HashMap<String, Vec<String>>,
    reachable: &HashSet<String>,
    name_map: &HashMap<&str, &str>,
) -> (f32, Reason) {
    let referrers = reference_map.get(&symbol.id);

    match referrers {
        None => {
            // No references at all
            let confidence = base_confidence(symbol.kind);
            (confidence, Reason::NoReferences)
        }
        Some(refs) if refs.is_empty() => {
            let confidence = base_confidence(symbol.kind);
            (confidence, Reason::NoReferences)
        }
        Some(refs) => {
            // Check if all references are from dead code
            let live_refs: Vec<_> = refs.iter().filter(|r| reachable.contains(*r)).collect();

            if live_refs.is_empty() {
                // Only referenced by dead code
                let dead_names: Vec<String> = refs
                    .iter()
                    .filter_map(|r| name_map.get(r.as_str()).map(|n| (*n).to_string()))
                    .collect();

                // Slightly lower confidence for transitive dead code
                let confidence = base_confidence(symbol.kind) * 0.9;
                (
                    confidence,
                    Reason::OnlyDeadReferences { from: dead_names },
                )
            } else {
                // Has live references but still unreachable?
                // This shouldn't happen often, but treat as low confidence
                let confidence = base_confidence(symbol.kind) * 0.5;
                (confidence, Reason::UnreachableFromEntryPoints)
            }
        }
    }
}

/// Get base confidence for a symbol kind.
fn base_confidence(kind: Kind) -> f32 {
    match kind {
        // High confidence for functions - they either get called or they don't
        Kind::Function | Kind::Method => 0.95,

        // Good confidence for types - they're either used or not
        Kind::Struct | Kind::Enum | Kind::Class | Kind::Interface | Kind::Trait => 0.90,

        // Medium confidence for constants - might be used in ways we don't see
        Kind::Const | Kind::Static => 0.85,

        // Lower confidence for modules/imports - complex semantics
        Kind::Module | Kind::Import => 0.75,

        // Type aliases might be used via traits or generics
        Kind::TypeAlias => 0.80,

        // Variables are often just local - lower confidence
        Kind::Variable => 0.70,

        // Impl blocks are complex
        Kind::Impl => 0.65,

        // Unknown - be conservative
        Kind::Unknown => 0.60,
    }
}
