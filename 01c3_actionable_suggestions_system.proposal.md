# Proposal 01c3: Actionable Suggestions - System Integration

**Status**: ❌ **NOT STARTED**
**Author**: Project Team
**Date**: 2025-10-13 (Split from 01c)
**Parent Proposal**: [01b_unified_analysis_api.md](01b_unified_analysis_api.md)
**Dependencies**: ✅ 01a, ✅ 01b, ⚠️ **BLOCKS ON 01c1** (Core), **PARALLEL WITH 01c2** (Integration)
**Branch**: `feature/01c3-suggestions-system`
**Estimated Effort**: 3-5 days (~25 hours) - Can start after 01c1 is 50% complete

---

## Executive Summary

**What**: Add configuration system, batch integration, CI validation, and documentation for actionable suggestions.

**Why**: Complete the "plumbing" work that makes suggestions production-ready and user-configurable.

**Impact**: Users can tune suggestion aggressiveness, batch operations generate suggestions, and CI prevents regressions.

**Parallel Work**: Can be done in parallel with 01c2 after 01c1 core is available.

---

## Scope - What This Branch Delivers

### Configuration System ✅
- `.codebuddy/analysis.toml` suggestion configuration
- Presets: `strict`, `default`, `relaxed`
- Filters: confidence threshold, safety levels, impact levels
- Configuration loading with graceful fallback

### Batch Integration ✅
- Integrate `SuggestionGenerator` into `analyze.batch` handler
- Suggestions generated for all batch queries
- AST caching optimization maintained

### CI Validation ✅
- Validation function for suggestion metadata
- CI test ensuring all suggestions valid
- Schema validation for `refactor_call`

### Documentation ✅
- Update `API_REFERENCE.md` with suggestion examples
- Update `CLAUDE.md` with suggestion configuration
- Add examples to `QUICK_REFERENCE.md`
- Update proposal 01b status

---

## Out of Scope - What This Branch Does NOT Deliver

❌ Core infrastructure - That's 01c1
❌ Analysis handler integration - That's 01c2
❌ Category-specific generators - That's 01c2

---

## Implementation Details

### Part 1: Configuration System

#### File: `.codebuddy/analysis.toml` (Example Config)

```toml
# Actionable Suggestions Configuration

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

# Presets: strict, default, relaxed
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

#### File: `crates/cb-handlers/src/handlers/tools/analysis/suggestions/config.rs`

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use super::{SafetyLevel, ImpactLevel};

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
    /// Load from .codebuddy/analysis.toml
    pub fn load() -> Result<Self> {
        let config_path = std::path::Path::new(".codebuddy/analysis.toml");
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
    pub fn filter(&self, suggestions: Vec<ActionableSuggestion>) -> Vec<ActionableSuggestion> {
        suggestions
            .into_iter()
            .filter(|s| s.confidence >= self.min_confidence)
            .filter(|s| self.include_safety_levels.contains(&s.safety))
            .filter(|s| {
                self.filters.allowed_impact_levels.is_empty()
                    || self.filters.allowed_impact_levels.contains(&s.estimated_impact)
            })
            .take(self.max_per_finding)
            .collect()
    }
}
```

#### Update: `generator.rs` to use config

```rust
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

    pub fn generate_from_candidate(
        &self,
        candidate: RefactoringCandidate,
        context: &AnalysisContext,
    ) -> Result<ActionableSuggestion> {
        // ... existing generation logic ...

        // Apply config: strip refactor_call if disabled
        let refactor_call = if self.config.generate_refactor_calls {
            Some(self.build_refactor_call(&candidate)?)
        } else {
            None
        };

        Ok(ActionableSuggestion {
            message: candidate.message,
            safety,
            confidence,
            reversible,
            estimated_impact,
            refactor_call,
            metadata: Some(metadata),
        })
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
        self.config.filter(suggestions)
    }
}
```

---

### Part 2: Batch Integration

#### File: `crates/cb-handlers/src/handlers/tools/analysis/batch.rs` (Modify)

