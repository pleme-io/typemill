# Proposal 16a: C# Language Plugin Parity Completion

**Status**: Pending
**Language**: C# (mill-lang-csharp)
**Current Completion**: ~58% (7/12 traits)
**Test Coverage**: 4 tests in lib.rs

## Current State

### ✅ Implemented (7/12 traits)
- ImportParser (via import_support)
- ImportRenameSupport (via import_support)
- ImportMoveSupport (via import_support)
- ImportMutationSupport (via import_support)
- ImportAdvancedSupport (via import_support)
- RefactoringProvider (direct impl - extract function/variable, inline variable)
- ProjectFactory (via project_factory)

### ❌ Missing (5/12 traits)
1. **WorkspaceSupport** - NO workspace_support module or field
2. **ModuleReferenceScanner** - Not delegated in impl_capability_delegations!
3. **ImportAnalyzer** - Not delegated
4. **ManifestUpdater** - Not delegated
5. **LspInstaller** - Not in fields

## Code Evidence

**File**: `languages/mill-lang-csharp/src/lib.rs`

```rust
// Line 23: Missing with_workspace in capabilities!
capabilities: [with_imports, with_project_factory],  // NO with_workspace!
fields: {
    import_support: import_support::CsharpImportSupport,
    project_factory: project_factory::CsharpProjectFactory,
    // NO workspace_support field
    // NO lsp_installer field
},
```

```rust
// Line 54: Only delegates RefactoringProvider
impl_capability_delegations! {
    this => {
        refactoring_provider: RefactoringProvider,  // Only this!
    },
    // Missing: module_reference_scanner, import_analyzer, manifest_updater
}
```

## Implementation Plan

### Phase 1: WorkspaceSupport (Priority: HIGH)
**Why**: Required for multi-project C# solutions (.sln files)

1. Create `workspace_support.rs` module:
   ```rust
   pub struct CsharpWorkspaceSupport;

   #[async_trait]
   impl WorkspaceSupport for CsharpWorkspaceSupport {
       async fn add_workspace_member(...) -> PluginResult<String> {
           // Add <Project Include="..."/> to .sln file
       }

       async fn remove_workspace_member(...) -> PluginResult<String> {
           // Remove project from .sln file
       }

       async fn list_workspace_members(...) -> PluginResult<Vec<String>> {
           // Parse .sln file for project references
       }
   }
   ```

2. Update lib.rs:
   ```rust
   capabilities: [with_imports, with_workspace, with_project_factory],
   fields: {
       workspace_support: workspace_support::CsharpWorkspaceSupport,
   }
   ```

3. Reference: Python workspace_support.rs (lines 1-200), Java workspace_support.rs

### Phase 2: Analysis Traits (Priority: MEDIUM)

#### 2a. ModuleReferenceScanner
Create impl block in lib.rs:
```rust
impl ModuleReferenceScanner for CsharpPlugin {
    fn scan_references(&self, content: &str, module_name: &str, scope: ScanScope)
        -> PluginResult<Vec<ModuleReference>>
    {
        // Scan for: using module_name;
        // Qualified paths: module_name.Class
        // String literals: "module_name/*.cs"
    }
}
```

**Reference**: Python lib.rs:305-388, TypeScript lib.rs:123-132

#### 2b. ImportAnalyzer
Add delegation in lib.rs:
```rust
impl_capability_delegations! {
    this => {
        import_analyzer: ImportAnalyzer,
    },
}

impl ImportAnalyzer for CsharpPlugin {
    fn build_import_graph(&self, file_path: &Path)
        -> PluginResult<ImportGraph>
    {
        // Build graph of using statements
    }
}
```

**Reference**: Python lib.rs:288-303, Rust lib.rs:336-351

#### 2c. ManifestUpdater
Add delegation and impl:
```rust
#[async_trait]
impl ManifestUpdater for CsharpPlugin {
    async fn update_dependency(&self, manifest_path: &Path,
        old_name: &str, new_name: &str, new_version: Option<&str>)
        -> PluginResult<String>
    {
        // Update <PackageReference Include="..."/> in .csproj
    }

    fn generate_manifest(&self, package_name: &str, dependencies: &[String])
        -> String
    {
        // Generate .csproj XML
    }
}
```

**Reference**: Python lib.rs:174-211, Rust lib.rs:358-392

### Phase 3: LspInstaller (Priority: LOW)
**Why**: Users can manually install csharp-ls

1. Create `lsp_installer.rs` module:
   ```rust
   pub struct CsharpLspInstaller;

   #[async_trait]
   impl LspInstaller for CsharpLspInstaller {
       async fn is_installed(&self) -> bool {
           // Check for csharp-ls in PATH
       }

       async fn install(&self) -> PluginResult<()> {
           // Install via: dotnet tool install --global csharp-ls
       }
   }
   ```

2. Reference: Python lsp_installer.rs, TypeScript lsp_installer.rs

### Phase 4: Test Coverage (Priority: HIGH)
**Current**: 4 basic tests
**Target**: 15+ tests covering all capabilities

Add to lib.rs #[cfg(test)]:
```rust
#[test]
fn test_workspace_support() {
    let plugin = CsharpPlugin::new();
    assert!(plugin.workspace_support().is_some());
}

#[test]
fn test_module_reference_scanner() {
    let source = "using OldModule;\npublic class Foo { }";
    let refs = plugin.scan_references(source, "OldModule", ScanScope::All).unwrap();
    assert_eq!(refs.len(), 1);
}

#[test]
fn test_manifest_updater() {
    let plugin = CsharpPlugin::new();
    assert!(plugin.manifest_updater().is_some());
}
```

**Reference**: Python lib.rs:390-505 (16 tests), Java lib.rs:134-182 (6 tests)

## Success Criteria

- [ ] WorkspaceSupport implemented with .sln file management
- [ ] ModuleReferenceScanner finds using statements and qualified paths
- [ ] ImportAnalyzer builds import dependency graph
- [ ] ManifestUpdater handles .csproj PackageReference updates
- [ ] LspInstaller can install csharp-ls via dotnet tool
- [ ] Test count increased from 4 to 15+
- [ ] All tests pass: `cargo nextest run -p mill-lang-csharp`
- [ ] CLAUDE.md parity table shows C# as 100% (currently shows ✅ incorrectly)

## Effort Estimate

- Phase 1 (WorkspaceSupport): 4-6 hours
- Phase 2 (Analysis traits): 6-8 hours
- Phase 3 (LspInstaller): 2-3 hours
- Phase 4 (Tests): 3-4 hours
- **Total**: 15-21 hours

## Dependencies

- None (can proceed independently)

## References

- Python plugin (100% parity): `languages/mill-lang-python/`
- Java plugin (100% parity): `languages/mill-lang-java/`
- Rust workspace support: `languages/mill-lang-rust/src/workspace_support.rs`
