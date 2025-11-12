# TypeMill Pending Features Integration Plan
**Generated**: 2025-11-12
**Status**: Ready for Implementation

## Executive Summary

Analysis of TypeMill's codebase reveals **8 pending feature areas** with varying implementation status. Of these, **5 are production-ready** and just need integration, **2 require moderate effort**, and **1 is not recommended** for integration.

**Quick Wins Available**: 3 features can be integrated in <1 hour with immediate user value.

---

## Feature Status Matrix

| Feature | Implementation Status | Code Ready | Effort | User Value | Recommendation |
|---------|----------------------|------------|--------|------------|----------------|
| **1. C Plugin Enhanced Imports** | ✅ Complete | Yes | 10 min | High | ✅ Integrate immediately |
| **2. Plugin Metrics in health_check** | ✅ Complete | Yes | 30 min | High | ✅ Integrate immediately |
| **3. Intent Workflow System** | ✅ Complete | Yes | 1-2 hours | Very High | ✅ Add configuration |
| **4. Java Dependency Management** | ✅ Complete | Yes | 2-4 hours | High | ✅ Create tool |
| **5. Enhanced CLI Formatting** | ✅ Complete | Yes | 2-4 hours | Medium | ✅ Integrate gradually |
| **6. Python Type-Based Refactoring** | ⚠️ Partial | Yes | 4-8 hours | Medium | ⚠️ Enhancement |
| **7. LSP Direct Access** | ⚠️ Models only | No | 3-5 days | Low | ❌ Not recommended |
| **8. FUSE Filesystem** | ⚠️ Models only | No | 8-13 days | Unknown | ❌ Not recommended |

---

## Phase 1: Quick Wins (Total: 40 minutes)

### 1.1 C Plugin: Enable Enhanced Import Analysis ⏱️ 10 minutes

**Problem**: C plugin has fully implemented detailed import analysis but it's not exposed.

**Solution**: Remove `#[allow(dead_code)]` and add override method.

**Files to Edit**:
```rust
// File: languages/mill-lang-c/src/import_support.rs:48
- #[allow(dead_code)] // Future enhancement: Detailed dependency analysis
  pub fn analyze_detailed_imports(

// File: languages/mill-lang-c/src/lib.rs (after line 100)
+ fn analyze_detailed_imports(&self, source: &str, file_path: Option<&Path>)
+     -> PluginResult<ImportGraph>
+ {
+     self.import_support.analyze_detailed_imports(source, file_path)
+ }
```

**Impact**:
- Enables `analyze.dependencies` and `analyze.module_dependencies` tools for C projects
- Provides detailed ImportGraph with system headers, locations, metadata

**Testing**:
```bash
cargo nextest run -p mill-lang-c
cargo nextest run -p e2e -- test_analyze_dependencies
```

---

### 1.2 Plugin Monitoring: Expose Metrics via health_check ⏱️ 30 minutes

**Problem**: PluginMetrics and RegistryStatistics are fully tracked but not exposed to users.

**Solution**: Enhance existing `health_check` tool with metrics.

**Files to Edit**:
```rust
// File: crates/mill-handlers/src/handlers/system_handler.rs:90-129
async fn health_check(context: &AppState) -> Result<Value, MillError> {
    let metrics = context.plugin_manager.get_metrics().await;
    let stats = context.plugin_manager.get_registry_statistics().await;

    Ok(json!({
        "status": "healthy",
        "uptime": { /* existing */ },
        "plugins": {
            "loaded": stats.total_plugins,
            "supported_extensions": stats.supported_extensions,
            "supported_methods": stats.supported_methods,
            "average_methods_per_plugin": stats.average_methods_per_plugin
        },
        "metrics": {
            "total_requests": metrics.total_requests,
            "successful_requests": metrics.successful_requests,
            "failed_requests": metrics.failed_requests,
            "success_rate": (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0,
            "average_processing_time_ms": metrics.average_processing_time_ms,
            "requests_per_plugin": metrics.requests_per_plugin,
            "processing_time_per_plugin": metrics.processing_time_per_plugin
        },
        "workflows": { /* existing */ }
    }))
}

// File: crates/mill-plugin-system/src/registry.rs:497-507
// Remove #[allow(dead_code)] from RegistryStatistics fields (now being used)

// File: crates/mill-services/src/services/ast/import_service.rs:212
// DELETE ImportUpdateReport struct (redundant with EditPlanResult)
```

