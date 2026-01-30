# Getting Started with TypeMill

**Complete guide from installation to team setup**

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Quick Setup](#quick-setup)
- [Starting the Server](#starting-the-server)
- [Connecting to AI Assistants](#connecting-to-ai-assistants)
- [First Commands](#first-commands)
- [Team Setup](#team-setup)
- [Next Steps](#next-steps)

---

## Prerequisites

- Git repository with code files (TypeScript, Rust, or Python)
- Terminal access
- **For building from source:** Rust toolchain (get from [rustup.rs](https://rustup.rs/))

---

## Installation

### Option 1: Install Script (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/goobits/mill/main/install.sh | bash
```
This installs TypeMill to `~/.local/bin/mill` and adds it to your PATH.

### Option 2: Cargo Install

```bash
cargo install mill --locked
```
### Option 3: Build from Source

```bash
git clone https://github.com/goobits/typemill
cd typemill
cargo build --release
sudo cp target/release/mill /usr/local/bin/
```
**Verify installation:**
```bash
mill --version
```
---

## Quick Setup

> [!TIP] First Time Setup
> The setup process takes 2-3 minutes and only needs to be run once per project. LSP servers are cached globally and reused across all your projects.

Navigate to your project and run the interactive setup:

```bash
cd /path/to/your/project
mill setup
```

**What happens:**
1. 🔍 Scans project and detects languages (TypeScript, Rust, Python)
2. 📋 Creates `.typemill/config.json` with LSP server configurations
3. 📥 Prompts to auto-download missing LSP servers
4. ✅ Verifies LSP servers are working
5. 💾 Caches LSPs in `~/.mill/lsp/` for reuse across projects

**Example output:**
```
🔍 Scanning project...
✅ Detected: TypeScript project (45 .ts files, 12 .tsx files)
📦 Required LSP: typescript-language-server

❓ Install typescript-language-server? [Y/n] y
⬇️  Downloading typescript-language-server v4.3.0...
✅ Installed to ~/.mill/lsp/typescript-language-server
📝 Created config at .typemill/config.json

✨ Setup complete!
```

> [!SUCCESS] Verification
> You should see `✨ Setup complete!` with no errors. If you encounter issues, see the [troubleshooting guide](./troubleshooting.md).
**Updating existing config:**
```bash
mill setup --update           # Re-run setup
mill setup --update --interactive  # Interactive mode
```
---

## Starting the Server

Start TypeMill in MCP (Model Context Protocol) mode:

```bash
mill start
```
**Check status:**
```bash
mill status
```
You should see:
```
✅ TypeMill is running (PID: 12345)
✅ LSP servers configured: typescript-language-server, rust-analyzer
📁 Config: .typemill/config.json
```
**Stop the server:**
```bash
mill stop
```
---

## Connecting to AI Assistants

### Claude Desktop

> [!IMPORTANT] Configuration Required
> TypeMill requires configuration in Claude Desktop to enable the MCP connection. Make sure to restart Claude Desktop after adding the configuration.

Add TypeMill to your Claude Desktop MCP configuration:

**Config file location:**
- **macOS/Linux:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

**Add this configuration:**
```json
{
  "mcpServers": {
    "mill": {
      "command": "mill",
      "args": ["start"]
    }
  }
}
```
**Restart Claude Desktop** to load the configuration.

### Other MCP Clients

TypeMill works with any MCP-compatible client. See [MCP documentation](https://modelcontextprotocol.io/) for integration details.

---

## First Commands

Once connected, try these commands with your AI assistant:

### 1. Find a Definition
```
"Find the definition of the App component in src/App.tsx"
```
TypeMill uses the `inspect_code` tool (with `include: ["definition"]`) to show the exact location.

### 2. Show All References
```
"Show me everywhere the Config type is used"
```
TypeMill uses the `inspect_code` tool (with `include: ["references"]`) to list all usages.

### 3. Rename Safely
```
"Rename the function fetchData to loadData and update all references"
```
TypeMill will:
1. Show a preview plan (dry run mode - safe default)
2. Ask for confirmation
3. Execute the rename and update all imports/references

## Team Setup

### Configuration Strategies

**For teams sharing a repository, choose the right configuration strategy:**

#### Portable Configuration (Recommended for Teams)

**Best for:** Teams, shared repositories, CI/CD

**Key principles:**
- ✅ Use **relative paths** for LSP commands (`typescript-language-server`)
- ✅ Use **relative paths** for `rootDir` (`web`, not `/home/user/project/web`)
- ✅ **Commit `.typemill/config.json`** to version control
- ✅ **Document PATH requirements** in project README

**Why this works:**
- Team members can use different installation methods (nvm, rustup, system packages)
- Works across different operating systems (Windows, macOS, Linux)
- No hardcoded absolute paths that break on other machines

**Example portable config:**
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
      }
    ]
  }
}
```
**Document in your project README:**
```markdown
## Development Setup

### Prerequisites

Ensure these LSP servers are in your PATH:
- **TypeScript:** `npm install -g typescript-language-server typescript`
- **Rust:** `rustup component add rust-analyzer`

Then run:
\`\`\`bash
mill setup
mill start
\`\`\`
```
#### Local Configuration (Single Developer)

**Best for:** Personal projects, local experimentation

**Strategy:**
- Use absolute paths for commands and `rootDir`
- Add `.typemill/config.json` to `.gitignore`
- Optimize for your specific machine setup

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
### Adding LSP Binaries to PATH

**macOS / Linux:**
```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.nvm/versions/node/v20.0.0/bin:$PATH"
export PATH="$HOME/.cargo/bin:$PATH"

# Reload
source ~/.bashrc  # or source ~/.zshrc
```
**Windows PowerShell:**
```powershell
# Add to $PROFILE
$env:PATH += ";C:\Users\YourName\AppData\Roaming\npm"
```
### Verifying Team Setup

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

## Troubleshooting

### LSP Server Not Found

If setup couldn't find an LSP server, install it manually:

**TypeScript:**
```bash
npm install -g typescript-language-server typescript
```
**Rust:**
```bash
rustup component add rust-analyzer
```
**Python:**
```bash
pipx install python-lsp-server  # Recommended (PEP 668 compliant)
# OR
pip install --user python-lsp-server
```
Then re-run setup:
```bash
mill setup --update
```
### Server Won't Start

Run diagnostics:
```bash
mill doctor
```
### Configuration Issues

View configuration:
```bash
cat .typemill/config.json
```
Reset configuration:
```bash
rm -rf .typemill
mill setup
```
**For detailed troubleshooting,** see **[troubleshooting.md](troubleshooting.md)**

---

## Next Steps

### Explore Tools

```bash
mill tools              # List all tools
mill docs tools         # View tool documentation
```
### Learn Common Workflows

- **[cheatsheet.md](cheatsheet.md)** - Quick command reference
- **[../tools/refactor.md](../tools/refactor.md)** - Advanced refactoring patterns

### Configure Advanced Features

- **[configuration.md](configuration.md)** - Complete configuration reference
- **[../operations/cache_configuration.md](../operations/cache_configuration.md)** - Performance tuning
- **[../operations/docker_deployment.md](../operations/docker_deployment.md)** - Docker deployment

### Use CLI Directly

You can call tools directly without an AI assistant:

```bash
# Find definition
mill tool inspect_code '{"filePath": "src/App.tsx", "line": 9, "character": 5, "include": ["definition"]}'

# Rename file (preview)
mill tool rename_all --target file:src/old.ts --new-name src/new.ts

# Rename file (execute)
mill tool rename_all --target file:src/old.ts --new-name src/new.ts --dry-run false
```
---

## Need Help?

- **Within TypeMill:** `mill docs --search "<keyword>"` or `mill doctor`
- **Online:** [GitHub Issues](https://github.com/goobits/typemill/issues) | [Discussions](https://github.com/goobits/typemill/discussions)
- **Security:** security@goobits.com (private disclosure)

---

**🎉 You're ready to use TypeMill!** Ask your AI assistant to refactor or navigate your codebase with precise LSP-powered intelligence.
