# Proposal 16c: Go Language Plugin Parity Completion

**Status**: Pending
**Language**: Go (mill-lang-go)
**Current Completion**: ~42% (5/12 traits)
**Test Coverage**: 3 tests in lib.rs

## Current State

### ✅ Implemented (5/12 traits)
- ImportParser (via import_support)
- ImportRenameSupport (via import_support)
- ImportMoveSupport (via import_support)
- ImportMutationSupport (via import_support)
- ProjectFactory (impl ProjectFactory for GoPlugin)

### ❌ Missing (7/12 traits)
1. **ImportAdvancedSupport** - Returns None on line 90! (CLAIMED but not implemented)
2. **WorkspaceSupport** - CAPABILITY CLAIMS workspace=true, but NO workspace_support() method!
3. **RefactoringProvider** - refactoring.rs EXISTS but NOT wired up in lib.rs!
4. **ModuleReferenceScanner** - Not implemented
5. **ImportAnalyzer** - Not implemented
6. **ManifestUpdater** - Not implemented
7. **LspInstaller** - Not implemented

## Code Evidence

**File**: `languages/mill-lang-go/src/lib.rs`

```rust
// Line 30: CLAIMS workspace support!
pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
    imports: true,
    workspace: true,  // ❌ CLAIMS workspace but doesn't implement!
    project_factory: true,
    path_alias_resolver: false,
};
```

```rust
// Line 89: Returns None for ImportAdvancedSupport!
fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
    None  // ❌ Returns None despite claiming imports=true
}

// NO workspace_support() method despite CAPABILITIES.workspace=true
// NO refactoring_provider() method despite refactoring.rs existing!
```

**Module**: `languages/mill-lang-go/src/refactoring.rs`
- File EXISTS with plan_extract_function, plan_inline_variable
- But NEVER wired up in lib.rs!

## Implementation Plan

### Phase 1: Fix False Claims (Priority: CRITICAL)
**Why**: Current code is misleading - claims features it doesn't have

1. **Fix ImportAdvancedSupport**:
   ```rust
   // In lib.rs line 89, change:
   fn import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport> {
       Some(&self.import_support)  // ✅ Return Some instead of None
   }
   ```

2. **Wire up existing refactoring.rs**:
   ```rust
   #[async_trait]
   impl RefactoringProvider for GoPlugin {
       fn supports_inline_variable(&self) -> bool { true }

       async fn plan_inline_variable(&self, source: &str, variable_line: u32,
           variable_col: u32, file_path: &str) -> PluginResult<EditPlan>
       {
           refactoring::plan_inline_variable(source, variable_line, variable_col, file_path)
               .map_err(|e| PluginError::internal(e.to_string()))
       }

       fn supports_extract_function(&self) -> bool { true }

       async fn plan_extract_function(&self, source: &str, start_line: u32,
           end_line: u32, function_name: &str, file_path: &str) -> PluginResult<EditPlan>
       {
           refactoring::plan_extract_function(source, start_line, end_line, function_name, file_path)
               .map_err(|e| PluginError::internal(e.to_string()))
       }

       fn supports_extract_variable(&self) -> bool { false } // TODO: implement
   }

   // Add to LanguagePlugin impl:
   fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
       Some(self)
   }
   ```

### Phase 2: WorkspaceSupport (Priority: HIGH)
**Why**: Go modules support multi-module workspaces (go.work files)

1. Create `workspace_support.rs` module:
   ```rust
   pub struct GoWorkspaceSupport;

   #[async_trait]
   impl WorkspaceSupport for GoWorkspaceSupport {
       async fn add_workspace_member(&self, workspace_manifest: &str, member_path: &str)
           -> PluginResult<String>
       {
           // Parse go.work file
           // Add: use ./member_path
       }

       async fn remove_workspace_member(&self, workspace_manifest: &str, member_path: &str)
           -> PluginResult<String>
       {
           // Remove use directive from go.work
       }

       async fn list_workspace_members(&self, workspace_manifest: &str)
           -> PluginResult<Vec<String>>
       {
           // Parse all "use ..." directives from go.work
       }
   }
   ```

2. Update lib.rs:
   ```rust
   pub mod workspace_support;

   pub struct GoPlugin {
       import_support: import_support::GoImportSupport,
       workspace_support: workspace_support::GoWorkspaceSupport,  // Add field
   }

   impl LanguagePlugin for GoPlugin {
       fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
           Some(&self.workspace_support)
       }
   }
   ```

**Reference**: Python workspace_support.rs, Java workspace_support.rs

### Phase 3: Analysis Traits (Priority: MEDIUM)

#### 3a. ModuleReferenceScanner
```rust
impl ModuleReferenceScanner for GoPlugin {
    fn scan_references(&self, content: &str, module_name: &str, scope: ScanScope)
        -> PluginResult<Vec<ModuleReference>>
    {
        // Scan for: import "module_name"
        // Qualified paths: module_name.Function()
        // String literals: "module_name/*.go"
    }
}
```

