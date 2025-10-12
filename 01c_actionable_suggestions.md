# Proposal: Actionable Suggestions with Safety Metadata

**Status**: 📋 **PROPOSED**
**Author**: Project Team
**Date**: 2025-10-12
**Parent Proposal**: [01b_unified_analysis_api.md](01b_unified_analysis_api.md)
**Dependencies**: 01a (Refactoring API ✅), 01b (Analysis Core ✅)

---

## Executive Summary

**What**: Add safety metadata, confidence scoring, and comprehensive refactor_call generation to analysis suggestions, enabling AI agents to autonomously apply safe refactorings.

**Why**: Complete the "closed-loop workflow" (analyze → suggest → refactor → re-analyze) by bridging analysis findings to refactoring commands with risk assessment.

**Impact**: Transform analysis from static reports into an autonomous coding agent that fixes safe issues automatically and flags risky ones for human review.

---

## Problem Statement

### Current State (95% Complete Analysis API)

Analysis tools return findings with basic messages:

```json
{
  "category": "quality",
  "findings": [
    {
      "kind": "complexity",
      "message": "Function 'processOrder' has cyclomatic complexity 15 (threshold: 10)",
      "location": { "file": "src/orders.ts", "line": 45 },
      "suggestions": [
        {
          "message": "Consider extracting helper methods"
        }
      ]
    }
  ]
}
```

**Problem**: AI agents can't act on this because:
- ❌ No safety classification (is it safe to auto-apply?)
- ❌ No confidence score (how likely is this a real problem?)
- ❌ No reversibility info (can we undo if wrong?)
- ❌ No actionable refactor_call (how exactly do we fix it?)

### Desired State

```json
{
  "category": "quality",
  "findings": [
    {
      "kind": "complexity",
      "message": "Function 'processOrder' has cyclomatic complexity 15 (threshold: 10)",
      "location": { "file": "src/orders.ts", "line": 45 },
      "suggestions": [
        {
          "message": "Extract order validation logic",
          "safety": "safe",              // ← AI knows it can auto-apply
          "confidence": 0.92,             // ← High confidence
          "reversible": true,             // ← Can undo if wrong
          "estimated_impact": "medium",   // ← Impact assessment
          "refactor_call": {              // ← Exact command to run
            "tool": "extract.plan",
            "arguments": {
              "file_path": "src/orders.ts",
              "start_line": 48,
              "end_line": 62,
              "new_function_name": "validateOrder"
            }
          }
        },
        {
          "message": "Inline single-use helper 'formatDate'",
          "safety": "requires_review",   // ← Needs human approval
          "confidence": 0.75,
          "reversible": true,
          "estimated_impact": "low",
          "refactor_call": {
            "tool": "inline.plan",
            "arguments": {
              "file_path": "src/orders.ts",
              "line": 89,
              "character": 10
            }
          }
        }
      ]
    }
  ]
}
```

**Solution**: AI can now:
1. ✅ Auto-apply suggestions with `safety: "safe"` and high confidence
2. ✅ Ask for approval on `safety: "requires_review"` suggestions
3. ✅ Skip or defer `safety: "experimental"` suggestions
4. ✅ Undo changes if post-refactor analysis shows regressions

---

## Design Overview

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Analysis Findings                            │
│  (From 01b: analyze.quality, dead_code, dependencies, etc.)    │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              Suggestion Generator (NEW)                         │
│  • Takes raw findings                                           │
│  • Generates actionable suggestions                             │
│  • Assigns safety/confidence/reversibility                      │
│  • Creates refactor_call structures                             │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              Safety Classifier (NEW)                            │
│  • Analyzes refactoring type                                    │
│  • Checks scope (local vs cross-file)                           │
│  • Evaluates code patterns                                      │
│  • Assigns: safe | requires_review | experimental               │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              Confidence Scorer (NEW)                            │
│  • Pattern matching confidence                                  │
│  • Type information availability                                │
│  • AST parse quality                                            │
│  • Returns: 0.0 to 1.0                                          │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              Suggestion Ranker (NEW)                            │
│  • Sort by: safety → confidence → impact                        │
│  • Apply user-defined filters                                   │
│  • Return ranked suggestions                                    │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Enhanced AnalysisResult                        │
│              (Returned to AI agent)                             │
└─────────────────────────────────────────────────────────────────┘
```

### Core Data Structures

#### Enhanced Suggestion Structure

```rust
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
    /// Examples: remove unused imports, fix formatting, simplify boolean
    Safe,

    /// Requires human review before applying
    /// Examples: extract complex method, inline function, move code
    RequiresReview,

    /// Experimental - may not work in all cases
    /// Examples: recursive inlining, cross-crate moves, generic refactoring
    Experimental,
}