**Impact**:
- Users can monitor plugin performance
- Debugging tool for identifying slow plugins
- Foundation for future observability features

**Testing**:
```bash
cargo nextest run -p mill-handlers -- health_check
# Manual test: Call health_check tool and verify metrics field
```

---

## Phase 2: High-Impact Integrations (Total: 3-6 hours)

### 2.1 Intent Workflow System: Create Default Workflows ⏱️ 1-2 hours

**Status**: ✅ **Fully operational** - just needs configuration file.

**Current State**:
- Planner, WorkflowExecutor, and handlers are production-ready
- `achieve_intent` tool already exposed via MCP
- Missing: `.typemill/workflows.json` configuration file

**Solution**: Create default workflow recipes for common operations.

**New File**: `.typemill/workflows.json`
```json
{
  "workflows": {
    "refactor.renameSymbol": {
      "name": "Rename symbol '{oldName}' to '{newName}'",
      "metadata": { "complexity": 2 },
      "steps": [
        {
          "tool": "rename",
          "params": {
            "target": { "kind": "symbol", "path": "{filePath}", "line": "{line}", "character": "{character}" },
            "newName": "{newName}",
            "options": { "dryRun": "{dryRun}", "scope": "standard" }
          },
          "description": "Rename symbol across project",
          "requires_confirmation": true
        }
      ],
      "required_params": ["filePath", "line", "character", "oldName", "newName"],
      "optional_params": ["dryRun"]
    },
    "refactor.extractFunction": {
      "name": "Extract function '{functionName}' from selection",
      "metadata": { "complexity": 3 },
      "steps": [
        {
          "tool": "extract",
          "params": {
            "kind": "function",
            "source": { "file": "{filePath}", "line": "{startLine}", "character": "{startChar}" },
            "name": "{functionName}",
            "options": { "dryRun": "{dryRun}" }
          },
          "description": "Extract selected code into new function",
          "requires_confirmation": true
        }
      ],
      "required_params": ["filePath", "startLine", "startChar", "functionName"],
      "optional_params": ["dryRun"]
    },
    "codebase.analyzeQuality": {
      "name": "Comprehensive code quality analysis",
      "metadata": { "complexity": 2 },
      "steps": [
        {
          "tool": "analyze.quality",
          "params": {
            "kind": "complexity",
            "scope": { "kind": "workspace" }
          },
          "description": "Analyze complexity metrics"
        },
        {
          "tool": "analyze.quality",
          "params": {
            "kind": "maintainability",
            "scope": { "kind": "workspace" }
          },
          "description": "Check maintainability indicators"
        },
        {
          "tool": "analyze.dead_code",
          "params": {
            "scope": { "kind": "workspace" }
          },
          "description": "Find unused code"
        }
      ],
      "required_params": [],
      "optional_params": []
    },
    "docs.generateAll": {
      "name": "Generate documentation for workspace",
      "metadata": { "complexity": 2 },
      "steps": [
        {
          "tool": "analyze.documentation",
          "params": {
            "scope": { "kind": "workspace" }
          },
          "description": "Analyze documentation coverage"
        },
        {
          "tool": "analyze.structure",
          "params": {
            "scope": { "kind": "workspace" }
          },
          "description": "Extract code structure"
        }
      ],
      "required_params": [],
      "optional_params": []
    }
  }
}
```

**Additional Enhancements**:

1. **Create workflow discovery tools** (optional, 1 hour):
```rust
// New file: crates/mill-handlers/src/handlers/workflow_discovery.rs
pub async fn list_available_intents(context: &AppState) -> Result<Value, MillError> {
    let recipes = context.planner.get_all_recipes();
    Ok(json!({
        "intents": recipes.keys().collect::<Vec<_>>(),
        "count": recipes.len()
    }))
}

pub async fn describe_intent(
    context: &AppState,
    intent_name: String
) -> Result<Value, MillError> {
    let recipe = context.planner.get_recipe(&intent_name)?;
    Ok(json!({
        "name": intent_name,
        "description": recipe.name,
        "required_params": recipe.required_params,
        "optional_params": recipe.optional_params,
        "steps": recipe.steps.len(),
        "complexity": recipe.metadata.complexity
    }))
}
```

