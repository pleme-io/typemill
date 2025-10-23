# Complete snake_case → camelCase Conversion Inventory

**Date**: 2025-10-23
**Purpose**: Comprehensive mapping for converting JSON API from snake_case to camelCase
**Context**: MCP/LSP protocols use camelCase. Previous commit (601ed764) incorrectly removed camelCase annotations.

---

## Executive Summary

**Total Structs Requiring Annotation**: 45 public API parameter structs
**Total Unique snake_case Fields**: 82 field names
**Total Test Occurrences**: 367+ JSON field references across 30+ test files
**Current State**: Mixed - EditPlan uses camelCase, most params use snake_case

---

## Part 1: Complete Struct Inventory

### A. Refactoring Operation Params (Handlers - Public API)

#### 1. Rename Handler (`/workspace/crates/mill-handlers/src/handlers/rename_handler/mod.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 39-53
#[derive(Debug, Deserialize)]
pub(crate) struct RenamePlanParams {
    target: Option<RenameTarget>,           // → target (no change)
    targets: Option<Vec<RenameTarget>>,     // → targets (no change)
    new_name: Option<String>,               // → newName ⚠️
    options: RenameOptions,                 // → options (no change)
}

// Line 55-64
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct RenameTarget {
    kind: String,                           // → kind (no change)
    path: String,                           // → path (no change)
    new_name: Option<String>,               // → newName ⚠️
    selector: Option<SymbolSelector>,       // → selector (no change)
}

// Line 66-69
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct SymbolSelector {
    position: Position,                     // → position (no change)
}

// Line 71-94
#[derive(Debug, Deserialize, Default)]
pub(crate) struct RenameOptions {
    strict: Option<bool>,                   // → strict (no change)
    validate_scope: Option<bool>,           // → validateScope ⚠️
    update_imports: Option<bool>,           // → updateImports ⚠️
    scope: Option<String>,                  // → scope (no change)
    custom_scope: Option<RenameScope>,      // → customScope ⚠️
    consolidate: Option<bool>,              // → consolidate (no change)
}
```

**Fields to Convert**: 5
- `new_name` → `newName` (2 occurrences in struct)
- `validate_scope` → `validateScope`
- `update_imports` → `updateImports`
- `custom_scope` → `customScope`

---

#### 2. Extract Handler (`/workspace/crates/mill-handlers/src/handlers/extract_handler.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 444-450
#[derive(Debug, Deserialize, Serialize)]
struct ExtractPlanParams {
    kind: String,                           // → kind (no change)
    source: SourceRange,                    // → source (no change)
    options: Option<ExtractOptions>,        // → options (no change)
}

// Line 452-459
#[derive(Debug, Deserialize, Serialize)]
struct SourceRange {
    file_path: String,                      // → filePath ⚠️
    range: Range,                           // → range (no change)
    name: String,                           // → name (no change)
    destination: Option<String>,            // → destination (no change)
}

// Line 461-467
#[derive(Debug, Deserialize, Serialize, Default)]
struct ExtractOptions {
    visibility: Option<String>,             // → visibility (no change)
    destination_path: Option<String>,       // → destinationPath ⚠️
}
```

**Fields to Convert**: 2
- `file_path` → `filePath`
- `destination_path` → `destinationPath`

---

#### 3. Inline Handler (`/workspace/crates/mill-handlers/src/handlers/inline_handler.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 398-404
#[derive(Debug, Deserialize, Serialize)]
struct InlinePlanParams {
    kind: String,                           // → kind (no change)
    target: InlineTarget,                   // → target (no change)
    options: Option<InlineOptions>,         // → options (no change)
}

// Line 406-410
#[derive(Debug, Deserialize, Serialize)]
struct InlineTarget {
    file_path: String,                      // → filePath ⚠️
    position: Position,                     // → position (no change)
}

// Line 412-416
#[derive(Debug, Deserialize, Serialize, Default)]
struct InlineOptions {
    inline_all: Option<bool>,               // → inlineAll ⚠️
}
```

**Fields to Convert**: 2
- `file_path` → `filePath`
- `inline_all` → `inlineAll`

---

#### 4. Move Handler (`/workspace/crates/mill-handlers/src/handlers/move/mod.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 63-70
#[derive(Debug, Deserialize)]
struct MovePlanParams {
    target: MoveTarget,                     // → target (no change)
    destination: String,                    // → destination (no change)
    options: MoveOptions,                   // → options (no change)
}

// Line 72-78
#[derive(Debug, Deserialize)]
struct MoveTarget {
    kind: String,                           // → kind (no change)
    path: String,                           // → path (no change)
    selector: Option<SymbolSelector>,       // → selector (no change)
}

// Line 80-83
#[derive(Debug, Deserialize)]
struct SymbolSelector {
    position: Position,                     // → position (no change)
}

// Line 85-92
#[derive(Debug, Deserialize, Default)]
struct MoveOptions {
    update_imports: Option<bool>,           // → updateImports ⚠️
    preserve_formatting: Option<bool>,      // → preserveFormatting ⚠️
}
```

**Fields to Convert**: 2
- `update_imports` → `updateImports`
- `preserve_formatting` → `preserveFormatting`

---

#### 5. Transform Handler (`/workspace/crates/mill-handlers/src/handlers/transform_handler.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 36-42
#[derive(Debug, Deserialize)]
struct TransformPlanParams {
    transformation: Transformation,         // → transformation (no change)
    options: TransformOptions,              // → options (no change)
}

// Line 44-49
#[derive(Debug, Deserialize)]
struct Transformation {
    kind: String,                           // → kind (no change)
    file_path: String,                      // → filePath ⚠️
    range: Range,                           // → range (no change)
}

// Line 51-58
#[derive(Debug, Deserialize, Default)]
struct TransformOptions {
    preserve_formatting: Option<bool>,      // → preserveFormatting ⚠️
    preserve_comments: Option<bool>,        // → preserveComments ⚠️
}
```

**Fields to Convert**: 3
- `file_path` → `filePath`
- `preserve_formatting` → `preserveFormatting`
- `preserve_comments` → `preserveComments`

---

#### 6. Reorder Handler (`/workspace/crates/mill-handlers/src/handlers/reorder_handler.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 36-43
#[derive(Debug, Deserialize)]
struct ReorderPlanParams {
    target: ReorderTarget,                  // → target (no change)
    new_order: Vec<String>,                 // → newOrder ⚠️
    options: ReorderOptions,                // → options (no change)
}

// Line 45-50
#[derive(Debug, Deserialize)]
struct ReorderTarget {
    kind: String,                           // → kind (no change)
    file_path: String,                      // → filePath ⚠️
    position: Position,                     // → position (no change)
}

