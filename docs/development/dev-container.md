# Development Container

Everything you need to develop TypeMill with a consistent, containerized environment using VS Code Dev Containers.

## Overview

TypeMill uses Dev Containers for a consistent development experience across all platforms. The devcontainer provides:

- ✅ Complete Rust toolchain (rustc, cargo, clippy, rustfmt)
- ✅ All language SDKs (Node.js, Python, Java, .NET, Go)
- ✅ Language servers pre-configured (rust-analyzer, typescript-language-server, pylsp)
- ✅ Build acceleration tools (sccache, mold)
- ✅ Testing frameworks (cargo-nextest)
- ✅ VS Code extensions and settings

## Quick Start

### Prerequisites

- [Docker](https://www.docker.com/products/docker-desktop)
- [VS Code](https://code.visualstudio.com/)
- [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

### Launch Development Environment

1. **Open in VS Code:**
   ```bash
   git clone https://github.com/goobits/typemill.git
   cd typemill
   code .
   ```

2. **Reopen in Container:**
   - Press `F1` or `Ctrl+Shift+P`
   - Select: `Dev Containers: Reopen in Container`
   - Wait for container to build (first time: ~5-10 minutes)

3. **Start developing:**
   ```bash
   # Inside the container terminal:
   make first-time-setup  # One-time setup
   cargo build            # Build project
   cargo nextest run      # Run tests
   ```

## Container Features

### Installed Tools

**Rust:**
- rust-analyzer (LSP)
- cargo, clippy, rustfmt
- cargo-nextest (fast test runner)

**Language Support:**
- Node.js LTS + npm
- Python 3.11 + pip
- Java 17 + Maven
- .NET 8.0
- Go 1.21

**Language Servers:**
- typescript-language-server
- pylsp (Python Language Server)
- gopls (Go Language Server)

**Development Tools:**
- Git configuration mounted from host
- Zsh + Oh My Zsh
- Common utilities (curl, wget, etc.)

### VS Code Extensions

Automatically installed:
- `rust-lang.rust-analyzer` - Rust LSP
- `tamasfe.even-better-toml` - TOML support
- `serayuzgur.crates` - Cargo.toml management
- `vadimcn.vscode-lldb` - Debugging

### Port Forwarding

Ports automatically forwarded from container to host:
- `3040` - Web documentation server
- `3000` - TypeMill MCP server

Access from your browser: `http://localhost:3040`

## Configuration

### Devcontainer Configuration

Location: `.devcontainer/devcontainer.json`

Key settings:
```json
{
  "name": "Typemill Development",
  "image": "mcr.microsoft.com/devcontainers/rust:1-bookworm",
  "features": {
    "python": { "version": "3.11" },
    "node": { "version": "lts" },
    "java": { "version": "17", "installMaven": true },
    "dotnet": { "version": "8.0" },
    "go": { "version": "1.21" }
  }
}
```
### Post-Create Setup

The container runs `.devcontainer/post-create.sh` after creation:
- Installs Rust components
- Installs language servers
- Configures build tools
- Sets up pre-commit hooks

## Common Tasks

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Check without building
cargo check
```
### Testing

```bash
# Fast tests only
make test

# Full test suite
make test-full

# Tests with LSP servers
make test-lsp

# Watch mode (auto-run on changes)
make dev-handlers
```
### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# All checks
make check
```
### Web Documentation

```bash
# Start documentation server
cd web
npm install  # First time only
npm run dev

# Access at http://localhost:3040
```
## Troubleshooting

### Container Won't Build

**Problem:** Timeout or network errors during build

**Solution:**
```bash
# Rebuild without cache
docker system prune -a
# Then reopen in container
```
### Extensions Not Loading

**Problem:** VS Code extensions missing after reopen

**Solution:**
```bash
# Rebuild container
F1 → "Dev Containers: Rebuild Container"
```
### Slow Performance

**Problem:** Container feels sluggish

**Solutions:**
- Increase Docker memory allocation (Docker Desktop → Settings → Resources)
- Enable BuildKit: `export DOCKER_BUILDKIT=1`
- Use named volumes instead of bind mounts (already configured)

### Ports Not Forwarding

**Problem:** Can't access localhost:3040 or localhost:3000

**Solution:**
```bash
# Check VS Code port forwarding
View → Ports (or F1 → "Forward a Port")
# Manually forward 3040 and 3000
```
## Advanced Usage

### Multiple Workspaces

Open multiple instances:
```bash
# Terminal 1: Main development
code typemill/

# Terminal 2: Experimental branch
code typemill-feature/
```
Each gets its own container instance.

### Attach Additional Terminal

While container is running:
1. Open VS Code terminal: `Ctrl+` ` (backtick)
2. Click `+` to add terminal
3. All terminals share the same container

### Custom Docker Settings

Edit `.devcontainer/devcontainer.json`:

```json
{
  "runArgs": [
    "--cpus=4",          // Limit CPU cores
    "--memory=8g"        // Limit RAM
  ],
  "mounts": [
    "source=${localEnv:HOME}/.ssh,target=/home/vscode/.ssh,readonly,type=bind"
  ]
}
```
### Debugging

Launch configurations in `.vscode/launch.json`:

```json
{
  "type": "lldb",
  "request": "launch",
  "name": "Debug mill",
  "cargo": {
    "args": ["build", "--bin=mill"]
  }
}
```
Press `F5` to start debugging.

## Architecture

```
Host Machine
    ↓
VS Code (local)
    ↓
Docker Container (devcontainer)
    ├── Rust toolchain
    ├── Language SDKs
    ├── Language servers
    ├── /workspace (mounted from host)
    └── VS Code Server (remote)
```
**Key benefits:**
- Source code stays on host (fast file I/O)
- Build artifacts in container (consistent environment)
- Extensions run in container (full language support)
- Git configuration shared from host

## Production Deployment

For production deployments, TypeMill is typically installed via:

### Cargo Install (Recommended)

```bash
cargo install mill --locked
mill setup
mill start
```
### From Source

```bash
git clone https://github.com/goobits/typemill.git
cd typemill
cargo build --release
./target/release/mill setup
./target/release/mill start
```
### Docker Production (Future)

Production Docker images are not currently provided but may be added in the future. For now, use cargo installation on your server.

## See Also

- [Contributing Guide](../../contributing.md) - Setup and development workflow
- [Core Concepts](../architecture/core-concepts.md) - System architecture
- [Development Guide](../development/overview.md) - Language plugin development
- [Testing Guide](../development/testing.md) - Test infrastructure
- [Logging Guidelines](../development/logging_guidelines.md) - Structured logging

## FAQ

**Q: Do I need to install Rust on my host machine?**
A: No, everything runs in the container.

**Q: Can I use this without VS Code?**
A: Yes, but you'll need to manually run `docker compose` with the devcontainer configuration. VS Code provides the best experience.

**Q: How much disk space does this use?**
A: ~5-10 GB for the container image and build cache.

**Q: Can I develop on Windows?**
A: Yes, Dev Containers work on Windows, macOS, and Linux with Docker Desktop.

**Q: What happens to my changes when the container stops?**
A: Source code changes persist (mounted from host). Build artifacts are preserved in Docker volumes.

---

**Last Updated:** 2025-10-27
**Container Image:** `mcr.microsoft.com/devcontainers/rust:1-bookworm`