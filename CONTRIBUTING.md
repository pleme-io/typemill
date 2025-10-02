# Contributing to Codeflow Buddy (Rust)

First off, thank you for considering contributing! It's people like you that make Codeflow Buddy such a great tool.

## Getting Started

### Prerequisites

- **Rust Toolchain:** This project is built with Rust. If you don't have it installed, you can get it from [rustup.rs](https://rustup.rs/).

### Setup

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/goobits/codebuddy.git
    cd codebuddy
    ```

2.  **Install build optimization tools (HIGHLY RECOMMENDED):**
    ```bash
    ./deployment/scripts/setup-dev-tools.sh
    ```
    This installs `sccache` (compilation cache) and `mold` (fast linker), which can speed up builds by 2-10x.

    **Expected improvements:**
    - Incremental builds: 2-5x faster
    - Link times: 3-10x faster
    - Clean builds: Cached across git branches

    The tools are automatically configured in `.cargo/config.toml`.

3.  **Build the project:**
    ```bash
    cargo build
    ```
    This will download all dependencies and compile the Rust crates.

    **First build:** ~2 minutes (one-time dependency compilation)
    **Incremental builds:** 5-20 seconds with sccache/mold

## Running Tests

We have a comprehensive test suite to ensure code quality and prevent regressions.

To run all tests for the entire Rust workspace:
```bash
cargo test --workspace
```

## Code Style and Linting

We use the standard Rust formatting and linting tools to maintain a consistent codebase.

- **Formatting:** Before committing your changes, please format your code with `cargo fmt`.
  ```bash
  cargo fmt --all
  ```

- **Linting:** We use `clippy` for catching common mistakes and improving code quality.
  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

## Pull Request Process

1.  **Create a Feature Branch:**
    ```bash
    git checkout -b your-feature-name
    ```

2.  **Commit Your Changes:** Make your changes and commit them with a descriptive message.
    ```bash
    git commit -m "feat: Add new feature" -m "Detailed description of the changes."
    ```

3.  **Ensure Tests Pass:** Run the full test suite one last time to make sure everything is working correctly.
    ```bash
    cargo test --workspace
    ```

4.  **Push to Your Branch:**
    ```bash
    git push origin your-feature-name
    ```

5.  **Open a Pull Request:** Go to the repository on GitHub and open a new pull request. Provide a clear title and description of your changes.

## Build Performance Tips

### Optimization Tools (Configured Automatically)

The project uses several build optimizations configured in `.cargo/config.toml`:

- **sccache**: Compilation cache that dramatically speeds up rebuilds
- **mold**: Modern, fast linker (3-10x faster than traditional linkers)
- **Dependency optimization**: Dependencies compiled with `-O2` in dev mode

### Quick Commands

```bash
# Check sccache statistics
sccache --show-stats

# Clear sccache (if having cache issues)
sccache --zero-stats

# Fast feedback during development (doesn't build binaries)
cargo check

# Build only changed code (fastest)
cargo build

# Full rebuild (slow, use only when necessary)
cargo clean && cargo build
```

### Build Times Reference

With sccache and mold installed:

| Build Type | Time (First) | Time (Incremental) |
|------------|--------------|-------------------|
| `cargo check` | ~30s | 2-5s |
| `cargo build` | ~2m | 5-20s |
| `cargo build --release` | ~3m | 30-60s |
| `cargo test` | ~2.5m | 10-30s |

**Note:** Times vary based on:
- CPU cores (6+ cores recommended)
- SSD vs HDD (SSD strongly recommended)
- Changes scope (few files vs many files)

### Troubleshooting Slow Builds

If builds are slower than expected:

1. **Verify sccache is working:**
   ```bash
   sccache --show-stats
   # Should show cache hits on second build
   ```

2. **Check mold is being used:**
   ```bash
   grep -r "fuse-ld=mold" .cargo/config.toml
   # Should show linker configuration
   ```

3. **Monitor build parallelism:**
   ```bash
   # Check CPU usage during builds
   # Should use 80-100% of all cores
   ```

4. **Clear cache if corrupted:**
   ```bash
   sccache --zero-stats
   rm -rf target/
   cargo build
   ```
