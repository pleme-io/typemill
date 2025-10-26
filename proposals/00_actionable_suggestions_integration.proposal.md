# Proposal 00: Actionable Suggestions - Analysis Integration

**Status**: ✅ **DEAD CODE COMPLETE** | ⚠️ Other categories pending
**Author**: Project Team
**Date**: 2025-10-13 (Split from 01c)
**Completion Date**: 2025-10-18 (Dead Code)
**Parent Proposal**: [01b_unified_analysis_api.md](01b_unified_analysis_api.md)
**Dependencies**: ✅ 01a, ✅ 01b, ✅ 01c1 (Core Infrastructure - MERGED)
**Branch**: ✅ `feature/01c2-suggestions-integration` (MERGED)

> **API Update Note (Phase 5)**: This proposal has been updated to use the unified dryRun API. Code examples now show single-tool execution with the `options.dryRun` parameter.
>
> **Historical Context (Pre-Phase 5, REMOVED)**: The two-step workflow with separate `.plan` tools and `workspace.apply_edit` has been replaced.
>
> ```diff
> - // Old: Two-step workflow (REMOVED)
> - const plan = await rename.plan({ target: {...}, newName: "..." });
> - await workspace.apply_edit({ edits: plan.edits });
>
> + // New: Single-step with dryRun option
> + await rename({ target: {...}, newName: "...", options: { dryRun: false } });
> ```
>
> **Current API (Phase 5+)**: All refactoring tools accept `options.dryRun: true` (preview) or `false` (execute). See [docs/tools/refactoring.md](../docs/tools/refactoring.md) for complete API reference.

---

## Executive Summary

**What**: Integrate suggestion generation into all 6 analysis category handlers, generating refactor_call structures for each finding kind.