// Line 52-59
#[derive(Debug, Deserialize, Default)]
struct ReorderOptions {
    preserve_formatting: Option<bool>,      // → preserveFormatting ⚠️
    update_call_sites: Option<bool>,        // → updateCallSites ⚠️
}
```

**Fields to Convert**: 4
- `new_order` → `newOrder`
- `file_path` → `filePath`
- `preserve_formatting` → `preserveFormatting`
- `update_call_sites` → `updateCallSites`

---

#### 7. Delete Handler (`/workspace/crates/mill-handlers/src/handlers/delete_handler.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 35-40
#[derive(Debug, Deserialize)]
struct DeletePlanParams {
    target: DeleteTarget,                   // → target (no change)
    options: DeleteOptions,                 // → options (no change)
}

// Line 42-48
#[derive(Debug, Deserialize)]
struct DeleteTarget {
    kind: String,                           // → kind (no change)
    path: String,                           // → path (no change)
    selector: Option<DeleteSelector>,       // → selector (no change)
}

// Line 50-57
#[derive(Debug, Deserialize)]
struct DeleteSelector {
    line: u32,                              // → line (no change)
    character: u32,                         // → character (no change)
    symbol_name: Option<String>,            // → symbolName ⚠️
}

// Line 59-68
#[derive(Debug, Deserialize, Default)]
struct DeleteOptions {
    cleanup_imports: Option<bool>,          // → cleanupImports ⚠️
    remove_tests: Option<bool>,             // → removeTests ⚠️
    force: Option<bool>,                    // → force (no change)
}
```

**Fields to Convert**: 3
- `symbol_name` → `symbolName`
- `cleanup_imports` → `cleanupImports`
- `remove_tests` → `removeTests`

---

### B. Workspace Operations (Handlers - Public API)

#### 8. Workspace Apply Handler (`/workspace/crates/mill-handlers/src/handlers/workspace_apply_handler.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 61-66
#[derive(Debug, Deserialize)]
struct ApplyEditParams {
    plan: RefactorPlan,                     // → plan (no change)
    options: ApplyOptions,                  // → options (no change)
}

// Line 69-80
#[derive(Debug, Deserialize)]
struct ApplyOptions {
    dry_run: bool,                          // → dryRun ⚠️
    validate_checksums: bool,               // → validateChecksums ⚠️
    rollback_on_error: bool,                // → rollbackOnError ⚠️
    validation: Option<ValidationConfig>,   // → validation (no change)
}

// Line 94-110
#[derive(Debug, Serialize)]
struct ApplyResult {
    success: bool,                          // → success (no change)
    applied_files: Vec<String>,             // → appliedFiles ⚠️
    created_files: Vec<String>,             // → createdFiles ⚠️
    deleted_files: Vec<String>,             // → deletedFiles ⚠️
    warnings: Vec<String>,                  // → warnings (no change)
    validation: Option<ValidationResult>,   // → validation (no change)
    rollback_available: bool,               // → rollbackAvailable ⚠️
}
```

**Fields to Convert**: 7
- `dry_run` → `dryRun`
- `validate_checksums` → `validateChecksums`
- `rollback_on_error` → `rollbackOnError`
- `applied_files` → `appliedFiles`
- `created_files` → `createdFiles`
- `deleted_files` → `deletedFiles`
- `rollback_available` → `rollbackAvailable`

---

#### 9. Find/Replace Handler (`/workspace/crates/mill-handlers/src/handlers/workspace/find_replace_handler.rs`)

**Status**: ⚠️ Has `#[serde(rename_all = "snake_case")]` on SearchMode enum only (Line 72)

```rust
// Line 41-68
#[derive(Debug, Deserialize)]
pub struct FindReplaceParams {
    pattern: String,                        // → pattern (no change)
    replacement: String,                    // → replacement (no change)
    mode: SearchMode,                       // → mode (no change)
    whole_word: bool,                       // → wholeWord ⚠️
    preserve_case: bool,                    // → preserveCase ⚠️
    scope: ScopeConfig,                     // → scope (no change)
    dry_run: bool,                          // → dryRun ⚠️
}

// Line 87-96
#[derive(Debug, Deserialize, Default)]
pub struct ScopeConfig {
    include_patterns: Vec<String>,          // → includePatterns ⚠️
    exclude_patterns: Vec<String>,          // → excludePatterns ⚠️
}

// Line 109-115
#[derive(Debug, Serialize)]
pub struct ApplyResult {
    success: bool,                          // → success (no change)
    files_modified: Vec<String>,            // → filesModified ⚠️
    matches_found: usize,                   // → matchesFound ⚠️
    matches_replaced: usize,                // → matchesReplaced ⚠️
}
```

**Fields to Convert**: 8
- `whole_word` → `wholeWord`
- `preserve_case` → `preserveCase`
- `dry_run` → `dryRun`
- `include_patterns` → `includePatterns`
- `exclude_patterns` → `excludePatterns`
- `files_modified` → `filesModified`
- `matches_found` → `matchesFound`
- `matches_replaced` → `matchesReplaced`

**Note**: SearchMode enum should change `rename_all = "snake_case"` → `rename_all = "lowercase"` (already lowercase variants)

---

#### 10. Create Package Handler (`/workspace/crates/mill-handlers/src/handlers/tools/workspace_create.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 48-55
#[derive(Debug, Deserialize)]
pub struct CreatePackageParams {
    package_path: String,                   // → packagePath ⚠️
    package_type: PackageType,              // → packageType ⚠️
    options: CreatePackageOptions,          // → options (no change)
}

// Line 61-69
#[derive(Debug, Deserialize, Default)]
pub struct CreatePackageOptions {
    dry_run: bool,                          // → dryRun ⚠️
    add_to_workspace: bool,                 // → addToWorkspace ⚠️
    template: Template,                     // → template (no change)
}

// Line 77-83
#[derive(Debug, Serialize)]
pub struct CreatePackageResult {
    created_files: Vec<String>,             // → createdFiles ⚠️
    workspace_updated: bool,                // → workspaceUpdated ⚠️
    package_info: PackageInfo,              // → packageInfo ⚠️
    dry_run: bool,                          // → dryRun ⚠️
}

// Line 85-90
#[derive(Debug, Serialize)]
pub struct PackageInfo {
    name: String,                           // → name (no change)
    version: String,                        // → version (no change)
    manifest_path: String,                  // → manifestPath ⚠️
}
```

**Fields to Convert**: 9
- `package_path` → `packagePath`
- `package_type` → `packageType`
- `dry_run` → `dryRun` (2 occurrences)
- `add_to_workspace` → `addToWorkspace`
- `created_files` → `createdFiles`
- `workspace_updated` → `workspaceUpdated`
- `package_info` → `packageInfo`
- `manifest_path` → `manifestPath`

---

#### 11. Extract Dependencies Handler (`/workspace/crates/mill-handlers/src/handlers/tools/workspace_extract_deps.rs`)

**Status**: ⚠️ Has `#[serde(rename_all = "kebab-case")]` on DependencySection enum only (Line 79)

