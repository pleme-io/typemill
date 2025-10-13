# Proposal 07: cargo-deny Integration for Dependency Auditing

**Status:** Draft
**Created:** 2025-10-13
**Author:** AI Assistant
**Tracking Issue:** TBD

## Summary

Integrate `cargo-deny` into the project for automated dependency auditing, security vulnerability scanning, license compliance, and duplicate dependency detection.

## Motivation

**Current Issues:**
- 6+ duplicate dependency versions (bitflags, dashmap, dirs, getrandom, hashbrown, phf_shared, proc-macro2)
- No automated security vulnerability scanning
- No license compliance checking
- Manual dependency review is error-prone

**Goals:**
1. Catch security vulnerabilities in dependencies automatically
2. Prevent duplicate dependencies from being introduced
3. Ensure license compliance (MIT/Apache-2.0)
4. Integrate into CI/CD for automatic checks

## Detailed Design

### 1. Install cargo-deny

```bash
cargo install cargo-deny
```

**In CI/CD:**
```yaml
# .github/workflows/ci.yml
- name: Install cargo-deny
  run: cargo install cargo-deny --locked
```

---

### 2. Initialize Configuration

```bash
cargo deny init
```

This creates `.config/cargo-deny.toml` (or `deny.toml` at root - we'll move it).

---

### 3. Configure cargo-deny

**File:** `.config/cargo-deny.toml`

```toml
# cargo-deny configuration for codebuddy
# https://embarkstudios.github.io/cargo-deny/

# Note: All sections are optional, uncomment to configure

# =============================================================================
# Security Advisories
# =============================================================================
[advisories]
# The path where the advisory database is cloned/fetched into
db-path = "~/.cargo/advisory-db"
# The url(s) of the advisory databases to use
db-urls = ["https://github.com/rustsec/advisory-db"]
# Ignore these advisories (only if you've assessed and accepted the risk)
ignore = [
    # Example: "RUSTSEC-2023-0001",
]
# Severity threshold - deny any advisory with this severity or higher
severity-threshold = "medium"

# =============================================================================
# License Configuration
# =============================================================================
[licenses]
# Confidence threshold for license detection (0.0 to 1.0)
confidence-threshold = 0.8

# List of allowed licenses
# Our project is MIT, so we allow MIT and compatible licenses
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",  # Used by some Rust compiler crates
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-DFS-2016",  # Unicode license
]

# List of explicitly denied licenses
deny = [
    "GPL-2.0",     # Copyleft incompatible with MIT
    "GPL-3.0",     # Copyleft incompatible with MIT
    "AGPL-3.0",    # Network copyleft
]

# Blanket approval or denial for OSI-approved licenses
# We're conservative here - explicitly list what we allow
copyleft = "deny"

# =============================================================================
# Bans Configuration (Duplicate Dependencies)
# =============================================================================
[bans]
# Lint level for when multiple versions of the same crate are detected
multiple-versions = "warn"  # Start with "warn", move to "deny" after cleanup

# Crates that are allowed to have multiple versions
skip = [
    # Keep this empty initially, add exceptions only after investigation
]

# Specific crates to deny
deny = [
    # Example: Crates with known issues
    # { name = "openssl", version = "<1.0" },
]

# =============================================================================
# Sources Configuration
# =============================================================================
[sources]
# Lint level for crates that are not from crates.io
unknown-registry = "warn"
unknown-git = "warn"

# Allow git sources (we may use forked dependencies for patches)
allow-git = [
    # Example: "https://github.com/organization/repo",
]
```

---

### 4. Address Current Duplicate Dependencies

**Identified duplicates from `cargo tree --duplicates`:**

#### 4.1: `bitflags` (v1.3.2, v2.9.4)
**Source:** Transitive dependency from `lsp-types` (v1) and `swc_*` crates (v2)

**Action:** Accept as exception (cannot control)
```toml
[bans.skip]
{ name = "bitflags", version = "=1.3.2" }  # lsp-types uses v1
```

---

#### 4.2: `dashmap` (v5.5.3, v6.1.0)
**Source:** Most crates use v5, `cb-plugins` uses v6

**Action:** Downgrade `cb-plugins` to dashmap 5.5.3
```toml
# crates/cb-plugins/Cargo.toml
[dependencies]
-dashmap = "6.1"
+dashmap = { workspace = true }  # Use workspace version (5.5.3)
```

---

#### 4.3: `dirs` (v5.0.1, v6.0.0)
**Source:** `cb-client` uses v5, `shellexpand` (test dep) pulls v6

**Action:** Update `cb-client` to dirs v6
```toml
# crates/cb-client/Cargo.toml
[dependencies]
-dirs = "5.0"
+dirs = "6.0"
```

---

#### 4.4: `getrandom` (v0.2.16, v0.3.3)
**Source:** v0.2 from jsonwebtoken, v0.3 from newer rand

**Action:** Accept as exception (cannot control, different major versions)
```toml
[bans.skip]
{ name = "getrandom", version = "=0.2.16" }  # jsonwebtoken indirect dep
```

---

#### 4.5: `hashbrown` (v0.14.5, v0.15.5, v0.16.0)
**Source:** Transitive from dashmap, indexmap, swc_allocator

**Action:** Accept as exception (low risk, internal to collections)
```toml
[bans.skip]
{ name = "hashbrown", version = "<0.16" }  # Transitive from collections
```

---

#### 4.6: `phf_shared` (appears twice)
**Source:** Internal to `phf` macros used by swc

**Action:** Accept as exception (proc-macro internal)
```toml
[bans.skip]
{ name = "phf_shared", version = "=0.11.3" }  # swc proc-macro internal
```

---

#### 4.7: `proc-macro2` (appears twice)
**Source:** Listed twice but likely same version - verify

**Action:** Investigate first
```bash
cargo tree -p proc-macro2 --duplicates
```

If truly duplicated, accept as exception (proc-macro crate).

---

### 5. Integrate into CI/CD

**Add to `.github/workflows/ci.yml`:**

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main]

