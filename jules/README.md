# Jules MCP Integration (Sub-Project)

This directory contains Jules-related crates extracted from the `jules-api` branch. These crates are **stored separately** from the main Codebuddy workspace and do not affect the main build.

## Structure

```
jules/
├── Cargo.toml           # Separate workspace for Jules crates
├── crates/
│   ├── jules-api/       # Jules API client library
│   ├── jules-mcp-server/# MCP server for Jules integration
│   └── jules-cli/       # CLI tool for Jules
└── README.md            # This file
```

## Status

✅ **Building Successfully** - This code was extracted from `origin/jules-api` branch and has been fixed.

### Fixed Issues

1. ✅ **Missing `auth` module** - Created stub implementation in `jules-api/src/auth.rs`
2. ✅ **Tokio features** - Added `io-std` feature to `jules-mcp-server/Cargo.toml`
3. ✅ **Tracing subscriber** - Added `json` feature to workspace dependencies
4. ✅ **Unused imports** - Cleaned up unused imports

**Build time:** ~25s
**Test status:** All tests passing (0 tests currently)
**Binaries:** `jules-cli` (28M), `jules-mcp-server` (32M)

## Building

To build the Jules workspace independently:

```bash
cd jules
cargo check
cargo build
cargo test
```

## Integration Plan

When ready to integrate with Codebuddy:

1. Fix all build errors
2. Add proper tests
3. Add to root workspace as optional feature
4. Update main Cargo.toml:
   ```toml
   members = [
       # ... existing members
       "jules/crates/jules-api",
       "jules/crates/jules-mcp-server",
       "jules/crates/jules-cli",
   ]
   ```

## Next Steps

- [ ] Fix tokio `io-std` feature requirement
- [ ] Implement or stub `auth` module
- [ ] Add comprehensive tests
- [ ] Document API usage
- [ ] Create integration examples

## Purpose

This setup allows Jules development to proceed independently without affecting the main Codebuddy codebase. Once stable, these crates can be integrated into the main workspace.
