# Proposal 01c1: Actionable Suggestions - Core Infrastructure

**Status**: ❌ **NOT STARTED**
**Author**: Project Team
**Date**: 2025-10-13 (Split from 01c)
**Parent Proposal**: [01b_unified_analysis_api.md](01b_unified_analysis_api.md)
**Dependencies**: ✅ 01a (Refactoring API COMPLETE), ✅ 01b (Analysis Core COMPLETE)
**Branch**: `feature/01c1-suggestions-core`
**Estimated Effort**: 1 week (5-7 days, ~40 hours)

---

## Executive Summary

**What**: Build core infrastructure for generating actionable suggestions with safety metadata, confidence scoring, and refactor_call generation.

**Why**: This is the foundation for all suggestion generation. Without it, analysis categories cannot produce actionable suggestions.

**Impact**: Enables subsequent branches (01c2, 01c3) to integrate suggestion generation into analysis handlers.

**Critical Path**: This MUST be completed before branches 01c2 and 01c3 can start.

---

## Scope - What This Branch Delivers

### Core Data Structures ✅
- `ActionableSuggestion` struct with all fields
- `SafetyLevel` enum (safe/requires_review/experimental)
- `ImpactLevel` enum (low/medium/high/critical)
- `RefactorCall` struct
- `SuggestionMetadata` struct
- `RefactoringCandidate` internal struct

### Core Services ✅
- `SuggestionGenerator` - Main orchestration service
- `SafetyClassifier` - Rule-based safety classification
- `ConfidenceScorer` - Multi-factor confidence scoring
- `SuggestionRanker` - Safety → confidence → impact ranking

### Utilities ✅
- Confidence serialization helper
- Refactor call builder skeleton
- Validation utilities

### Testing ✅
- Unit tests for each component (>80% coverage)
- Test harness for validator
- Mock refactoring candidates

---

## Out of Scope - What This Branch Does NOT Deliver

❌ Integration with analysis handlers (quality, dead_code, etc.) - That's 01c2
❌ Configuration loading from `.codebuddy/analysis.toml` - That's 01c3
❌ `analyze.batch` integration - That's 01c3
❌ CI validation checks - That's 01c3
❌ Category-specific refactoring generators (complexity, dead code, etc.) - That's 01c2
❌ Documentation updates - That's 01c3

---

## Implementation Details

### File Structure

```
crates/cb-handlers/src/handlers/tools/analysis/suggestions/
├── mod.rs              # Module exports
├── types.rs            # Data structures (ActionableSuggestion, SafetyLevel, etc.)
├── generator.rs        # SuggestionGenerator orchestrator
├── classifier.rs       # SafetyClassifier
├── scorer.rs           # ConfidenceScorer
├── ranker.rs           # SuggestionRanker
├── builder.rs          # RefactorCall builder utilities
└── tests.rs            # Unit tests
```

### Core Data Structures

#### types.rs

