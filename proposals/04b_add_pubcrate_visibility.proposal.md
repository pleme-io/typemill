# Add pub(crate) Visibility Markers

## Problem

~~Only 38 instances of `pub(crate)` exist across entire codebase (should be 200+).~~ **UPDATE (Phase 3.1):** Now 66 instances (+28 markers, +74% increase). Many types are still marked `pub` when they're only used within their own crate, unnecessarily expanding the public API surface. **Remaining:** ~134 more markers needed to reach 200+ target.

Examples:
- `mill-services/src/services/reference_updater/mod.rs:10` - `pub use cache::FileImportInfo;` (only used internally)
- `mill-handlers/src/handlers/common/mod.rs:8` - `pub use checksums::{...}` (internal utilities)

## Solution

Audit all `pub` items across workspace and change to `pub(crate)` where appropriate.

## Checklists

### Audit mill-services
- [x] Grep for all `pub struct`, `pub enum`, `pub fn` in mill-services
- [x] Identify which items are only used within mill-services
- [x] Change internal-only items to `pub(crate)` (5 markers added - Phase 3.1)
- [x] Verify public API still exported from lib.rs

### Audit mill-handlers
- [x] Grep for all `pub struct`, `pub enum`, `pub fn` in mill-handlers
- [x] Identify common/ utilities that should be `pub(crate)` (checksums marked)
- [x] Identify internal tool helpers that should be `pub(crate)` (workspace tool params marked)
- [x] Change internal-only items to `pub(crate)` (16 markers added - Phase 3.1)

### Audit mill-ast (Partial)
- [x] Review cache module - Already clean (all types are public API)
- [ ] Review analyzer module - mark internal analyzer types `pub(crate)`
- [x] Review transformer module - Already clean (all types are public API)
- [x] Review import_updater module - mark internal helpers `pub(crate)` (1 marker added - Phase 3.1)

### Audit mill-foundation
- [ ] Review protocol module internals
- [ ] Review model module internals
- [ ] Review core module internals
- [ ] Mark implementation details as `pub(crate)`
- [ ] Keep stable API types as `pub`

### Audit mill-lsp
- [ ] Review LSP client internals
- [ ] Mark protocol handling details `pub(crate)`
- [ ] Keep public LSP interface as `pub`

### Audit mill-plugin-system
- [ ] Review registry internals
- [ ] Review adapter internals
- [ ] Mark implementation details `pub(crate)`
- [ ] Keep plugin interface as `pub`

### Audit Language Plugins (Partial)
- [x] Review rust plugin internals (3 markers added - Phase 3.1)
- [ ] Review typescript plugin internals
- [ ] Review python plugin internals
- [ ] Review markdown plugin internals
- [ ] Mark parser internals `pub(crate)`
- [x] Keep LanguagePlugin implementation as `pub` (verified)

### Create Visibility Guidelines
- [ ] Document when to use `pub` vs `pub(crate)`
- [ ] Add guidelines to contributing.md
- [ ] Create checklist for PR reviews

### Verification (Phase 3.1)
- [x] Run `cargo check --workspace` (passing)
- [x] Run `cargo clippy --workspace` (passing)
- [x] Run `cargo nextest run --workspace` (298+ tests passing)
- [x] Count `pub(crate)` instances (current: 66, target: 200+, remaining: ~134)
- [x] Verify crate public APIs unchanged (verified - no breaking changes)
- [x] Check dependent crates still compile (verified - mill-server compiles)

## Success Criteria

- 200+ uses of `pub(crate)` across workspace
- All internal-only types marked `pub(crate)`
- Public API surface reduced by ~70%
- No broken imports in dependent crates
- All tests pass
- Visibility guidelines documented

## Benefits

- Reduced public API surface area
- Clearer distinction between public and internal APIs
- Prevents accidental coupling to internals
- Enables refactoring internals without breaking changes
- Better encapsulation of implementation details
- AI agents see only intended public API
- Prevents future API creep
