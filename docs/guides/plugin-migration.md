# Plugin Refactoring Migration Guide

Guide for migrating existing language plugins to use the new `define_language_plugin!` macro and consolidated refactoring structs.

## Overview

The plugin refactoring (2025-10) introduced two major improvements:
1. **Phase 1:** Consolidated refactoring data structures into `mill-lang-common`
2. **Phase 2:** Created `define_language_plugin!` macro to eliminate scaffolding boilerplate

**Benefits:**
- ~70 lines of boilerplate eliminated per plugin
- Single source of truth for plugin metadata
- Impossible to have mismatched METADATA/CAPABILITIES/mill_plugin! values
- Compile-time validation of plugin structure

## Phase 1 Migration: Consolidate Refactoring Structs

### Before (Old Pattern)
```rust
// In your plugin's lib.rs or refactoring.rs
# [derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

# [derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableUsage {
    pub name: String,
    pub declaration_location: Option<CodeRange>,
    // ... rest of fields
}

// Similar duplicates for ExtractableFunction, InlineVariableAnalysis, etc.
```text
### After (New Pattern)
```rust
// In your plugin's refactoring.rs
use mill_lang_common::{
    CodeRange, ExtractVariableAnalysis, ExtractableFunction,
    InlineVariableAnalysis, VariableUsage,
};

// Optionally re-export for internal use
pub use mill_lang_common::CodeRange;

// Your refactoring logic using the imported types
```text
### Migration Steps
1. **Remove local definitions** of `CodeRange`, `VariableUsage`, `ExtractableFunction`, `InlineVariableAnalysis`, `ExtractVariableAnalysis`
2. **Add imports** from `mill_lang_common`
3. **Update Cargo.toml** if needed (mill-lang-common should already be a dependency)
4. **Run tests** to verify nothing broke

## Phase 2 Migration: Use define_language_plugin! Macro

### Before (Old Pattern - ~70 lines of boilerplate)
```rust
use mill_plugin_api::mill_plugin;
use mill_plugin_api::{
    LanguageMetadata, LanguagePlugin, LspConfig, ManifestData,
    ParsedSource, PluginCapabilities, PluginResult,
};

mill_plugin! {
    name: "mylang",
    extensions: ["ml"],
    manifest: "Package.mylang",
    capabilities: MyLanguagePlugin::CAPABILITIES,
    factory: MyLanguagePlugin::new,
    lsp: Some(LspConfig::new("mylang-lsp", &["mylang-lsp", "--stdio"]))
}

# [derive(Default)]
pub struct MyLanguagePlugin {
    import_support: import_support::MyLanguageImportSupport,
    workspace_support: workspace_support::MyLanguageWorkspaceSupport,
}

impl MyLanguagePlugin {
    pub const METADATA: LanguageMetadata = LanguageMetadata {
        name: "mylang",
        extensions: &["ml"],
        manifest_filename: "Package.mylang",
        source_dir: "src",
        entry_point: "main.ml",
        module_separator: "::",
    };

    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none()
        .with_imports()
        .with_workspace();

    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<dyn LanguagePlugin> {
        Box::new(Self::default())
    }
}

# [async_trait]
impl LanguagePlugin for MyLanguagePlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &Self::METADATA
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    // ... rest of implementation ...

    fn import_parser(&self) -> Option<&dyn ImportParser> {
        Some(&self.import_support)
    }

    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> {
        Some(&self.workspace_support)
    }

    // ... more delegation methods ...
}
```text
### After (New Pattern - ~20 lines)
```rust
use mill_lang_common::{
    define_language_plugin, impl_capability_delegations,
    impl_language_plugin_basics
};
use mill_plugin_api::{LanguagePlugin, ManifestData, ParsedSource, PluginResult};

define_language_plugin! {
    struct: MyLanguagePlugin,
    name: "mylang",
    extensions: ["ml"],
    manifest: "Package.mylang",
    lsp_command: "mylang-lsp",
    lsp_args: ["mylang-lsp", "--stdio"],
    source_dir: "src",
    entry_point: "main.ml",
    module_separator: "::",
    capabilities: [with_imports, with_workspace],
    fields: {
        import_support: import_support::MyLanguageImportSupport,
        workspace_support: workspace_support::MyLanguageWorkspaceSupport,
    },
    doc: "MyLanguage plugin implementation"
}

# [async_trait]
impl LanguagePlugin for MyLanguagePlugin {
    impl_language_plugin_basics!();

    // Your implementation here
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        // ...
    }

    impl_capability_delegations! {
        import_support => {
            import_parser: ImportParser,
        },
        workspace_support => {
            workspace_support: WorkspaceSupport,
        },
    }
}
```text
### Migration Steps