```rust
use serde::{Deserialize, Serialize};

/// Enhanced suggestion with safety metadata and actionable refactor call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableSuggestion {
    /// Human-readable suggestion message
    pub message: String,

    /// Safety classification
    pub safety: SafetyLevel,

    /// Confidence score (0.0 to 1.0)
    #[serde(serialize_with = "serialize_confidence")]
    pub confidence: f64,

    /// Can this refactoring be undone?
    pub reversible: bool,

    /// Estimated impact of applying this suggestion
    pub estimated_impact: ImpactLevel,

    /// Exact refactoring command to execute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refactor_call: Option<RefactorCall>,

    /// Additional context for decision-making
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SuggestionMetadata>,
}

/// Safety level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    /// Safe to auto-apply without human review
    Safe,
    /// Requires human review before applying
    RequiresReview,
    /// Experimental - may not work in all cases
    Experimental,
}

/// Impact level of suggested change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ImpactLevel {
    Low,    // Single line, local scope
    Medium, // Multiple lines, function scope
    High,   // Cross-function, file scope
    Critical, // Cross-file, module scope
}

/// Refactoring command reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorCall {
    /// Tool name (e.g., "extract.plan", "inline.plan")
    pub tool: String,

    /// Arguments to pass to the tool
    pub arguments: serde_json::Value,
}

/// Additional metadata for suggestion evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionMetadata {
    /// Why this suggestion was made
    pub rationale: String,

    /// Potential risks or edge cases
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub risks: Vec<String>,

    /// Expected benefits
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub benefits: Vec<String>,
}

/// Serialize confidence with 2 decimal places
fn serialize_confidence<S>(confidence: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64((*confidence * 100.0).round() / 100.0)
}

/// Internal structure for refactoring candidates (used during generation)
#[derive(Debug, Clone)]
pub struct RefactoringCandidate {
    pub refactor_type: RefactorType,
    pub message: String,
    pub scope: Scope,
    pub has_side_effects: bool,
    pub reference_count: Option<usize>,
    pub is_unreachable: bool,
    pub is_recursive: bool,
    pub involves_generics: bool,
    pub involves_macros: bool,
    pub evidence_strength: EvidenceStrength,
    pub location: Location,
    pub refactor_call_args: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefactorType {
    RemoveUnusedImport,
    RemoveUnusedVariable,
    RemoveDeadCode,
    SimplifyBooleanExpression,
    ExtractMethod,
    Inline,
    Move,
    Rename,
    Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Local,
    Function,
    File,
    CrossFile,
    CrossCrate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStrength {
    Weak,   // Pattern matching only
    Medium, // AST shows no references
    Strong, // LSP confirms unused
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub character: usize,
}

/// Analysis context for suggestion generation
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    pub file_path: String,
    pub has_full_type_info: bool,
    pub has_partial_type_info: bool,
    pub ast_parse_errors: usize,
}
```

#### generator.rs

```rust
use super::*;
use anyhow::Result;

/// Generates actionable suggestions from analysis findings
pub struct SuggestionGenerator {
    classifier: SafetyClassifier,
    scorer: ConfidenceScorer,
    ranker: SuggestionRanker,
}

impl SuggestionGenerator {
    pub fn new() -> Self {
        Self {
            classifier: SafetyClassifier::new(),
            scorer: ConfidenceScorer::new(),
            ranker: SuggestionRanker::new(),
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
            RefactorType::ExtractMethod => "extract.plan",
            RefactorType::Inline => "inline.plan",
            RefactorType::Move => "move.plan",
            RefactorType::Rename => "rename.plan",
            RefactorType::Transform | RefactorType::SimplifyBooleanExpression => "transform.plan",
            RefactorType::RemoveUnusedImport
            | RefactorType::RemoveUnusedVariable
            | RefactorType::RemoveDeadCode => "delete.plan",
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
                metadata.benefits.push("Improves maintainability".to_string());
            }
            RefactorType::ExtractMethod => {
                metadata.benefits.push("Improves code organization".to_string());
                metadata.benefits.push("Enables code reuse".to_string());
                metadata.risks.push("May need to pass additional context".to_string());
            }
            _ => {}
        }

        metadata
    }
}
```

#### classifier.rs

```rust
use super::*;
use anyhow::Result;

pub struct SafetyClassifier;

impl SafetyClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Classify refactoring safety level
    pub fn classify(
        &self,
        refactoring: &RefactoringCandidate,
        context: &AnalysisContext,
    ) -> Result<SafetyLevel> {
        // SAFE: Low-risk, localized changes
        if self.is_safe_refactoring(refactoring, context) {
            return Ok(SafetyLevel::Safe);
        }

        // EXPERIMENTAL: High-risk, complex changes
        if self.is_experimental_refactoring(refactoring, context) {
            return Ok(SafetyLevel::Experimental);
        }

        // REQUIRES_REVIEW: Default for everything else
        Ok(SafetyLevel::RequiresReview)
    }

    fn is_safe_refactoring(
        &self,
        refactoring: &RefactoringCandidate,
        _context: &AnalysisContext,
    ) -> bool {
        match refactoring.refactor_type {
            // Always safe
            RefactorType::RemoveUnusedImport => true,
            RefactorType::RemoveUnusedVariable => {
                // Safe if truly unused (no references)
                refactoring.reference_count == Some(0)
            }
            RefactorType::SimplifyBooleanExpression => {
                // Safe if local scope, no side effects
                refactoring.scope == Scope::Local && !refactoring.has_side_effects
            }
            RefactorType::RemoveDeadCode => {
                // Safe if unreachable and no references
                refactoring.is_unreachable && refactoring.reference_count == Some(0)
            }
            // Everything else requires review or is experimental
            _ => false,
        }
    }

    fn is_experimental_refactoring(
        &self,
        refactoring: &RefactoringCandidate,
        _context: &AnalysisContext,
    ) -> bool {
        // Experimental if:
        // 1. Cross-crate changes
        if refactoring.scope == Scope::CrossCrate {
            return true;
        }

        // 2. Recursive operations
        if refactoring.is_recursive {
            return true;
        }

        // 3. Generic/template transformations
        if refactoring.involves_generics {
            return true;
        }

        // 4. Macro transformations
        if refactoring.involves_macros {
            return true;
        }

        false
    }
}
```