/// Impact level of suggested change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
```

---

## Implementation Details

### Core Infrastructure

#### Suggestion Generation Framework

**Files to create**:
- `crates/cb-handlers/src/handlers/tools/analysis/suggestions/mod.rs`
- `crates/cb-handlers/src/handlers/tools/analysis/suggestions/generator.rs`
- `crates/cb-handlers/src/handlers/tools/analysis/suggestions/classifier.rs`
- `crates/cb-handlers/src/handlers/tools/analysis/suggestions/scorer.rs`
- `crates/cb-handlers/src/handlers/tools/analysis/suggestions/ranker.rs`

**Tasks**:
```rust
// crates/cb-handlers/src/handlers/tools/analysis/suggestions/generator.rs

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

    /// Generate suggestions for a finding
    pub fn generate(
        &self,
        finding: &Finding,
        context: &AnalysisContext,
    ) -> Result<Vec<ActionableSuggestion>> {
        // 1. Determine potential refactorings based on finding kind
        let refactorings = self.identify_refactorings(finding, context)?;

        // 2. For each refactoring, generate full suggestion
        let mut suggestions = Vec::new();
        for refactoring in refactorings {
            let suggestion = self.build_suggestion(refactoring, finding, context)?;
            suggestions.push(suggestion);
        }

        // 3. Rank suggestions by safety → confidence → impact
        self.ranker.rank(&mut suggestions);

        Ok(suggestions)
    }

    fn identify_refactorings(
        &self,
        finding: &Finding,
        context: &AnalysisContext,
    ) -> Result<Vec<RefactoringCandidate>> {
        // Map finding kinds to potential refactorings
        match finding.kind.as_str() {
            "complexity" => self.complexity_refactorings(finding, context),
            "unused_code" => self.unused_code_refactorings(finding, context),
            "duplicate_code" => self.duplicate_code_refactorings(finding, context),
            // ... other kinds
            _ => Ok(vec![]),
        }
    }

    fn build_suggestion(
        &self,
        refactoring: RefactoringCandidate,
        finding: &Finding,
        context: &AnalysisContext,
    ) -> Result<ActionableSuggestion> {
        // 1. Classify safety
        let safety = self.classifier.classify(&refactoring, context)?;

        // 2. Calculate confidence
        let confidence = self.scorer.score(&refactoring, context)?;

        // 3. Determine reversibility
        let reversible = refactoring.is_reversible();

        // 4. Estimate impact
        let estimated_impact = self.estimate_impact(&refactoring, context)?;

        // 5. Generate refactor_call
        let refactor_call = self.build_refactor_call(&refactoring)?;

        // 6. Generate metadata
        let metadata = self.build_metadata(&refactoring, finding)?;

        Ok(ActionableSuggestion {
            message: refactoring.message,
            safety,
            confidence,
            reversible,
            estimated_impact,
            refactor_call: Some(refactor_call),
            metadata: Some(metadata),
        })
    }
}
```

#### Safety Classifier

**Algorithm**:

```rust
// crates/cb-handlers/src/handlers/tools/analysis/suggestions/classifier.rs

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
        // Rule-based classification

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
        context: &AnalysisContext,
    ) -> bool {
        match refactoring.refactor_type {
            // Always safe
            RefactorType::RemoveUnusedImport => true,
            RefactorType::RemoveUnusedVariable => {
                // Safe if truly unused (no references)
                refactoring.reference_count == 0
            }
            RefactorType::SimplifyBooleanExpression => {
                // Safe if local scope, no side effects
                refactoring.scope == Scope::Local
                    && !refactoring.has_side_effects
            }
            RefactorType::RemoveDeadCode => {
                // Safe if unreachable and no references
                refactoring.is_unreachable
                    && refactoring.reference_count == 0
            }

            // Everything else requires review or is experimental
            _ => false,
        }
    }

    fn is_experimental_refactoring(
        &self,
        refactoring: &RefactoringCandidate,
        context: &AnalysisContext,
    ) -> bool {
        // Experimental if:
        // 1. Cross-crate changes
        if refactoring.scope == Scope::CrossCrate {
            return true;
        }

        // 2. Recursive inlining
        if refactoring.refactor_type == RefactorType::Inline
            && refactoring.is_recursive {
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

**Safety Classification Rules** (to be refined during implementation):

| Refactoring Type | Default Safety | Conditions for "Safe" | Conditions for "Experimental" |
|------------------|----------------|----------------------|------------------------------|
| Remove unused import | Safe | Always | Never |
| Remove unused variable | Safe | 0 references | Cross-crate usage |
| Remove dead code | Safe | Unreachable + 0 refs | Dynamic dispatch involved |
| Simplify boolean | Safe | Local scope, no side effects | Cross-function |
| Extract method | RequiresReview | N/A | Cross-crate, involves generics |
| Inline function | RequiresReview | Single use, simple body | Recursive, complex body |
| Move code | RequiresReview | Same file | Cross-file, cross-crate |
| Rename | RequiresReview | Local scope | Public API |
| Transform | Experimental | N/A | Always experimental |

#### Confidence Scorer

**Algorithm**:

```rust
// crates/cb-handlers/src/handlers/tools/analysis/suggestions/scorer.rs

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
        confidence += self.type_confidence(refactoring, context) * 0.25;

        // Factor 3: AST parse quality (0.2 weight)
        confidence += self.ast_confidence(refactoring, context) * 0.2;

        // Factor 4: Static analysis certainty (0.15 weight)
        confidence += self.analysis_confidence(refactoring) * 0.15;

        // Factor 5: Historical success rate (0.1 weight)
        confidence += self.historical_confidence(refactoring) * 0.1;

        // Clamp to [0.0, 1.0]
        Ok(confidence.max(0.0).min(1.0))
    }

    fn pattern_confidence(&self, refactoring: &RefactoringCandidate) -> f64 {
        match refactoring.refactor_type {
            // High confidence patterns
            RefactorType::RemoveUnusedImport => 0.5, // Clear pattern
            RefactorType::RemoveUnusedVariable if refactoring.reference_count == 0 => 0.5,
            RefactorType::SimplifyBooleanExpression => 0.4,

            // Medium confidence patterns
            RefactorType::ExtractMethod => 0.2,
            RefactorType::Inline => 0.2,

            // Low confidence patterns
            _ => 0.0,
        }
    }

    fn type_confidence(&self, refactoring: &RefactoringCandidate, context: &AnalysisContext) -> f64 {
        if context.has_full_type_info {
            0.25 // Full type info available
        } else if context.has_partial_type_info {
            0.15 // Partial type info
        } else {
            0.0 // No type info (syntax-only analysis)
        }
    }

    fn ast_confidence(&self, refactoring: &RefactoringCandidate, context: &AnalysisContext) -> f64 {
        if context.ast_parse_errors == 0 {
            0.2 // Clean parse
        } else if context.ast_parse_errors < 3 {
            0.1 // Minor errors
        } else {
            0.0 // Significant errors
        }
    }

    fn analysis_confidence(&self, refactoring: &RefactoringCandidate) -> f64 {
        // Higher confidence if we have strong evidence
        match refactoring.evidence_strength {
            EvidenceStrength::Strong => 0.15, // e.g., LSP says "unused"
            EvidenceStrength::Medium => 0.1,  // e.g., AST shows no references
            EvidenceStrength::Weak => 0.0,    // e.g., pattern matching only
        }
    }

    fn historical_confidence(&self, refactoring: &RefactoringCandidate) -> f64 {
        // TODO: Track success rates of past refactorings
        // For now, return neutral
        0.05
    }
}
```

#### Suggestion Ranker

```rust
// crates/cb-handlers/src/handlers/tools/analysis/suggestions/ranker.rs

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
            let confidence_order = b.confidence.partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal);
            if confidence_order != std::cmp::Ordering::Equal {
                return confidence_order;
            }

            // 3. Impact third (lower is better - prefer small changes)
            let impact_order = self.impact_order(a.estimated_impact)
                .cmp(&self.impact_order(b.estimated_impact));
            impact_order
        });
    }

    fn safety_order(&self, safety: SafetyLevel) -> u8 {
        match safety {
            SafetyLevel::Safe => 0,
            SafetyLevel::RequiresReview => 1,
            SafetyLevel::Experimental => 2,
        }
    }

    fn impact_order(&self, impact: ImpactLevel) -> u8 {
        match impact {
            ImpactLevel::Low => 0,
            ImpactLevel::Medium => 1,
            ImpactLevel::High => 2,
            ImpactLevel::Critical => 3,
        }
    }
}
```

---

### Refactoring-Specific Generators

#### Complexity Suggestions

```rust
impl SuggestionGenerator {
    fn complexity_refactorings(
        &self,
        finding: &Finding,
        context: &AnalysisContext,
    ) -> Result<Vec<RefactoringCandidate>> {
        let mut candidates = Vec::new();

        // Parse finding to extract complexity details
        let complexity_value = finding.metadata
            .as_ref()
            .and_then(|m| m.get("complexity"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if complexity_value > 10 {
            // Suggest: Extract method
            candidates.push(RefactoringCandidate {
                refactor_type: RefactorType::ExtractMethod,
                message: "Extract helper methods to reduce complexity".to_string(),
                scope: Scope::Function,
                has_side_effects: false, // Would need deeper analysis
                reference_count: None,
                is_unreachable: false,
                is_recursive: false,
                involves_generics: false,
                involves_macros: false,
                evidence_strength: EvidenceStrength::Medium,
                location: finding.location.clone(),
                // ... other fields
            });
        }

        if complexity_value > 15 {
            // Suggest: Simplify boolean expressions
            candidates.push(RefactoringCandidate {
                refactor_type: RefactorType::SimplifyBooleanExpression,
                message: "Simplify complex boolean conditions".to_string(),
                scope: Scope::Local,
                has_side_effects: false,
                reference_count: None,
                is_unreachable: false,
                is_recursive: false,
                involves_generics: false,
                involves_macros: false,
                evidence_strength: EvidenceStrength::Weak, // Pattern matching only
                location: finding.location.clone(),
                // ... other fields
            });
        }

        Ok(candidates)
    }
}
```

#### Dead Code Suggestions

```rust
impl SuggestionGenerator {
    fn unused_code_refactorings(
        &self,
        finding: &Finding,
        context: &AnalysisContext,
    ) -> Result<Vec<RefactoringCandidate>> {
        let mut candidates = Vec::new();

        match finding.kind.as_str() {
            "unused_import" => {
                candidates.push(RefactoringCandidate {
                    refactor_type: RefactorType::RemoveUnusedImport,
                    message: format!("Remove unused import '{}'", finding.symbol_name()),
                    scope: Scope::Local,
                    has_side_effects: false,
                    reference_count: Some(0),
                    is_unreachable: false,
                    is_recursive: false,
                    involves_generics: false,
                    involves_macros: false,
                    evidence_strength: EvidenceStrength::Strong, // LSP confirms
                    location: finding.location.clone(),
                    refactor_call_args: json!({
                        "file_path": finding.location.file,
                        "line": finding.location.line,
                        "import_name": finding.symbol_name(),
                    }),
                });
            }
            "unused_variable" => {
                candidates.push(RefactoringCandidate {
                    refactor_type: RefactorType::RemoveUnusedVariable,
                    message: format!("Remove unused variable '{}'", finding.symbol_name()),
                    scope: Scope::Function,
                    has_side_effects: false,
                    reference_count: Some(0),
                    is_unreachable: false,
                    is_recursive: false,
                    involves_generics: false,
                    involves_macros: false,
                    evidence_strength: EvidenceStrength::Strong,
                    location: finding.location.clone(),
                    refactor_call_args: json!({
                        "file_path": finding.location.file,
                        "line": finding.location.line,
                        "variable_name": finding.symbol_name(),
                    }),
                });
            }
            "unreachable_code" => {
                candidates.push(RefactoringCandidate {
                    refactor_type: RefactorType::RemoveDeadCode,
                    message: "Remove unreachable code".to_string(),
                    scope: Scope::Function,
                    has_side_effects: false,
                    reference_count: Some(0),
                    is_unreachable: true,
                    is_recursive: false,
                    involves_generics: false,
                    involves_macros: false,
                    evidence_strength: EvidenceStrength::Strong,
                    location: finding.location.clone(),
                    refactor_call_args: json!({
                        "file_path": finding.location.file,
                        "start_line": finding.location.line,
                        "end_line": finding.metadata
                            .as_ref()
                            .and_then(|m| m.get("end_line"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(finding.location.line),
                    }),
                });
            }
            _ => {}
        }

        Ok(candidates)
    }
}
```

#### Duplicate Code Suggestions

```rust
impl SuggestionGenerator {
    fn duplicate_code_refactorings(
        &self,
        finding: &Finding,
        context: &AnalysisContext,
    ) -> Result<Vec<RefactoringCandidate>> {
        let mut candidates = Vec::new();

        // Parse duplicate locations from metadata
        let duplicate_locations = finding.metadata
            .as_ref()
            .and_then(|m| m.get("duplicates"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        if duplicate_locations > 1 {
            candidates.push(RefactoringCandidate {
                refactor_type: RefactorType::ExtractMethod,
                message: format!(
                    "Extract duplicated code into a shared function ({} occurrences)",
                    duplicate_locations
                ),
                scope: if duplicate_locations > 3 {
                    Scope::File
                } else {
                    Scope::Function
                },
                has_side_effects: true, // Conservative: assume side effects
                reference_count: None,
                is_unreachable: false,
                is_recursive: false,
                involves_generics: false,
                involves_macros: false,
                evidence_strength: EvidenceStrength::Medium,
                location: finding.location.clone(),
                refactor_call_args: json!({
                    "file_path": finding.location.file,
                    "start_line": finding.location.line,
                    "end_line": finding.metadata
                        .as_ref()
                        .and_then(|m| m.get("end_line"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(finding.location.line + 10),
                    "new_function_name": format!(
                        "extracted_{}",
                        finding.location.line
                    ),
                }),
            });
        }

        Ok(candidates)
    }
}
```

#### Refactor Call Builder

```rust
impl SuggestionGenerator {
    fn build_refactor_call(
        &self,
        refactoring: &RefactoringCandidate,
    ) -> Result<RefactorCall> {
        let (tool, arguments) = match refactoring.refactor_type {
            RefactorType::ExtractMethod => (
                "extract.plan",
                refactoring.refactor_call_args.clone(),
            ),
            RefactorType::Inline => (
                "inline.plan",
                refactoring.refactor_call_args.clone(),
            ),
            RefactorType::RemoveUnusedImport |
            RefactorType::RemoveUnusedVariable |
            RefactorType::RemoveDeadCode => (
                "delete.plan",
                refactoring.refactor_call_args.clone(),
            ),
            RefactorType::SimplifyBooleanExpression => (
                "transform.plan",
                json!({
                    "file_path": refactoring.location.file,
                    "start_line": refactoring.location.line,
                    "end_line": refactoring.location.line,
                    "transformation_kind": "simplify_boolean",
                }),
            ),
            // ... other types
            _ => return Err(anyhow::anyhow!("No refactor_call mapping for {:?}", refactoring.refactor_type)),
        };

        Ok(RefactorCall {
            tool: tool.to_string(),
            arguments,
        })
    }
}
```

---

### Integration Points

#### Analysis Handler Integration

Integrate suggestion generator into all 6 analysis categories:

```rust
// crates/cb-handlers/src/handlers/tools/analysis/quality.rs

pub async fn analyze_quality(
    params: AnalysisParams,
    lsp_client: &LspClient,
    ast_service: &AstService,
    suggestion_generator: &SuggestionGenerator, // ← NEW
) -> Result<AnalysisResult> {
    // ... existing analysis logic ...

    // Generate findings
    let mut findings = detect_quality_issues(&parsed_source, &params)?;

    // ← NEW: Enhance findings with actionable suggestions
    for finding in &mut findings {
        let suggestions = suggestion_generator.generate(
            finding,
            &AnalysisContext {
                file_path: &params.file_path,
                has_full_type_info: lsp_client.has_type_info(),
                has_partial_type_info: parsed_source.has_type_annotations(),
                ast_parse_errors: parsed_source.errors.len(),
            },
        )?;

        finding.suggestions = suggestions;
    }

    Ok(AnalysisResult {
        category: "quality".to_string(),
        findings,
        // ... other fields
    })
}
```

Apply same pattern to:
- `analyze_dead_code` (crates/cb-handlers/src/handlers/tools/analysis/dead_code.rs)
- `analyze_dependencies` (crates/cb-handlers/src/handlers/tools/analysis/dependencies.rs)
- `analyze_structure` (crates/cb-handlers/src/handlers/tools/analysis/structure.rs)
- `analyze_documentation` (crates/cb-handlers/src/handlers/tools/analysis/documentation.rs)
- `analyze_tests` (crates/cb-handlers/src/handlers/tools/analysis/tests.rs)

#### Batch Analysis Integration

```rust
// crates/cb-handlers/src/handlers/tools/analysis/batch.rs

pub async fn analyze_batch(
    params: BatchAnalysisParams,
    lsp_client: &LspClient,
    ast_service: &AstService,
    suggestion_generator: &SuggestionGenerator, // ← NEW
) -> Result<BatchAnalysisResult> {
    // ... existing batch logic with AST caching ...

    // For each query, enhance findings with suggestions
    for result in &mut results {
        for finding in &mut result.findings {
            let suggestions = suggestion_generator.generate(
                finding,
                &AnalysisContext {
                    file_path: &result.file_path,
                    has_full_type_info: lsp_client.has_type_info(),
                    has_partial_type_info: cached_ast.has_type_annotations(),
                    ast_parse_errors: cached_ast.errors.len(),
                },
            )?;

            finding.suggestions = suggestions;
        }
    }

    Ok(BatchAnalysisResult {
        results,
        // ... other fields
    })
}
```

---

## Configuration

Add suggestion-related config to `.codebuddy/analysis.toml`:

```toml
# .codebuddy/analysis.toml

[suggestions]
# Minimum confidence threshold (0.0 to 1.0)
min_confidence = 0.7

# Safety levels to include
include_safety_levels = ["safe", "requires_review"]

# Maximum suggestions per finding
max_per_finding = 3

# Generate refactor_call for all suggestions (default: true)
generate_refactor_calls = true

[suggestions.filters]
# Skip suggestions for these refactoring types
exclude_refactor_types = ["transform"]

# Only show suggestions with these impact levels
allowed_impact_levels = ["low", "medium", "high"]
```

Load config in suggestion generator:

```rust
impl SuggestionGenerator {
    pub fn new(config: SuggestionConfig) -> Self {
        Self {
            classifier: SafetyClassifier::new(),
            scorer: ConfidenceScorer::new(),
            ranker: SuggestionRanker::new(),
            config,
        }
    }

    pub fn generate(
        &self,
        finding: &Finding,
        context: &AnalysisContext,
    ) -> Result<Vec<ActionableSuggestion>> {
        // ... generate suggestions ...

        // Apply config filters
        let filtered_suggestions = suggestions.into_iter()
            .filter(|s| s.confidence >= self.config.min_confidence)
            .filter(|s| self.config.include_safety_levels.contains(&s.safety))
            .take(self.config.max_per_finding)
            .collect();

        Ok(filtered_suggestions)
    }
}
```

---

## Testing Strategy

### Unit Tests

```rust
// crates/cb-handlers/src/handlers/tools/analysis/suggestions/tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_classifier_unused_import() {
        let classifier = SafetyClassifier::new();
        let refactoring = RefactoringCandidate {
            refactor_type: RefactorType::RemoveUnusedImport,
            reference_count: Some(0),
            // ... other fields
        };
        let context = AnalysisContext::default();

        let safety = classifier.classify(&refactoring, &context).unwrap();
        assert_eq!(safety, SafetyLevel::Safe);
    }

    #[test]
    fn test_confidence_scorer_high_confidence() {
        let scorer = ConfidenceScorer::new();
        let refactoring = RefactoringCandidate {
            refactor_type: RefactorType::RemoveUnusedImport,
            evidence_strength: EvidenceStrength::Strong,
            // ... other fields
        };
        let context = AnalysisContext {
            has_full_type_info: true,
            ast_parse_errors: 0,
            // ... other fields
        };

        let confidence = scorer.score(&refactoring, &context).unwrap();
        assert!(confidence > 0.8, "Expected high confidence, got {}", confidence);
    }

    #[test]
    fn test_suggestion_ranking() {
        let ranker = SuggestionRanker::new();
        let mut suggestions = vec![
            ActionableSuggestion {
                safety: SafetyLevel::RequiresReview,
                confidence: 0.9,
                estimated_impact: ImpactLevel::Medium,
                // ... other fields
            },
            ActionableSuggestion {
                safety: SafetyLevel::Safe,
                confidence: 0.7,
                estimated_impact: ImpactLevel::Low,
                // ... other fields
            },
        ];

        ranker.rank(&mut suggestions);

        // Safe should come first even with lower confidence
        assert_eq!(suggestions[0].safety, SafetyLevel::Safe);
    }
}
```

### Integration Tests

```rust
// integration-tests/src/test_actionable_suggestions.rs

#[tokio::test]
async fn test_quality_analysis_generates_suggestions() {
    let server = setup_test_server().await;

    // Analyze file with complexity issues
    let result = server.call_tool(
        "analyze.quality",
        json!({
            "file_path": "test_data/complex_function.ts",
            "kinds": ["complexity"],
        }),
    ).await.unwrap();

    let findings: Vec<Finding> = serde_json::from_value(result["findings"].clone()).unwrap();

    // Assert suggestions exist
    assert!(!findings.is_empty(), "Should have complexity findings");
    let finding = &findings[0];
    assert!(!finding.suggestions.is_empty(), "Should have suggestions");

    // Assert suggestion has required fields
    let suggestion = &finding.suggestions[0];
    assert!(matches!(suggestion.safety, SafetyLevel::Safe | SafetyLevel::RequiresReview));
    assert!(suggestion.confidence >= 0.0 && suggestion.confidence <= 1.0);
    assert!(suggestion.refactor_call.is_some(), "Should have refactor_call");
}

#[tokio::test]
async fn test_dead_code_removal_suggestion() {
    let server = setup_test_server().await;

    let result = server.call_tool(
        "analyze.dead_code",
        json!({
            "file_path": "test_data/unused_imports.ts",
            "kinds": ["unused_import"],
        }),
    ).await.unwrap();

    let findings: Vec<Finding> = serde_json::from_value(result["findings"].clone()).unwrap();
    let suggestion = &findings[0].suggestions[0];

    // Unused imports should be safe to remove
    assert_eq!(suggestion.safety, SafetyLevel::Safe);
    assert!(suggestion.confidence > 0.8);
    assert_eq!(suggestion.reversible, true);

    // Should have delete.plan refactor_call
    let refactor_call = suggestion.refactor_call.as_ref().unwrap();
    assert_eq!(refactor_call.tool, "delete.plan");
}

#[tokio::test]
async fn test_closed_loop_workflow() {
    let server = setup_test_server().await;

    // Step 1: Analyze
    let analysis_result = server.call_tool(
        "analyze.dead_code",
        json!({
            "file_path": "test_data/unused_code.ts",
            "kinds": ["unused_import"],
        }),
    ).await.unwrap();

    let findings: Vec<Finding> = serde_json::from_value(analysis_result["findings"].clone()).unwrap();
    let safe_suggestion = findings[0].suggestions.iter()
        .find(|s| s.safety == SafetyLevel::Safe && s.confidence > 0.9)
        .expect("Should have safe suggestion");

    // Step 2: Apply suggestion
    let refactor_call = safe_suggestion.refactor_call.as_ref().unwrap();
    let plan_result = server.call_tool(
        &refactor_call.tool,
        refactor_call.arguments.clone(),
    ).await.unwrap();

    let apply_result = server.call_tool(
        "workspace.apply_edit",
        json!({
            "plan": plan_result,
        }),
    ).await.unwrap();

    assert_eq!(apply_result["success"], true);

    // Step 3: Re-analyze to verify fix
    let reanalysis_result = server.call_tool(
        "analyze.dead_code",
        json!({
            "file_path": "test_data/unused_code.ts",
            "kinds": ["unused_import"],
        }),
    ).await.unwrap();

    let new_findings: Vec<Finding> = serde_json::from_value(reanalysis_result["findings"].clone()).unwrap();

    // Issue should be fixed
    assert!(new_findings.is_empty() || new_findings.len() < findings.len());
}
```

### CI Validation

Add CI check to ensure all suggestions have required metadata:

```rust
// crates/cb-handlers/src/handlers/tools/analysis/suggestions/validation.rs

/// Validates that suggestion has all required metadata
pub fn validate_suggestion(suggestion: &ActionableSuggestion) -> Result<()> {
    // Check required fields
    if suggestion.message.is_empty() {
        bail!("Suggestion missing message");
    }

    // Check confidence range
    if suggestion.confidence < 0.0 || suggestion.confidence > 1.0 {
        bail!("Confidence out of range: {}", suggestion.confidence);
    }

    // Check refactor_call for actionable suggestions
    if suggestion.safety == SafetyLevel::Safe
        || suggestion.safety == SafetyLevel::RequiresReview {
        if suggestion.refactor_call.is_none() {
            bail!("Safe/RequiresReview suggestion missing refactor_call");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_generated_suggestions_valid() {
        // Test harness that runs all analysis tools on test files
        // and validates every suggestion generated

        let test_cases = vec![
            ("test_data/complex.ts", "analyze.quality", vec!["complexity"]),
            ("test_data/unused.ts", "analyze.dead_code", vec!["unused_import"]),
            // ... more test cases
        ];

        for (file, tool, kinds) in test_cases {
            let result = run_analysis(file, tool, kinds);
            for finding in result.findings {
                for suggestion in finding.suggestions {
                    validate_suggestion(&suggestion)
                        .expect(&format!("Invalid suggestion in {}: {:?}", file, suggestion));
                }
            }
        }
    }
}
```

---

## Success Criteria

### Core Infrastructure
- [x] `ActionableSuggestion` data structure defined
- [ ] `SuggestionGenerator` framework implemented
- [ ] `SafetyClassifier` with rule-based classification
- [ ] `ConfidenceScorer` with multi-factor scoring
- [ ] `SuggestionRanker` with safety → confidence → impact ordering
- [ ] Unit tests for each component (>80% coverage)

### Refactoring-Specific Generators
- [ ] Complexity suggestions (extract method, simplify boolean)
- [ ] Dead code suggestions (remove unused, delete unreachable)
- [ ] Duplicate code suggestions (extract common code)
- [ ] Dependency suggestions (remove unused imports, optimize imports)
- [ ] Documentation suggestions (add missing docs)
- [ ] Test suggestions (add missing tests, improve assertions)
- [ ] All suggestions include `refactor_call` with valid arguments

### Integration
- [ ] All 6 analysis categories generate suggestions
- [ ] `analyze.batch` generates suggestions with AST caching
- [ ] Configuration system supports suggestion filters
- [ ] Suggestions respect min_confidence threshold
- [ ] Suggestions respect safety level filters

### Testing & Validation
- [ ] Unit tests for all suggestion generators
- [ ] Integration tests for closed-loop workflow
- [ ] CI validation ensures all suggestions have required metadata
- [ ] Performance benchmarks (suggestion generation <100ms per finding)
- [ ] Documentation updated with examples

### User-Facing Metrics
- [ ] AI agent can auto-apply ≥50% of "safe" suggestions without errors
- [ ] "Requires review" suggestions have <10% false positive rate
- [ ] Closed-loop workflow completes in <5 seconds for typical file
- [ ] User can configure aggressiveness (strict → relaxed presets)

---

## Configuration Example

`.codebuddy/analysis.toml`:

```toml
[suggestions]
# Minimum confidence threshold (0.0 to 1.0)
# Higher = fewer but more reliable suggestions
min_confidence = 0.7

# Safety levels to include in results
# Options: "safe", "requires_review", "experimental"
include_safety_levels = ["safe", "requires_review"]

# Maximum suggestions per finding
# Prevents overwhelming results
max_per_finding = 3

# Generate refactor_call for all suggestions
# Disable for read-only analysis
generate_refactor_calls = true

[suggestions.filters]
# Skip suggestions for these refactoring types
# Options: "extract", "inline", "move", "rename", "transform", "delete"
exclude_refactor_types = []

# Only show suggestions with these impact levels
# Options: "low", "medium", "high", "critical"
allowed_impact_levels = ["low", "medium", "high"]

# Exclude suggestions for specific files/patterns
exclude_files = [
    "generated/*",
    "*.test.ts",
]

[suggestions.presets.strict]
min_confidence = 0.9
include_safety_levels = ["safe"]
max_per_finding = 1

[suggestions.presets.default]
min_confidence = 0.7
include_safety_levels = ["safe", "requires_review"]
max_per_finding = 3

[suggestions.presets.relaxed]
min_confidence = 0.5
include_safety_levels = ["safe", "requires_review", "experimental"]
max_per_finding = 5
```

---

## Example Output

### Before (Current State)

```json
{
  "category": "quality",
  "findings": [
    {
      "kind": "complexity",
      "severity": "warning",
      "message": "Function 'processOrder' has cyclomatic complexity 15 (threshold: 10)",
      "location": {
        "file": "src/orders.ts",
        "line": 45,
        "character": 0
      },
      "suggestions": [
        {
          "message": "Consider extracting helper methods"
        }
      ]
    }
  ]
}
```

### After (With Actionable Suggestions)

```json
{
  "category": "quality",
  "findings": [
    {
      "kind": "complexity",
      "severity": "warning",
      "message": "Function 'processOrder' has cyclomatic complexity 15 (threshold: 10)",
      "location": {
        "file": "src/orders.ts",
        "line": 45,
        "character": 0
      },
      "suggestions": [
        {
          "message": "Extract order validation logic into 'validateOrder' function",
          "safety": "requires_review",
          "confidence": 0.85,
          "reversible": true,
          "estimated_impact": "medium",
          "refactor_call": {
            "tool": "extract.plan",
            "arguments": {
              "file_path": "src/orders.ts",
              "start_line": 48,
              "end_line": 62,
              "new_function_name": "validateOrder"
            }
          },
          "metadata": {
            "rationale": "Separating validation logic will reduce complexity from 15 to 8",
            "benefits": [
              "Easier to test validation independently",
              "Reusable validation logic",
              "Clearer separation of concerns"
            ],
            "risks": [
              "May need to pass additional context to extracted function"
            ]
          }
        },
        {
          "message": "Simplify nested if-else chains",
          "safety": "safe",
          "confidence": 0.78,
          "reversible": true,
          "estimated_impact": "low",
          "refactor_call": {
            "tool": "transform.plan",
            "arguments": {
              "file_path": "src/orders.ts",
              "start_line": 55,
              "end_line": 58,
              "transformation_kind": "simplify_boolean"
            }
          },
          "metadata": {
            "rationale": "Nested conditions can be flattened with guard clauses",
            "benefits": [
              "Reduces nesting depth",
              "Improves readability"
            ],
            "risks": []
          }
        }
      ]
    }
  ]
}
```

---

## Open Questions & Future Work

### Open Questions
1. **Historical success tracking**: Should we persist refactoring success rates to improve confidence scoring?
   - **Proposal**: Add telemetry collection (opt-in) to track applied suggestions and their outcomes
2. **User feedback loop**: How do users provide feedback on suggestion quality?
   - **Proposal**: Add optional `feedback` parameter to `workspace.apply_edit` (success/failure/issues)
3. **Language-specific tuning**: Do safety rules differ significantly per language?
   - **Proposal**: Start with language-agnostic rules, add overrides as needed

### Future Enhancements
1. **Machine learning confidence scoring**: Train model on historical data
2. **Semantic similarity for duplicate detection**: Beyond exact matches
3. **Impact estimation based on call graph**: Analyze downstream effects
4. **Suggestion dependency tracking**: "If you apply A, you should also apply B"
5. **Batch suggestion application**: "Apply all safe suggestions" command
6. **Suggestion explanation**: Natural language explanation of why each suggestion was made

---

## Risk Mitigation

### Risk: False Positives in Safety Classification
**Mitigation**:
- Conservative defaults (most things are "requires_review")
- User can override via configuration
- Reversible flag allows undo of mistakes
- Extensive testing with real-world codebases

### Risk: Performance Degradation
**Mitigation**:
- Suggestion generation happens after analysis (already paid parsing cost)
- Cache suggestion generator instances
- Make suggestion generation optional (config flag)
- Benchmark: target <100ms per finding

### Risk: Incomplete Refactor Call Arguments
**Mitigation**:
- Validate refactor_call in CI tests
- Provide clear error messages for missing args
- Support partial refactor_call (AI fills in missing args)

### Risk: User Overwhelm (Too Many Suggestions)
**Mitigation**:
- `max_per_finding` configuration
- Default to high confidence threshold (0.7)
- Safety filtering (exclude experimental by default)
- Clear visual hierarchy in UI/CLI output

---

## References

- **Parent Proposal**: [01b_unified_analysis_api.md](01b_unified_analysis_api.md)
- **Refactoring API**: [01a_unified_refactoring_api.md](01a_unified_refactoring_api.md)
- **Implementation Sequencing**: [05_implementation_sequencing.md](05_implementation_sequencing.md)
- **Formal Spec**: [docs/design/unified_api_contracts.md](docs/design/unified_api_contracts.md)

---

## Appendix: Safety Classification Matrix

Complete matrix of refactoring types → default safety levels:

| Refactoring Type | Default Safety | Safe Conditions | Experimental Conditions |
|------------------|----------------|-----------------|------------------------|
| **Remove unused import** | Safe | Always | Never |
| **Remove unused variable** | Safe | 0 references, local scope | Cross-crate references |
| **Remove dead code** | Safe | Unreachable + 0 references | Dynamic dispatch involved |
| **Simplify boolean** | Safe | Local scope, no side effects | Cross-function, side effects possible |
| **Remove unused parameter** | Requires Review | Private function, 0 callers | Public API |
| **Inline function** | Requires Review | Single use, simple body | Recursive, complex body |
| **Extract method** | Requires Review | Clear scope boundaries | Involves closures/generics |
| **Move code** | Requires Review | Same file | Cross-file, cross-crate |
| **Rename** | Requires Review | Local scope | Public API |
| **Reorder** | Requires Review | Independent declarations | Order-dependent code |
| **Transform** | Experimental | N/A | Always experimental |
| **Generic refactoring** | Experimental | N/A | Involves type parameters |
| **Macro refactoring** | Experimental | N/A | Involves macro expansion |

---

**Status**: 📋 Ready for Implementation
