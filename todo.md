# TypeMill Language Parity TODO

> **Goal**: Ensure all three primary languages (Rust, TypeScript, Python) have equivalent functionality for core operations.

Last updated: 2026-01-30

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

- [ ] **TypeScript: Add npm package consolidation support**
  - Location: `crates/mill-handlers/src/handlers/rename_ops/directory_rename.rs`
  - Extend `is_consolidation_move()` to detect package.json
  - Add dependency merging for package.json (similar to Cargo.toml merging)
  - Handle: devDependencies, peerDependencies, scripts merging

- [ ] **Python: Add Python package consolidation support**
  - Extend `is_consolidation_move()` to detect pyproject.toml
  - Add dependency merging for pyproject.toml
  - Handle: [project.dependencies], [project.optional-dependencies]

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
| consolidation | ✅ | ⬜ | ⬜ |
| workspace_support | ✅ | ✅ | ⚠️ (Hatch incomplete) |
| test_fixtures | ✅ | ✅ | ✅ |
| create_package | ✅ | ✅ | ✅ |
| real-world tests | ⬜ | ✅ (Zod) | ⬜ |

**Legend**: ✅ Complete | ⚠️ Partial | ⬜ Missing

---

## Implementation Order (Recommended)

1. **TypeScript reference_detector** - High impact, enables cross-package renames
2. **Python reference_detector** - Same impact for Python projects
3. **Python extract_dependencies** - Completes workspace tooling for Python
4. **TypeScript consolidation** - Enables monorepo refactoring
5. **Test fixtures parity** - Improves test coverage
6. **Real-world project tests** - Validates implementations