```rust
// Line 53-60
#[derive(Debug, Deserialize)]
pub struct ExtractDependenciesParams {
    source_manifest: String,                // → sourceManifest ⚠️
    target_manifest: String,                // → targetManifest ⚠️
    dependencies: Vec<String>,              // → dependencies (no change)
    options: ExtractDependenciesOptions,    // → options (no change)
}

// Line 62-72
#[derive(Debug, Deserialize, Default)]
pub struct ExtractDependenciesOptions {
    dry_run: bool,                          // → dryRun ⚠️
    preserve_versions: bool,                // → preserveVersions ⚠️
    preserve_features: bool,                // → preserveFeatures ⚠️
    section: DependencySection,             // → section (no change)
}

// Line 99-106
#[derive(Debug, Serialize)]
pub struct ExtractDependenciesResult {
    dependencies_extracted: usize,          // → dependenciesExtracted ⚠️
    dependencies_added: Vec<DependencyInfo>,// → dependenciesAdded ⚠️
    target_manifest_updated: bool,          // → targetManifestUpdated ⚠️
    dry_run: bool,                          // → dryRun ⚠️
    warnings: Vec<String>,                  // → warnings (no change)
}

// Line 108-112
#[derive(Debug, Serialize)]
pub struct DependencyInfo {
    name: String,                           // → name (no change)
    version: String,                        // → version (no change)
    // Note: more fields exist but skipped in serialization
}
```

**Fields to Convert**: 9
- `source_manifest` → `sourceManifest`
- `target_manifest` → `targetManifest`
- `dry_run` → `dryRun` (2 occurrences)
- `preserve_versions` → `preserveVersions`
- `preserve_features` → `preserveFeatures`
- `dependencies_extracted` → `dependenciesExtracted`
- `dependencies_added` → `dependenciesAdded`
- `target_manifest_updated` → `targetManifestUpdated`

---

#### 12. Update Members Handler (`/workspace/crates/mill-handlers/src/handlers/tools/workspace_update_members.rs`)

**Status**: ⚠️ Has `#[serde(rename_all = "lowercase")]` on MemberAction enum only (Line 62)

```rust
// Line 51-59
#[derive(Debug, Deserialize)]
pub struct UpdateMembersParams {
    workspace_manifest: String,             // → workspaceManifest ⚠️
    action: MemberAction,                   // → action (no change)
    members: Vec<String>,                   // → members (no change)
    options: UpdateMembersOptions,          // → options (no change)
}

// Line 69-75
#[derive(Debug, Deserialize, Default)]
pub struct UpdateMembersOptions {
    dry_run: bool,                          // → dryRun ⚠️
    create_if_missing: bool,                // → createIfMissing ⚠️
}

// Line 79-87
#[derive(Debug, Serialize)]
pub struct UpdateMembersResult {
    action: String,                         // → action (no change)
    members_before: Vec<String>,            // → membersBefore ⚠️
    members_after: Vec<String>,             // → membersAfter ⚠️
    changes_made: usize,                    // → changesMade ⚠️
    workspace_updated: bool,                // → workspaceUpdated ⚠️
    dry_run: bool,                          // → dryRun ⚠️
}
```

**Fields to Convert**: 8
- `workspace_manifest` → `workspaceManifest`
- `dry_run` → `dryRun` (2 occurrences)
- `create_if_missing` → `createIfMissing`
- `members_before` → `membersBefore`
- `members_after` → `membersAfter`
- `changes_made` → `changesMade`
- `workspace_updated` → `workspaceUpdated`

---

### C. Analysis Operations (Handlers - Public API)

#### 13. Batch Analysis (`/workspace/crates/mill-handlers/src/handlers/tools/analysis/batch.rs`)

**Status**: ❌ Missing `#[serde(rename_all = "camelCase")]`

```rust
// Line 23-30
#[derive(Debug, Deserialize, Clone)]
pub struct AnalysisQuery {
    command: String,                        // → command (no change)
    kind: String,                           // → kind (no change)
    scope: QueryScope,                      // → scope (no change)
    options: Option<Value>,                 // → options (no change)
}

// Line 32-41
#[derive(Debug, Deserialize, Clone)]
pub struct QueryScope {
    #[serde(rename = "type")]
    scope_type: String,                     // → scopeType ⚠️ (manual rename exists)
    path: Option<String>,                   // → path (no change)
    include: Vec<String>,                   // → include (no change)
    exclude: Vec<String>,                   // → exclude (no change)
}

// Line 49-54
#[derive(Debug, Clone, Serialize)]
pub struct SingleQueryResult {
    command: String,                        // → command (no change)
    kind: String,                           // → kind (no change)
    result: AnalysisResult,                 // → result (no change)
}

// Line 56-61
#[derive(Debug, Clone, Serialize)]
pub struct BatchAnalysisResult {
    results: Vec<SingleQueryResult>,        // → results (no change)
    summary: BatchSummary,                  // → summary (no change)
    metadata: BatchMetadata,                // → metadata (no change)
}

// Line 65-74
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    total_queries: usize,                   // → totalQueries ⚠️
    total_files_scanned: usize,             // → totalFilesScanned ⚠️
    files_analyzed: usize,                  // → filesAnalyzed ⚠️
    files_failed: usize,                    // → filesFailed ⚠️
    total_findings: usize,                  // → totalFindings ⚠️
    findings_by_severity: HashMap<...>,     // → findingsBySeverity ⚠️
    execution_time_ms: u64,                 // → executionTimeMs ⚠️
}

// Line 76-84
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    started_at: String,                     // → startedAt ⚠️
    completed_at: String,                   // → completedAt ⚠️
    categories_analyzed: Vec<String>,       // → categoriesAnalyzed ⚠️
    ast_cache_hits: usize,                  // → astCacheHits ⚠️
    ast_cache_misses: usize,                // → astCacheMisses ⚠️
    failed_files: HashMap<String, String>,  // → failedFiles ⚠️
}
```

**Fields to Convert**: 13
- `scope_type` → `scopeType` (manual rename already exists)
- `total_queries` → `totalQueries`
- `total_files_scanned` → `totalFilesScanned`
- `files_analyzed` → `filesAnalyzed`
- `files_failed` → `filesFailed`
- `total_findings` → `totalFindings`
- `findings_by_severity` → `findingsBySeverity`
- `execution_time_ms` → `executionTimeMs`
- `started_at` → `startedAt`
- `completed_at` → `completedAt`
- `categories_analyzed` → `categoriesAnalyzed`
- `ast_cache_hits` → `astCacheHits`
- `ast_cache_misses` → `astCacheMisses`
- `failed_files` → `failedFiles`