```rust
use super::suggestions::{SuggestionGenerator, SuggestionConfig, AnalysisContext};

pub async fn analyze_batch(
    params: BatchAnalysisParams,
    lsp_client: &LspClient,
    ast_service: &AstService,
) -> Result<BatchAnalysisResult> {
    // Load suggestion config once for entire batch
    let suggestion_config = SuggestionConfig::load().unwrap_or_default();
    let suggestion_generator = SuggestionGenerator::with_config(suggestion_config);

    // ... existing batch logic with AST caching ...

    // For each query result, enhance findings with suggestions
    for result in &mut results {
        let context = AnalysisContext {
            file_path: result.file_path.clone(),
            has_full_type_info: lsp_client.has_type_info(),
            has_partial_type_info: cached_ast.has_type_annotations(),
            ast_parse_errors: cached_ast.errors.len(),
        };

        for finding in &mut result.findings {
            // Generate candidates (category-specific logic from 01c2)
            let candidates = generate_refactoring_candidates_for_category(
                &result.category,
                finding,
                &cached_ast,
            )?;

            // Generate and filter suggestions
            let suggestions = suggestion_generator.generate_multiple(candidates, &context);

            finding.suggestions = suggestions;
        }
    }

    Ok(BatchAnalysisResult {
        results,
        // ... other fields
    })
}

/// Helper to route candidate generation by category
fn generate_refactoring_candidates_for_category(
    category: &str,
    finding: &Finding,
    parsed_source: &ParsedSource,
) -> Result<Vec<RefactoringCandidate>> {
    match category {
        "quality" => generate_quality_refactoring_candidates(finding, parsed_source),
        "dead_code" => generate_dead_code_refactoring_candidates(finding, parsed_source),
        "dependencies" => generate_dependencies_refactoring_candidates(finding, parsed_source),
        "structure" => generate_structure_refactoring_candidates(finding, parsed_source),
        "documentation" => generate_documentation_refactoring_candidates(finding, parsed_source),
        "tests" => generate_tests_refactoring_candidates(finding, parsed_source),
        _ => Ok(Vec::new()),
    }
}
```

---

### Part 3: CI Validation

#### File: `crates/cb-handlers/src/handlers/tools/analysis/suggestions/validation.rs`

