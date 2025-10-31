# Proposal 16b: Swift Language Plugin Parity Completion

**Status**: Pending
**Language**: Swift (mill-lang-swift)
**Current Completion**: ~42% (5/12 traits)
**Test Coverage**: 3 tests in lib.rs

## Current State

### ✅ Implemented (5/12 traits)
- ImportParser (via import_support)
- ImportRenameSupport (via import_support)
- ImportMoveSupport (via import_support)
- ImportMutationSupport (via import_support)
- ImportAdvancedSupport (via import_support)

### ❌ Missing (7/12 traits)
1. **WorkspaceSupport** - NO workspace_support module or field
2. **RefactoringProvider** - NO refactoring module or impl block
3. **ModuleReferenceScanner** - Not delegated
4. **ImportAnalyzer** - Not delegated
5. **ManifestUpdater** - Not delegated
6. **LspInstaller** - Not in fields
7. **ProjectFactory** - EXISTS but minimal (only 3 tests)

## Code Evidence

**File**: `languages/mill-lang-swift/src/lib.rs`

```rust
// Line 13: Missing with_workspace in capabilities!
define_language_plugin! {
    capabilities: [with_imports, with_project_factory],  // NO with_workspace!
    fields: {
        import_support: import_support::SwiftImportSupport,
        project_factory: project_factory::SwiftProjectFactory,
        // NO workspace_support field
        // NO refactoring module
        // NO lsp_installer field
    },
}
```

```rust
// Line 108: Only delegates import traits
impl_capability_delegations! {
    import_support => {
        import_parser: ImportParser,
        // ... other import traits
    },
    // Missing: refactoring_provider, module_reference_scanner, import_analyzer, manifest_updater
}
```

## Implementation Plan

### Phase 1: WorkspaceSupport (Priority: HIGH)
**Why**: Required for Swift Package Manager multi-package workspaces

1. Create `workspace_support.rs` module:
   ```rust
   pub struct SwiftWorkspaceSupport;

   #[async_trait]
   impl WorkspaceSupport for SwiftWorkspaceSupport {
       async fn add_workspace_member(&self, workspace_manifest: &str, member_path: &str)
           -> PluginResult<String>
       {
           // Parse Package.swift
           // Add to dependencies: .package(path: "...")
       }

       async fn remove_workspace_member(&self, workspace_manifest: &str, member_path: &str)
           -> PluginResult<String>
       {
           // Remove .package(...) entry
       }

       async fn list_workspace_members(&self, workspace_manifest: &str)
           -> PluginResult<Vec<String>>
       {
           // Parse all .package(path: "...") entries
           // Return list of local dependencies
       }
   }
   ```

2. Update lib.rs:
   ```rust
   pub mod workspace_support;

   define_language_plugin! {
       capabilities: [with_imports, with_workspace, with_project_factory],
       fields: {
           workspace_support: workspace_support::SwiftWorkspaceSupport,
       }
   }

   impl_capability_delegations! {
       workspace_support => {
           workspace_support: WorkspaceSupport,
       },
   }
   ```

**Reference**: Java workspace_support.rs, Python workspace_support.rs

### Phase 2: RefactoringProvider (Priority: HIGH)
**Why**: Core feature for code manipulation

1. Create `refactoring.rs` module:
   ```rust
   use mill_lang_common::CodeRange;

   pub fn plan_extract_function(
       source: &str,
       range: &CodeRange,
       function_name: &str,
       file_path: &str,
   ) -> Result<EditPlan, String> {
       // Extract selected lines into a new func
       // Handle parameter detection (no complex scope analysis needed)
   }

   pub fn plan_inline_variable(
       source: &str,
       variable_line: u32,
       variable_col: u32,
       file_path: &str,
   ) -> Result<EditPlan, String> {
       // Find: let varName = <value>
       // Replace all references with <value>
   }

   pub fn plan_extract_variable(
       source: &str,
       start_line: u32,
       start_col: u32,
       end_line: u32,
       end_col: u32,
       variable_name: Option<String>,
       file_path: &str,
   ) -> Result<EditPlan, String> {
       // Extract expression into: let varName = <expression>
   }
   ```

2. Add impl block in lib.rs:
   ```rust
   #[async_trait]
   impl RefactoringProvider for SwiftPlugin {
       fn supports_extract_function(&self) -> bool { true }

       async fn plan_extract_function(&self, ...) -> PluginResult<EditPlan> {
           refactoring::plan_extract_function(source, &range, function_name, file_path)
               .map_err(|e| PluginError::internal(e))
       }

       fn supports_inline_variable(&self) -> bool { true }

       async fn plan_inline_variable(&self, ...) -> PluginResult<EditPlan> {
           refactoring::plan_inline_variable(source, variable_line, variable_col, file_path)
               .map_err(|e| PluginError::internal(e))
       }

       fn supports_extract_variable(&self) -> bool { true }

       async fn plan_extract_variable(&self, ...) -> PluginResult<EditPlan> {
           refactoring::plan_extract_variable(...)
               .map_err(|e| PluginError::internal(e))
       }
   }

   impl_capability_delegations! {
       this => {
           refactoring_provider: RefactoringProvider,
       },
   }
   ```

