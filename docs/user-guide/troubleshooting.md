# Mill Troubleshooting Guide

**Solutions to common Mill setup and usage issues**

---

## ⚡ Quick FAQ

> **Fast answers to the most common issues** - Jump to [detailed solutions](#🛠️-setup-issues) below

### Setup & Installation

**Q: Configuration file already exists, what do I do?**
A: Run `mill setup --update` to update existing config, or `rm -rf .typemill && mill setup` to start fresh.

**Q: LSP server not found in PATH?**
A: Install it: `mill install-lsp <language>` or `npm install -g typescript-language-server` (TypeScript).

**Q: How do I verify LSP servers are working?**
A: Run `mill status` to see all configured LSP servers and their status.

**Q: Mill setup doesn't detect my language?**
A: Ensure your project has recognizable files (package.json for TypeScript, Cargo.toml for Rust). Run `mill setup --force`.

### Server Issues

**Q: Server won't start?**
A: Check `mill doctor` for diagnostics. Ensure LSP servers are installed (`which typescript-language-server`).

**Q: How do I restart the server?**
A: Run `mill stop && mill start` or just `mill start` (it will restart if already running).

**Q: Where are server logs?**
A: Run `mill start` in foreground to see output, or check `~/.mill/logs/mill.log` if running as daemon.

**Q: Port 3040 already in use?**
A: Set a different port: `export TYPEMILL__SERVER__PORT=3050` then `mill serve`.

### Tool Issues

**Q: Tool returns "file not found"?**
A: Use absolute paths or paths relative to workspace root. Check current directory with `pwd`.

**Q: rename_all didn't update all imports?**
A: Ensure LSP server is running (`mill status`). Check file extensions match LSP config (`.typemill/config.json`).

**Q: dryRun mode - how do I actually execute changes?**
A: All refactoring tools default to `dryRun: true`. Set `"options": {"dryRun": false}` to execute.

**Q: Tool call failed with JSON parse error?**
A: Ensure JSON is valid. Use single quotes around JSON, double quotes inside: `'{"key": "value"}'`.

### LSP & Language Support

**Q: TypeScript LSP can't find node_modules?**
A: Set `rootDir` in `.typemill/config.json` or ensure `tsconfig.json` exists in project root.

**Q: Rust Analyzer takes forever to start?**
A: Large projects need time to index. Wait 30-60s. Check `mill status` to see if it's initializing.

**Q: Python LSP not working?**
A: Install pylsp: `pipx install python-lsp-server` or `pip install --user python-lsp-server`.

**Q: Multiple language support in one project?**
A: Yes! `mill setup` detects all languages. Each gets its own LSP server configuration.

### Performance

**Q: Mill is slow?**
A: Enable caching: `unset TYPEMILL_DISABLE_CACHE`. Adjust `restartInterval` in config (increase to 30-60 minutes).

**Q: High memory usage?**
A: LSP servers (especially rust-analyzer) can use 1-2GB. Close unused editors/IDEs to free memory.

**Q: Can I use caching in production?**
A: Yes. Set `TYPEMILL__CACHE__ENABLED=true` and `TYPEMILL__CACHE__TTL_SECONDS=3600` for 1-hour cache.

---

## 🛠️ Setup Issues

### "Configuration file already exists"

**Problem:**
```bash
$ mill setup
⚠️  Configuration file already exists at: .typemill/config.json
   To recreate configuration, please delete the existing file first.
```
**Solution:**
```bash
# Option 1: Update existing config
mill setup --update

# Option 2: Interactive update
mill setup --update --interactive

# Option 3: Start fresh (deletes existing config)
rm -rf .typemill
mill setup
```
---

### "LSP server not found in PATH"

**Problem:**
```bash
$ mill doctor
Checking for 'typescript-language-server'... [✗] Not found in PATH.
```
**Diagnosis:**
The LSP binary isn't in your system PATH.

**Solutions:**

**Option 1: Install the LSP server**
```bash
# TypeScript
npm install -g typescript-language-server typescript

# Rust
rustup component add rust-analyzer

# Python
pip install python-lsp-server
```
**Option 2: Add to PATH**
```bash
# Find where it's installed
which typescript-language-server
# or
npm list -g | grep typescript-language-server

# Add to PATH (bash/zsh)
echo 'export PATH="$HOME/.nvm/versions/node/vXX.X.X/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```
**Option 3: Use absolute path in config** (see **[configuration.md](configuration.md#lsp-server-configuration)** for details)

---

### "Could not find a valid TypeScript installation"

**Problem:**
LSP logs show:
```
Could not find a valid TypeScript installation.
Please ensure that the "typescript" dependency is installed in the workspace.
```
**Root cause:** `rootDir` is not set, so LSP can't find `node_modules/typescript`.

**Solution:**

**Auto-detect:**
```bash
mill setup --update  # Detects and sets rootDir automatically
```
**Manual fix:**
1. Find your TypeScript project root:
```bash
find . -name "tsconfig.json" -o -name "package.json"
```
2. Update config to include `rootDir` (see **[configuration.md](configuration.md#language-specific-configuration)** for details)

3. Restart Mill:
```bash
mill stop
mill start
```
---

## 🔌 LSP Issues

### TypeScript LSP can't find `node_modules`

**Symptoms:**
- "Cannot find module" errors
- Imports not resolving
- No autocomplete

**Solution:**
Ensure `rootDir` points to the directory containing `node_modules`:
```json
{
  "rootDir": "web"  // Directory with web/node_modules/
}
```
---

### Rust Analyzer crashes on startup

**Symptoms:**
```bash
$ mill status
Rust LSP: ❌ Crashed
```
**Common causes:**
1. **Large workspace:** Increase restart interval
```json
{
  "extensions": ["rs"],
  "command": ["rust-analyzer"],
  "restartInterval": 20  // ← Increase from default 15
}
```
2. **Corrupted cache:** Clear rust-analyzer cache
```bash
rm -rf ~/.cache/rust-analyzer
mill stop
mill start
```
3. **Old version:** Update rust-analyzer
```bash
rustup update
rustup component add rust-analyzer --force
```
---

### Python LSP not responding

**Symptoms:**
- Long startup time
- No responses to queries

**Solutions:**

1. **Check if `pylsp` is installed:**
```bash
which pylsp
pylsp --version
```
2. **Virtual environment issues:**
```bash
# Ensure pylsp is in the active venv
source venv/bin/activate
pip install python-lsp-server
```
3. **Set rootDir to project root:**
```json
{
  "extensions": ["py"],
  "command": ["pylsp"],
  "rootDir": "."  // ← Project root
}
```
---

## 🛠️ Tool Usage Issues

### "Tool does not support flag-based arguments"

**Problem:**
```bash
$ mill tool inspect_code --target file:src/app.rs:10:5
Error: Tool 'inspect_code' does not support flag-based arguments
```
**Explanation:**
Code intelligence tools require JSON arguments, not flags.

**Solution:**
```bash
# Use JSON arguments
mill tool inspect_code '{
  "filePath": "src/app.rs",
  "line": 10,
  "character": 5,
  "include": ["definition"]
}'
```
**Which tools need JSON:**
- Code intelligence: `inspect_code`, `search_code`
- Refactoring: `rename_all`, `refactor`, `relocate`, `prune`
- Workspace: `workspace`
- Use `mill tools` to see all tools and their argument types

---

### "Required arguments were not provided: <ARGS>"

**Problem:**
```bash
$ mill tool workspace
Error: required arguments were not provided: <ARGS>
```
**Solution:**
```bash
# workspace requires an action parameter
mill tool workspace '{"action": "verify_project"}'

# Legacy (internal-only)
mill tool health_check '{}'
```
---

### "Invalid JSON arguments"

**Problem:**
```bash
$ mill tool search_code {"query":"test"}
Error: Invalid JSON arguments: expected value at line 1 column 2
```
**Cause:** Shell is interpreting the JSON.

**Solution:** Use single quotes around JSON:
```bash
# ✅ Correct
mill tool search_code '{"query":"test","limit":10}'

# ❌ Wrong (shell interprets {})
mill tool search_code {"query":"test"}

# Alternative: Use a file
echo '{"query":"test","limit":10}' > args.json
mill tool search_code --input-file args.json
```
---

## 🐛 Debug Mode

Enable detailed logging:

```bash
# Option 1: Environment variable (recommended)
export TYPEMILL__LOGGING__LEVEL=debug
mill start

# Option 2: Edit config
# See configuration.md for logging configuration details
```
View logs:
```bash
# Logs go to stderr
mill start 2> debug.log

# Or follow in real-time
mill start 2>&1 | tee debug.log
```
**For complete logging configuration,** see **[configuration.md](configuration.md#logging-configuration)**

---

## 💡 Still Stuck?

1. **Check configuration:**
```bash
mill doctor
cat .typemill/config.json
```
2. **Check server status:**
```bash
mill status
```
3. **Review documentation:**
```bash
mill docs setup-guide
mill docs tools
```
4. **File an issue:**
Include:
- `mill doctor` output
- `.typemill/config.json` (redact sensitive paths)
- Error messages
- OS and Mill version (`mill --version`)

Report at: https://github.com/goobits/typemill/issues

---

## 📚 Related Documentation

- **[configuration.md](configuration.md)** - Complete configuration reference
- **[getting-started.md](getting-started.md)** - Getting started guide
- **[cheatsheet.md](cheatsheet.md)** - Quick command reference
- **[tools/README.md](../tools/README.md)** - Tool catalog
