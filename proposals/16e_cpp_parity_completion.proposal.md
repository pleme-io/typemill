# Proposal 16e: C++ Language Plugin Parity Completion & Validation

**Status**: Pending (Validation Required)
**Language**: C++ (mill-lang-cpp)
**Current Completion**: ~83% (10/12 traits claimed, needs validation)
**Test Coverage**: 0 tests (no lib.rs #[cfg(test)] module)

## Current State

### ✅ Claimed Implementation (10/12 traits)
- ImportParser (via CppImportSupport)
- ImportRenameSupport (via CppImportSupport)
- ImportMoveSupport (via CppImportSupport)
- ImportMutationSupport (via CppImportSupport)
- ImportAdvancedSupport (via CppImportSupport)
- WorkspaceSupport (via CppWorkspaceSupport)
- RefactoringProvider (via CppRefactoringProvider)
- ProjectFactory (via CppProjectFactory)
- ModuleReferenceScanner (via CppAnalysisProvider)
- ImportAnalyzer (via CppAnalysisProvider)

### ❌ Missing (2/12 traits)
1. **ManifestUpdater** - Not implemented
2. **LspInstaller** - Not implemented

### ⚠️ Needs Validation (All 10 claimed traits)
**Critical**: NO TESTS exist to validate any claimed functionality!

## Code Evidence

**File**: `languages/mill-lang-cpp/src/lib.rs`

```rust
// Line 70: Claims imports + workspace
fn capabilities(&self) -> PluginCapabilities {
    PluginCapabilities::none()
        .with_imports()
        .with_workspace()
}

// Lines 76-119: Returns Some for many traits
impl LanguagePlugin for CppPlugin {
    fn import_parser(&self) -> Option<&dyn ImportParser> {
        Some(&import_support::CppImportSupport)  // ⚠️ Not validated
    }

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        Some(&workspace_support::CppWorkspaceSupport)  // ⚠️ Not validated
    }

    fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
        Some(&refactoring::CppRefactoringProvider)  // ⚠️ Not validated
    }

    fn project_factory(&self) -> Option<&dyn ProjectFactory> {
        Some(&project_factory::CppProjectFactory)  // ⚠️ Not validated
    }

    fn module_reference_scanner(&self) -> Option<&dyn ModuleReferenceScanner> {
        Some(&analysis::CppAnalysisProvider)  // ⚠️ Not validated
    }

    fn import_analyzer(&self) -> Option<&dyn ImportAnalyzer> {
        Some(&analysis::CppAnalysisProvider)  // ⚠️ Not validated
    }
}
```

**Critical Issue**: lib.rs has NO #[cfg(test)] module at all!

## Implementation Plan

### Phase 1: Validation & Test Coverage (Priority: CRITICAL)
**Why**: Can't trust claimed features without tests

1. **Create tests.rs or lib.rs #[cfg(test)] module**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       // Basic plugin tests
       #[test]
       fn test_cpp_plugin_creation() {
           let plugin = CppPlugin::default();
           assert_eq!(plugin.metadata().name, "C++");
       }

       #[test]
       fn test_cpp_capabilities() {
           let plugin = CppPlugin::default();
           let caps = plugin.capabilities();
           assert!(caps.imports);
           assert!(caps.workspace);
       }

       // Import tests
       #[test]
       fn test_import_parser() {
           let plugin = CppPlugin::default();
           let parser = plugin.import_parser().unwrap();
           let source = r#"
   #include <iostream>
   #include "myheader.hpp"
   "#;
           let imports = parser.parse_imports(source);
           assert_eq!(imports.len(), 2);
           assert!(imports.contains(&"iostream".to_string()));
       }

       // Workspace tests
       #[tokio::test]
       async fn test_workspace_support() {
           let plugin = CppPlugin::default();
           let ws = plugin.workspace_support().unwrap();

           let cmake_content = r#"
   cmake_minimum_required(VERSION 3.10)
   project(MyProject)
   "#;

           let result = ws.add_workspace_member(cmake_content, "subdir").await;
           assert!(result.is_ok());
           assert!(result.unwrap().contains("add_subdirectory(subdir)"));
       }

       // Refactoring tests
       #[tokio::test]
       async fn test_refactoring_inline_variable() {
           let plugin = CppPlugin::default();
           let refactor = plugin.refactoring_provider().unwrap();

           let source = r#"
   int main() {
       int x = 5;
       int y = x * 2;
       return y;
   }
   "#;

           let result = refactor.plan_inline_variable(source, 2, 8, "test.cpp").await;
           assert!(result.is_ok());
       }

       // ProjectFactory tests
       #[test]
       fn test_project_factory() {
           let plugin = CppPlugin::default();
           let factory = plugin.project_factory().unwrap();

           let temp_dir = tempfile::tempdir().unwrap();
           let config = mill_plugin_api::CreatePackageConfig {
               workspace_root: temp_dir.path().to_str().unwrap().to_string(),
               package_path: "my_cpp_project".to_string(),
               package_type: mill_plugin_api::PackageType::Binary,
               template: mill_plugin_api::Template::Minimal,
               add_to_workspace: false,
           };

           let result = factory.create_package(&config);
           assert!(result.is_ok());
       }

       // Analysis tests
       #[test]
       fn test_module_reference_scanner() {
           let plugin = CppPlugin::default();
           let scanner = plugin.module_reference_scanner().unwrap();

           let source = r#"
   #include "OldModule.hpp"
   void foo() {
       OldModule::doSomething();
   }
   "#;

           let refs = scanner.scan_references(source, "OldModule", mill_plugin_api::ScanScope::All);
           assert!(refs.is_ok());
           assert!(!refs.unwrap().is_empty());
       }

       #[test]
       fn test_import_analyzer() {
           let plugin = CppPlugin::default();
           let analyzer = plugin.import_analyzer().unwrap();

           let temp_file = tempfile::NamedTempFile::new().unwrap();
           std::fs::write(&temp_file, r#"
   #include <vector>
   #include "myheader.hpp"
   "#).unwrap();

           let result = analyzer.build_import_graph(temp_file.path());
           assert!(result.is_ok());
       }
   }
   ```

**Target**: 15+ tests covering all 10 claimed traits

### Phase 2: Fix Discovered Issues (Priority: HIGH)
**After Phase 1 testing, likely to find**:

1. **Incomplete implementations**: Traits return Some but methods are stubs
2. **Parsing bugs**: tree-sitter-cpp integration issues
3. **Manifest parsing**: CMakeLists.txt, conanfile, vcpkg.json edge cases
4. **Refactoring bugs**: C++ template/namespace complexity

**Action**: Fix issues discovered during validation

### Phase 3: ManifestUpdater (Priority: MEDIUM)
**Why**: Required for updating CMakeLists.txt dependencies

1. Create `manifest_updater.rs` module:
   ```rust
   pub struct CppManifestUpdater;

   #[async_trait]
   impl ManifestUpdater for CppManifestUpdater {
       async fn update_dependency(&self, manifest_path: &Path,
           old_name: &str, new_name: &str, new_version: Option<&str>)
           -> PluginResult<String>
       {
           // Handle multiple manifest types:
           // - CMakeLists.txt: find_package(OldName) -> find_package(NewName)
           // - conanfile.txt: [requires] old/version -> new/version
           // - vcpkg.json: "dependencies": ["old"] -> ["new"]
       }

       fn generate_manifest(&self, package_name: &str, dependencies: &[String])
           -> String
       {
           // Generate minimal CMakeLists.txt
       }
   }
   ```

2. Update lib.rs:
   ```rust
   fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> {
       Some(&manifest_updater::CppManifestUpdater)
   }
   ```

**Reference**: Rust ManifestUpdater (Cargo.toml updates)

### Phase 4: LspInstaller (Priority: LOW)
**Why**: clangd installation is platform-specific

1. Create `lsp_installer.rs` module:
   ```rust
   pub struct CppLspInstaller;

   #[async_trait]
   impl LspInstaller for CppLspInstaller {
       async fn is_installed(&self) -> bool {
           Command::new("clangd").arg("--version").status().is_ok()
       }

       async fn install(&self) -> PluginResult<()> {
           // Platform-specific installation:
           // - Ubuntu/Debian: apt-get install clangd
           // - macOS: brew install llvm
           // - Windows: Download from LLVM releases
           Err(PluginError::not_supported(
               "clangd installation is platform-specific. Please install manually."
           ))
       }
   }
   ```

2. Update lib.rs:
   ```rust
   fn lsp_installer(&self) -> Option<&dyn LspInstaller> {
       Some(&lsp_installer::CppLspInstaller)
   }
   ```

**Recommendation**: Provide platform-specific installation instructions instead of auto-install

### Phase 5: Integration Tests (Priority: MEDIUM)
**After unit tests pass**, add integration tests:

```rust
#[tokio::test]
async fn test_cpp_end_to_end_workflow() {
    // 1. Create project with ProjectFactory
    // 2. Parse generated files
    // 3. Test workspace operations
    // 4. Test refactoring operations
    // 5. Validate all changes
}
```

## Success Criteria

- [ ] Test count increased from 0 to 15+
- [ ] All unit tests pass: `cargo nextest run -p mill-lang-cpp`
- [ ] Validation confirms all 10 claimed traits work correctly
- [ ] ManifestUpdater handles CMakeLists.txt/conanfile/vcpkg.json
- [ ] LspInstaller provides installation guidance (even if not auto-installing)
- [ ] Integration tests cover end-to-end workflows
- [ ] Documentation clarifies C++ support status (100% or experimental)
- [ ] CLAUDE.md parity table accurate (currently shows ⚠️ correctly)

## Effort Estimate

- Phase 1 (Test coverage + validation): 8-10 hours
- Phase 2 (Fix discovered issues): 10-15 hours (depends on findings)
- Phase 3 (ManifestUpdater): 6-8 hours
- Phase 4 (LspInstaller): 2-3 hours
- Phase 5 (Integration tests): 4-5 hours
- **Total**: 30-41 hours

## Risk Assessment

**HIGH RISK**: No tests exist, so claimed features are unvalidated

**Likely issues to discover**:
1. tree-sitter-cpp parsing edge cases
2. CMakeLists.txt parsing fragility
3. C++ template/namespace handling bugs
4. Workspace support may be incomplete
5. Refactoring may only handle simple cases

**Mitigation**: Phase 1 test suite will reveal actual state

## Recommendation

1. **Priority 1**: Add comprehensive test suite (Phase 1)
2. **Priority 2**: Fix issues revealed by tests (Phase 2)
3. **Priority 3**: Add ManifestUpdater (Phase 3)
4. **Priority 4**: Document limitations clearly
5. **Optional**: LspInstaller with manual instructions (Phase 4)

## Alternative: Mark as Experimental

If Phase 1 testing reveals major gaps:
1. Mark C++ plugin as "experimental"
2. Update CLAUDE.md to show ⚠️ Experimental (not 100%)
3. Document known limitations
4. Set realistic expectations for users

## Dependencies

- None (can proceed independently)
- **Blocker**: Phase 2 depends on Phase 1 test results

## References

- Rust plugin (mature, well-tested): `languages/mill-lang-rust/`
- Python plugin (100% validated): `languages/mill-lang-python/`
- Tree-sitter C++ docs: https://github.com/tree-sitter/tree-sitter-cpp