**Reference**: C# refactoring.rs (similar syntax), Java refactoring.rs

### Phase 3: Analysis Traits (Priority: MEDIUM)

#### 3a. ModuleReferenceScanner
Add impl block in lib.rs:
```rust
impl ModuleReferenceScanner for SwiftPlugin {
    fn scan_references(&self, content: &str, module_name: &str, scope: ScanScope)
        -> PluginResult<Vec<ModuleReference>>
    {
        // Scan for: import module_name
        // Qualified paths: module_name.Type
        // String literals: "module_name/*.swift"
    }
}
```

#### 3b. ImportAnalyzer
```rust
impl ImportAnalyzer for SwiftPlugin {
    fn build_import_graph(&self, file_path: &Path)
        -> PluginResult<ImportGraph>
    {
        let content = std::fs::read_to_string(file_path)?;
        // Parse import statements
        // Build dependency graph
    }
}
```

#### 3c. ManifestUpdater
```rust
#[async_trait]
impl ManifestUpdater for SwiftPlugin {
    async fn update_dependency(&self, manifest_path: &Path,
        old_name: &str, new_name: &str, new_version: Option<&str>)
        -> PluginResult<String>
    {
        // Update .package(name: "...", ...) in Package.swift
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String])
        -> String
    {
        // Generate Package.swift
    }
}
```

### Phase 4: LspInstaller (Priority: LOW)
**Why**: sourcekit-lsp comes with Xcode, usually pre-installed

1. Create `lsp_installer.rs` module:
   ```rust
   pub struct SwiftLspInstaller;

   #[async_trait]
   impl LspInstaller for SwiftLspInstaller {
       async fn is_installed(&self) -> bool {
           // Check for sourcekit-lsp in PATH
           Command::new("sourcekit-lsp").arg("--version").status().is_ok()
       }

       async fn install(&self) -> PluginResult<()> {
           // On macOS: Comes with Xcode
           // On Linux: apt-get install swift sourcekit-lsp
       }
   }
   ```

### Phase 5: Test Coverage (Priority: HIGH)
**Current**: 3 basic tests
**Target**: 15+ tests covering all capabilities

Add to lib.rs #[cfg(test)]:
```rust
#[tokio::test]
async fn test_workspace_support() {
    let plugin = SwiftPlugin::new();
    assert!(plugin.workspace_support().is_some());
}

#[tokio::test]
async fn test_refactoring_extract_function() {
    let plugin = SwiftPlugin::new();
    let source = r#"
func main() {
    let x = 5
    let y = x * 2
    print(y)
}
"#;
    let result = plugin.plan_extract_function(source, 2, 4, "calculate", "test.swift").await;
    assert!(result.is_ok());
}

#[test]
fn test_module_reference_scanner() {
    let plugin = SwiftPlugin::new();
    let source = "import Foundation\nlet x = Foundation.Date()";
    let refs = plugin.scan_references(source, "Foundation", ScanScope::All).unwrap();
    assert_eq!(refs.len(), 2); // import + qualified path
}
```

## Success Criteria

- [ ] WorkspaceSupport implemented with Package.swift management
- [ ] RefactoringProvider implements 3 operations (extract function/variable, inline variable)
- [ ] ModuleReferenceScanner finds import statements and qualified paths
- [ ] ImportAnalyzer builds import dependency graph
- [ ] ManifestUpdater handles Package.swift updates
- [ ] LspInstaller can detect/install sourcekit-lsp
- [ ] Test count increased from 3 to 15+
- [ ] All tests pass: `cargo nextest run -p mill-lang-swift`
- [ ] CLAUDE.md parity table shows Swift as 100% (currently shows ✅ incorrectly)

## Effort Estimate

- Phase 1 (WorkspaceSupport): 5-7 hours
- Phase 2 (RefactoringProvider): 8-10 hours
- Phase 3 (Analysis traits): 6-8 hours
- Phase 4 (LspInstaller): 2-3 hours
- Phase 5 (Tests): 4-5 hours
- **Total**: 25-33 hours

## Dependencies

- None (can proceed independently)

## References

- Python plugin (100% parity): `languages/mill-lang-python/`
- Java plugin (100% parity): `languages/mill-lang-java/`
- C# refactoring: `languages/mill-lang-csharp/src/refactoring.rs`