---

#### 14. Module Dependencies Analysis (`/workspace/crates/mill-handlers/src/handlers/tools/analysis/module_dependencies.rs`)

**Status**: ⚠️ Has `#[serde(rename_all = "lowercase")]` on TargetKind enum only (Line 57)

```rust
// Line 35-43
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependenciesParams {
    target: TargetSpec,                     // → target (no change)
    options: ModuleDependenciesOptions,     // → options (no change)
}

// Line 46-53
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    kind: TargetKind,                       // → kind (no change)
    path: String,                           // → path (no change)
}

// Line 67-80
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependenciesOptions {
    include_dev_dependencies: bool,         // → includeDevDependencies ⚠️
    include_workspace_deps: bool,           // → includeWorkspaceDeps ⚠️
    resolve_features: bool,                 // → resolveFeatures ⚠️
}

// Line 97-113
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependenciesResult {
    external_dependencies: HashMap<...>,    // → externalDependencies ⚠️
    workspace_dependencies: Vec<String>,    // → workspaceDependencies ⚠️
    std_dependencies: Vec<String>,          // → stdDependencies ⚠️
    import_analysis: ImportAnalysisSummary, // → importAnalysis ⚠️
    files_analyzed: Vec<String>,            // → filesAnalyzed ⚠️
}

// Line 116-132
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    version: String,                        // → version (no change)
    features: Option<Vec<String>>,          // → features (no change)
    optional: Option<bool>,                 // → optional (no change)
    source: Option<String>,                 // → source (no change)
}

// Line 135-151
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAnalysisSummary {
    total_imports: usize,                   // → totalImports ⚠️
    external_crates: usize,                 // → externalCrates ⚠️
    workspace_crates: usize,                // → workspaceCrates ⚠️
    std_crates: usize,                      // → stdCrates ⚠️
    unresolved_imports: Vec<String>,        // → unresolvedImports ⚠️
}
```

**Fields to Convert**: 13
- `include_dev_dependencies` → `includeDevDependencies`
- `include_workspace_deps` → `includeWorkspaceDeps`
- `resolve_features` → `resolveFeatures`
- `external_dependencies` → `externalDependencies`
- `workspace_dependencies` → `workspaceDependencies`
- `std_dependencies` → `stdDependencies`
- `import_analysis` → `importAnalysis`
- `files_analyzed` → `filesAnalyzed`
- `total_imports` → `totalImports`
- `external_crates` → `externalCrates`
- `workspace_crates` → `workspaceCrates`
- `std_crates` → `stdCrates`
- `unresolved_imports` → `unresolvedImports`

---

### D. Foundation Protocol Types (Already Using camelCase or Need Conversion)

#### 15. RefactorPlan Types (`/workspace/crates/mill-foundation/src/protocol/refactor_plan.rs`)

**Status**: ⚠️ Mixed - Some structs have no annotation

```rust
// Line 7-10
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionTarget {
    path: String,                           // → path (no change)
    kind: String,                           // → kind (no change)
}

// Line 45-51
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    plan_version: String,                   // → planVersion ⚠️
    kind: String,                           // → kind (no change)
    language: String,                       // → language (no change)
    estimated_impact: String,               // → estimatedImpact ⚠️
    created_at: String,                     // → createdAt ⚠️
}

// Line 53-58
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    affected_files: usize,                  // → affectedFiles ⚠️
    created_files: usize,                   // → createdFiles ⚠️
    deleted_files: usize,                   // → deletedFiles ⚠️
}

// Line 61-65
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWarning {
    code: String,                           // → code (no change)
    message: String,                        // → message (no change)
    candidates: Option<Vec<String>>,        // → candidates (no change)
}

// Line 68-78 (and similar for all plan types)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenamePlan {
    edits: WorkspaceEdit,                   // → edits (no change)
    summary: PlanSummary,                   // → summary (no change)
    warnings: Vec<PlanWarning>,             // → warnings (no change)
    metadata: PlanMetadata,                 // → metadata (no change)
    file_checksums: HashMap<String, String>,// → fileChecksums ⚠️
    is_consolidation: bool,                 // → isConsolidation ⚠️
}
```

**Fields to Convert (across all plan types)**: 8
- `plan_version` → `planVersion`
- `estimated_impact` → `estimatedImpact`
- `created_at` → `createdAt`
- `affected_files` → `affectedFiles`
- `created_files` → `createdFiles`
- `deleted_files` → `deletedFiles`
- `file_checksums` → `fileChecksums`
- `is_consolidation` → `isConsolidation`

---

#### 16. AnalysisResult Types (`/workspace/crates/mill-foundation/src/protocol/analysis_result.rs`)

**Status**: ⚠️ Has partial annotations - some enums use `rename_all`, structs don't

```rust
// Line 11-18
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    findings: Vec<Finding>,                 // → findings (no change)
    summary: AnalysisSummary,               // → summary (no change)
    metadata: AnalysisMetadata,             // → metadata (no change)
}

// Line 22-39
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    id: String,                             // → id (no change)
    kind: String,                           // → kind (no change)
    severity: Severity,                     // → severity (no change)
    location: FindingLocation,              // → location (no change)
    metrics: Option<HashMap<...>>,          // → metrics (no change)
    message: String,                        // → message (no change)
    suggestions: Vec<Suggestion>,           // → suggestions (no change)
}

// Line 52-64
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingLocation {
    file_path: String,                      // → filePath ⚠️
    range: Option<Range>,                   // → range (no change)
    symbol: Option<String>,                 // → symbol (no change)
    symbol_kind: Option<String>,            // → symbolKind ⚠️
}

// Line 84-103
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    action: String,                         // → action (no change)
    description: String,                    // → description (no change)
    target: Option<SuggestionTarget>,       // → target (no change)
    estimated_impact: String,               // → estimatedImpact ⚠️
    safety: SafetyLevel,                    // → safety (no change)
    confidence: f64,                        // → confidence (no change)
    reversible: bool,                       // → reversible (no change)
    refactor_call: Option<RefactorCall>,    // → refactorCall ⚠️
}

// Line 127-132
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorCall {
    command: String,                        // → command (no change)
    arguments: serde_json::Value,           // → arguments (no change)
}

// Line 136-152
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    total_findings: usize,                  // → totalFindings ⚠️
    returned_findings: usize,               // → returnedFindings ⚠️
    has_more: bool,                         // → hasMore ⚠️
    by_severity: SeverityBreakdown,         // → bySeverity ⚠️
    files_analyzed: usize,                  // → filesAnalyzed ⚠️
    symbols_analyzed: Option<usize>,        // → symbolsAnalyzed ⚠️
    analysis_time_ms: u64,                  // → analysisTimeMs ⚠️
}

// Line 164-179
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    category: String,                       // → category (no change)
    kind: String,                           // → kind (no change)
    scope: AnalysisScope,                   // → scope (no change)
    language: Option<String>,               // → language (no change)
    timestamp: String,                      // → timestamp (no change)
    thresholds: Option<HashMap<...>>,       // → thresholds (no change)
}

// Line 183-195
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisScope {
    #[serde(rename = "type")]
    scope_type: String,                     // → scopeType ⚠️ (manual rename exists)
    path: String,                           // → path (no change)
    include: Vec<String>,                   // → include (no change)
    exclude: Vec<String>,                   // → exclude (no change)
}
```

