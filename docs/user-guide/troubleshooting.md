# Mill Troubleshooting Guide

**Solutions to common Mill setup and usage issues**

---

## 🛠️ Setup Issues

### "Configuration file already exists"

**Problem:**
```bash
$ mill setup
⚠️  Configuration file already exists at: .typemill/config.json
   To recreate configuration, please delete the existing file first.
```text
**Solution:**
```bash
# Option 1: Update existing config
mill setup --update

# Option 2: Interactive update
mill setup --update --interactive

# Option 3: Start fresh (deletes existing config)
rm -rf .typemill
mill setup
```text
---

### "LSP server not found in PATH"

**Problem:**
```bash
$ mill doctor
Checking for 'typescript-language-server'... [✗] Not found in PATH.
```text
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
```text
**Option 2: Add to PATH**
```bash
# Find where it's installed
which typescript-language-server
# or
npm list -g | grep typescript-language-server

# Add to PATH (bash/zsh)
echo 'export PATH="$HOME/.nvm/versions/node/vXX.X.X/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```text
**Option 3: Use absolute path in config** (see **[configuration.md](configuration.md#lsp-server-configuration)** for details)

---

### "Could not find a valid TypeScript installation"

**Problem:**
LSP logs show:
```text
Could not find a valid TypeScript installation.
Please ensure that the "typescript" dependency is installed in the workspace.
```text
**Root cause:** `rootDir` is not set, so LSP can't find `node_modules/typescript`.

**Solution:**

**Auto-detect:**
```bash
mill setup --update  # Detects and sets rootDir automatically
```text
**Manual fix:**
1. Find your TypeScript project root:
```bash
find . -name "tsconfig.json" -o -name "package.json"
```text
2. Update config to include `rootDir` (see **[configuration.md](configuration.md#language-specific-configuration)** for details)

3. Restart Mill:
```bash
mill stop
mill start
```text
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
```text
---

### Rust Analyzer crashes on startup

**Symptoms:**
```bash
$ mill status
Rust LSP: ❌ Crashed
```text
**Common causes:**
1. **Large workspace:** Increase restart interval
```json
{
  "extensions": ["rs"],
  "command": ["rust-analyzer"],
  "restartInterval": 20  // ← Increase from default 15
}
```text
2. **Corrupted cache:** Clear rust-analyzer cache
```bash
rm -rf ~/.cache/rust-analyzer
mill stop
mill start
```text
3. **Old version:** Update rust-analyzer
```bash
rustup update
rustup component add rust-analyzer --force
```text
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
```text
2. **Virtual environment issues:**
```bash
# Ensure pylsp is in the active venv
source venv/bin/activate
pip install python-lsp-server
```text
3. **Set rootDir to project root:**
```json
{
  "extensions": ["py"],
  "command": ["pylsp"],
  "rootDir": "."  // ← Project root
}
```text
---

## 🛠️ Tool Usage Issues

### "Tool does not support flag-based arguments"

**Problem:**
```bash
$ mill tool find_definition --target file:src/app.rs:10:5
Error: Tool 'find_definition' does not support flag-based arguments
```text
**Explanation:**
Navigation and analysis tools require JSON arguments, not flags.

**Solution:**
```bash
# Use JSON arguments
mill tool find_definition '{
  "file_path": "src/app.rs",
  "line": 10,
  "character": 5
}'
```text
**Which tools need JSON:**
- Navigation: `find_definition`, `find_references`, `search_symbols`, etc.
- Analysis: `get_diagnostics`, `analyze.*`
- Use `mill tools` to see all tools and their argument types

---

### "Required arguments were not provided: <ARGS>"

**Problem:**
```bash
$ mill tool health_check
Error: required arguments were not provided: <ARGS>
```text
**Solution:**
```bash
# health_check takes an empty JSON object
mill tool health_check '{}'
```text
---

### "Invalid JSON arguments"

**Problem:**
```bash
$ mill tool search_symbols {"query":"test"}
Error: Invalid JSON arguments: expected value at line 1 column 2
```text
**Cause:** Shell is interpreting the JSON.

**Solution:** Use single quotes around JSON:
```bash
# ✅ Correct
mill tool search_symbols '{"query":"test","limit":10}'

# ❌ Wrong (shell interprets {})
mill tool search_symbols {"query":"test"}

# Alternative: Use a file
echo '{"query":"test","limit":10}' > args.json
mill tool search_symbols --input-file args.json
```text
---

## 🐛 Debug Mode

Enable detailed logging:

```bash
# Option 1: Environment variable (recommended)
export TYPEMILL__LOGGING__LEVEL=debug
mill start

# Option 2: Edit config
# See configuration.md for logging configuration details
```text
View logs:
```bash
# Logs go to stderr
mill start 2> debug.log

# Or follow in real-time
mill start 2>&1 | tee debug.log
```text
**For complete logging configuration,** see **[configuration.md](configuration.md#logging-configuration)**

---

## 💡 Still Stuck?

1. **Check configuration:**
```bash
mill doctor
cat .typemill/config.json
```text
2. **Check server status:**
```bash
mill status
```text
3. **Review documentation:**
```bash
mill docs setup-guide
mill docs tools
```text
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
- **[setup-guide.md](setup-guide.md)** - Team setup strategies
- **[cheatsheet.md](cheatsheet.md)** - Quick command reference
- **[tools/README.md](tools/README.md)** - All 29 tools