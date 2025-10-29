# Proposal 08: Batch Analysis Integration with Actionable Suggestions

## Problem

The `analyze.batch` tool needs integration with Proposal 00's actionable suggestions system:

1. **No Suggestions**: Batch analysis doesn't generate actionable suggestions
2. **Inconsistent Output**: Single-file analysis has suggestions, batch doesn't
3. **Manual Aggregation**: Users must manually combine findings from multiple files
4. **Lost Context**: Can't see cross-file patterns or workspace-level issues
5. **Performance Gap**: No caching strategy for multi-file analysis

Example workflow that doesn't work:
- Analyze 50 files for code quality issues
- Get raw findings but no suggestions
- Must manually determine which tools to run
- No cross-file deduplication or priority ranking

## Solution(s)

### 1. Integrate Suggestion Generation

Extend `analyze.batch` to generate suggestions per file:

```rust
pub struct BatchAnalysisResult {
    pub file_results: Vec<FileAnalysisResult>,
    pub summary: BatchSummary,
    pub suggestions: Vec<ActionableSuggestion>,  // NEW
}

pub struct FileAnalysisResult {
    pub file_path: PathBuf,
    pub findings: Vec<AnalysisFinding>,
    pub suggestions: Vec<ActionableSuggestion>,  // NEW
}
```text
### 2. Cross-File Suggestion Deduplication

Avoid duplicate suggestions across files:

```rust
pub fn deduplicate_suggestions(
    file_results: &[FileAnalysisResult]
) -> Vec<ActionableSuggestion> {
    // Group by tool + similar parameters
    // Keep highest confidence version
    // Merge affected files list
}
```text
### 3. Workspace-Level Suggestions

Generate suggestions for workspace-wide patterns:

```rust
pub fn generate_workspace_suggestions(
    findings: &[AnalysisFinding]
) -> Vec<ActionableSuggestion> {
    // Pattern: Similar code smells across files
    // Suggestion: Batch transform operation

    // Pattern: Unused imports in multiple files
    // Suggestion: Workspace-wide cleanup
}
```text
### 4. Optimized Caching Strategy

Cache AST and analysis results:

```rust
pub struct BatchAnalysisCache {
    ast_cache: HashMap<PathBuf, ParsedSource>,
    analysis_cache: HashMap<PathBuf, Vec<AnalysisFinding>>,
}
```text
### 5. Suggestion Ranking

Prioritize suggestions across files:

```rust
pub fn rank_suggestions(
    suggestions: Vec<ActionableSuggestion>
) -> Vec<ActionableSuggestion> {
    // Sort by: confidence, impact, affected files
    // Group by category
    // Limit to top N per category
}
```text
## Checklists

### Data Structures
- [ ] Add `suggestions` field to `BatchAnalysisResult`
- [ ] Add `suggestions` field to `FileAnalysisResult`
- [ ] Add `BatchSummary` with aggregated suggestion counts
- [ ] Add `WorkspaceSuggestion` for cross-file patterns

### Suggestion Generation
- [ ] Generate suggestions per file during batch analysis
- [ ] Implement cross-file deduplication logic
- [ ] Implement workspace-level pattern detection
- [ ] Implement suggestion ranking algorithm

### Caching
- [ ] Implement `BatchAnalysisCache` for AST reuse
- [ ] Add cache invalidation on file changes
- [ ] Add cache statistics to output
- [ ] Optimize memory usage for large batches

### Integration
- [ ] Update `analyze.batch` to call suggestion generator
- [ ] Update output format to include suggestions
- [ ] Add `--no-suggestions` flag to disable
- [ ] Add `--max-suggestions` parameter

### Output Formats
- [ ] Add suggestions section to JSON output
- [ ] Add suggestions section to human-readable output
- [ ] Add summary statistics for suggestions
- [ ] Add file-level grouping option

### Testing
- [ ] Test suggestion generation for batch analysis
- [ ] Test deduplication across files
- [ ] Test workspace-level suggestions
- [ ] Test caching effectiveness
- [ ] Test with large file sets (50+ files)

### Documentation
- [ ] Document batch analysis with suggestions
- [ ] Document suggestion deduplication logic
- [ ] Document workspace-level patterns
- [ ] Add examples for common use cases

## Success Criteria

1. **Consistent Output**: Batch analysis includes suggestions like single-file analysis
2. **No Duplicates**: Cross-file suggestions are deduplicated intelligently
3. **Workspace Patterns**: Detects and suggests cross-file improvements
4. **Performance**: Batch analysis with suggestions completes in reasonable time (< 5s for 50 files)
5. **Ranking**: Suggestions are prioritized by confidence and impact
6. **Opt-Out**: Users can disable suggestions with `--no-suggestions` flag

## Benefits

1. **Consistent UX**: All analysis tools provide actionable suggestions
2. **Efficiency**: One batch command instead of multiple single-file analyses
3. **Better Insights**: Cross-file patterns detected automatically
4. **Reduced Noise**: Deduplication prevents duplicate suggestions
5. **Scalability**: Caching enables analysis of large codebases
6. **Actionable Output**: Users know exactly what to do next