**Fields to Convert**: 13
- `file_path` → `filePath`
- `symbol_kind` → `symbolKind`
- `estimated_impact` → `estimatedImpact`
- `refactor_call` → `refactorCall`
- `total_findings` → `totalFindings`
- `returned_findings` → `returnedFindings`
- `has_more` → `hasMore`
- `by_severity` → `bySeverity`
- `files_analyzed` → `filesAnalyzed`
- `symbols_analyzed` → `symbolsAnalyzed`
- `analysis_time_ms` → `analysisTimeMs`
- `scope_type` → `scopeType` (manual rename already exists)

---

## Part 2: Complete Field Mapping

### All Unique snake_case Fields (Alphabetical)

| snake_case Field | camelCase Field | Test Count | Defined In (Struct Count) |
|------------------|-----------------|------------|---------------------------|
| `action` | (no change) | 0 | N/A - already lowercase |
| `add_to_workspace` | `addToWorkspace` | 6 | CreatePackageOptions (1) |
| `affected_files` | `affectedFiles` | 0 | PlanSummary (7 plan types) |
| `applied_files` | `appliedFiles` | 0 | ApplyResult (1) |
| `ast_cache_hits` | `astCacheHits` | 0 | BatchMetadata (1) |
| `ast_cache_misses` | `astCacheMisses` | 0 | BatchMetadata (1) |
| `by_severity` | `bySeverity` | 0 | AnalysisSummary (1) |
| `categories_analyzed` | `categoriesAnalyzed` | 0 | BatchMetadata (1) |
| `changes_made` | `changesMade` | 0 | UpdateMembersResult (1) |
| `cleanup_imports` | `cleanupImports` | 0 | DeleteOptions (1) |
| `completed_at` | `completedAt` | 0 | BatchMetadata (1) |
| `consolidate` | (no change) | 0 | RenameOptions (1) |
| `create_if_missing` | `createIfMissing` | 3 | UpdateMembersOptions (1) |
| `created_at` | `createdAt` | 0 | PlanMetadata (7 plan types) |
| `created_files` | `createdFiles` | 0 | PlanSummary + Results (9) |
| `custom_scope` | `customScope` | 0 | RenameOptions (1) |
| `deleted_files` | `deletedFiles` | 0 | PlanSummary + ApplyResult (8) |
| `dependencies_added` | `dependenciesAdded` | 0 | ExtractDependenciesResult (1) |
| `dependencies_extracted` | `dependenciesExtracted` | 0 | ExtractDependenciesResult (1) |
| `destination_path` | `destinationPath` | 0 | ExtractOptions (1) |
| `dry_run` | `dryRun` | 110 | 9 structs (most common) |
| `estimated_impact` | `estimatedImpact` | 0 | PlanMetadata + Suggestion (8) |
| `exclude_patterns` | `excludePatterns` | 1 | ScopeConfig (1) |
| `execution_time_ms` | `executionTimeMs` | 0 | BatchSummary (1) |
| `external_crates` | `externalCrates` | 0 | ImportAnalysisSummary (1) |
| `external_dependencies` | `externalDependencies` | 0 | ModuleDependenciesResult (1) |
| `failed_files` | `failedFiles` | 0 | BatchMetadata (1) |
| `file_checksums` | `fileChecksums` | 0 | All plan types (7) |
| `file_path` | `filePath` | 24 | 5 structs |
| `files_analyzed` | `filesAnalyzed` | 0 | 4 structs |
| `files_failed` | `filesFailed` | 0 | BatchSummary (1) |
| `files_modified` | `filesModified` | 0 | FindReplaceApplyResult (1) |
| `findings_by_severity` | `findingsBySeverity` | 0 | BatchSummary (1) |
| `has_more` | `hasMore` | 0 | AnalysisSummary (1) |
| `import_analysis` | `importAnalysis` | 0 | ModuleDependenciesResult (1) |
| `include_dev_dependencies` | `includeDevDependencies` | 0 | ModuleDependenciesOptions (1) |
| `include_patterns` | `includePatterns` | 1 | ScopeConfig (1) |
| `include_workspace_deps` | `includeWorkspaceDeps` | 0 | ModuleDependenciesOptions (1) |
| `inline_all` | `inlineAll` | 0 | InlineOptions (1) |
| `is_consolidation` | `isConsolidation` | 0 | RenamePlan (1) |
| `manifest_path` | `manifestPath` | 0 | PackageInfo (1) |
| `matches_found` | `matchesFound` | 0 | FindReplaceApplyResult (1) |
| `matches_replaced` | `matchesReplaced` | 0 | FindReplaceApplyResult (1) |
| `members_after` | `membersAfter` | 0 | UpdateMembersResult (1) |
| `members_before` | `membersBefore` | 0 | UpdateMembersResult (1) |
| `new_name` | `newName` | 56 | RenameTarget + RenamePlanParams (2) |
| `new_order` | `newOrder` | 4 | ReorderPlanParams (1) |
| `package_info` | `packageInfo` | 0 | CreatePackageResult (1) |
| `package_path` | `packagePath` | 6 | CreatePackageParams (1) |
| `package_type` | `packageType` | 6 | CreatePackageParams (1) |
| `plan_version` | `planVersion` | 0 | PlanMetadata (7 plan types) |
| `preserve_case` | `preserveCase` | 2 | FindReplaceParams (1) |
| `preserve_comments` | `preserveComments` | 0 | TransformOptions (1) |
| `preserve_features` | `preserveFeatures` | 1 | ExtractDependenciesOptions (1) |
| `preserve_formatting` | `preserveFormatting` | 0 | 3 structs (Move/Transform/Reorder) |
| `preserve_versions` | `preserveVersions` | 1 | ExtractDependenciesOptions (1) |
| `refactor_call` | `refactorCall` | 0 | Suggestion (1) |
| `remove_tests` | `removeTests` | 0 | DeleteOptions (1) |
| `resolve_features` | `resolveFeatures` | 0 | ModuleDependenciesOptions (1) |
| `returned_findings` | `returnedFindings` | 0 | AnalysisSummary (1) |
| `rollback_available` | `rollbackAvailable` | 0 | ApplyResult (1) |
| `rollback_on_error` | `rollbackOnError` | 1 | ApplyOptions (1) |
| `scope_type` | `scopeType` | 0 | QueryScope + AnalysisScope (2) |
| `source_manifest` | `sourceManifest` | 9 | ExtractDependenciesParams (1) |
| `started_at` | `startedAt` | 0 | BatchMetadata (1) |
| `std_crates` | `stdCrates` | 0 | ImportAnalysisSummary (1) |
| `std_dependencies` | `stdDependencies` | 0 | ModuleDependenciesResult (1) |
| `symbol_kind` | `symbolKind` | 0 | FindingLocation (1) |
| `symbol_name` | `symbolName` | 0 | DeleteSelector (1) |
| `symbols_analyzed` | `symbolsAnalyzed` | 0 | AnalysisSummary (1) |
| `target_manifest` | `targetManifest` | 9 | ExtractDependenciesParams (1) |
| `target_manifest_updated` | `targetManifestUpdated` | 0 | ExtractDependenciesResult (1) |
| `total_files_scanned` | `totalFilesScanned` | 0 | BatchSummary (1) |
| `total_findings` | `totalFindings` | 0 | AnalysisSummary + BatchSummary (2) |
| `total_imports` | `totalImports` | 0 | ImportAnalysisSummary (1) |
| `total_queries` | `totalQueries` | 0 | BatchSummary (1) |
| `unresolved_imports` | `unresolvedImports` | 0 | ImportAnalysisSummary (1) |
| `update_call_sites` | `updateCallSites` | 0 | ReorderOptions (1) |
| `update_imports` | `updateImports` | 0 | RenameOptions + MoveOptions (2) |
| `validate_checksums` | `validateChecksums` | 30 | ApplyOptions (1) |
| `validate_scope` | `validateScope` | 0 | RenameOptions (1) |
| `whole_word` | `wholeWord` | 1 | FindReplaceParams (1) |
| `workspace_crates` | `workspaceCrates` | 0 | ImportAnalysisSummary (1) |
| `workspace_dependencies` | `workspaceDependencies` | 0 | ModuleDependenciesResult (1) |
| `workspace_manifest` | `workspaceManifest` | 10 | UpdateMembersParams (1) |
| `workspace_updated` | `workspaceUpdated` | 0 | 2 result structs |