jobs:
  # ... existing jobs

  # New job for cargo-deny checks
  deny:
    name: Dependency Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-deny
        run: cargo install cargo-deny --locked

      - name: Check advisories
        run: cargo deny check advisories

      - name: Check licenses
        run: cargo deny check licenses

      - name: Check bans (duplicates)
        run: cargo deny check bans

      - name: Check sources
        run: cargo deny check sources
```

**Or use the official action:**
```yaml
  deny:
    name: Dependency Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v1
        with:
          log-level: warn
          command: check all
```

---

### 6. Add Makefile Target

```makefile
# Makefile
.PHONY: deny
deny:
	@echo "Running cargo-deny checks..."
	cargo deny check

.PHONY: deny-update
deny-update:
	@echo "Updating advisory database..."
	cargo deny fetch

.PHONY: check-all
check-all: fmt clippy test deny  ## Run all checks (fmt, clippy, test, deny)
```

---

### 7. Documentation Updates

#### 7.1: Update CONTRIBUTING.md

```markdown
## Dependency Management

Before adding new dependencies:

1. Check if the functionality already exists in the workspace
2. Evaluate the dependency's maintenance status, license, and security
3. Run `cargo deny check` to ensure no issues are introduced

### Running Dependency Checks

```bash
# Check all: advisories, licenses, bans, sources
cargo deny check

# Check only security advisories
cargo deny check advisories

# Check only licenses
cargo deny check licenses

# Update advisory database
cargo deny fetch
```

### Handling cargo-deny Failures

If `cargo deny check` fails:

- **Advisories:** Investigate the CVE, assess risk, update dependency if possible
- **Licenses:** Ensure new dependency has compatible license (MIT/Apache-2.0/BSD)
- **Bans (duplicates):** Try to use workspace version or consolidate versions
- **Sources:** Avoid git dependencies unless necessary, prefer crates.io

If an exception is truly needed, update `.config/cargo-deny.toml` with justification.
```

#### 7.2: Update README.md

```markdown
## Security

This project uses [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) for:
- Security vulnerability scanning
- License compliance checking
- Duplicate dependency detection

