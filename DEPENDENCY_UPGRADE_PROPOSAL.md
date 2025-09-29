# Dependency Upgrade Proposal

## Summary

This document outlines the major version dependency upgrades that require code changes to implement.

---

## ✅ Completed: Minor Version Updates

The following minor version updates have been **automatically applied** and verified:

- **lsp-types**: `0.94` → `0.97.0` ✓
- **insta**: `1.34.0` → `1.43.2` ✓
- **simd-json**: `0.13` → `0.16.0` ✓
- **toml_edit**: `0.20` → `0.23.6` ✓
- **axum-extra**: `0.10` → `0.10.2` ✓
- **tower**: `0.5` → `0.5.2` ✓
- **hyper**: `1.0` → `1.7.0` ✓
- **fuser**: `0.14` → `0.16.0` ✓ (with API signature fix for `getattr` method)

All core libraries (`cb-server`, `cb-core`, `cb-api`, `cb-ast`, `cb-plugins`, `cb-transport`, `cb-vfs`, `cb-client`) build successfully with these updates.

---

## 🔄 Major Version Updates Requiring Review

### 1. **tokio-tungstenite**: `0.21`/`0.26` → `0.28.0` (PRIORITY: HIGH)

**Current State:**
- `cb-transport/Cargo.toml` uses version `0.21`
- `apps/server/Cargo.toml` uses version `0.26`
- Latest version: `0.28.0`

**Breaking Changes:**
- Updated to `tungstenite` 0.27 with connection state improvements
- Changed WebSocket handshake response handling
- Updated error types and result handling

**Required Code Changes:**
1. Unify both usages to `0.28.0`
2. Update WebSocket connection handling in:
   - `rust/crates/cb-transport/src/websocket.rs` (if exists)
   - `rust/apps/server/src/main.rs` WebSocket routes
3. Review and update error handling for new error types

**Files to Modify:**
- `rust/crates/cb-transport/Cargo.toml`
- `rust/apps/server/Cargo.toml`
- Any WebSocket connection/handshake logic

---

### 2. **config**: `0.14` → `0.15.17` (PRIORITY: MEDIUM)

**Current State:**
- Workspace-level dependency in `rust/Cargo.toml`
- Used in `cb-core` for configuration parsing

**Breaking Changes:**
- Renamed configuration builder methods
- Changed `File` source API
- Updated error types and enum variants
- Modified configuration merge behavior

**Required Code Changes:**
1. Update configuration loading in `rust/crates/cb-core/src/config.rs`:
   - Replace `Config::builder()` calls with new API
   - Update `File::with_name()` to new signature
   - Update configuration merge logic
2. Review error handling for new error types
3. Update configuration tests

**Files to Modify:**
- `rust/Cargo.toml` (workspace dependency)
- `rust/crates/cb-core/src/config.rs`
- Any configuration loading tests

**Migration Guide:** https://github.com/mehcode/config-rs/blob/main/CHANGELOG.md

---

### 3. **jsonwebtoken**: `8` → `10.0.0` (PRIORITY: MEDIUM)

**Current State:**
- Used in `cb-server` and `cb-transport` for JWT authentication
- Version `8` in both crates

**Breaking Changes:**
- Version 9:
  - Changed `Validation` struct fields from public to builder pattern
  - Updated `decode` function signature
  - New `DecodingKey` and `EncodingKey` construction methods
- Version 10:
  - Further API refinements
  - Updated algorithm handling
  - Improved validation options

**Required Code Changes:**
1. Update JWT token encoding/decoding in:
   - `rust/crates/cb-server/src/auth.rs` (if exists)
   - `rust/crates/cb-transport/src/auth.rs` (if exists)
2. Replace direct `Validation` struct initialization with builder:
   ```rust
   // Old (v8)
   let validation = Validation {
       validate_exp: true,
       leeway: 60,
       ..Validation::default()
   };

   // New (v10)
   let mut validation = Validation::new(Algorithm::HS256);
   validation.set_required_spec_claims(&["exp"]);
   validation.leeway = 60;
   ```
3. Update key construction:
   ```rust
   // Old (v8)
   let key = b"secret";

   // New (v10)
   let key = EncodingKey::from_secret(b"secret");
   let dec_key = DecodingKey::from_secret(b"secret");
   ```

**Files to Modify:**
- `rust/crates/cb-server/Cargo.toml`
- `rust/crates/cb-transport/Cargo.toml`
- All JWT authentication/authorization code
- Authentication tests

**Migration Guide:** https://github.com/Keats/jsonwebtoken/blob/master/CHANGELOG.md

---

### 4. **mockall**: `0.12` → `0.13.1` (PRIORITY: LOW)

**Current State:**
- Workspace-level dev dependency in `rust/Cargo.toml`
- Used for mocking in tests

**Breaking Changes:**
- Updated trait mocking syntax
- Changed return type expectations
- Modified async trait mocking

**Required Code Changes:**
1. Review all mock implementations in test files
2. Update mock expectations for changed API:
   ```rust
   // Old (v0.12)
   mock.expect_method()
       .returning(|| Ok(value));

   // New (v0.13)
   mock.expect_method()
       .return_once(|| Ok(value));
   ```
3. Update async trait mocks if used

**Files to Modify:**
- `rust/Cargo.toml` (workspace dependency)
- All test files using `mockall` (search for `#[automock]` or `mock!` macro)

**Migration Guide:** https://github.com/asomers/mockall/blob/master/CHANGELOG.md

---

### 5. **reqwest**: `0.11` → `0.12.23` (PRIORITY: LOW)

**Current State:**
- Used in `cb-plugins` for HTTP requests
- Version `0.11` with `rustls-tls` feature

**Breaking Changes:**
- Updated to `hyper` 1.0 (major architectural change)
- Changed body handling API
- Modified multipart form data API
- Updated error types

**Required Code Changes:**
1. Update HTTP client usage in `rust/crates/cb-plugins/src/system_tools_plugin.rs`:
   - Review body handling (`.body()` method changes)
   - Update multipart form uploads if used
   - Check error handling for new error types
2. Verify feature flags are compatible (`rustls-tls` is still available)
3. Test all HTTP request functionality

**Files to Modify:**
- `rust/crates/cb-plugins/Cargo.toml`
- `rust/crates/cb-plugins/src/system_tools_plugin.rs`
- Any plugin code making HTTP requests

**Migration Guide:** https://github.com/seanmonstar/reqwest/releases/tag/v0.12.0

---

## Implementation Priority

### Phase 1 (High Priority)
1. **tokio-tungstenite** - Active WebSocket version conflicts need resolution

### Phase 2 (Medium Priority)
2. **config** - Core configuration system
3. **jsonwebtoken** - Authentication/security improvements

### Phase 3 (Low Priority)
4. **mockall** - Dev dependency, impacts tests only
5. **reqwest** - Limited usage, plugin system only

---

## Testing Strategy

For each upgrade:
1. Update the dependency version
2. Fix compilation errors
3. Run unit tests: `cargo test`
4. Run integration tests: `cargo test --test '*'`
5. Manual testing of affected features
6. Review for runtime behavior changes

---

## Rollback Plan

If any upgrade causes issues:
1. Git revert the specific dependency change
2. Document the blocker in this file
3. Consider pinning to current version with comment explaining why

---

## Notes

- All minor version updates have been applied and verified
- Pre-existing build issues exist in `benchmark-harness` and `apps/server` binaries (unrelated to these updates)
- Core libraries all compile successfully with minor updates