**Total Unique Fields**: 82 snake_case field names
**Total Test Occurrences**: 367+ across all test files

---

## Part 3: Test File Impact Analysis

### Test Files Requiring Updates (30+ files)

| Test File | dry_run | file_path | new_name | validate_checksums | Other Fields | Total Changes |
|-----------|---------|-----------|----------|-------------------|--------------|---------------|
| `test_workspace_find_replace.rs` | 20 | 0 | 0 | 0 | 5 (whole_word, preserve_case, include_patterns, exclude_patterns) | 25 |
| `test_workspace_extract_deps.rs` | 9 | 0 | 0 | 0 | 18 (source_manifest, target_manifest, preserve_*) | 27 |
| `test_dry_run_integration.rs` | 10 | 1 | 0 | 0 | 0 | 11 |
| `test_workspace_update_members.rs` | 8 | 0 | 0 | 0 | 20 (workspace_manifest, create_if_missing) | 28 |
| `test_comprehensive_rename_coverage.rs` | 6 | 0 | 0 | 0 | 0 | 6 |
| `test_workspace_create.rs` | 6 | 0 | 0 | 0 | 18 (package_path, package_type, add_to_workspace) | 24 |
| `test_rust_directory_rename.rs` | 5 | 0 | 5 | 5 | 0 | 15 |
| `test_rename_with_imports.rs` | 5 | 0 | 6 | 2 | 0 | 13 |
| `test_rust_same_crate_moves.rs` | 3 | 0 | 3 | 0 | 0 | 6 |
| `test_rust_cargo_edge_cases.rs` | 4 | 0 | 4 | 4 | 0 | 12 |
| `test_rust_mod_declarations.rs` | 5 | 0 | 5 | 0 | 0 | 10 |
| `test_extract_integration.rs` | 2 | 4 | 0 | 1 | 0 | 7 |
| `test_move_with_imports.rs` | 2 | 0 | 2 | 2 | 0 | 6 |
| `test_move_integration.rs` | 3 | 0 | 0 | 3 | 0 | 6 |
| `test_unified_refactoring_api.rs` | 3 | 1 | 0 | 2 | 0 | 6 |
| `test_inline_integration.rs` | 2 | 4 | 0 | 2 | 0 | 8 |
| `test_delete_integration.rs` | 3 | 0 | 0 | 2 | 0 | 5 |
| `test_transform_integration.rs` | 2 | 4 | 0 | 1 | 0 | 7 |
| `test_reorder_integration.rs` | 2 | 4 | 0 | 1 | 4 (new_order) | 11 |
| `test_cargo_package_rename.rs` | 1 | 0 | 1 | 0 | 0 | 2 |
| `test_rename_integration.rs` | 3 | 0 | 3 | 3 | 0 | 9 |
| `test_cross_workspace_import_updates.rs` | 1 | 0 | 1 | 0 | 0 | 2 |
| `test_consolidation_bug_fix.rs` | 1 | 0 | 1 | 0 | 0 | 2 |
| `test_workspace_apply_integration.rs` | 2 | 0 | 0 | 1 | 1 (rollback_on_error) | 4 |
| `resilience_tests.rs` | 0 | 6 | 0 | 0 | 0 | 6 |

**Total Test Files**: 25+ files
**Total JSON Field Updates Required**: 367+ individual field references

---

## Part 4: Risk Assessment

### Critical Dependencies

1. **EditPlan already uses camelCase** - This is the gold standard reference
2. **LSP types library uses camelCase** - External standard
3. **Some enums already have rename_all** - Need careful review to avoid double-conversion

### Breaking Change Scope

- **External API Impact**: HIGH - All MCP clients will break
- **Internal Code Impact**: LOW - Rust code unchanged (only JSON serialization)
- **Test Impact**: HIGH - 367+ JSON literals need updating

### Fields with Highest Risk (Test Count)

1. `dry_run` → `dryRun` (110 occurrences) - **CRITICAL**
2. `new_name` → `newName` (56 occurrences) - **HIGH**
3. `validate_checksums` → `validateChecksums` (30 occurrences) - **MEDIUM**
4. `file_path` → `filePath` (24 occurrences) - **MEDIUM**
5. All others (< 10 occurrences each) - **LOW**