#### 3b. ImportAnalyzer
```rust
impl ImportAnalyzer for GoPlugin {
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
impl ManifestUpdater for GoPlugin {
    async fn update_dependency(&self, manifest_path: &Path,
        old_name: &str, new_name: &str, new_version: Option<&str>)
        -> PluginResult<String>
    {
        // Update require directive in go.mod
        // Format: require module_name v1.2.3
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String])
        -> String
    {
        // Generate go.mod file
        manifest::generate_manifest(package_name, "1.21")
    }
}
```

### Phase 4: LspInstaller (Priority: LOW)
**Why**: gopls is typically installed via `go install`

1. Create `lsp_installer.rs` module:
   ```rust
   pub struct GoLspInstaller;

   #[async_trait]
   impl LspInstaller for GoLspInstaller {
       async fn is_installed(&self) -> bool {
           Command::new("gopls").arg("version").status().is_ok()
       }

       async fn install(&self) -> PluginResult<()> {
           // Install via: go install golang.org/x/tools/gopls@latest
           let output = Command::new("go")
               .args(&["install", "golang.org/x/tools/gopls@latest"])
               .output()?;

           if !output.status.success() {
               return Err(PluginError::internal("Failed to install gopls"));
           }
           Ok(())
       }
   }
   ```

2. Update GoPlugin struct:
   ```rust
   pub struct GoPlugin {
       import_support: import_support::GoImportSupport,
       workspace_support: workspace_support::GoWorkspaceSupport,
       lsp_installer: lsp_installer::GoLspInstaller,
   }

   impl LanguagePlugin for GoPlugin {
       fn lsp_installer(&self) -> Option<&dyn LspInstaller> {
           Some(&self.lsp_installer)
       }
   }
   ```

### Phase 5: Test Coverage (Priority: HIGH)
**Current**: 3 basic tests (metadata, capabilities, create_package)
**Target**: 15+ tests covering all capabilities

Add to lib.rs #[cfg(test)]:
```rust
#[test]
fn test_import_advanced_support() {
    let plugin = GoPlugin::default();
    assert!(plugin.import_advanced_support().is_some(),
        "ImportAdvancedSupport should not return None");
}

#[test]
fn test_workspace_support() {
    let plugin = GoPlugin::default();
    assert!(plugin.workspace_support().is_some(),
        "WorkspaceSupport should be implemented");
}

#[test]
fn test_refactoring_provider() {
    let plugin = GoPlugin::default();
    assert!(plugin.refactoring_provider().is_some(),
        "RefactoringProvider should be wired up");
}

#[tokio::test]
async fn test_refactoring_inline_variable() {
    let plugin = GoPlugin::default();
    let source = r#"
func main() {
    x := 5
    y := x * 2
    fmt.Println(y)
}
"#;
    let result = plugin.refactoring_provider().unwrap()
        .plan_inline_variable(source, 2, 4, "test.go").await;
    assert!(result.is_ok());
}

#[test]
fn test_module_reference_scanner() {
    let plugin = GoPlugin::default();
    let source = r#"import "fmt"\nfmt.Println("hello")"#;
    let refs = plugin.module_reference_scanner().unwrap()
        .scan_references(source, "fmt", ScanScope::All).unwrap();
    assert_eq!(refs.len(), 2); // import + qualified path
}
```

## Success Criteria

- [ ] ImportAdvancedSupport returns Some (not None)
- [ ] Existing refactoring.rs wired up to RefactoringProvider trait
- [ ] WorkspaceSupport implemented with go.work file management
- [ ] ModuleReferenceScanner finds import statements and qualified paths
- [ ] ImportAnalyzer builds import dependency graph
- [ ] ManifestUpdater handles go.mod updates
- [ ] LspInstaller can install gopls
- [ ] Test count increased from 3 to 15+
- [ ] All tests pass: `cargo nextest run -p mill-lang-go`
- [ ] CLAUDE.md parity table shows Go as 100% (currently shows ✅ incorrectly)
- [ ] CAPABILITIES struct matches actual implementation (no false claims)

## Effort Estimate

- Phase 1 (Fix false claims + wire refactoring): 3-4 hours
- Phase 2 (WorkspaceSupport): 5-6 hours
- Phase 3 (Analysis traits): 6-8 hours
- Phase 4 (LspInstaller): 2-3 hours
- Phase 5 (Tests): 4-5 hours
- **Total**: 20-26 hours

## Dependencies

- None (can proceed independently)

## Notes

- **Critical bug**: Line 90 returns None for ImportAdvancedSupport despite claiming imports=true
- **Unused code**: refactoring.rs exists but never called
- **False advertising**: CAPABILITIES.workspace=true but no implementation

## References

- Python plugin (100% parity): `languages/mill-lang-python/`
- Java plugin (100% parity): `languages/mill-lang-java/`
- Existing Go refactoring: `languages/mill-lang-go/src/refactoring.rs`