2. **Register new tools** (if implementing discovery):
```rust
// File: crates/mill-plugin-system/src/system_tools_plugin.rs
// Add to tool list:
Tool {
    name: "list_available_intents".to_string(),
    description: "List all available workflow intents".to_string(),
    // ...
},
Tool {
    name: "describe_intent".to_string(),
    description: "Get details about a specific intent".to_string(),
    // ...
}
```

**Impact**:
- Multi-step operations become single tool calls
- Consistent workflow patterns across codebase
- Foundation for AI-driven automated refactoring

**Testing**:
```bash
cargo nextest run -p mill-handlers -- workflow
cargo nextest run -p mill-services -- planner
# Manual: Call achieve_intent with sample workflows
```

---

### 2.2 Java Plugin: Maven Dependency Management Tool ⏱️ 2-4 hours

**Status**: ✅ Functions fully implemented and tested, needs tool wrapper.

**Current State**:
- `add_dependency_to_pom()` and `write_dependency()` are production-ready
- Comprehensive tests already exist
- Missing: MCP tool handler

**Solution**: Create `workspace.add_java_dependency` tool following Rust pattern.

**New File**: `crates/mill-handlers/src/handlers/tools/workspace_add_java_dependency.rs`
```rust
use crate::AppState;
use mill_foundation::{MillError, MillResult};
use mill_lang_java::JavaManifestUpdater;
use serde_json::{json, Value};
use std::path::PathBuf;

pub async fn workspace_add_java_dependency(
    context: &AppState,
    manifest_path: PathBuf,
    group_id: String,
    artifact_id: String,
    version: String,
    dry_run: Option<bool>,
) -> MillResult<Value> {
    let dry_run = dry_run.unwrap_or(true);

    // Read current pom.xml
    let content = tokio::fs::read_to_string(&manifest_path)
        .await
        .map_err(|e| MillError::file_read(&manifest_path, e))?;

    // Add dependency using Java plugin function
    let updater = JavaManifestUpdater::new();
    let updated_content = updater.add_dependency_to_pom(
        &content,
        &group_id,
        &artifact_id,
        &version,
    )?;

    if !dry_run {
        tokio::fs::write(&manifest_path, &updated_content)
            .await
            .map_err(|e| MillError::file_write(&manifest_path, e))?;
    }

    Ok(json!({
        "success": true,
        "manifest_path": manifest_path,
        "dependency": {
            "groupId": group_id,
            "artifactId": artifact_id,
            "version": version
        },
        "dry_run": dry_run,
        "preview": if dry_run { Some(updated_content) } else { None }
    }))
}
```

**Files to Edit**:
```rust
// File: languages/mill-lang-java/src/manifest_updater.rs:246,312
// Remove #[allow(dead_code)] from add_dependency_to_pom and write_dependency

// File: languages/mill-lang-java/src/lib.rs
// Add public method to expose functionality:
impl JavaManifestUpdater {
    pub fn add_dependency_to_pom(
        &self,
        content: &str,
        group_id: &str,
        artifact_id: &str,
        version: &str,
    ) -> PluginResult<String> {
        crate::manifest_updater::add_dependency_to_pom(content, group_id, artifact_id, version)
    }
}

// File: crates/mill-handlers/src/handlers/tools/mod.rs
pub mod workspace_add_java_dependency;

// File: crates/mill-plugin-system/src/system_tools_plugin.rs
// Add to tool list:
Tool {
    name: "workspace.add_java_dependency".to_string(),
    description: "Add Maven dependency to pom.xml".to_string(),
    inputSchema: json!({
        "type": "object",
        "properties": {
            "manifest_path": {
                "type": "string",
                "description": "Path to pom.xml file"
            },
            "group_id": {
                "type": "string",
                "description": "Maven group ID (e.g., 'org.junit')"
            },
            "artifact_id": {
                "type": "string",
                "description": "Maven artifact ID (e.g., 'junit')"
            },
            "version": {
                "type": "string",
                "description": "Dependency version (e.g., '4.13.2')"
            },
            "dry_run": {
                "type": "boolean",
                "description": "Preview changes without applying",
                "default": true
            }
        },
        "required": ["manifest_path", "group_id", "artifact_id", "version"]
    })
}
```