Run security checks: `cargo deny check`
```

---

## Migration Checklist

### Setup Phase
- [ ] Install cargo-deny: `cargo install cargo-deny`
- [ ] Initialize config: `cargo deny init`
- [ ] Move `deny.toml` to `.config/cargo-deny.toml` (if not already there)
- [ ] Customize config with project-specific rules

### Cleanup Phase (Address Duplicates)
- [ ] Update `cb-plugins` to use workspace dashmap version (5.5.3)
- [ ] Update `cb-client` to dirs v6
- [ ] Add exceptions for unfixable duplicates (bitflags, getrandom, hashbrown)
- [ ] Verify no new duplicates introduced: `cargo deny check bans`

### Integration Phase
- [ ] Add cargo-deny to CI/CD workflow
- [ ] Add Makefile targets
- [ ] Update CONTRIBUTING.md with dependency guidelines
- [ ] Update README.md with security section
- [ ] Run full check: `cargo deny check`

### Validation Phase
- [ ] Verify CI passes with cargo-deny checks
- [ ] Test that legitimate violations are caught
- [ ] Document any accepted exceptions

---

## Expected Results

### Before
```bash
$ cargo tree --duplicates
bitflags v1.3.2
bitflags v2.9.4
dashmap v5.5.3
dashmap v6.1.0
dirs v5.0.1
dirs v6.0.0
getrandom v0.2.16
getrandom v0.3.3
hashbrown v0.14.5
hashbrown v0.15.5
hashbrown v0.16.0
# ... (11 duplicate dependency versions)
```

### After
```bash
$ cargo tree --duplicates
bitflags v1.3.2  # Exception: lsp-types (cannot control)
bitflags v2.9.4
getrandom v0.2.16  # Exception: jsonwebtoken (cannot control)
getrandom v0.3.3
hashbrown v0.14.5  # Exception: transitive collection deps
hashbrown v0.15.5
hashbrown v0.16.0
# (Reduced from 11 to ~6-7 duplicates, with documented exceptions)

$ cargo deny check
✅ advisories: ok
✅ licenses: ok
✅ bans: ok (with documented exceptions)
✅ sources: ok
```

---

## Risks and Mitigations

### Risk: False positives from advisories
**Likelihood:** Medium
**Impact:** Low (can be reviewed and ignored)
**Mitigation:** Investigate each advisory, add to `ignore` list with comment if not applicable

### Risk: License incompatibility discovered
**Likelihood:** Low (already using standard licenses)
**Impact:** High (may need to remove dependency)
**Mitigation:** Audit current dependencies first, establish policy before enforcement

### Risk: CI failures on dependency updates
**Likelihood:** Medium
**Impact:** Medium (blocks PRs)
**Mitigation:**
- Start with `warn` level, move to `deny` after cleanup
- Provide clear documentation on how to handle failures
- Make cargo-deny check part of local development workflow

---

## Alternatives Considered

### Alternative 1: cargo-audit only
**Pros:** Simpler, only checks security
**Cons:** Doesn't check licenses or duplicates

### Alternative 2: Manual review
**Pros:** No tooling required
**Cons:** Error-prone, doesn't scale

### Alternative 3: Dependabot only
**Pros:** Automatic PRs
**Cons:** Doesn't enforce policy, doesn't check duplicates

**Chosen:** cargo-deny provides comprehensive checking with CI integration.

---

## Success Criteria

- [ ] cargo-deny installed and configured
- [ ] Duplicate dependencies reduced by >30%
- [ ] All remaining duplicates documented with exceptions
- [ ] CI/CD enforces cargo-deny checks
- [ ] Documentation updated with dependency guidelines
- [ ] No security advisories in production dependencies

---

## References

- [cargo-deny documentation](https://embarkstudios.github.io/cargo-deny/)
- [cargo-deny GitHub](https://github.com/EmbarkStudios/cargo-deny)
- [RustSec Advisory Database](https://github.com/rustsec/advisory-db)
- [Embark Studios - Why we use cargo-deny](https://embark.dev/blog/2020/08/18/why-rust-deny/)

---

## Timeline

**Estimated effort:** 3-4 hours

- Setup: 30 minutes
- Cleanup duplicates: 1-2 hours
- CI integration: 30 minutes
- Documentation: 1 hour
- Testing: 30 minutes

**Recommended:** Execute in one session for consistency.
