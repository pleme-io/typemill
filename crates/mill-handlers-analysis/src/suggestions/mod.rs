pub mod builder;
pub mod classifier;
pub mod config;
pub mod generator;
pub mod ranker;
pub mod scorer;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

// Re-export suggestion types
pub use self::classifier::SafetyClassifier;
pub use self::config::{SuggestionConfig, SuggestionFilters};
pub use self::generator::SuggestionGenerator;
pub use self::ranker::SuggestionRanker;
pub use self::scorer::ConfidenceScorer;
pub use self::types::{
    ActionableSuggestion, AnalysisContext, EvidenceStrength, ImpactLevel, Location, RefactorCall,
    RefactorType, RefactoringCandidate, SafetyLevel, Scope, SuggestionMetadata,
};
// validation module has no public items
