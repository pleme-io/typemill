# Segregate ImportSupport Trait

**Status:** ✅ **COMPLETED** (Commit: 47a758d4)

## Problem

`ImportSupport` trait has 8 methods forcing every language plugin to implement all functionality even when only basic import parsing is needed. This violates Interface Segregation Principle and creates unnecessary coupling.

Current implementations use default "not supported" stubs for 60% of methods they don't need.

**File:** `crates/cb-plugin-api/src/import_support.rs:15-128`

## Solution

Split into 5 focused traits by responsibility. Plugins implement only what they support via optional trait objects pattern.

```rust
trait ImportParser { /* parse, contains */ }
trait ImportRenameSupport { /* rewrite_for_rename */ }
trait ImportMoveSupport { /* rewrite_for_move */ }
trait ImportMutationSupport { /* add, remove, remove_named */ }
trait ImportAdvancedSupport { /* update_reference */ }
```

## Checklists

### Define Segregated Traits
- [x] Create `ImportParser` trait (parse_imports, contains_import) - cb-plugin-api/src/import_support.rs:15-28
- [x] Create `ImportRenameSupport` trait (rewrite_imports_for_rename) - cb-plugin-api/src/import_support.rs:30-37
- [x] Create `ImportMoveSupport` trait (rewrite_imports_for_move) - cb-plugin-api/src/import_support.rs:39-46
- [x] Create `ImportMutationSupport` trait (add_import, remove_import, remove_named_import) - cb-plugin-api/src/import_support.rs:48-61
- [x] Create `ImportAdvancedSupport` trait (update_import_reference) - cb-plugin-api/src/import_support.rs:63-70
- [x] All traits marked `Send + Sync`

### Update LanguagePlugin Trait
- [x] Add `import_parser(&self) -> Option<&dyn ImportParser>` - cb-plugin-api/src/lib.rs:373-374
- [x] Add `import_rename_support(&self) -> Option<&dyn ImportRenameSupport>` - cb-plugin-api/src/lib.rs:377-378
- [x] Add `import_move_support(&self) -> Option<&dyn ImportMoveSupport>` - cb-plugin-api/src/lib.rs:381-382
- [x] Add `import_mutation_support(&self) -> Option<&dyn ImportMutationSupport>` - cb-plugin-api/src/lib.rs:385-386
- [x] Add `import_advanced_support(&self) -> Option<&dyn ImportAdvancedSupport>` - cb-plugin-api/src/lib.rs:389-390
- [x] Deprecate old `import_support() -> Option<&dyn ImportSupport>` - cb-plugin-api/src/lib.rs:369-370
- [x] Default implementations return `None`

### Update Existing Plugins
- [x] Update `RustPlugin` to implement segregated traits - cb-lang-rust/src/import_support.rs:26-362
- [x] Add 5 trait accessor methods to RustPlugin - cb-lang-rust/src/lib.rs:132-150
- [x] Update `TypeScriptPlugin` to implement segregated traits - cb-lang-typescript/src/import_support.rs:200-255, lib.rs:94-112
- [x] Update `MarkdownPlugin` to implement segregated traits - cb-lang-markdown/src/import_support_impl.rs:477-535, lib.rs:117-135

### Update Consumers
- [x] Update `ReferenceUpdater::update_import_reference()` to use `import_advanced_support()` - cb-services/src/services/reference_updater/mod.rs:497
- [x] Update `GenericDetector::detect_dependencies()` to use `import_parser()` - cb-services/src/services/reference_updater/detectors/generic.rs:79
- [x] Fixed ambiguous trait resolution in RustImportSupport - cb-lang-rust/src/import_support.rs:50
- [x] Fixed ambiguous trait calls in RustPlugin - cb-lang-rust/src/lib.rs:710, 816

### Deprecation
- [x] Mark old `ImportSupport` trait as deprecated - cb-plugin-api/src/import_support.rs:72
- [x] Add deprecation notice to old `import_support()` method - cb-plugin-api/src/lib.rs:369
- [x] Backward compatibility maintained through deprecated trait

## Success Criteria

- ✅ Lightweight language plugin can implement only `ImportParser` (2 methods)
- ✅ Full-featured plugin can implement all 5 traits (8 methods total)
- ✅ Calling code checks trait availability before use (ReferenceUpdater, GenericDetector)
- ✅ All existing import functionality works unchanged
- ✅ Existing tests pass (730 passed, 6 pre-existing failures)
- ✅ TypeScript and Markdown plugins fully migrated to segregated traits

## Benefits

- Reduces implementation burden for simple language plugins by 60%
- Clear separation between parsing, renaming, moving, mutation, and advanced operations
- Clients depend only on interfaces they use
- Easier to add partial import support for new languages
- Compiler prevents calling unsupported operations