**Impact**:
- Enables Java workspace dependency management
- Matches existing Rust `workspace.extract_dependencies` pattern
- Completes workspace operations for Java projects

**Testing**:
```bash
# Unit tests (already exist):
cargo nextest run -p mill-lang-java -- add_dependency

# Integration test (new):
# File: tests/e2e/src/test_workspace_add_java_dependency.rs
```

---

### 2.3 Enhanced CLI: Integrate mill-client Formatting ⏱️ 2-4 hours

**Status**: ✅ All functions production-ready, needs gradual integration into mill CLI.

**Current State**:
- `Formatter` and `Interactive` fully implemented
- Only `format_plan()` currently used
- Existing CLI uses basic `println!` with emojis

**Solution**: Incremental enhancement of existing commands.

**Priority Commands**:

1. **`mill status`** - Use `Formatter::status_summary()` (30 min)
```rust
// File: apps/mill/src/cli/mod.rs:989-1096
use mill_client::formatting::Formatter;

let formatter = Formatter::default();
let status_items = vec![
    ("Server".to_string(), if is_running { "Running" } else { "Stopped" }.to_string(), is_running),
    ("Config".to_string(), config_path.display().to_string(), true),
    ("Port".to_string(), format!("{}", port), true),
];
println!("{}", formatter.status_summary(&status_items));
```

2. **`mill tools`** - Use `Formatter::table()` (30 min)
```rust
// File: apps/mill/src/cli/mod.rs (around line 1600)
use mill_client::formatting::Formatter;

let formatter = Formatter::default();
let headers = &["Tool", "Description", "Category"];
let rows: Vec<Vec<String>> = tools.iter().map(|t| {
    vec![t.name.clone(), t.description.clone(), t.category.unwrap_or_default()]
}).collect();
println!("{}", formatter.table(headers, &rows));
```

3. **`mill setup --interactive`** - Use `Interactive::wizard()` (1-2 hours)
```rust
// File: apps/mill/src/cli/mod.rs:805-978
use mill_client::interactive::Interactive;

let interactive = Interactive::new();
interactive.banner("🚀 TypeMill Setup", Some("Configure your server"))?;

let config_path = interactive.input(
    "Configuration directory",
    Some(&default_config_dir),
    None
)?;

let port = interactive.number_input(
    "Server port",
    3040,
    Some((1024, 65535))
)?;

let should_install_lsp = interactive.confirm(
    "Install LSP servers now?",
    Some(false)
)?;
```

4. **`mill doctor`** - Use `Formatter::table()` and `status_summary()` (30 min)

**Impact**:
- Better user experience with formatted output
- Consistent styling across all commands
- Foundation for full interactive CLI mode

**Testing**:
```bash
# Manual testing recommended (CLI output visual)
cargo build --release
./target/release/mill status
./target/release/mill tools
./target/release/mill setup --interactive
```

---

## Phase 3: Medium-Priority Enhancements (Total: 4-8 hours)

### 3.1 Python Plugin: Type-Based Refactoring Safety ⏱️ 4-8 hours

**Current State**:
- `PythonValueType` enum fully implemented and populated
- Refactoring operations don't use type information for safety checks

**Solution**: Enhance refactoring logic to check value types.

**Example Enhancement**:
```rust
// File: languages/mill-lang-python/src/refactoring.rs:93-143
pub(crate) fn analyze_inline_variable(
    source: &str,
    variable_name: &str,
    file_path: &str,
) -> RefactoringResult<InlinableVariable> {
    let variables = extract_python_variables(source)?;
    let variable = variables.iter()
        .find(|v| v.name == variable_name)
        .ok_or_else(|| RefactoringError::NotFound(format!("Variable '{}'", variable_name)))?;

    // NEW: Type-based safety check
    let is_safe_to_inline = matches!(
        variable.value_type,
        PythonValueType::String
        | PythonValueType::Number
        | PythonValueType::Boolean
        | PythonValueType::None
    );

    if !is_safe_to_inline {
        return Ok(InlinableVariable {
            can_inline: false,
            blocking_reasons: vec![
                format!("Cannot inline mutable type: {:?}", variable.value_type)
            ],
            // ...
        });
    }

    // ... rest of analysis
}
```