```rust
use super::*;
use anyhow::{bail, Result};

/// Validates that suggestion has all required metadata
pub fn validate_suggestion(suggestion: &ActionableSuggestion) -> Result<()> {
    // Check required fields
    if suggestion.message.is_empty() {
        bail!("Suggestion missing message");
    }

    // Check confidence range
    if !(0.0..=1.0).contains(&suggestion.confidence) {
        bail!("Confidence out of range: {}", suggestion.confidence);
    }

    // Check refactor_call for safe/requires_review suggestions
    if matches!(
        suggestion.safety,
        SafetyLevel::Safe | SafetyLevel::RequiresReview
    ) {
        if let Some(ref refactor_call) = suggestion.refactor_call {
            validate_refactor_call(refactor_call)?;
        } else {
            bail!(
                "Safe/RequiresReview suggestion missing refactor_call: {:?}",
                suggestion.safety
            );
        }
    }

    Ok(())
}

/// Validates refactor_call structure
fn validate_refactor_call(refactor_call: &RefactorCall) -> Result<()> {
    // Valid tool names
    let valid_tools = [
        "extract.plan",
        "inline.plan",
        "move.plan",
        "rename.plan",
        "transform.plan",
        "delete.plan",
        "reorder.plan",
    ];

    if !valid_tools.contains(&refactor_call.tool.as_str()) {
        bail!("Invalid tool name: {}", refactor_call.tool);
    }

    // Arguments must be an object
    if !refactor_call.arguments.is_object() {
        bail!("refactor_call.arguments must be an object");
    }

    // Tool-specific argument validation
    match refactor_call.tool.as_str() {
        "delete.plan" => validate_delete_args(&refactor_call.arguments)?,
        "extract.plan" => validate_extract_args(&refactor_call.arguments)?,
        "inline.plan" => validate_inline_args(&refactor_call.arguments)?,
        // ... other tools
        _ => {}
    }

    Ok(())
}

fn validate_delete_args(args: &serde_json::Value) -> Result<()> {
    if args.get("file_path").is_none() {
        bail!("delete.plan missing file_path");
    }
    if args.get("line").is_none() && args.get("start_line").is_none() {
        bail!("delete.plan missing line or start_line");
    }
    Ok(())
}

fn validate_extract_args(args: &serde_json::Value) -> Result<()> {
    if args.get("file_path").is_none() {
        bail!("extract.plan missing file_path");
    }
    if args.get("start_line").is_none() || args.get("end_line").is_none() {
        bail!("extract.plan missing start_line/end_line");
    }
    Ok(())
}

fn validate_inline_args(args: &serde_json::Value) -> Result<()> {
    if args.get("file_path").is_none() {
        bail!("inline.plan missing file_path");
    }
    if args.get("line").is_none() {
        bail!("inline.plan missing line");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_complete_suggestion() {
        let suggestion = ActionableSuggestion {
            message: "Test suggestion".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.85,
            reversible: true,
            estimated_impact: ImpactLevel::Low,
            refactor_call: Some(RefactorCall {
                tool: "delete.plan".to_string(),
                arguments: json!({
                    "file_path": "test.rs",
                    "line": 10,
                }),
            }),
            metadata: None,
        };

        validate_suggestion(&suggestion).unwrap();
    }

    #[test]
    fn test_validate_missing_refactor_call() {
        let suggestion = ActionableSuggestion {
            message: "Test".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.8,
            reversible: true,
            estimated_impact: ImpactLevel::Low,
            refactor_call: None, // Invalid for Safe suggestion
            metadata: None,
        };

        assert!(validate_suggestion(&suggestion).is_err());
    }

    #[test]
    fn test_validate_invalid_confidence() {
        let suggestion = ActionableSuggestion {
            message: "Test".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 1.5, // Out of range
            reversible: true,
            estimated_impact: ImpactLevel::Low,
            refactor_call: None,
            metadata: None,
        };

        assert!(validate_suggestion(&suggestion).is_err());
    }
}
```

#### CI Test File: `integration-tests/src/test_suggestion_validation.rs`

```rust
use crate::harness::*;

#[tokio::test]
async fn test_all_suggestions_pass_validation() {
    let server = setup_test_server().await;

    // Test files covering all analysis categories
    let test_cases = vec![
        ("test_data/complex.ts", "analyze.quality", vec!["complexity"]),
        ("test_data/unused.ts", "analyze.dead_code", vec!["unused_import"]),
        ("test_data/circular.ts", "analyze.dependencies", vec!["circular"]),
        ("test_data/hierarchy.ts", "analyze.structure", vec!["hierarchy"]),
        ("test_data/undocumented.ts", "analyze.documentation", vec!["coverage"]),
        ("test_data/untested.ts", "analyze.tests", vec!["coverage"]),
    ];

    for (file, tool, kinds) in test_cases {
        let result = server
            .call_tool(
                tool,
                json!({
                    "file_path": file,
                    "kinds": kinds,
                }),
            )
            .await
            .unwrap();

        let findings: Vec<Finding> = serde_json::from_value(result["findings"].clone()).unwrap();

        for finding in findings {
            for suggestion in finding.suggestions {
                validate_suggestion(&suggestion).expect(&format!(
                    "Invalid suggestion in {} (tool: {}): {:?}",
                    file, tool, suggestion
                ));
            }
        }
    }
}
```

---

### Part 4: Documentation Updates

#### API_REFERENCE.md

Add section after existing `analyze.*` tools:

```markdown
### Actionable Suggestions

All analysis tools now return suggestions with safety metadata:

**Suggestion Structure**:
```json
{
  "message": "Extract helper methods to reduce complexity",
  "safety": "safe" | "requires_review" | "experimental",
  "confidence": 0.85,
  "reversible": true,
  "estimated_impact": "low" | "medium" | "high" | "critical",
  "refactor_call": {
    "tool": "extract.plan",
    "arguments": { ... }
  },
  "metadata": {
    "rationale": "...",
    "benefits": [...],
    "risks": [...]
  }
}
```

**Safety Levels**:
- `"safe"` - Auto-apply without review (e.g., remove unused import)
- `"requires_review"` - Human review recommended (e.g., extract method)
- `"experimental"` - High-risk, test thoroughly (e.g., cross-crate moves)

**Confidence Score**: 0.0 to 1.0 (higher = more confident)

**Configuration**: See `.codebuddy/analysis.toml` for tuning suggestion generation.
```

