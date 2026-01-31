# TypeMill Language Parity TODO

> **Goal**: Ensure all three primary languages (Rust, TypeScript, Python) have equivalent functionality for core operations.

Last updated: 2026-01-31

---

## Critical Gaps (Affects Core Functionality)

### Extract Dependencies

- [x] **Python: Add pyproject.toml support to extract_dependencies** ✅ DONE
  - Location: `crates/mill-handlers/src/handlers/tools/workspace_extract/`
  - Created: `pyproject_manifest.rs` with PEP 621 and Poetry support
  - Updated: `mod.rs` to detect and handle `ManifestType::PyProject`
  - Tests: 10 unit tests added

### Reference Detector (Cross-Package Reference Detection)

- [x] **TypeScript: Implement reference_detector** ✅ DONE
  - Location: `languages/mill-lang-typescript/src/reference_detector.rs`
  - Detects: ES6 imports, CommonJS requires, dynamic imports, re-exports
  - Tests: 6 unit tests added

- [x] **Python: Implement reference_detector** ✅ DONE
  - Location: `languages/mill-lang-python/src/reference_detector.rs`
  - Detects: `import x`, `from x import y`, relative imports
  - Tests: 6 unit tests added

### Consolidation (Package Merging)

- [x] **TypeScript: Add npm package consolidation support** ✅ DONE
  - Location: `crates/mill-handlers/src/handlers/rename_ops/directory_rename.rs`
  - Added `PackageType::Npm` detection with `find_target_npm_root()`
  - Created: `languages/mill-lang-typescript/src/consolidation.rs`
  - Features: dependency merging (dependencies, devDependencies, peerDependencies)
  - Import updates, workspace cleanup, nested src/ flattening
  - Tests: 3 unit tests in consolidation.rs + 1 detection test in directory_rename.rs

- [x] **Python: Add Python package consolidation support** ✅ DONE
  - Location: `crates/mill-handlers/src/handlers/rename_ops/directory_rename.rs`
  - Added `PackageType::Python` detection with `find_target_python_root()`
  - Created: `languages/mill-lang-python/src/consolidation.rs`
  - Features: dependency merging (PEP 621 + Poetry), __init__.py creation
  - Import updates, workspace cleanup, nested src/ flattening
  - Tests: 4 unit tests in consolidation.rs + 1 detection test in directory_rename.rs

---

## Consistency Gaps

### Test Fixtures

- [x] **Rust: Add language-specific test fixtures** ✅ DONE
  - Created: `languages/mill-lang-rust/src/test_fixtures.rs`
  - 14 complexity scenarios (lifetimes, generics, macros, traits, async)
  - 8 refactoring scenarios

- [x] **TypeScript: Add language-specific test fixtures** ✅ DONE
  - Created: `languages/mill-lang-typescript/src/test_fixtures.rs`
  - 10 complexity scenarios (generics, async/await, JSX, decorators)
  - 8 refactoring scenarios

### Real-World Project Tests

- [ ] **Rust: Add real-world Rust project tests**
  - Similar to Zod tests in `tests/e2e/src/test_real_project_zod.rs`
  - Target project: Consider serde, tokio, or smaller crate

- [ ] **Python: Add real-world Python project tests**
  - Similar to Zod tests
  - Target project: Consider requests, httpx, or pydantic

---

## Code Quality

### Workspace Support Completeness

- [ ] **Python: Complete Hatch workspace support**
  - Location: `languages/mill-lang-python/src/workspace_support.rs`
  - Line 134: Implement `add_workspace_member` for Hatch
  - Line 156: Implement `remove_workspace_member` for Hatch

---

## Nice to Have (Lower Priority)

### Module/Package Structure

- [ ] **TypeScript: Barrel file (index.ts) re-export handling**
  - When moving symbols, update re-exports in barrel files
  - Detect `export * from` and `export { x } from` patterns

- [ ] **Python: `__init__.py` `__all__` list handling**
  - When moving files, update `__all__` lists if present

### Path Alias Support

- [ ] **Python: Consider path alias support**
  - Some Python projects use src-layout with custom import roots
  - Could benefit from resolution similar to TypeScript's tsconfig paths

---

## Documentation

- [ ] **Verify workspace-rust.md accuracy**
  - Location: `docs/tools/workspace-rust.md`
  - Ensure documented features match implementation

- [ ] **Verify workspace-python.md accuracy**
  - Location: `docs/tools/workspace-python.md`
  - Ensure documented features match implementation

- [ ] **Verify workspace-typescript.md accuracy**
  - Location: `docs/tools/workspace-typescript.md`
  - Ensure documented features match implementation

---

## Progress Tracking

| Category | Rust | TypeScript | Python |
|----------|------|------------|--------|
| extract_dependencies | ✅ | ✅ | ✅ |
| reference_detector | ✅ | ✅ | ✅ |
| consolidation | ✅ | ✅ | ✅ |
| workspace_support | ✅ | ✅ | ⚠️ (Hatch incomplete) |
| test_fixtures | ✅ | ✅ | ✅ |
| create_package | ✅ | ✅ | ✅ |
| real-world tests | ⬜ | ✅ (Zod) | ⬜ |

**Legend**: ✅ Complete | ⚠️ Partial | ⬜ Missing

---

## Implementation Order (Recommended)

1. ~~**TypeScript reference_detector** - High impact, enables cross-package renames~~ ✅ DONE
2. ~~**Python reference_detector** - Same impact for Python projects~~ ✅ DONE
3. ~~**Python extract_dependencies** - Completes workspace tooling for Python~~ ✅ DONE
4. ~~**TypeScript consolidation** - Enables monorepo refactoring~~ ✅ DONE
5. ~~**Python consolidation** - Enables monorepo refactoring~~ ✅ DONE
6. ~~**Test fixtures parity** - Improves test coverage~~ ✅ DONE
7. **Real-world project tests** - Validates implementations (Rust + Python pending)
8. **Hatch workspace support** - Low priority, limited adoption
