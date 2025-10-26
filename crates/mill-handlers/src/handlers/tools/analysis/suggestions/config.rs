use super::types::{ImpactLevel, SafetyLevel};
use anyhow::Result;
use mill_foundation::protocol::analysis_result::Suggestion;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionConfig {
    /// Minimum confidence threshold
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,

    /// Safety levels to include
    #[serde(default = "default_safety_levels")]
    pub include_safety_levels: HashSet<SafetyLevel>,

    /// Maximum suggestions per finding
    #[serde(default = "default_max_per_finding")]
    pub max_per_finding: usize,

    /// Generate refactor_call for suggestions
    #[serde(default = "default_generate_refactor_calls")]
    pub generate_refactor_calls: bool,

    /// Filters
    #[serde(default)]
    pub filters: SuggestionFilters,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestionFilters {
    /// Exclude these refactoring types
    #[serde(default)]
    pub exclude_refactor_types: HashSet<String>,

    /// Only include these impact levels
    #[serde(default)]
    pub allowed_impact_levels: HashSet<ImpactLevel>,

    /// Exclude files matching these patterns
    #[serde(default)]
    pub exclude_files: Vec<String>,
}

fn default_min_confidence() -> f64 {
    0.7
}

fn default_safety_levels() -> HashSet<SafetyLevel> {
    [SafetyLevel::Safe, SafetyLevel::RequiresReview]
        .iter()
        .copied()
        .collect()
}

fn default_max_per_finding() -> usize {
    3
}

fn default_generate_refactor_calls() -> bool {
    true
}

impl Default for SuggestionConfig {
    fn default() -> Self {
        Self {
            min_confidence: default_min_confidence(),
            include_safety_levels: default_safety_levels(),
            max_per_finding: default_max_per_finding(),
            generate_refactor_calls: default_generate_refactor_calls(),
            filters: SuggestionFilters::default(),
        }
    }
}

impl SuggestionConfig {
    /// Load from .typemill/analysis.toml
    pub fn load() -> Result<Self> {
        let config_path = std::path::Path::new(".typemill/analysis.toml");
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_str = std::fs::read_to_string(config_path)?;
        let config: toml::Value = toml::from_str(&config_str)?;

        let suggestions_config = config
            .get("suggestions")
            .and_then(|v| v.as_table())
            .ok_or_else(|| anyhow::anyhow!("Missing [suggestions] section"))?;

        let parsed: SuggestionConfig = toml::from_str(&toml::to_string(suggestions_config)?)?;

        Ok(parsed)
    }

    /// Apply filters to suggestions
    pub fn filter(&self, suggestions: Vec<Suggestion>) -> Vec<Suggestion> {
        suggestions
            .into_iter()
            .filter(|s| s.confidence >= self.min_confidence)
            .filter(|s| {
                let suggestion_safety_level: SafetyLevel = s.safety.into();
                self.include_safety_levels
                    .contains(&suggestion_safety_level)
            })
            .filter(|s| {
                if self.filters.allowed_impact_levels.is_empty() {
                    return true;
                }
                // Check if the string contains any of the allowed levels
                let impact_str = s.estimated_impact.to_lowercase();
                self.filters.allowed_impact_levels.iter().any(|level| {
                    let level_str = format!("{:?}", level).to_lowercase();
                    impact_str.contains(&level_str)
                })
            })
            .filter(|s| {
                self.filters
                    .exclude_refactor_types
                    .iter()
                    .all(|excluded| !s.action.contains(excluded))
            })
            .collect()
    }
}

// Convert from mill_foundation::protocol::analysis_result::SafetyLevel to our local SafetyLevel
impl From<mill_foundation::protocol::analysis_result::SafetyLevel> for SafetyLevel {
    fn from(protocol_level: mill_foundation::protocol::analysis_result::SafetyLevel) -> Self {
        match protocol_level {
            mill_foundation::protocol::analysis_result::SafetyLevel::Safe => SafetyLevel::Safe,
            mill_foundation::protocol::analysis_result::SafetyLevel::RequiresReview => {
                SafetyLevel::RequiresReview
            }
            mill_foundation::protocol::analysis_result::SafetyLevel::Experimental => {
                SafetyLevel::Experimental
            }
        }
    }
}
