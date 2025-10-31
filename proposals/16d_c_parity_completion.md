# Proposal 16d: C Language Plugin Parity Completion

**Status**: Pending (Experimental Language)
**Language**: C (mill-lang-c)
**Current Completion**: ~42% (5/12 traits)
**Test Coverage**: 0 tests (tests module exists but empty)

## Current State

### ✅ Implemented (5/12 traits)
- ImportParser (via CImportSupport)
- ImportRenameSupport (via CImportSupport)
- ImportMoveSupport (via CImportSupport)
- ImportMutationSupport (via CImportSupport)
- ImportAdvancedSupport (via CImportSupport)

### ⚠️ Stub Implementation (1/12 traits)
- RefactoringProvider - EXISTS but all supports_*() return false (stubs only)

### ❌ Missing (6/12 traits)
1. **WorkspaceSupport** - Not implemented
2. **ProjectFactory** - Not implemented
3. **ModuleReferenceScanner** - Not implemented
4. **ImportAnalyzer** - Not implemented
5. **ManifestUpdater** - Not implemented
6. **LspInstaller** - Not implemented

## Code Evidence

**File**: `languages/mill-lang-c/src/lib.rs`

```rust
// Line 62: Only claims imports
fn capabilities(&self) -> PluginCapabilities {
    PluginCapabilities::none().with_imports()  // Only imports!
}

// Line 104-159: RefactoringProvider exists but is all stubs
impl mill_plugin_api::RefactoringProvider for CPlugin {
    fn supports_inline_variable(&self) -> bool {
        false  // ❌ Stub - not yet supported
    }

    fn supports_extract_function(&self) -> bool {
        false  // ❌ Stub - not yet supported
    }

    fn supports_extract_variable(&self) -> bool {
        false  // ❌ Stub - not yet supported
    }
}
```

**File**: `languages/mill-lang-c/src/tests.rs`
- Exists but has no test functions! (Empty module)

## Recommendation: Keep as Experimental

C language support should remain **experimental** with limited features due to:

