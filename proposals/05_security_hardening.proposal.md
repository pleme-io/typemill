# Security Hardening

## Problem
- File operations accept unvalidated paths, enabling traversal outside the configured project root and trusting user-supplied relative components across 28 call sites.
- Environment-variable overrides (via `TYPEMILL__` prefix) are undocumented, so secrets and production-only settings are currently stored in plain config files.
- JWT validation disables the `aud` check globally and allows tokens without `project_id`, which weakens multi-tenant isolation and enables token reuse across services.
- TLS configuration exists but is never enforced, so binding to non-loopback addresses (e.g., Docker examples) yields plaintext traffic without any warning.
- WebSocket handling ignores the configured `max_clients`, providing no guardrail against runaway local connections.

## Solution(s)
1. Document environment-variable secret management in `CLAUDE.md` and `README.md`, including examples for JWT secrets and guidance for keeping the server on loopback by default.
2. Add a `to_absolute_path_checked(&self, path: &Path) -> ServerResult<PathBuf>` helper that canonicalizes, verifies containment within `project_root`, returns detailed errors, and marks the existing `to_absolute_path` as deprecated until all callers migrate.
3. Extend `AuthConfig` with `validate_audience: bool` (default `false`) and optionally `jwt_audience_override`, wiring this through token validation paths plus documentation describing when to enable it.
4. Enforce TLS when `server.host` is not loopback (or emit a startup error), and emit a warning when TLS is absent even on loopback to keep operators informed.
5. Respect `server.max_clients` by tracking concurrent WebSocket sessions (simple `AtomicUsize` guard) and log/warn when the limit is hit; add a warning for tokens missing `project_id` to start the deprecation path.

## Checklists

### Documentation
- [ ] Add “Secrets Management via Environment Variables” section to `CLAUDE.md` with `TYPEMILL__SERVER__AUTH__JWT_SECRET` and other examples.
- [ ] Mirror the section (summary level) in `README.md` and reference `.typemill/config.toml` overrides vs. environment variables.
- [ ] Update `deployment/` docs to mention loopback-only default and TLS requirement for non-loopback hosts.

### Path Safety
- [ ] Implement `to_absolute_path_checked` with canonicalization and root containment checks, returning `ServerResult<PathBuf>`.
- [ ] Deprecate `to_absolute_path` (log warning + doc comment) and migrate high-risk call sites (basic ops, rename, move, edit-plan loaders) to the checked version.
- [ ] Add regression tests covering traversal attempts, symlink escapes, and non-existent path creation flows.

### Auth & Transport
- [ ] Extend `AuthConfig` with `validate_audience` and wire it into all four validation paths (mill-auth + transport ws/admin).
- [ ] Add config/CLAUDE documentation describing how to enable the flag and set audience values.
- [ ] Enforce TLS (error) when binding to non-loopback addresses without certificates; log a warning when TLS is missing on loopback.
- [ ] Honor `server.max_clients` by tracking live WebSocket sessions and rejecting new connections when the cap is reached with a clear log entry.
- [ ] Emit a warning whenever a token lacking `project_id` is accepted, documenting the impending requirement.

## Success Criteria
- Environment-variable guidance appears in both `CLAUDE.md` and `README.md`, including working examples verified manually.
- All external file operations use the checked path helper, and traversal regression tests fail if paths escape the project root.
- Configuration includes the new JWT audience flag, and setting it to `true` enforces `aud` validation across WebSocket and admin flows.
- Server startup refuses to bind to non-loopback hosts without TLS and logs a warning when running without TLS on loopback.
- WebSocket server enforces the `max_clients` limit, with unit tests covering acceptance/rejection paths, and logs warn when legacy tokens omit `project_id`.

## Benefits
- Prevents local file service operations from touching files outside the intended workspace, closing a concrete traversal gap.
- Provides a documented, supported path for keeping secrets out of repo-tracked config files.
- Enables operators to opt into stronger JWT guarantees without blocking existing users, while laying groundwork to require `project_id`.
- Reduces risk of accidentally exposing plaintext traffic when running on broader networks.
- Adds basic resource controls to WebSocket handling, limiting self-inflicted DoS scenarios on the developer’s machine.
