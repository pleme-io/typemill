# Proposal 01c2: Actionable Suggestions - Analysis Integration

**Status**: ⚠️ **PARTIALLY COMPLETE** (Dead Code Integration Only)
**Author**: Project Team
**Date**: 2025-10-13 (Split from 01c)
**Parent Proposal**: [01b_unified_analysis_api.md](01b_unified_analysis_api.md)
**Dependencies**: ✅ 01a, ✅ 01b, ✅ 01c1 (Core Infrastructure - MERGED)
**Branch**: ✅ `feature/01c2-suggestions-integration` (MERGED - Partial)

---

## Executive Summary

**What**: Integrate suggestion generation into all 6 analysis category handlers, generating refactor_call structures for each finding kind.

**Why**: Complete the bridge between analysis (what's wrong) and refactoring (how to fix it).

**Impact**: AI agents can now receive actionable suggestions with every analysis result.

**Depends On**: ✅ 01c1 (Core Infrastructure) - MERGED

**Current Status**: ⚠️ PARTIAL IMPLEMENTATION MERGED
- ✅ Dead code integration (2 of 6 kinds: unused_imports, unused_symbols)
- ❌ Remaining 4 dead code kinds (unreachable_code, unused_parameters, unused_types, unused_variables)
- ❌ Quality, dependencies, structure, documentation, tests categories
- ❌ Closed-loop workflow test

**Known Issues**: 5 of 6 integration tests fail because suggestion generation only works for 2 dead code kinds.

---

## Scope - What This Branch Delivers

### Category Integration ⚠️ PARTIAL
Integrate `SuggestionGenerator` into analysis handlers:
- ❌ `analyze.quality` (complexity, code smells, maintainability, readability) - NOT STARTED
- ⚠️ `analyze.dead_code` (unused imports, symbols) - **PARTIAL: 2 of 6 kinds**
  - ✅ unused_imports - suggestion generation working
  - ✅ unused_symbols - suggestion generation working
  - ❌ unreachable_code - NO suggestions (still uses old path)
  - ❌ unused_parameters - NO suggestions (still uses old path)
  - ❌ unused_types - NO suggestions (still uses old path)
  - ❌ unused_variables - NO suggestions (still uses old path)
- ❌ `analyze.dependencies` (circular deps, coupling, cohesion) - NOT STARTED
- ❌ `analyze.structure` (hierarchy, interfaces, inheritance) - NOT STARTED
- ❌ `analyze.documentation` (coverage, quality, style) - NOT STARTED
- ❌ `analyze.tests` (coverage, quality, assertions) - NOT STARTED

### Refactoring Generators ⚠️ PARTIAL
Implement finding-specific refactoring candidate generators:
- ❌ **Quality**: Complexity → extract method, simplify boolean - NOT STARTED
- ⚠️ **Dead Code**: Unused → delete (PARTIAL - only 2 kinds generate candidates)
  - ✅ `generate_dead_code_refactoring_candidates()` function exists
  - ✅ Maps dead code findings to `delete.plan` refactor_call
  - ❌ Only integrated for unused_imports and unused_symbols
- ❌ **Dependencies**: Circular deps → move/restructure - NOT STARTED
- ❌ **Structure**: Poor hierarchy → reorganize - NOT STARTED
- ❌ **Documentation**: Missing docs → add documentation - NOT STARTED
- ❌ **Tests**: Low coverage → suggest test additions - NOT STARTED

### Testing ⚠️ PARTIAL
- ⚠️ Integration tests for dead code category (6 tests, 5 FAILING)
  - ✅ test_dead_code_analysis_generates_suggestions_for_unused_import - PASSING
  - ❌ test_dead_code_analysis_generates_suggestions_for_unused_function - FAILING
  - ❌ test_dead_code_analysis_generates_suggestions_for_unreachable_code - FAILING
  - ❌ test_dead_code_analysis_generates_suggestions_for_unused_parameter - FAILING
  - ❌ test_dead_code_analysis_generates_suggestions_for_unused_type - FAILING
  - ❌ test_dead_code_analysis_generates_suggestions_for_unused_variable - FAILING
- ❌ Integration tests for other 5 categories - NOT STARTED
- ❌ End-to-end closed-loop workflow test - NOT STARTED

---

## Out of Scope - What This Branch Does NOT Deliver

❌ Core infrastructure (SafetyClassifier, etc.) - That's 01c1
❌ Configuration system - That's 01c3
❌ `analyze.batch` integration - That's 01c3
❌ CI validation - That's 01c3
❌ Documentation updates - That's 01c3

---

## Implementation Pattern (All Handlers Follow This)

### Step 1: Add SuggestionGenerator to Handler

```rust
// Example: crates/cb-handlers/src/handlers/tools/analysis/quality.rs

use crate::handlers::tools::analysis::suggestions::{
    SuggestionGenerator, AnalysisContext, RefactoringCandidate,
};

pub async fn analyze_quality(
    params: AnalysisParams,
    lsp_client: &LspClient,
    ast_service: &AstService,
) -> Result<AnalysisResult> {
    // ... existing analysis logic ...

    // Generate findings
    let mut findings = detect_quality_issues(&parsed_source, &params)?;

    // NEW: Initialize suggestion generator
    let suggestion_generator = SuggestionGenerator::new();

    // NEW: Enhance findings with actionable suggestions
    for finding in &mut findings {
        let candidates = generate_quality_refactoring_candidates(finding, &parsed_source)?;

        let context = AnalysisContext {
            file_path: params.file_path.clone(),
            has_full_type_info: lsp_client.has_type_info(),
            has_partial_type_info: parsed_source.has_type_annotations(),
            ast_parse_errors: parsed_source.errors.len(),
        };

        let mut suggestions = Vec::new();
        for candidate in candidates {
            match suggestion_generator.generate_from_candidate(candidate, &context) {
                Ok(suggestion) => suggestions.push(suggestion),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        finding_kind = %finding.kind,
                        "Failed to generate suggestion"
                    );
                }
            }
        }

        finding.suggestions = suggestions;
    }

    Ok(AnalysisResult {
        category: "quality".to_string(),
        findings,
        // ... other fields
    })
}
```

### Step 2: Implement Refactoring Candidate Generator

```rust
// Example: Quality analysis refactoring generators

fn generate_quality_refactoring_candidates(
    finding: &Finding,
    parsed_source: &ParsedSource,
) -> Result<Vec<RefactoringCandidate>> {
    let mut candidates = Vec::new();

    match finding.kind.as_str() {
        "complexity" => {
            candidates.extend(generate_complexity_candidates(finding, parsed_source)?);
        }
        "code_smell" => {
            candidates.extend(generate_code_smell_candidates(finding, parsed_source)?);
        }
        "maintainability" => {
            candidates.extend(generate_maintainability_candidates(finding, parsed_source)?);
        }
        "readability" => {
            candidates.extend(generate_readability_candidates(finding, parsed_source)?);
        }
        _ => {}
    }

    Ok(candidates)
}

fn generate_complexity_candidates(
    finding: &Finding,
    _parsed_source: &ParsedSource,
) -> Result<Vec<RefactoringCandidate>> {
    let mut candidates = Vec::new();

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
            has_side_effects: false,
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
                // Would need deeper analysis to determine exact range
            }),
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
            evidence_strength: EvidenceStrength::Weak,
            location: finding.location.clone(),
            refactor_call_args: json!({
                "file_path": finding.location.file,
                "start_line": finding.location.line,
                "end_line": finding.location.line,
                "transformation_kind": "simplify_boolean",
            }),
        });
    }

    Ok(candidates)
}
```

---

## Category-Specific Generators

### Quality (Agent 1)

**Generators needed**:
- `generate_complexity_candidates()` - Extract method, simplify boolean
- `generate_code_smell_candidates()` - Refactor smelly patterns
- `generate_maintainability_candidates()` - Improve structure
- `generate_readability_candidates()` - Rename, format

**Mapping**:
- Complexity > 10 → `extract.plan` (extract method)
- Complexity > 15 → `transform.plan` (simplify boolean)
- Long method → `extract.plan`
- Deep nesting → `transform.plan` (flatten)

### Dead Code (Agent 1)

**Generators needed**:
- `generate_unused_import_candidates()` - Delete unused imports
- `generate_unused_variable_candidates()` - Delete unused variables
- `generate_unused_function_candidates()` - Delete unused functions
- `generate_unreachable_code_candidates()` - Remove unreachable code

**Mapping**:
- Unused import → `delete.plan` (remove import)
- Unused variable → `delete.plan` (remove variable)
- Unused function → `delete.plan` (remove function)
- Unreachable code → `delete.plan` (remove block)

### Dependencies (Agent 1)

**Generators needed**:
- `generate_circular_dependency_candidates()` - Move/restructure to break cycle
- `generate_high_coupling_candidates()` - Extract interface, dependency injection
- `generate_low_cohesion_candidates()` - Split module

**Mapping**:
- Circular dependency → `move.plan` (move to break cycle)
- High coupling → `extract.plan` (extract interface)
- Low cohesion → Split module (may need multiple refactorings)

### Structure (Agent 2)

**Generators needed**:
- `generate_hierarchy_candidates()` - Improve inheritance structure
- `generate_interface_candidates()` - Extract interface
- `generate_module_candidates()` - Reorganize modules

**Mapping**:
- Deep hierarchy → `move.plan` (flatten hierarchy)
- Missing interface → `extract.plan` (extract interface)
- Poor module organization → `move.plan` (reorganize)

### Documentation (Agent 2)

**Generators needed**:
- `generate_missing_doc_candidates()` - Add documentation
- `generate_outdated_doc_candidates()` - Update documentation
- `generate_style_violation_candidates()` - Fix doc style

**Mapping**:
- Missing docs → Suggest doc template (no refactor_call yet - needs new tool)
- Outdated docs → Suggest update (no refactor_call yet)
- Style violation → `transform.plan` (format docs)

**Note**: Documentation suggestions may need a new `add_documentation` tool in the future.

### Tests (Agent 2)

**Generators needed**:
- `generate_missing_test_candidates()` - Add test
- `generate_weak_assertion_candidates()` - Improve assertions
- `generate_test_organization_candidates()` - Reorganize tests

**Mapping**:
- Missing test → Suggest test template (no refactor_call yet - needs new tool)
- Weak assertion → Suggest improvement (no refactor_call yet)
- Poor organization → `move.plan` (reorganize test files)

**Note**: Test suggestions may need a new `generate_test` tool in the future.

---

## Integration Testing

### Per-Category Integration Tests

Each agent writes 3 integration tests (one per category they own).

Example for Quality (Agent 1):

```rust
// integration-tests/src/test_suggestions_quality.rs

#[tokio::test]
async fn test_quality_analysis_generates_suggestions() {
    let server = setup_test_server().await;

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
```

### Closed-Loop Workflow Test (Shared)

One agent writes this test demonstrating the full workflow:

```rust
// integration-tests/src/test_closed_loop_workflow.rs

#[tokio::test]
async fn test_closed_loop_workflow_dead_code_removal() {
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
    assert!(!findings.is_empty(), "Should have findings");

    // Step 2: Find safe suggestion
    let safe_suggestion = findings[0].suggestions.iter()
        .find(|s| s.safety == SafetyLevel::Safe && s.confidence > 0.9)
        .expect("Should have safe suggestion");

    // Step 3: Apply suggestion via refactor_call
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

    // Step 4: Re-analyze to verify fix
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

---

## Success Criteria

### Integration
- [ ] All 6 analysis handlers call `SuggestionGenerator` - **0/6 COMPLETE**
  - [ ] analyze.quality - NOT STARTED
  - [x] analyze.dead_code - **PARTIAL (2/6 kinds)**
    - [x] unused_imports - DONE
    - [x] unused_symbols - DONE
    - [ ] unreachable_code - TODO
    - [ ] unused_parameters - TODO
    - [ ] unused_types - TODO
    - [ ] unused_variables - TODO
  - [ ] analyze.dependencies - NOT STARTED
  - [ ] analyze.structure - NOT STARTED
  - [ ] analyze.documentation - NOT STARTED
  - [ ] analyze.tests - NOT STARTED
- [x] Suggestions generated for every finding where applicable (for completed kinds)
- [x] Errors in suggestion generation logged but don't fail analysis
- [x] All `refactor_call` fields populated with valid tool names and arguments (for completed kinds)

### Testing
- [ ] 6 integration tests (one per category) passing - **1/6 COMPLETE**
  - [x] Dead code tests exist - **1/6 passing, 5/6 failing**
  - [ ] Quality tests - NOT STARTED
  - [ ] Dependencies tests - NOT STARTED
  - [ ] Structure tests - NOT STARTED
  - [ ] Documentation tests - NOT STARTED
  - [ ] Tests tests - NOT STARTED
- [ ] 1 closed-loop workflow test passing - NOT STARTED
- [x] No regressions in existing analysis tests

### Code Quality
- [x] Zero clippy warnings
- [x] Proper error handling (no unwrap/expect in production code)
- [x] Structured logging for suggestion generation

---

## Merge Strategy

### Parallel Development
- Agent 1 works on branch `feature/01c2-suggestions-agent1`
- Agent 2 works on branch `feature/01c2-suggestions-agent2`

### Merge Order
1. Agent 1 merges their branch to `feature/01c2-suggestions-integration`
2. Agent 2 rebases on Agent 1's changes
3. Agent 2 merges their branch to `feature/01c2-suggestions-integration`
4. Full integration test suite runs
5. Merge `feature/01c2-suggestions-integration` to `main`

### Conflict Prevention
- Each agent owns 3 separate files (quality.rs vs structure.rs, etc.)
- Minimal overlap - only shared dependency on 01c1 core types
- Coordinate on who writes `test_closed_loop_workflow.rs` up front

---

## Merge Requirements

Before merging to main:
1. All 7 integration tests passing
2. Code review approved
3. Clippy clean (zero warnings)
4. No breaking changes to existing APIs
5. 01c1 must be merged first

After merge:
- Tag as `01c2-integration-complete`
- Ready for 01c3 (System Integration) to start

---

**Status**: 📋 Ready for Implementation (Week 2 - Depends on 01c1)