#### CLAUDE.md

Add section after Analysis tools:

```markdown
### Actionable Suggestions Configuration

Configure suggestion generation in `.codebuddy/analysis.toml`:

```toml
[suggestions]
min_confidence = 0.7  # Minimum confidence threshold
include_safety_levels = ["safe", "requires_review"]
max_per_finding = 3
generate_refactor_calls = true
```

**Presets**:
- `strict` - Only safe suggestions, high confidence
- `default` - Safe + requires_review, medium confidence
- `relaxed` - All levels, low confidence
```

---

## Testing Strategy

### Configuration Tests

```rust
#[test]
fn test_load_default_config() {
    let config = SuggestionConfig::default();
    assert_eq!(config.min_confidence, 0.7);
    assert_eq!(config.max_per_finding, 3);
}

#[test]
fn test_config_filtering() {
    let config = SuggestionConfig {
        min_confidence: 0.8,
        include_safety_levels: [SafetyLevel::Safe].iter().copied().collect(),
        max_per_finding: 2,
        ..Default::default()
    };

    let suggestions = vec![
        ActionableSuggestion {
            safety: SafetyLevel::Safe,
            confidence: 0.9,
            ..Default::default()
        },
        ActionableSuggestion {
            safety: SafetyLevel::RequiresReview,
            confidence: 0.85,
            ..Default::default()
        },
        ActionableSuggestion {
            safety: SafetyLevel::Safe,
            confidence: 0.75,
            ..Default::default()
        },
    ];

    let filtered = config.filter(suggestions);
    assert_eq!(filtered.len(), 1); // Only first one passes
}
```

### Batch Integration Test

```rust
#[tokio::test]
async fn test_batch_generates_suggestions() {
    let server = setup_test_server().await;

    let result = server.call_tool(
        "analyze.batch",
        json!({
            "queries": [
                {
                    "command": "analyze.quality",
                    "kind": "complexity",
                    "scope": { "file_path": "test_data/complex.ts" }
                },
                {
                    "command": "analyze.dead_code",
                    "kind": "unused_import",
                    "scope": { "file_path": "test_data/unused.ts" }
                }
            ]
        }),
    ).await.unwrap();

    let results: Vec<AnalysisResult> = serde_json::from_value(result["results"].clone()).unwrap();

    // Both results should have suggestions
    for result in results {
        for finding in result.findings {
            assert!(!finding.suggestions.is_empty(), "Batch should generate suggestions");
        }
    }
}
```

---

## Success Criteria

### Configuration
- [ ] `SuggestionConfig` loads from `.codebuddy/analysis.toml`
- [ ] Default config used when file missing
- [ ] Presets (strict, default, relaxed) work correctly
- [ ] Filters applied correctly (confidence, safety, impact)

### Batch Integration
- [ ] `analyze.batch` generates suggestions for all queries
- [ ] AST caching still works correctly
- [ ] Configuration respected in batch mode

### CI Validation
- [ ] `validate_suggestion()` checks all required fields
- [ ] Tool-specific argument validation works
- [ ] CI test runs on all categories and catches invalid suggestions

### Documentation
- [ ] API_REFERENCE.md updated with suggestion structure
- [ ] CLAUDE.md updated with configuration examples
- [ ] QUICK_REFERENCE.md includes suggestion examples

---

## Merge Requirements

Before merging to main:
1. All tests passing (unit + integration)
2. Documentation complete and reviewed
3. Clippy clean
4. 01c1 merged
5. 01c2 can be parallel or after (coordinate timing)

After merge:
- Tag as `01c3-system-complete`
- Full actionable suggestions feature is complete

---

**Status**: 📋 Ready for Implementation (Week 2 - After 01c1 50% complete, parallel with 01c2)