1. **Update imports:**
   ```rust
   // Remove these:
   - use mill_plugin_api::mill_plugin;
   - use mill_plugin_api::{LanguageMetadata, LspConfig, PluginCapabilities};

   // Add these:
   + use mill_lang_common::{define_language_plugin, impl_capability_delegations, impl_language_plugin_basics};

   // Keep these:
   use mill_plugin_api::{LanguagePlugin, ManifestData, ParsedSource, PluginResult};
   ```

2. **Replace scaffolding with macro:**
   - Remove `mill_plugin!` block
   - Remove struct definition
   - Remove `METADATA` constant
   - Remove `CAPABILITIES` constant
   - Remove `new()` method
   - Add `define_language_plugin!` macro invocation

3. **Update LanguagePlugin impl:**
   ```rust
   // Add at the top of impl block:
   impl_language_plugin_basics!();

   // Remove these methods (now generated by macro):
   - fn metadata(&self) -> &LanguageMetadata { ... }
   - fn capabilities(&self) -> PluginCapabilities { ... }
   - fn as_any(&self) -> &dyn std::any::Any { ... }

   // Keep your implementation methods:
   - async fn parse(&self, source: &str) -> PluginResult<ParsedSource>
   - async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>
   - etc.
   ```

4. **Replace capability delegation methods with macro:**
   ```rust
   // Remove manual delegation methods:
   - fn import_parser(&self) -> Option<&dyn ImportParser> { ... }
   - fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> { ... }
   // etc.

   // Add macro at end of impl block:
   impl_capability_delegations! {
       import_support => {
           import_parser: ImportParser,
       },
       workspace_support => {
           workspace_support: WorkspaceSupport,
       },
   }
   ```

5. **Run tests:**
   ```bash
   cargo test -p mill-lang-yourplugin --lib
   cargo check --workspace
   cargo clippy --workspace -- -D warnings
   ```

## Capability Mapping Reference

Map your old capability methods to the new macro syntax:

| Old Method | Macro Syntax |
|------------|--------------|
| `fn import_parser(&self) -> Option<&dyn ImportParser>` | `import_parser: ImportParser` in `{field} =>` block |
| `fn workspace_support(&self) -> Option<&dyn WorkspaceSupport>` | `workspace_support: WorkspaceSupport` in `{field} =>` block |
| `fn module_reference_scanner(&self) -> Option<&dyn ModuleReferenceScanner>` | `module_reference_scanner: ModuleReferenceScanner` in `this =>` block |
| `fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider>` | `refactoring_provider: RefactoringProvider` in `this =>` block |

**Pattern:**
- **Field delegation** (`field =>` block): When capability is implemented by a field
- **Self delegation** (`this =>` block): When capability is implemented by the plugin itself

## Complete Migration Example

See these real-world migrations for reference:
- **Python:** `crates/mill-lang-python/src/lib.rs` (Commit: 23e63c00)
- **TypeScript:** `crates/mill-lang-typescript/src/lib.rs` (Commit: 23e63c00)
- **Rust:** `crates/mill-lang-rust/src/lib.rs` (Commit: 23e63c00)

## Validation Checklist

After migration:
- [ ] `cargo check -p mill-lang-yourplugin` passes
- [ ] `cargo test -p mill-lang-yourplugin --lib` all tests pass
- [ ] `cargo clippy -p mill-lang-yourplugin -- -D warnings` zero warnings
- [ ] Plugin metadata still correct (`mill tool health_check` if available)
- [ ] Plugin capabilities still work (test parse, manifest, imports, etc.)

## Common Issues

### Issue: Field visibility errors
**Problem:** Tests can't access plugin fields
**Solution:** Fields are `pub(crate)` by default. Tests in `#[cfg(test)]` modules work, but external integration tests may need trait methods instead of direct field access.

### Issue: Capability not delegated
**Problem:** `None` returned for capability trait method
**Solution:** Verify you've added the correct mapping in `impl_capability_delegations!` block. Check spelling and trait name match exactly.

### Issue: Compilation error in macro expansion
**Problem:** Macro syntax error
**Solution:**
- Verify all parameters are present and spelled correctly
- Check comma placement (trailing commas allowed)
- Ensure field types match your struct field types
- Look at reference implementations for correct syntax

## Benefits Summary

**Before:** ~70 lines of boilerplate per plugin
**After:** ~20 lines with macro

**Lines saved:**
- Plugin scaffolding: ~35 lines (struct, METADATA, CAPABILITIES, mill_plugin!, new())
- Trait delegation: ~35 lines (metadata(), capabilities(), delegation methods)

**Quality improvements:**
- Single source of truth for plugin metadata
- Impossible to have mismatched values
- Compile-time validation
- Easier to maintain and refactor

## Questions?

See `proposals/01_plugin_refactoring.proposal.md` for complete implementation details and rationale.