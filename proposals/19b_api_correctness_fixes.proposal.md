# Proposal 19b: API Correctness Fixes for Language Plugins

**Status**: Ready for Implementation
**Scope**: C#, Go, Swift - Fix incorrect API usage and inconsistent patterns
**Priority**: HIGH

## Problem

The C#, Go, and Swift language plugins contain **9 instances of incorrect API usage** that violate language semantics and create inconsistent behavior:

**Critical API Misuse Examples**:

```rust
// C# lib.rs:239 - Wrong import type for C#
ImportInfo {
    module_path: s.name,
    import_type: ImportType::EsModule,  // ❌ WRONG - C# uses namespaces, not ES modules
}

// Swift lib.rs:217 - Checking undefined enum variants
if scope == ScanScope::TopLevelOnly       // ❌ DOESN'T EXIST
    || scope == ScanScope::AllUseStatements { // ❌ DOESN'T EXIST

// Go lib.rs:171 - 0-indexed line numbers (inconsistent with C# and Swift)
for (i, line) in content.lines().enumerate() {
    references.push(ModuleReference {
        line: i,  // ❌ Should be i + 1 for consistency
    });
}
```

**Impact**:
- Misleading metadata for downstream tools
- Dead code that never executes
- Inconsistent line numbering across plugins
- Hardcoded values that should be configurable

## Solution

Fix all 9 API misuse instances by correcting enum variants, standardizing line numbering to 1-indexed convention, and extracting hardcoded values to constants.

**All tasks should be completed in one implementation session** to ensure atomic correctness improvements across all three plugins.

## Checklists

### Fix Incorrect Enum Usage

- [ ] C#: Change `ImportType::EsModule` → `ImportType::Namespace` (lib.rs:239)
- [ ] C#: Add proper PluginError variant for invalid ranges (if needed)
- [ ] Swift: Remove undefined `ScanScope::TopLevelOnly` check (lib.rs:217)
- [ ] Swift: Remove undefined `ScanScope::AllUseStatements` check (lib.rs:217)
- [ ] Verify correct enum definitions in mill-plugin-api

### Standardize Line Numbering

- [ ] Go: Fix 0-indexed line numbers → 1-indexed (lib.rs:171-177)
- [ ] Go: Update all `ModuleReference` creation to use `line_num = i + 1`
- [ ] Go: Verify consistency with C# and Swift implementations
- [ ] Add integration tests verifying line number accuracy

### Extract Hardcoded Constants

- [ ] Go: Make Go version configurable, remove hardcoded "1.21" (lib.rs:145)
- [ ] Go: Add `DEFAULT_GO_VERSION` constant or environment variable detection
- [ ] C#: Extract hardcoded parser version "0.20.0" to constant (lib.rs:256)
- [ ] Swift: Fix empty version string for dependencies (lib.rs:103)
- [ ] Document all extracted constants with rustdoc comments

### Fix Boundary Handling

- [ ] C#: Review `end_col` fallback to 0 logic (lib.rs:104-107)
- [ ] C#: Add proper error handling if fallback is incorrect
- [ ] Add bounds checking tests for edge cases

### Integration Testing

- [ ] Add tests verifying ImportType correctness for each language
- [ ] Add tests verifying ScanScope behavior matches enum definition
- [ ] Add tests verifying line numbers are 1-indexed across all plugins
- [ ] Test with edge cases (line 0, column 0, end of file)

### Verification

- [ ] Run `cargo clippy --all-targets -- -D warnings` for all three plugins
- [ ] Verify all 64 existing tests still pass (25 C# + 30 Go + 9 Swift)
- [ ] Run integration tests with real-world code samples
- [ ] Document API usage patterns in rustdoc

## Success Criteria

- [ ] Zero incorrect enum variant usage
- [ ] Consistent 1-indexed line numbering across all plugins
- [ ] All hardcoded versions extracted to constants
- [ ] `cargo clippy --all-targets` passes with zero warnings
- [ ] All 64 existing tests continue to pass
- [ ] New integration tests verify API correctness

## Benefits

- **Correct Semantics**: API usage matches language conventions (C# namespaces, not ES modules)
- **Consistency**: All plugins use same line numbering convention (1-indexed)
- **Maintainability**: Hardcoded values extracted to named constants
- **Debuggable**: Line numbers match what developers see in their editors
- **Configurable**: Version numbers can be overridden when needed

## Implementation Notes

### C# Import Type Fix

**Before**:
```rust
ImportInfo {
    module_path: s.name,
    import_type: ImportType::EsModule,  // ❌ Wrong for C#
}
```

**After**:
```rust
ImportInfo {
    module_path: s.name,
    import_type: ImportType::Namespace,  // ✅ C# uses namespaces
}
```

### Go Line Numbering Fix

**Before**:
```rust
for (i, line) in content.lines().enumerate() {
    references.push(ModuleReference {
        line: i,  // ❌ 0-indexed
    });
}
```

**After**:
```rust
for (i, line) in content.lines().enumerate() {
    let line_num = (i + 1) as u32;  // ✅ 1-indexed
    references.push(ModuleReference {
        line: line_num,
    });
}
```

### Version Constant Extraction

**Before**:
```rust
fn generate_manifest(&self, package_name: &str, _dependencies: &[String]) -> String {
    manifest::generate_manifest(package_name, "1.21")  // ❌ Hardcoded
}
```

**After**:
```rust
const DEFAULT_GO_VERSION: &str = "1.21";

fn generate_manifest(&self, package_name: &str, _dependencies: &[String]) -> String {
    manifest::generate_manifest(package_name, DEFAULT_GO_VERSION)
}
```

## References

- Rust API guidelines: https://rust-lang.github.io/api-guidelines/
- Analysis document: `.debug/parity-refinement-proposal-2025-10-31.md`
- Mill plugin API documentation: `crates/mill-plugin-api/src/lib.rs`

## Detailed Issue Catalog

**API Misuse Instances**:
1. C# `ImportType::EsModule` for namespace imports (lib.rs:239)
2. Swift undefined `ScanScope::TopLevelOnly` (lib.rs:217)
3. Swift undefined `ScanScope::AllUseStatements` (lib.rs:217)
4. Go 0-indexed line numbers (lib.rs:171-177)
5. C# questionable `end_col` fallback to 0 (lib.rs:104-107)
6. Swift empty version string for dependencies (lib.rs:103)
7. Go hardcoded version "1.21" (lib.rs:145)
8. C# hardcoded parser version "0.20.0" (lib.rs:256)
9. All plugins: Using `ImportType` enum needs verification against actual language semantics
