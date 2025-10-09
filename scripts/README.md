# Scripts Directory

## Recommended: Use Makefile Instead

For faster, more reliable setup, use the Makefile commands instead of these scripts:

```bash
# Quick setup (~30 seconds with pre-built binaries)
make setup

# Full development environment (~2-3 minutes)
make setup-full

# Install LSP servers for testing
make install-lsp-servers
```

## Legacy Scripts

### `setup-dev-tools.sh` ⚠️ Deprecated

**Why deprecated:**
- Compiles all tools from source (~30+ minutes)
- Frequently times out in CI/CD environments
- Slower than Make file-based setup with cargo-binstall

**When to use it:**
- Environments without `make`
- Need advanced tools like `cargo-flamegraph`, `jscpd`
- Prefer standalone script over Makefile

**Modern alternative:**
```bash
make setup-full    # Faster, uses pre-built binaries
```

### `check-duplicates.sh`

Still actively used by `make check-duplicates`.

## Migration Guide

If you were using:
```bash
./scripts/setup-dev-tools.sh
```

Switch to:
```bash
make setup        # Essential tools only (~30s)
make setup-full   # All optimization tools (~2-3min)
```

**Benefits:**
- 10-20x faster (pre-built binaries vs compiling)
- No timeouts
- Auto-installs cargo-binstall
- Clearer error messages