**Why**: Complete the bridge between analysis (what's wrong) and refactoring (how to fix it).

**Impact**: AI agents can now receive actionable suggestions with every analysis result.

**Depends On**: ✅ 01c1 (Core Infrastructure) - MERGED

**Current Status**: ✅ **DEAD CODE COMPLETE (All 6 kinds)**
- ✅ Dead code integration (6 of 6 kinds complete)
  - ✅ unused_imports - COMPLETE
  - ✅ unused_symbols - COMPLETE
  - ✅ unreachable_code - COMPLETE
  - ✅ unused_parameters - COMPLETE
  - ✅ unused_types - COMPLETE
  - ✅ unused_variables - COMPLETE
- ✅ All 6 dead code integration tests passing
- ❌ Quality analysis - NOT STARTED
- ❌ Dependencies analysis - NOT STARTED
- ❌ Structure analysis - NOT STARTED
- ❌ Documentation analysis - NOT STARTED
- ❌ Tests analysis - NOT STARTED
- ❌ Closed-loop workflow test - NOT STARTED

**Test Results**: ✅ 6/6 dead code tests passing, 822/822 workspace tests passing

---

## Scope - What This Branch Delivers

### Category Integration ✅ DEAD CODE COMPLETE
Integrate `SuggestionGenerator` into analysis handlers:
- ❌ `analyze.quality` (complexity, code smells, maintainability, readability) - NOT STARTED
- ✅ `analyze.dead_code` (unused imports, symbols) - **COMPLETE: All 6 kinds**
  - ✅ unused_imports - COMPLETE
  - ✅ unused_symbols - COMPLETE
  - ✅ unreachable_code - COMPLETE
  - ✅ unused_parameters - COMPLETE
  - ✅ unused_types - COMPLETE
  - ✅ unused_variables - COMPLETE
- ❌ `analyze.dependencies` (circular deps, coupling, cohesion) - NOT STARTED
- ❌ `analyze.structure` (hierarchy, interfaces, inheritance) - NOT STARTED
- ❌ `analyze.documentation` (coverage, quality, style) - NOT STARTED
- ❌ `analyze.tests` (coverage, quality, assertions) - NOT STARTED

### Refactoring Generators ✅ DEAD CODE COMPLETE
Implement finding-specific refactoring candidate generators:
- ❌ **Quality**: Complexity → extract method, simplify boolean - NOT STARTED
- ✅ **Dead Code**: Unused → delete (COMPLETE - all 6 kinds generate candidates)
  - ✅ `generate_dead_code_refactoring_candidates()` function implemented
  - ✅ Maps all dead code finding kinds to `delete (with dryRun option)` refactor_call
  - ✅ Integrated for all 6 kinds: imports, symbols, unreachable, parameters, types, variables
- ❌ **Dependencies**: Circular deps → move/restructure - NOT STARTED
- ❌ **Structure**: Poor hierarchy → reorganize - NOT STARTED
- ❌ **Documentation**: Missing docs → add documentation - NOT STARTED
- ❌ **Tests**: Low coverage → suggest test additions - NOT STARTED

### Testing ✅ DEAD CODE COMPLETE
- ✅ Integration tests for dead code category (6 tests, all PASSING)
  - ✅ test_dead_code_analysis_generates_suggestions_for_unused_import - PASSING
  - ✅ test_dead_code_analysis_generates_suggestions_for_unused_function - PASSING
  - ✅ test_dead_code_analysis_generates_suggestions_for_unreachable_code - PASSING
  - ✅ test_dead_code_analysis_generates_suggestions_for_unused_parameter - PASSING
  - ✅ test_dead_code_analysis_generates_suggestions_for_unused_type - PASSING
  - ✅ test_dead_code_analysis_generates_suggestions_for_unused_variable - PASSING
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
// Example: ../../crates/mill-handlers/src/handlers/tools/analysis/quality.rs

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
- Complexity > 10 → `extract (with dryRun option)` (extract method)
- Complexity > 15 → `transform (with dryRun option)` (simplify boolean)
- Long method → `extract (with dryRun option)`
- Deep nesting → `transform (with dryRun option)` (flatten)

### Dead Code (Agent 1)

**Generators needed**:
- `generate_unused_import_candidates()` - Delete unused imports
- `generate_unused_variable_candidates()` - Delete unused variables
- `generate_unused_function_candidates()` - Delete unused functions
- `generate_unreachable_code_candidates()` - Remove unreachable code

**Mapping**:
- Unused import → `delete (with dryRun option)` (remove import)
- Unused variable → `delete (with dryRun option)` (remove variable)
- Unused function → `delete (with dryRun option)` (remove function)
- Unreachable code → `delete (with dryRun option)` (remove block)

### Dependencies (Agent 1)

**Generators needed**:
- `generate_circular_dependency_candidates()` - Move/restructure to break cycle
- `generate_high_coupling_candidates()` - Extract interface, dependency injection
- `generate_low_cohesion_candidates()` - Split module

**Mapping**:
- Circular dependency → `move (with dryRun option)` (move to break cycle)
- High coupling → `extract (with dryRun option)` (extract interface)
- Low cohesion → Split module (may need multiple refactorings)

### Structure (Agent 2)

**Generators needed**:
- `generate_hierarchy_candidates()` - Improve inheritance structure
- `generate_interface_candidates()` - Extract interface
- `generate_module_candidates()` - Reorganize modules

**Mapping**:
- Deep hierarchy → `move (with dryRun option)` (flatten hierarchy)
- Missing interface → `extract (with dryRun option)` (extract interface)
- Poor module organization → `move (with dryRun option)` (reorganize)

### Documentation (Agent 2)

**Generators needed**:
- `generate_missing_doc_candidates()` - Add documentation
- `generate_outdated_doc_candidates()` - Update documentation
- `generate_style_violation_candidates()` - Fix doc style

**Mapping**:
- Missing docs → Suggest doc template (no refactor_call yet - needs new tool)
- Outdated docs → Suggest update (no refactor_call yet)
- Style violation → `transform (with dryRun option)` (format docs)

**Note**: Documentation suggestions may need a new `add_documentation` tool in the future.

### Tests (Agent 2)

**Generators needed**:
- `generate_missing_test_candidates()` - Add test
- `generate_weak_assertion_candidates()` - Improve assertions
- `generate_test_organization_candidates()` - Reorganize tests

**Mapping**:
- Missing test → Suggest test template (no refactor_call yet - needs new tool)
- Weak assertion → Suggest improvement (no refactor_call yet)
- Poor organization → `move (with dryRun option)` (reorganize test files)

**Note**: Test suggestions may need a new `generate_test` tool in the future.

---

## Integration Testing

### Per-Category Integration Tests

Each agent writes 3 integration tests (one per category they own).

Example for Quality (Agent 1):

```rust
// ../tests/e2e/src/test_suggestions_quality.rs

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
// ../tests/e2e/src/test_closed_loop_workflow.rs

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

    // Step 3: Apply suggestion via refactor_call (unified dryRun API)
    let refactor_call = safe_suggestion.refactor_call.as_ref().unwrap();

    // Add dryRun: false to execute the refactoring
    let mut arguments = refactor_call.arguments.clone();
    arguments["options"] = json!({ "dryRun": false });

    let apply_result = server.call_tool(
        &refactor_call.tool,
        arguments,
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

### Integration ✅ DEAD CODE COMPLETE
- [ ] All 6 analysis handlers call `SuggestionGenerator` - **1/6 COMPLETE**
  - [ ] analyze.quality - NOT STARTED
  - [x] analyze.dead_code - **COMPLETE (6/6 kinds)**
    - [x] unused_imports - COMPLETE
    - [x] unused_symbols - COMPLETE
    - [x] unreachable_code - COMPLETE
    - [x] unused_parameters - COMPLETE
    - [x] unused_types - COMPLETE
    - [x] unused_variables - COMPLETE
  - [ ] analyze.dependencies - NOT STARTED
  - [ ] analyze.structure - NOT STARTED
  - [ ] analyze.documentation - NOT STARTED
  - [ ] analyze.tests - NOT STARTED
- [x] Suggestions generated for every finding where applicable (dead code complete)
- [x] Errors in suggestion generation logged but don't fail analysis
- [x] All `refactor_call` fields populated with valid tool names and arguments

### Testing ✅ DEAD CODE COMPLETE
- [ ] 6 integration tests (one per category) passing - **1/6 COMPLETE**
  - [x] Dead code tests - **6/6 passing**
  - [ ] Quality tests - NOT STARTED
  - [ ] Dependencies tests - NOT STARTED
  - [ ] Structure tests - NOT STARTED
  - [ ] Documentation tests - NOT STARTED
  - [ ] Tests tests - NOT STARTED
- [ ] 1 closed-loop workflow test passing - NOT STARTED
- [x] No regressions in existing analysis tests (822/822 passing)

### Code Quality ✅ COMPLETE
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

## 🎉 Completion Summary (Dead Code - 2025-10-18)

### What Was Accomplished

**Dead Code Suggestion Generation - 100% Complete**

All 6 dead code analysis kinds now generate actionable `delete (with dryRun option)` suggestions:

1. ✅ **unused_imports** - Removes entire import statements or unused symbols
2. ✅ **unused_symbols** - Removes unused functions/classes/variables
3. ✅ **unreachable_code** - Removes code after return/throw/break/continue
4. ✅ **unused_parameters** - Removes unused function parameters
5. ✅ **unused_types** - Removes unused type definitions (interfaces, enums, structs)
6. ✅ **unused_variables** - Removes unused local variable declarations

**Implementation Location:**
- Handler: `../../crates/mill-handlers/src/handlers/tools/analysis/dead_code.rs`
- Generator: `generate_dead_code_refactoring_candidates()` (lines 1445-1489)
- Integration: Lines 1978-2171 (all 6 kinds)

**Test Coverage:**
- ✅ 6/6 integration tests passing
- ✅ 822/822 workspace tests passing (no regressions)
- Test file: `tests/e2e/src/test_suggestions_dead_code.rs`

**Value Delivered:**
- AI agents can now automatically fix unused code issues
- Each suggestion includes:
  - `refactor_call` with `delete (with dryRun option)` tool and arguments
  - Safety level (Safe or RequiresReview)
  - Confidence score (0.7-0.9)
  - Reversibility flag
  - Estimated impact description

### What Remains (Deferred to Future Proposals)

**5 Analysis Categories (Not Started):**
1. ❌ Quality (complexity, code smells, maintainability, readability)
2. ❌ Dependencies (circular deps, coupling, cohesion)
3. ❌ Structure (hierarchy, interfaces, modules)
4. ❌ Documentation (coverage, quality, style)
5. ❌ Tests (coverage, quality, assertions)

**Future Work:**
- Closed-loop workflow test (analysis → suggestion → apply → verify)
- Quality analysis suggestions (extract method, simplify boolean)
- Dependencies analysis suggestions (move to break cycles)
- Structure analysis suggestions (reorganize modules)
- Documentation/Tests may need new MCP tools (`add_documentation`, `generate_test`)

**Recommendation:**
Create separate proposals for each remaining category:
- **Proposal 00b**: Quality Analysis Suggestions (6 hours)
- **Proposal 00c**: Dependencies Analysis Suggestions (4 hours)
- **Proposal 00d**: Structure Analysis Suggestions (4 hours)

---

**Status**: ✅ Dead Code Complete (2025-10-18) | ⚠️ Other categories deferred
