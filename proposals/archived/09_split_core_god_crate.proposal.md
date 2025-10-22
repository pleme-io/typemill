# Proposal 09: Decompose `codebuddy-core` God Crate

## Problem

The architecture audit identified `codebuddy-core` as a "God Crate" containing at least nine unrelated modules (e.g., `auth`, `config`, `logging`, `workspaces`). This violates the Single Responsibility Principle, resulting in low cohesion. It bloats the dependency graph for any crate needing just one small utility from `core` and makes the project's foundation difficult to understand and maintain.

## Solution(s)

To resolve this, `codebuddy-core` will be systematically decomposed into smaller, single-responsibility crates. This proposal covers the initial, highest-priority extractions.

1.  **Create New Focused Crates:** For each identified domain (`auth`, `config`, `workspaces`), a new, dedicated crate will be created (e.g., `../../crates/mill-auth`).
2.  **Refactor and Move Code:** The source code for each module will be moved from `codebuddy-core` into its new, dedicated crate.
3.  **Update Dependencies:** All crates in the workspace will be updated to depend directly on the new, focused crates instead of transitively through `codebuddy-core`.
4.  **Cleanup `codebuddy-core`:** The extracted modules will be removed from the `codebuddy-core` source tree, leaving it as a lean foundation for truly shared concerns (e.g., error types, core traits).

## Checklists

*The following phases can be executed in parallel.*

### 09a: Extract `mill-auth`
- [ ] Create the new crate `../../crates/mill-auth`.
- [ ] Move the `auth` module from `codebuddy-core/src/` to `mill-auth/src/`.
- [ ] Update `mill-auth/Cargo.toml` with dependencies that were previously required by `codebuddy-core` for the `auth` module.
- [ ] Update all workspace crates that use `codebuddy_core::auth` to depend on and use `codebuddy_auth` instead.
- [ ] Remove the `auth` module from `codebuddy-core/src/lib.rs`.

### 09b: Extract `mill-config`
- [ ] Create the new crate `../../crates/mill-config`.
- [ ] Move the `config` and `refactor_config` modules from `codebuddy-core/src/` to the new crate.
- [ ] Update `mill-config/Cargo.toml` with its required dependencies.
- [ ] Update all workspace crates that use `codebuddy_core::config` to depend on and use `codebuddy_config` instead.
- [ ] Remove the `config` and `refactor_config` modules from `codebuddy-core/src/lib.rs`.

### 09c: Extract `mill-workspaces`
- [ ] Create the new crate `../../crates/mill-workspaces`.
- [ ] Move the `workspaces` module from `codebuddy-core/src/` to the new crate.
- [ ] Update `mill-workspaces/Cargo.toml` with its required dependencies.
- [ ] Update all workspace crates that use `codebuddy_core::workspaces` to depend on and use `codebuddy_workspaces` instead.
- [ ] Remove the `workspaces` module from `codebuddy-core/src/lib.rs`.

### 10a: Verification (after 09a, 09b, 09c are complete)
- [ ] Run `cargo check --workspace` to ensure all dependency and type errors are resolved.
- [ ] Run `cargo test --workspace` to confirm all existing functionality works correctly.
- [ ] Use `analyze.dependencies` to verify that crates now depend on the new, focused crates.

## Success Criteria

1.  New crates `mill-auth`, `mill-config`, and `mill-workspaces` exist and are used by the workspace.
2.  The `auth`, `config`, `refactor_config`, and `workspaces` modules are no longer present in the `codebuddy-core` source tree.
3.  `codebuddy-core` is significantly smaller and more focused on its core responsibilities.
4.  The entire workspace successfully builds and passes all tests via `cargo test --workspace`.

## Benefits

-   Resolves the "God Crate" architectural smell identified in the audit.
-   Improves cohesion and adheres to the Single Responsibility Principle.
-   Makes the codebase easier to understand, navigate, and maintain.
-   Reduces dependency bloat for consumers of `codebuddy-core`.
-   Clarifies ownership of distinct domains (`auth`, `config`, etc.).
