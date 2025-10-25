# Workspace Tools - Rust

Language-specific details for `workspace.create_package` with Rust/Cargo projects.

**See [workspace.md](workspace.md) for shared API documentation.**

## Language: Rust

**Manifest file:** `Cargo.toml`
**Workspace config:** `[workspace]` section in root `Cargo.toml`

## Template Structure

### Minimal Template
Creates baseline project structure:
- `Cargo.toml` - Package manifest with metadata
- `src/lib.rs` (library) or `src/main.rs` (binary)
- `README.md` - Basic project documentation
- `.gitignore` - Rust-specific ignore patterns
- `tests/integration_test.rs` - Starter integration test

### Full Template
Minimal template + extras:
- `examples/basic.rs` - Example usage code

## Package Types

| Type | Entry Point | Binary Config |
|------|-------------|---------------|
| `library` | `src/lib.rs` | None |
| `binary` | `src/main.rs` | `[[bin]]` section in Cargo.toml |

## Generated Cargo.toml

**Library:**
```toml
[package]
name = "my-lib"
version = "0.1.0"
edition = "2021"

[dependencies]
```

**Binary:**
```toml
[package]
name = "my-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "my-cli"
path = "src/main.rs"

[dependencies]
```

## Naming Conventions

- **Package path:** `crates/my-lib` (kebab-case recommended)
- **Crate name:** `my_lib` (underscores, derived from path)
- **Binary name:** `my-cli` (hyphens preserved from package name)

## Workspace Integration

When `addToWorkspace: true`:
- Adds relative path to `[workspace.members]` in root `Cargo.toml`
- Paths normalized to forward slashes (cross-platform)
- Example: `members = ["crates/my-lib"]`

## Example Usage

```bash
# Create library package
mill tool workspace.create_package '{
  "packagePath": "crates/mill-utils",
  "package_type": "library",
  "options": {
    "template": "minimal",
    "addToWorkspace": true
  }
}'

# Creates:
# - crates/mill-utils/Cargo.toml
# - crates/mill-utils/src/lib.rs
# - crates/mill-utils/README.md
# - crates/mill-utils/.gitignore
# - crates/mill-utils/tests/integration_test.rs

# Create binary with full template
mill tool workspace.create_package '{
  "packagePath": "crates/mill-cli",
  "package_type": "binary",
  "options": {
    "template": "full"
  }
}'

# Creates minimal files + examples/basic.rs
```

## Notes

- Edition defaults to "2021" (current stable)
- Package name must be valid Rust identifier (after underscore conversion)
- Binary packages can have multiple binaries (edit Cargo.toml manually)
- Workspace members support glob patterns: `["crates/*"]`
