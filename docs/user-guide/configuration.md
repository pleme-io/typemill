# Mill Configuration Reference

**Complete guide to configuring Mill for all deployment scenarios**

---

## Table of Contents

- [Configuration File Location](#configuration-file-location)
- [Configuration File Structure](#configuration-file-structure)
- [LSP Server Configuration](#lsp-server-configuration)
- [Environment Variables](#environment-variables)
- [Cache Configuration](#cache-configuration)
- [Logging Configuration](#logging-configuration)
- [Security & Authentication](#security--authentication)
- [Configuration Strategies](#configuration-strategies)
- [Common Configuration Examples](#common-configuration-examples)
- [Language-Specific Configuration](#language-specific-configuration)

---

## Configuration File Location

Mill looks for configuration in the following order:

1. `.typemill/config.json` (project-specific, recommended)
2. `~/.config/typemill/config.json` (user-wide)
3. Default configuration (built-in)

**Creating configuration:**
```bash
# Auto-detect languages and create config
mill setup

# Update existing config
mill setup --update

# Interactive setup
mill setup --interactive
```
---

## Configuration File Structure

**Minimal configuration:**
```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts", "tsx", "js", "jsx"],
        "command": ["typescript-language-server", "--stdio"]
      }
    ]
  }
}
```
**Complete configuration with all options:**
```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts", "tsx", "js", "jsx"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "web",
        "restartInterval": 10
      }
    ]
  },
  "cache": {
    "enabled": true,
    "maxSizeBytes": 268435456,
    "ttlSeconds": 3600,
    "persistent": false
  },
  "logging": {
    "level": "info",
    "format": "pretty"
  },
  "server": {
    "host": "127.0.0.1",
    "port": 3040
  }
}
```
---

## LSP Server Configuration

### Server Configuration Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `extensions` | string[] | ✅ | File extensions this LSP handles (e.g., `["ts", "tsx"]`) |
| `command` | string[] | ✅ | LSP server command and arguments |
| `rootDir` | string | Optional | Working directory for LSP (relative or absolute path) |
| `restartInterval` | number | Optional | Minutes before LSP restart (default: 15) |

### Why `rootDir` Matters

**TypeScript/JavaScript:**
- LSP needs to find `node_modules/typescript` and `tsconfig.json`
- Set to directory containing `package.json` or `tsconfig.json`

**Rust:**
- Rust Analyzer needs to find `Cargo.toml`
- Set to workspace root or crate root

**Python:**
- Python LSP needs project root for imports
- Set to directory containing Python project files

### Multiple LSP Instances

You can configure multiple instances of the same LSP for different directories:

```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "packages/frontend"
      },
      {
        "extensions": ["tsx"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "packages/backend"
      }
    ]
  }
}
```
---

## Environment Variables

**All configuration values can be overridden using environment variables** with the `TYPEMILL__` prefix (double underscores separate nested keys).

### Server Configuration

```bash
# Server host and port
export TYPEMILL__SERVER__HOST="127.0.0.1"
export TYPEMILL__SERVER__PORT=3040

# Authentication (JWT secret)
export TYPEMILL__SERVER__AUTH__JWT_SECRET="your-secret-key"
```
### Cache Configuration

```bash
# Enable/disable all caches
export TYPEMILL_DISABLE_CACHE=1

# Disable specific caches
export TYPEMILL_DISABLE_AST_CACHE=1
export TYPEMILL_DISABLE_IMPORT_CACHE=1
export TYPEMILL_DISABLE_LSP_METHOD_CACHE=1

# Or configure via structured env vars
export TYPEMILL__CACHE__ENABLED=true
export TYPEMILL__CACHE__TTL_SECONDS=3600
export TYPEMILL__CACHE__MAX_SIZE_BYTES=268435456
```
### Logging Configuration

```bash
# Logging level (error, warn, info, debug, trace)
export TYPEMILL__LOGGING__LEVEL="info"

# Logging format (pretty, json)
export TYPEMILL__LOGGING__FORMAT="pretty"
```
### Priority Order

Environment variables override configuration file values:

```
1. Environment variables (TYPEMILL__*)
2. Configuration file (.typemill/config.json)
3. Default values
```
### Using .env Files

Create a `.env` file in your project root (gitignored by default):

```bash
# .env
TYPEMILL__SERVER__AUTH__JWT_SECRET=dev-secret-key
TYPEMILL__LOGGING__LEVEL=debug
TYPEMILL_DISABLE_CACHE=0
```
Mill will automatically load `.env` files when present.

---

## Cache Configuration

Mill uses multiple caching layers for performance optimization.

### Cache Types

1. **AST Cache** - Parsed Abstract Syntax Trees and import graphs
2. **Import Cache** - File import lists (used in directory renames)
3. **LSP Method Cache** - Method name translations (plugin API ↔ LSP)

### Configuration Options

```json
{
  "cache": {
    "enabled": true,
    "maxSizeBytes": 268435456,
    "ttlSeconds": 3600,
    "persistent": false,
    "cacheDir": null
  }
}
```
| Option | Default | Description |
|--------|---------|-------------|
| `enabled` | `true` | Enable/disable AST cache |
| `maxSizeBytes` | `268435456` (256 MB) | Maximum cache size |
| `ttlSeconds` | `3600` (1 hour) | Time-to-live for entries |
| `persistent` | `false` | Persistent disk cache (not implemented) |
| `cacheDir` | `null` | Directory for persistent cache |

### Cache Control via Environment Variables

```bash
# Disable all caches (master switch)
TYPEMILL_DISABLE_CACHE=1 mill serve

# Disable specific caches
TYPEMILL_DISABLE_AST_CACHE=1 mill serve
TYPEMILL_DISABLE_IMPORT_CACHE=1 mill serve
```
### When to Disable Caches

**Development:**
```bash
# Force fresh data during development
export TYPEMILL_DISABLE_CACHE=1
```
**CI/CD:**
```bash
# Ensure fresh results in pipelines
TYPEMILL_DISABLE_CACHE=1 cargo test
```
**Debugging:**
```bash
# Isolate cache-related issues
TYPEMILL_DISABLE_AST_CACHE=1 mill tool rename_all ...
```
---

## Logging Configuration

### Log Levels

| Level | Description | Use Case |
|-------|-------------|----------|
| `error` | Errors only | Production |
| `warn` | Warnings and errors | Production |
| `info` | General information | Default |
| `debug` | Detailed debugging info | Development |
| `trace` | Very verbose output | Troubleshooting |

### Configuration

**Via config file:**
```json
{
  "logging": {
    "level": "debug",
    "format": "pretty"
  }
}
```
**Via environment variable:**
```bash
export TYPEMILL__LOGGING__LEVEL=debug
mill start
```
### Capturing Logs

```bash
# Save logs to file
mill start 2> logs.txt

# Follow in real-time
mill start 2>&1 | tee logs.txt
```
---

## Security & Authentication

### JWT Authentication

**Enable for production deployments:**

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 3040,
    "auth": {
      "jwtSecret": "your-secret-key",
      "jwtExpirySeconds": 86400,
      "jwtIssuer": "mill",
      "jwtAudience": "codeflow-clients"
    }
  }
}
```
**⚠️ Security Best Practices:**
- ✅ **Never commit secrets to config files** - use environment variables
- ✅ Use `TYPEMILL__SERVER__AUTH__JWT_SECRET` environment variable
- ✅ Keep server on `127.0.0.1` for local development
- ✅ Enable TLS for non-loopback addresses in production
- ✅ Use secret management services (Vault, AWS Secrets Manager)

**Recommended approach:**
```bash
# Store secrets in environment, not config files
export TYPEMILL__SERVER__AUTH__JWT_SECRET="$(openssl rand -hex 32)"
mill serve
```
### Network Binding

**Local development (default):**
```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 3040
  }
}
```
**Production (bind to all interfaces):**
```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 3040,
    "auth": {
      "enabled": true
    }
  }
}
```
---

## Configuration Strategies

### Portable Configuration (Recommended for Teams)

**Best for:** Teams, shared repositories, CI/CD

**Strategy:**
- ✅ Use relative paths for commands (`typescript-language-server`)
- ✅ Use relative paths for `rootDir` (`web`, not `/home/user/project/web`)
- ✅ Commit `.typemill/config.json` to version control
- ✅ Document PATH requirements in project README

**Example:**
```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts", "tsx", "js", "jsx"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "web"
      }
    ]
  }
}
```
**Team README should include:**
```markdown
## LSP Requirements