**Files to Edit**:
```rust
// File: languages/mill-lang-python/src/parser.rs:306,337,442
// Remove #[allow(dead_code)] from helper functions

// File: languages/mill-lang-python/src/refactoring.rs:93-143
// Add type safety checks to analyze_inline_variable

// File: languages/mill-lang-python/src/refactoring.rs:146-191
// Consider type in analyze_extract_variable for smart naming
```

**Impact**:
- Safer Python refactoring operations
- Prevents inlining of mutable types (lists, dicts)
- Better error messages for users

**Testing**:
```bash
cargo nextest run -p mill-lang-python
# Add tests for type-based refactoring safety
```

---

## Phase 4: Not Recommended for Integration

### 4.1 LSP Direct Access ❌ Not Recommended

**Reason**: Duplicates existing functionality without clear value proposition.

**Current State**:
- 28 LSP types defined but not integrated
- Existing `lsp-types` crate already provides these types
- 29 MCP tools already cover standard LSP operations
- Would bypass safety/validation layers

**Recommendation**:
- **Option A**: Delete model files (reduce maintenance burden)
- **Option B**: Convert to internal-only tools for workflows (not MCP-exposed)
- **Option C**: Extend existing tools with missing LSP methods instead

**If pursuing Option C**:
- Add `get_inlay_hints`, `get_semantic_tokens`, `get_folding_ranges` as specific tools
- Use existing `lsp-types` crate, not custom models
- Follow proven architecture of 29 existing tools

---

### 4.2 FUSE Filesystem ❌ Not Recommended

**Reason**: Significant implementation effort with unclear use case.

**Current State**:
- Only model definitions exist (0% implementation)
- Requires `fuser` crate + system dependencies
- Platform-limited (Unix/Linux only)
- No design docs or user stories

**Recommendation**:
- Defer until clear use case emerges
- If pursued: design security model, start read-only, feature-gate

**Estimated Effort**: 8-13 days (1.5-2.5 weeks)

---

## Implementation Timeline

### Week 1: Quick Wins + High-Impact

| Day | Task | Hours | Status |
|-----|------|-------|--------|
| Mon | Phase 1.1: C Plugin Enhanced Imports | 0.5 | Ready |
| Mon | Phase 1.2: Plugin Metrics in health_check | 0.5 | Ready |
| Mon | Phase 2.1: Create default workflows.json | 2 | Ready |
| Tue | Phase 2.2: Java Dependency Management Tool | 4 | Ready |
| Wed | Phase 2.3: Enhanced CLI Formatting (part 1) | 4 | Ready |
| Thu | Phase 2.3: Enhanced CLI Formatting (part 2) | 2 | Ready |
| Fri | Testing & Documentation | 4 | Ready |

**Total Week 1**: ~17 hours

### Week 2: Enhancements + Polish

| Day | Task | Hours | Status |
|-----|------|-------|--------|
| Mon | Phase 3.1: Python Type-Based Refactoring | 4 | Ready |
| Tue | Phase 3.1: Python Type-Based Refactoring (cont.) | 4 | Ready |
| Wed | Additional workflow recipes | 2 | Optional |
| Thu | Workflow discovery tools | 2 | Optional |
| Fri | Comprehensive testing & bug fixes | 4 | Ready |

**Total Week 2**: ~16 hours

---

## Testing Strategy

### Unit Tests
```bash
# All unit tests must pass before integration
cargo nextest run --workspace
```

### Integration Tests
```bash
# Test specific features
cargo nextest run -p mill-lang-c
cargo nextest run -p mill-lang-java -- add_dependency
cargo nextest run -p mill-handlers -- health_check
cargo nextest run -p mill-services -- planner
```

### E2E Tests
```bash
# Full workflow testing
cargo nextest run -p mill --test e2e_*
```

### Manual Testing Checklist