#### scorer.rs

```rust
use super::*;
use anyhow::Result;

pub struct ConfidenceScorer;

impl ConfidenceScorer {
    pub fn new() -> Self {
        Self
    }

    /// Calculate confidence score (0.0 to 1.0)
    pub fn score(
        &self,
        refactoring: &RefactoringCandidate,
        context: &AnalysisContext,
    ) -> Result<f64> {
        let mut confidence = 0.5; // Start at neutral

        // Factor 1: Pattern matching confidence (0.3 weight)
        confidence += self.pattern_confidence(refactoring) * 0.3;

        // Factor 2: Type information availability (0.25 weight)
        confidence += self.type_confidence(context) * 0.25;

        // Factor 3: AST parse quality (0.2 weight)
        confidence += self.ast_confidence(context) * 0.2;

        // Factor 4: Static analysis certainty (0.15 weight)
        confidence += self.analysis_confidence(refactoring) * 0.15;

        // Factor 5: Historical success rate (0.1 weight)
        confidence += self.historical_confidence() * 0.1;

        // Clamp to [0.0, 1.0]
        Ok(confidence.max(0.0).min(1.0))
    }

    fn pattern_confidence(&self, refactoring: &RefactoringCandidate) -> f64 {
        match refactoring.refactor_type {
            // High confidence patterns
            RefactorType::RemoveUnusedImport => 0.5,
            RefactorType::RemoveUnusedVariable if refactoring.reference_count == Some(0) => 0.5,
            RefactorType::SimplifyBooleanExpression => 0.4,

            // Medium confidence patterns
            RefactorType::ExtractMethod | RefactorType::Inline => 0.2,

            // Low confidence patterns
            _ => 0.0,
        }
    }

    fn type_confidence(&self, context: &AnalysisContext) -> f64 {
        if context.has_full_type_info {
            0.25 // Full type info available
        } else if context.has_partial_type_info {
            0.15 // Partial type info
        } else {
            0.0 // No type info (syntax-only analysis)
        }
    }

    fn ast_confidence(&self, context: &AnalysisContext) -> f64 {
        if context.ast_parse_errors == 0 {
            0.2 // Clean parse
        } else if context.ast_parse_errors < 3 {
            0.1 // Minor errors
        } else {
            0.0 // Significant errors
        }
    }

    fn analysis_confidence(&self, refactoring: &RefactoringCandidate) -> f64 {
        match refactoring.evidence_strength {
            EvidenceStrength::Strong => 0.15, // LSP confirms
            EvidenceStrength::Medium => 0.1,  // AST shows evidence
            EvidenceStrength::Weak => 0.0,    // Pattern only
        }
    }

    fn historical_confidence(&self) -> f64 {
        // TODO: Track success rates of past refactorings
        // For now, return neutral
        0.05
    }
}
```

#### ranker.rs

