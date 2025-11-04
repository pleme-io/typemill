#![allow(
    dead_code,
    unused_variables,
    clippy::mutable_key_type,
    clippy::needless_range_loop,
    clippy::ptr_arg,
    clippy::manual_clamp
)]

use super::*;
use anyhow::Result;

/// Generates actionable suggestions from analysis findings
pub struct SuggestionGenerator {
    classifier: SafetyClassifier,
    scorer: ConfidenceScorer,
    ranker: SuggestionRanker,
    config: SuggestionConfig,
}

impl SuggestionGenerator {
    pub fn new() -> Self {
        Self::with_config(SuggestionConfig::default())
    }

    pub fn with_config(config: SuggestionConfig) -> Self {
        Self {
            classifier: SafetyClassifier::new(),
            scorer: ConfidenceScorer::new(),
            ranker: SuggestionRanker::new(),
            config,
        }
    }

    /// Generate suggestions for a refactoring candidate
    pub fn generate_from_candidate(
        &self,
        candidate: RefactoringCandidate,
        context: &AnalysisContext,
    ) -> Result<ActionableSuggestion> {
        // 1. Classify safety
        let safety = self.classifier.classify(&candidate, context)?;

        // 2. Calculate confidence
        let confidence = self.scorer.score(&candidate, context)?;

        // 3. Determine reversibility
        let reversible = self.is_reversible(&candidate);

        // 4. Estimate impact
        let estimated_impact = self.estimate_impact(&candidate);

        // 5. Generate refactor_call
        let refactor_call = self.build_refactor_call(&candidate)?;

        // 6. Generate metadata
        let metadata = self.build_metadata(&candidate);

        Ok(ActionableSuggestion {
            message: candidate.message,
            safety,
            confidence,
            reversible,
            estimated_impact,
            refactor_call: Some(refactor_call),
            metadata: Some(metadata),
        })
    }

    fn is_reversible(&self, candidate: &RefactoringCandidate) -> bool {
        // Most refactorings are reversible with version control
        // Only destructive operations are not
        !matches!(candidate.refactor_type, RefactorType::RemoveDeadCode)
            || candidate.reference_count == Some(0)
    }

    fn estimate_impact(&self, candidate: &RefactoringCandidate) -> ImpactLevel {
        match candidate.scope {
            Scope::Local => ImpactLevel::Low,
            Scope::Function => ImpactLevel::Medium,
            Scope::File => ImpactLevel::High,
            Scope::CrossFile | Scope::CrossCrate => ImpactLevel::Critical,
        }
    }

    fn build_refactor_call(&self, candidate: &RefactoringCandidate) -> Result<RefactorCall> {
        let tool = match candidate.refactor_type {
            RefactorType::ExtractMethod => "extract",
            RefactorType::Inline => "inline",
            RefactorType::Move => "move",
            RefactorType::Rename => "rename",
            RefactorType::Transform | RefactorType::SimplifyBooleanExpression => "transform",
            RefactorType::RemoveUnusedImport
            | RefactorType::RemoveUnusedVariable
            | RefactorType::RemoveDeadCode => "delete",
        };

        Ok(RefactorCall {
            tool: tool.to_string(),
            arguments: candidate.refactor_call_args.clone(),
        })
    }

    fn build_metadata(&self, candidate: &RefactoringCandidate) -> SuggestionMetadata {
        let mut metadata = SuggestionMetadata {
            rationale: format!("Refactoring type: {:?}", candidate.refactor_type),
            risks: Vec::new(),
            benefits: Vec::new(),
        };

        // Add type-specific risks/benefits
        match candidate.refactor_type {
            RefactorType::RemoveUnusedImport | RefactorType::RemoveUnusedVariable => {
                metadata.benefits.push("Reduces code clutter".to_string());
                metadata
                    .benefits
                    .push("Improves maintainability".to_string());
            }
            RefactorType::ExtractMethod => {
                metadata
                    .benefits
                    .push("Improves code organization".to_string());
                metadata.benefits.push("Enables code reuse".to_string());
                metadata
                    .risks
                    .push("May need to pass additional context".to_string());
            }
            _ => {}
        }

        metadata
    }

    /// Generate multiple suggestions and apply config filters
    pub fn generate_multiple(
        &self,
        candidates: Vec<RefactoringCandidate>,
        context: &AnalysisContext,
    ) -> Vec<ActionableSuggestion> {
        let mut suggestions = Vec::new();

        for candidate in candidates {
            match self.generate_from_candidate(candidate, context) {
                Ok(suggestion) => suggestions.push(suggestion),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to generate suggestion");
                }
            }
        }

        // Apply config filters
        suggestions
            .into_iter()
            .filter(|s| s.confidence >= self.config.min_confidence)
            .filter(|s| self.config.include_safety_levels.contains(&s.safety))
            .filter(|s| {
                self.config.filters.allowed_impact_levels.is_empty()
                    || self
                        .config
                        .filters
                        .allowed_impact_levels
                        .contains(&s.estimated_impact)
            })
            .take(self.config.max_per_finding)
            .collect()
    }
}