**C Plugin Enhanced Imports**:
- [ ] Create C project with #include directives
- [ ] Call `analyze.dependencies` tool
- [ ] Verify ImportGraph includes system headers
- [ ] Check source locations are accurate

**Plugin Metrics**:
- [ ] Call `health_check` tool
- [ ] Verify `plugins` and `metrics` fields exist
- [ ] Make several tool calls
- [ ] Call `health_check` again, verify counts increased

**Intent Workflows**:
- [ ] Place workflows.json in .typemill/
- [ ] Call `achieve_intent` with refactor.renameSymbol
- [ ] Verify workflow plan is generated
- [ ] Execute with `execute: true`
- [ ] Verify rename succeeded

**Java Dependency Tool**:
- [ ] Create Maven project with pom.xml
- [ ] Call `workspace.add_java_dependency`
- [ ] Verify dependency added to XML
- [ ] Test dry_run mode

**Enhanced CLI**:
- [ ] Run `mill status` - verify formatted output
- [ ] Run `mill tools` - verify table format
- [ ] Run `mill doctor` - verify status summary

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking changes to public API | Low | High | All changes are additive |
| Test failures | Medium | Medium | Comprehensive test suite in place |
| Performance regression | Low | Medium | Metrics are already tracked, minimal overhead |
| Configuration file issues | Medium | Low | Planner gracefully handles missing file |
| CLI formatting regressions | Low | Low | Incremental rollout, easy to revert |

---

## Success Metrics

### Quantitative
- ✅ All existing tests pass (244+ tests)
- ✅ New features have >80% test coverage
- ✅ No performance regression (metrics tracked)
- ✅ Zero compiler warnings maintained

### Qualitative
- ✅ Enhanced user experience (formatted output, better errors)
- ✅ Feature parity with TypeScript version (workflows restored)
- ✅ Improved observability (metrics exposed)
- ✅ Java workspace operations complete

---

## Next Steps (Immediate)

1. **Review this plan** - Approve/modify approach
2. **Start with Phase 1** - Quick wins for immediate value
3. **Create feature branch** - `feature/pending-integrations`
4. **Implement incrementally** - One feature at a time with tests
5. **Document as you go** - Update user-facing docs

---

## Files Summary

### Files to Create
- `.typemill/workflows.json` - Default workflow recipes
- `crates/mill-handlers/src/handlers/tools/workspace_add_java_dependency.rs` - Java dependency tool
- `crates/mill-handlers/src/handlers/workflow_discovery.rs` - Optional workflow tools
- `tests/e2e/src/test_workspace_add_java_dependency.rs` - Integration tests

### Files to Modify
- `languages/mill-lang-c/src/import_support.rs` - Remove dead_code
- `languages/mill-lang-c/src/lib.rs` - Add override method
- `languages/mill-lang-java/src/manifest_updater.rs` - Remove dead_code
- `languages/mill-lang-python/src/parser.rs` - Remove dead_code
- `crates/mill-handlers/src/handlers/system_handler.rs` - Enhance health_check
- `crates/mill-plugin-system/src/registry.rs` - Remove dead_code
- `crates/mill-services/src/services/ast/import_service.rs` - Delete ImportUpdateReport
- `apps/mill/src/cli/mod.rs` - Integrate mill-client formatting

### Files to Consider Deleting
- `crates/mill-foundation/src/model/lsp.rs` - Duplicate of lsp-types
- `crates/mill-foundation/src/model/fuse.rs` - Unimplemented experimental feature

---

## Questions for Review

1. **Workflow Recipes**: What other common workflows should be included?
2. **CLI Integration**: Full rewrite or incremental enhancement? (Recommendation: incremental)
3. **LSP Models**: Delete or keep for future? (Recommendation: delete)
4. **FUSE**: Archive for later or remove entirely? (Recommendation: archive with design doc)

---

## Conclusion

TypeMill has **excellent foundations** with most pending features being **production-ready code** that just needs wiring. The integration effort is **modest** (17-33 hours total) with **high user value**.

**Recommended Start**: Phase 1 quick wins (40 minutes) for immediate impact, then proceed to high-value integrations in Phase 2.

All code has been thoroughly analyzed, test infrastructure is mature, and there are no major blockers to integration.