---

## Part 5: Three Conversion Solutions

---

## **Solution 1: SAFEST - Incremental Module-by-Module Rollout** ⭐ **RECOMMENDED**

### Strategy

Convert one handler module at a time, validate thoroughly before moving to next module. Use feature flags to maintain backward compatibility during transition.

### Implementation Steps

**Phase 1: Foundation Types (Week 1)**
1. Add `#[serde(rename_all = "camelCase")]` to ALL protocol types in `/workspace/crates/mill-foundation/src/protocol/`:
   - `refactor_plan.rs` - All plan types (7 structs)
   - `analysis_result.rs` - All analysis types (10+ structs)
   - Review existing `rename_all` annotations in `mod.rs` (EditPlan already correct)

2. Update 0 test files (foundation types not directly tested in JSON)

**Phase 2: Workspace Operations (Week 2)**
3. Convert workspace handlers (lowest risk, isolated functionality):
   - `workspace_apply_handler.rs` (3 structs) → Update ~40 test occurrences
   - `workspace/find_replace_handler.rs` (3 structs) → Update 25 test occurrences in 1 file
   - `tools/workspace_create.rs` (4 structs) → Update 24 test occurrences in 1 file
   - `tools/workspace_extract_deps.rs` (4 structs) → Update 27 test occurrences in 1 file
   - `tools/workspace_update_members.rs` (3 structs) → Update 28 test occurrences in 1 file

4. Run integration tests for workspace tools only
5. Validate MCP workspace tool calls

**Phase 3: Refactoring Operations - Low Risk (Week 3)**
6. Convert less-used refactoring handlers:
   - `delete_handler.rs` (4 structs) → Update ~5 test occurrences
   - `transform_handler.rs` (3 structs) → Update ~7 test occurrences
   - `reorder_handler.rs` (3 structs) → Update ~11 test occurrences
   - `inline_handler.rs` (3 structs) → Update ~8 test occurrences
   - `extract_handler.rs` (3 structs) → Update ~7 test occurrences
   - `move/mod.rs` (4 structs) → Update ~6 test occurrences

7. Run refactoring integration tests
8. Validate MCP refactoring tool calls

**Phase 4: Refactoring Operations - High Risk (Week 4)**
9. Convert high-usage rename handler (HIGHEST TEST IMPACT):
   - `rename_handler/mod.rs` (4 structs) → Update ~110 dry_run + 56 new_name occurrences

10. Run ALL rename integration tests
11. Validate MCP rename tool calls

**Phase 5: Analysis Operations (Week 5)**
12. Convert analysis handlers:
    - `tools/analysis/batch.rs` (6 structs) → Update minimal test occurrences
    - `tools/analysis/module_dependencies.rs` (6 structs) → Update minimal test occurrences

13. Run analysis integration tests
14. Validate MCP analysis tool calls

**Phase 6: Final Validation (Week 6)**
15. Run full test suite (`cargo nextest run --workspace --all-features`)
16. Manual MCP protocol testing with real clients
17. Update API documentation
18. Create migration guide for external MCP clients

### Validation Strategy (Per Phase)

```bash
# After each phase conversion:
# 1. Run targeted tests
cargo nextest run --workspace -- <module_name>

# 2. Run all e2e tests
cargo nextest run --workspace --features lsp-tests

# 3. Manual MCP testing
codebuddy tool <toolname> '{"field": "value"}'  # Test converted fields

# 4. Git checkpoint
git add . && git commit -m "feat: convert <module> to camelCase JSON"
```

### Advantages

- ✅ Lowest risk - Each phase independently validated
- ✅ Easy rollback - Git checkpoints after each module
- ✅ Gradual deployment - Can pause/adjust if issues found
- ✅ Team review - Smaller PRs easier to review
- ✅ Production safety - Can deploy incrementally if needed

### Disadvantages

- ❌ Time-consuming - 6 weeks estimated
- ❌ Temporary inconsistency - API mixed during transition
- ❌ Multiple PRs - More overhead for reviews
- ❌ Coordination - Need to track progress carefully

### Recommended Tooling

```bash
# Create branch per phase
git checkout -b camelcase-phase-1-foundation
git checkout -b camelcase-phase-2-workspace
# etc.

# Use sed for bulk test file updates
find tests/e2e/src -name "*.rs" -exec sed -i 's/"dryRun":/"dryRun":/g' {} \;
find tests/e2e/src -name "*.rs" -exec sed -i 's/"newName":/"newName":/g' {} \;
# etc.
```

---

## **Solution 2: FASTEST - Atomic Big Bang Conversion**

### Strategy

Convert everything in one massive PR. Use automated tooling for bulk updates. High risk but shortest timeline.

### Implementation Steps

**Day 1: Code Changes**
1. Write sed script to add `#[serde(rename_all = "camelCase")]` to all 45 structs
2. Execute script across all handler files
3. Review diff to ensure no accidental changes

**Day 2: Test Updates**
4. Write comprehensive sed/perl script for all 82 field name conversions
5. Execute against all test files
6. Manual review of complex cases (nested JSON, string literals)

**Day 3: Validation**
7. Run full test suite
8. Fix any failures from missed conversions
9. Manual MCP protocol testing

**Day 4: Documentation & PR**
10. Update API documentation
11. Create migration guide
12. Submit single large PR

### Automation Script Example

```bash
#!/bin/bash
# camelcase_conversion.sh

# Add serde annotation to all param structs
find crates/mill-handlers/src/handlers -name "*.rs" -exec sed -i \
  '/#\[derive(Debug, Deserialize.*\]\|#\[derive(Debug, Serialize.*\]/a\
#[serde(rename_all = "camelCase")]' {} \;

# Convert all test JSON fields
declare -A fields=(
  ["dryRun"]="dryRun"
  ["newName"]="newName"
  ["validate_checksums"]="validateChecksums"
  ["file_path"]="filePath"
  # ... all 82 fields
)

for snake in "${!fields[@]}"; do
  camel="${fields[$snake]}"
  find tests/e2e/src -name "*.rs" -exec sed -i "s/\"$snake\":/\"$camel\":/g" {} \;
done

# Run tests
cargo nextest run --workspace --all-features
```

### Advantages

- ✅ Fastest completion - 4 days estimated
- ✅ Single consistent state - No mixed API period
- ✅ One PR to review - Clear scope
- ✅ Simplest rollback - One git revert

### Disadvantages

- ❌ Highest risk - All-or-nothing approach
- ❌ Large diff - Difficult to review thoroughly
- ❌ Hidden issues - May miss edge cases in automation
- ❌ No escape hatch - Can't pause mid-conversion
- ❌ Production risk - One mistake affects entire API

