# Dev Container for Typemill

This directory contains configuration for [VS Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/containers) and [GitHub Codespaces](https://github.com/features/codespaces).

## What's Included

The dev container provides a fully-configured development environment with:

**Core Development (Required):**
- **Rust toolchain** (latest stable)
- **Development utilities**:
  - `cargo-nextest` (fast test runner)
  - `sccache` (build cache)
  - `cargo-watch` (auto-rebuild)
- **VS Code extensions**:
  - `rust-analyzer` (Rust language support)
  - `even-better-toml` (TOML support)
  - `crates` (Cargo.toml management)
  - `vscode-lldb` (debugging)

**Language Plugin Development (Optional):**

TypeMill's language plugins are optional and maintained in the repository. The devcontainer includes tooling for all plugins so you can work on any of them without additional setup:

- **TypeScript/JavaScript**: Node.js LTS + `typescript-language-server`
- **Python**: Python 3 + `pylsp` (Python Language Server)
- **Go**: `gopls` (Go Language Server)
- **Java**: Java 17 + Maven (for Java AST parser)
- **C#**: .NET 8.0 SDK (for C# AST parser)
- **Rust**: `rust-analyzer` (core language)

> **Note:** You only need the tools for plugins you're actively developing. The setup script will build available plugins and skip missing ones.

## Getting Started

### Option 1: VS Code (Local)

1. Install [Docker Desktop](https://www.docker.com/products/docker-desktop)
2. Install [VS Code Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
3. Open this repository in VS Code
4. Click "Reopen in Container" when prompted (or use Command Palette: `Dev Containers: Reopen in Container`)
5. Wait for the container to build and initialize (~5-10 minutes first time)
6. Start coding!

### Option 2: GitHub Codespaces (Cloud)

1. Go to the repository on GitHub
2. Click "Code" → "Codespaces" → "Create codespace on main"
3. Wait for environment to initialize (~5-10 minutes)
4. Start coding in your browser!

## What Happens on First Start

The `post-create.sh` script automatically:

1. Installs Rust development tools (`cargo-nextest`, `sccache`, `cargo-watch`)
2. Installs LSP servers (rust-analyzer, typescript-language-server)
3. Builds available language plugin parsers (detects what's present in `crates/`)
4. Runs initial `cargo build` (cached for faster subsequent builds)
5. Runs quick test suite to verify core functionality works
6. Creates default `.typemill/config.json`

> **Smart Detection:** The script only builds language parsers for plugins that exist in the repository. Missing plugins are silently skipped.

## Customization

### Add More Extensions

Edit `.devcontainer/devcontainer.json`:

```json
"customizations": {
  "vscode": {
    "extensions": [
      "rust-lang.rust-analyzer",
      "your-extension-id-here"
    ]
  }
}
```text
### Skip Initial Build

Edit `post-create.sh` and comment out the build steps.

### Add System Packages

Edit `.devcontainer/devcontainer.json` features or add `apt-get install` to `post-create.sh`.

## Troubleshooting

### Container won't start
- Ensure Docker Desktop is running
- Check Docker Desktop has enough resources (4GB+ RAM recommended)

### Build fails during post-create
- The environment is still usable, just run `make first-time-setup` manually

### LSP server not working
- Run `mill doctor` to diagnose
- Check PATH includes `~/.cargo/bin` and `~/.local/bin`

## Performance Tips

- **First build is slow** (~5-10 minutes): Subsequent builds are much faster thanks to `sccache`
- **Use cargo check**: Faster than full builds for quick feedback
- **Use make test**: Runs fast tests only (~10s)

## Learn More

- [VS Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/containers)
- [GitHub Codespaces](https://docs.github.com/en/codespaces)
- [Dev Container Specification](https://containers.dev/)