1. **Complexity**: C has no standard package manager (uses Makefile/CMake/autotools)
2. **Module system**: C has no native module system (only #include)
3. **LSP support**: clangd works but requires compile_commands.json
4. **Use case**: Most users need C++ support instead

### Proposed Experimental Feature Set (60% parity)

Keep C plugin at **intentionally limited scope** with focus on import/refactoring basics:

## Implementation Plan (If Full Parity Desired)

### Phase 1: ProjectFactory (Priority: MEDIUM)
**Why**: Allow creating new C projects with Makefile templates

1. Create `project_factory.rs` module:
   ```rust
   pub struct CProjectFactory;

   impl ProjectFactory for CProjectFactory {
       fn create_package(&self, config: &CreatePackageConfig)
           -> PluginResult<CreatePackageResult>
       {
           // Create directory structure:
           // - src/
           // - include/
           // - Makefile (basic template)
           // - main.c (hello world)
       }
   }
   ```

2. Update lib.rs:
   ```rust
   fn capabilities(&self) -> PluginCapabilities {
       PluginCapabilities::none()
           .with_imports()
           .with_project_factory()  // Add this
   }

   fn project_factory(&self) -> Option<&dyn ProjectFactory> {
       Some(&project_factory::CProjectFactory)
   }
   ```

### Phase 2: Implement RefactoringProvider (Priority: LOW)
**Why**: Currently just stubs

1. Update `refactoring.rs` to implement:
   - `plan_inline_variable`: Replace variable uses with its value
   - `plan_extract_function`: Extract code block into new function
   - `plan_extract_variable`: Extract expression into variable

2. Change supports_*() to return true in lib.rs

**Challenge**: C refactoring is complex due to:
- Pointer semantics
- Manual memory management
- Preprocessor macros
- Type inference difficulties

**Recommendation**: Keep as stubs or implement basic cases only

### Phase 3: Test Coverage (Priority: HIGH if implementing)
**Current**: 0 tests (empty tests.rs module)
**Target**: 10+ tests

Add to tests.rs:
```rust
#[test]
fn test_c_plugin_creation() {
    let plugin = CPlugin::default();
    assert_eq!(plugin.metadata().name, "C");
}

#[test]
fn test_c_import_parsing() {
    let plugin = CPlugin::default();
    let source = r#"
#include <stdio.h>
#include "myheader.h"
"#;
    let parser = plugin.import_parser().unwrap();
    let imports = parser.parse_imports(source);
    assert_eq!(imports.len(), 2);
}

#[tokio::test]
async fn test_parse_source() {
    let plugin = CPlugin::default();
    let source = r#"
int add(int a, int b) {
    return a + b;
}
"#;
    let result = plugin.parse(source).await;
    assert!(result.is_ok());
}
```

### Phase 4: WorkspaceSupport (Priority: LOW)
**Why**: C doesn't have standard workspace format

**Options**:
1. Support CMakeLists.txt with add_subdirectory()
2. Support multi-Makefile projects
3. Skip entirely (C projects typically monolithic)

**Recommendation**: Skip WorkspaceSupport for C

### Phase 5: Analysis Traits (Priority: LOW)

Skip these for experimental C support:
- ModuleReferenceScanner (C has no modules)
- ImportAnalyzer (basic #include scanning sufficient)
- ManifestUpdater (no standard manifest format)
- LspInstaller (clangd installation varies by platform)

## Success Criteria (Experimental Scope)

- [ ] ProjectFactory can create basic C projects with Makefile
- [ ] RefactoringProvider stubs remain as-is (don't claim support)
- [ ] Import parsing works for #include directives
- [ ] Test count increased from 0 to 10+
- [ ] All tests pass: `cargo nextest run -p mill-lang-c`
- [ ] CLAUDE.md parity table shows C as ⚠️ Experimental (not ✅)
- [ ] Documentation clarifies C is experimental with limited scope

## Success Criteria (Full Parity - NOT RECOMMENDED)

- [ ] ProjectFactory creates C projects
- [ ] RefactoringProvider implements 3 operations
- [ ] WorkspaceSupport handles CMake/Makefile workspaces
- [ ] ModuleReferenceScanner finds #include directives
- [ ] ImportAnalyzer builds include dependency graph
- [ ] ManifestUpdater handles Makefile updates
- [ ] LspInstaller can install clangd
- [ ] Test count 15+
- [ ] CLAUDE.md shows C as 100%

## Effort Estimate

**Experimental scope** (Recommended):
- ProjectFactory: 4-5 hours
- Test coverage: 3-4 hours
- Documentation: 1-2 hours
- **Total**: 8-11 hours

**Full parity** (Not recommended):
- ProjectFactory: 4-5 hours
- RefactoringProvider: 10-12 hours (complex due to C semantics)
- WorkspaceSupport: 6-8 hours
- Analysis traits: 8-10 hours
- Test coverage: 5-6 hours
- **Total**: 33-41 hours

## Recommendation

**Keep C plugin as experimental (60% parity)** with:
- ✅ Import parsing (#include directives)
- ✅ ProjectFactory (Makefile-based projects)
- ✅ Basic parsing (AST extraction)
- ❌ Skip RefactoringProvider (too complex for C)
- ❌ Skip WorkspaceSupport (no standard format)
- ❌ Skip advanced analysis traits

**Rationale**:
1. C has no standard package manager or module system
2. Most users wanting C support actually need C++
3. Refactoring C code safely requires deep semantic analysis
4. Limited ROI for full C parity vs. C++ improvements

## Alternative: Deprecate C Plugin

If not worth maintaining, consider:
1. Remove from default build
2. Mark as "community supported"
3. Focus resources on C++ plugin instead

## Dependencies

- None (can proceed independently)

## References

- C++ plugin (similar AST parsing): `languages/mill-lang-cpp/`
- Python ProjectFactory: `languages/mill-lang-python/src/project_factory.rs`