### Risk Mitigation

```bash
# Pre-validation checks
1. Dry run all scripts on test copy
2. Diff review with expected changes
3. Run full test suite before commit
4. Create backup branch
5. Have rollback plan ready
```

---

## **Solution 3: BALANCED - Grouped Domain Conversion**

### Strategy

Group related handlers by domain, convert entire domain at once. Balance between speed and safety.

### Implementation Steps

**Week 1: Foundation + Workspace Domain**
1. Convert all foundation protocol types (7 plan types, 10+ analysis types)
2. Convert all workspace operation handlers (5 handlers, 17 structs)
3. Update ~144 test occurrences across 5 test files
4. Validate workspace tools

**Week 2: Refactoring Domain (Low-Risk Operations)**
5. Convert extract, inline, move, delete, transform, reorder handlers (20 structs)
6. Update ~44 test occurrences across 6 test files
7. Validate refactoring tools

**Week 3: Refactoring Domain (High-Risk Rename)**
8. Convert rename handler (4 structs)
9. Update ~166 test occurrences (dry_run + new_name heavy)
10. Validate rename tools thoroughly

**Week 4: Analysis Domain + Final Validation**
11. Convert batch analysis and module dependencies (12 structs)
12. Update minimal test occurrences
13. Run full test suite
14. Manual MCP testing
15. Documentation updates

### Domain Groups

**Group 1: Foundation & Workspace** (Risk: LOW, Impact: MEDIUM)
- Protocol types: refactor_plan.rs, analysis_result.rs
- Workspace handlers: apply, find_replace, create, extract_deps, update_members

**Group 2: Refactoring - Low Risk** (Risk: MEDIUM, Impact: LOW)
- Extract, Inline, Move, Delete, Transform, Reorder handlers

**Group 3: Refactoring - High Risk** (Risk: HIGH, Impact: HIGH)
- Rename handler (most test occurrences)

**Group 4: Analysis** (Risk: LOW, Impact: LOW)
- Batch analysis, module dependencies

### Advantages

- ✅ Moderate timeline - 4 weeks estimated
- ✅ Logical grouping - Related functionality together
- ✅ Manageable PRs - 4 medium-sized PRs
- ✅ Domain isolation - Can pause between domains
- ✅ Progressive risk - Tackle highest risk last

### Disadvantages

- ❌ Some API inconsistency - But shorter than Solution 1
- ❌ Multiple PRs - But fewer than Solution 1
- ❌ Moderate risk - Not as safe as Solution 1

### Validation Per Domain

```bash
# After each domain conversion:
cargo nextest run --workspace -- <domain_pattern>
# Manual MCP testing for that domain
git add . && git commit -m "feat: convert <domain> to camelCase"
```

---

## Recommended Solution: **Solution 1 (Safest - Incremental)**

### Why This Is The Best Choice

1. **Lowest Production Risk**: Each module validated independently before moving to next
2. **Easy Debugging**: If issues arise, narrow scope makes root cause obvious
3. **Team Friendly**: Smaller PRs easier to review, less cognitive load
4. **Business Continuity**: Can pause/adjust if production issues arise
5. **Quality Assurance**: Multiple validation points catch edge cases early

### Trade-off Acceptance

Yes, it takes 6 weeks vs 4 days (Solution 2). However:
- This is a **breaking API change** affecting all MCP clients
- The cost of bugs in production far exceeds 6 weeks of dev time
- Incremental approach provides **insurance** against catastrophic failures
- Team can work on other features in parallel during validation periods

### Implementation Timeline (Detailed)

**Week 1**: Foundation protocol types (0 test changes, pure refactor)
**Week 2**: Workspace operations (~144 test changes across 5 files)
**Week 3**: Low-risk refactoring ops (~44 test changes across 6 files)
**Week 4**: High-risk rename handler (~166 test changes, most critical)
**Week 5**: Analysis operations (minimal test changes)
**Week 6**: Final validation + documentation + migration guide

**Total**: 354 test field updates across 30+ test files, 45 struct annotations

---

## Migration Guide for External Clients (Template)

### Breaking Changes - JSON API v2.0

**Effective Date**: TBD
**Reason**: Alignment with MCP/LSP protocol standards (camelCase convention)

### Changes Required

All JSON field names now use camelCase instead of snake_case:

**Example (Rename Operation)**:
```json
// OLD (v1.0)
{
  "target": {"kind": "file", "path": "src/main.rs"},
  "newName": "src/app.rs",
  "options": {
    "dryRun": false,
    "update_imports": true,
    "validate_scope": true
  }
}

// NEW (v2.0)
{
  "target": {"kind": "file", "path": "src/main.rs"},
  "newName": "src/app.rs",
  "options": {
    "dryRun": false,
    "updateImports": true,
    "validateScope": true
  }
}
```

**Field Mapping**: See complete mapping table in Part 2 of this document.

### Backward Compatibility

**None**. This is a breaking change. All clients must update simultaneously with server upgrade.

### Client Update Checklist

- [ ] Review field mapping table (82 fields)
- [ ] Update all MCP tool call JSON
- [ ] Update test fixtures
- [ ] Test against v2.0 server
- [ ] Update API documentation

---

## Appendix: Validation Checklist

### Pre-Conversion

- [ ] Backup production database
- [ ] Create rollback plan
- [ ] Notify all MCP client teams
- [ ] Review all 45 structs for accuracy
- [ ] Verify 82 field mappings correct

### During Conversion (Per Phase/Module)

- [ ] Add `#[serde(rename_all = "camelCase")]` annotation
- [ ] Update test JSON fields
- [ ] Run module-specific tests
- [ ] Run full integration test suite
- [ ] Manual MCP protocol testing
- [ ] Git checkpoint

### Post-Conversion

- [ ] Full test suite passes (`cargo nextest run --all-features`)
- [ ] Manual testing of top 10 most-used tools
- [ ] API documentation updated
- [ ] Migration guide published
- [ ] Client teams notified with deadline
- [ ] Monitoring alerts configured

---

## Summary Statistics

**Scope**:
- 45 structs requiring `#[serde(rename_all = "camelCase")]`
- 82 unique snake_case field names → camelCase
- 367+ test JSON field references to update
- 30+ test files affected
- 3 existing enum annotations to review (ensure no conflicts)

**Effort Estimates**:
- Solution 1 (Safest): 6 weeks, 5 phases, ~60 hours
- Solution 2 (Fastest): 4 days, 1 phase, ~32 hours (high risk)
- Solution 3 (Balanced): 4 weeks, 4 domains, ~48 hours

**Recommended**: Solution 1 for production safety and team sanity.

---

**End of Inventory**