```rust
use super::*;

pub struct SuggestionRanker;

impl SuggestionRanker {
    pub fn new() -> Self {
        Self
    }

    /// Rank suggestions by: safety → confidence → impact
    pub fn rank(&self, suggestions: &mut Vec<ActionableSuggestion>) {
        suggestions.sort_by(|a, b| {
            // 1. Safety first (safe > requires_review > experimental)
            let safety_order = self.safety_order(a.safety).cmp(&self.safety_order(b.safety));
            if safety_order != std::cmp::Ordering::Equal {
                return safety_order;
            }

            // 2. Confidence second (higher is better)
            let confidence_order = b
                .confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal);
            if confidence_order != std::cmp::Ordering::Equal {
                return confidence_order;
            }

            // 3. Impact third (lower is better - prefer small changes)
            a.estimated_impact.cmp(&b.estimated_impact)
        });
    }

    fn safety_order(&self, safety: SafetyLevel) -> u8 {
        match safety {
            SafetyLevel::Safe => 0,
            SafetyLevel::RequiresReview => 1,
            SafetyLevel::Experimental => 2,
        }
    }
}
```

---

## Testing Strategy

### Unit Tests (tests.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_classifier_unused_import() {
        let classifier = SafetyClassifier::new();
        let refactoring = RefactoringCandidate {
            refactor_type: RefactorType::RemoveUnusedImport,
            reference_count: Some(0),
            scope: Scope::Local,
            has_side_effects: false,
            is_unreachable: false,
            is_recursive: false,
            involves_generics: false,
            involves_macros: false,
            evidence_strength: EvidenceStrength::Strong,
            // ... minimal fields
        };
        let context = AnalysisContext {
            file_path: "test.rs".to_string(),
            has_full_type_info: true,
            has_partial_type_info: true,
            ast_parse_errors: 0,
        };

        let safety = classifier.classify(&refactoring, &context).unwrap();
        assert_eq!(safety, SafetyLevel::Safe);
    }

    #[test]
    fn test_confidence_scorer_high_confidence() {
        let scorer = ConfidenceScorer::new();
        let refactoring = RefactoringCandidate {
            refactor_type: RefactorType::RemoveUnusedImport,
            evidence_strength: EvidenceStrength::Strong,
            reference_count: Some(0),
            // ... minimal fields
        };
        let context = AnalysisContext {
            file_path: "test.rs".to_string(),
            has_full_type_info: true,
            has_partial_type_info: true,
            ast_parse_errors: 0,
        };

        let confidence = scorer.score(&refactoring, &context).unwrap();
        assert!(confidence > 0.8, "Expected high confidence, got {}", confidence);
    }

    #[test]
    fn test_suggestion_ranking() {
        let ranker = SuggestionRanker::new();
        let mut suggestions = vec![
            ActionableSuggestion {
                message: "Review".to_string(),
                safety: SafetyLevel::RequiresReview,
                confidence: 0.9,
                reversible: true,
                estimated_impact: ImpactLevel::Medium,
                refactor_call: None,
                metadata: None,
            },
            ActionableSuggestion {
                message: "Safe".to_string(),
                safety: SafetyLevel::Safe,
                confidence: 0.7,
                reversible: true,
                estimated_impact: ImpactLevel::Low,
                refactor_call: None,
                metadata: None,
            },
        ];

        ranker.rank(&mut suggestions);

        // Safe should come first even with lower confidence
        assert_eq!(suggestions[0].safety, SafetyLevel::Safe);
        assert_eq!(suggestions[1].safety, SafetyLevel::RequiresReview);
    }
}
```

---

## Success Criteria

- [ ] All data structures compile and serialize correctly
- [ ] `SafetyClassifier` correctly classifies all RefactorType variants
- [ ] `ConfidenceScorer` produces scores in [0.0, 1.0] range
- [ ] `SuggestionRanker` sorts by safety → confidence → impact
- [ ] Unit tests achieve >80% code coverage
- [ ] Zero clippy warnings in new code
- [ ] Documentation comments on all public APIs
- [ ] Module exports properly organized in `mod.rs`

---

## Merge Requirements

Before merging to main:
1. All unit tests passing
2. Code review approved
3. Clippy clean (zero warnings)
4. Documentation complete
5. No breaking changes to existing APIs

After merge:
- Tag as `01c1-core-complete` for 01c2/01c3 to depend on

---

**Status**: 📋 Ready for Implementation (Week 1 - Critical Path)