Ensure these are installed and in PATH:
- `typescript-language-server` - `npm install -g typescript-language-server`
- `rust-analyzer` - `rustup component add rust-analyzer`
```
### Local Configuration (Single Developer)

**Best for:** Personal projects, local experimentation

**Strategy:**
- ✅ Use absolute paths for commands
- ✅ Use absolute paths for `rootDir`
- ✅ Add `.typemill/config.json` to `.gitignore`

**Example:**
```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts", "tsx"],
        "command": ["/home/user/.nvm/versions/node/v20.0.0/bin/typescript-language-server", "--stdio"],
        "rootDir": "/home/user/projects/myapp/web"
      }
    ]
  }
}
```
---

## Common Configuration Examples

### TypeScript Monorepo

```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts", "tsx", "js", "jsx"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "."
      }
    ]
  }
}
```
### Rust Workspace

```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["rs"],
        "command": ["rust-analyzer"],
        "rootDir": ".",
        "restartInterval": 15
      }
    ]
  }
}
```
### Full-Stack (TypeScript + Rust + Python)

```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts", "tsx", "js", "jsx"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "web"
      },
      {
        "extensions": ["rs"],
        "command": ["rust-analyzer"],
        "rootDir": "."
      },
      {
        "extensions": ["py"],
        "command": ["pylsp"],
        "rootDir": "scripts"
      }
    ]
  }
}
```
### Development Mode (Verbose Logging + No Cache)

```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["rs"],
        "command": ["rust-analyzer"],
        "rootDir": "."
      }
    ]
  },
  "cache": {
    "enabled": false
  },
  "logging": {
    "level": "debug",
    "format": "pretty"
  }
}
```
Or via environment:
```bash
export TYPEMILL_DISABLE_CACHE=1
export TYPEMILL__LOGGING__LEVEL=debug
mill serve
```
### Production (Optimized)

```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts", "tsx"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "."
      }
    ]
  },
  "cache": {
    "enabled": true,
    "maxSizeBytes": 536870912,
    "ttlSeconds": 7200
  },
  "logging": {
    "level": "warn",
    "format": "json"
  },
  "server": {
    "host": "0.0.0.0",
    "port": 3040,
    "auth": {
      "enabled": true
    }
  }
}
```
---

## Language-Specific Configuration

### TypeScript / JavaScript

**Why `rootDir` matters:**
TypeScript LSP needs to find `node_modules/typescript` and `tsconfig.json`.

**Auto-detection:**
```bash
mill setup --update  # Automatically detects TS projects
```
**Manual configuration:**
```json
{
  "extensions": ["ts", "tsx", "js", "jsx"],
  "command": ["typescript-language-server", "--stdio"],
  "rootDir": "web"
}
```
**Monorepos with multiple TS projects:**
```json
{
  "lsp": {
    "servers": [
      {
        "extensions": ["ts"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "packages/frontend"
      },
      {
        "extensions": ["tsx"],
        "command": ["typescript-language-server", "--stdio"],
        "rootDir": "packages/backend"
      }
    ]
  }
}
```
### Rust

**Simple projects:**
```json
{
  "extensions": ["rs"],
  "command": ["rust-analyzer"],
  "rootDir": "."
}
```
**Large workspaces (increase restart interval):**
```json
{
  "extensions": ["rs"],
  "command": ["rust-analyzer"],
  "rootDir": ".",
  "restartInterval": 20
}
```
Rust Analyzer automatically discovers workspace members from `Cargo.toml`.

### Python

**Basic configuration:**
```json
{
  "extensions": ["py"],
  "command": ["pylsp"],
  "rootDir": "."
}
```
**With virtual environment:**
Ensure `pylsp` is installed in the activated virtual environment:
```bash
source venv/bin/activate
pip install python-lsp-server
```
---

## Verifying Configuration

```bash
# Check configuration and LSP availability
mill doctor

# View current configuration
cat .typemill/config.json

# Check server status
mill status

# Test with a tool call
mill tool workspace '{"action": "verify_project"}'
```
---

## Troubleshooting Configuration

### LSP Not Found in PATH

**Problem:**
```bash
$ mill doctor
Checking for 'typescript-language-server'... [✗] Not found in PATH.
```
**Solutions:**

1. **Install the LSP:**
```bash
npm install -g typescript-language-server typescript
```
2. **Add to PATH:**
```bash
# Find installation location
which typescript-language-server
npm list -g | grep typescript-language-server

# Add to shell profile (~/.bashrc or ~/.zshrc)
export PATH="$HOME/.nvm/versions/node/v20.0.0/bin:$PATH"
source ~/.bashrc
```
3. **Use absolute path in config:**
```json
{
  "command": ["/full/path/to/typescript-language-server", "--stdio"]
}
```
### Invalid `rootDir`

**Problem:** Tools not working, "Cannot find module" errors

**Solution:** Ensure `rootDir` points to correct project directory:
```bash
# Find your project root
find . -name "tsconfig.json" -o -name "Cargo.toml"

# Update config
mill setup --update  # Auto-detects correct rootDir
```
### Configuration Not Loading

**Problem:** Changes to config.json not taking effect

**Solution:** Restart Mill:
```bash
mill stop
mill start
```
---

## See Also

- **[getting-started.md](getting-started.md)** - Complete setup guide
- **[troubleshooting.md](troubleshooting.md)** - Common issues
- **[../operations/docker_deployment.md](../operations/docker_deployment.md)** - Docker configuration
