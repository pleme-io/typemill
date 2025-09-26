# Offline Build and Test Guide

This guide explains how to build and test the Rust workspace in environments with restricted or no network access.

## Prerequisites

Before going offline, ensure you have:
1. Rust toolchain installed (rustc, cargo)
2. All dependencies vendored locally

## Vendoring Dependencies

### Step 1: Create vendor directory

```bash
# While online, vendor all dependencies
cd /workspace/rust
cargo vendor
```

This creates a `vendor/` directory with all dependencies.

### Step 2: Configure cargo to use vendored deps

Create `.cargo/config.toml`:

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```

### Step 3: Commit vendor directory

```bash
git add vendor/ .cargo/config.toml
git commit -m "Add vendored dependencies for offline builds"
```

## Offline Building

Once dependencies are vendored:

```bash
# Build without network access
cargo build --offline

# Build release version
cargo build --release --offline

# Build specific crate
cargo build -p cb-core --offline
```

## Offline Testing

```bash
# Run all tests offline
cargo test --workspace --offline

# Run specific crate tests
cargo test -p cb-ast --offline

# Run with verbose output
cargo test --workspace --offline -- --nocapture
```

## Docker Support for Offline Builds

Create a Docker image with vendored dependencies:

```dockerfile
FROM rust:1.75-slim AS builder
WORKDIR /app

# Copy vendored dependencies first
COPY vendor vendor
COPY .cargo .cargo

# Copy source code
COPY . .

# Build offline
RUN cargo build --release --offline

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/cb-server /usr/local/bin/
COPY --from=builder /app/target/release/cb-client /usr/local/bin/
CMD ["cb-server"]
```

## CI/CD Configuration

For CI environments with network restrictions:

```yaml
# .github/workflows/offline-build.yml
name: Offline Build
on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Restore vendor cache
        uses: actions/cache@v3
        with:
          path: vendor
          key: ${{ runner.os }}-vendor-${{ hashFiles('**/Cargo.lock') }}
      - name: Build offline
        run: cargo build --workspace --offline
      - name: Test offline
        run: cargo test --workspace --offline
```

## Alternative: Local Registry Mirror

For environments that can maintain a local registry:

1. Set up a local registry mirror using `cargo-local-registry`:
```bash
cargo install cargo-local-registry
cargo local-registry --sync Cargo.lock target/registry
```

2. Configure cargo to use local registry:
```toml
[source.crates-io]
replace-with = "local-registry"

[source.local-registry]
local-registry = "target/registry"
```

## Troubleshooting

### Missing Dependencies

If build fails with missing dependencies:
1. Go online temporarily
2. Run `cargo vendor` again
3. Commit updated vendor directory

### Cargo.lock Conflicts

Always commit `Cargo.lock` to ensure reproducible builds:
```bash
git add Cargo.lock
git commit -m "Update Cargo.lock"
```

### Binary Size Optimization

For smaller binaries in offline environments:
```toml
# In Cargo.toml
[profile.release]
strip = true
opt-level = "z"
lto = true
codegen-units = 1
```

## Verification

To verify offline capability:
```bash
# Disconnect network or use network namespace
unshare -n cargo build --workspace
unshare -n cargo test --workspace
```

## Notes

- Vendored dependencies increase repo size (~50-100MB)
- Consider using Git LFS for vendor directory
- Update vendored deps regularly for security patches
- Some crates may require system libraries (install before going offline)