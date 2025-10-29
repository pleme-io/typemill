# Mill Command Cheat Sheet

**Quick reference for the most common Mill commands**

---

## Server Management

```bash
mill setup               # Auto-detect languages and configure LSP servers
mill start               # Start MCP server (stdio mode for Claude)
mill serve               # Start WebSocket server (default port: 3040)
mill stop                # Stop running server
mill status              # Show server status
mill doctor              # Diagnose configuration issues
```text
---

## LSP Management

```bash
mill install-lsp typescript  # Install TypeScript LSP server
mill install-lsp rust        # Install Rust LSP server
mill install-lsp python      # Install Python LSP server
```text
---

## Documentation

```bash
mill docs                       # List all documentation topics
mill docs quickstart            # View quick start guide
mill docs cheatsheet            # View this cheat sheet
mill docs tools                 # View tools catalog
mill docs tools/refactoring     # View refactoring guide
mill docs --search "keyword"    # Search all documentation
mill docs <topic> --raw         # View raw markdown
```text
---

## Tool Usage

### List Tools
```bash
mill tools                  # Show all public tools (table format)
mill tools --format json    # Output as JSON
mill tools --format names   # List names only
```text
### Call Tools Directly

**General syntax:**
```bash
mill tool <tool_name> '{"param": "value"}'     # JSON format
mill tool <tool_name> --param value            # Flag format (supported tools)
```text
---

## Navigation Tools

### Find Definition
```bash
mill tool find_definition '{"file_path": "src/app.ts", "line": 10, "character": 5}'
```text
### Find References
```bash
mill tool find_references '{"file_path": "src/app.ts", "line": 10, "character": 5}'
```text
### Search Symbols
```bash
mill tool search_symbols '{"query": "Config", "scope": "workspace"}'
```text
### Get Diagnostics (Errors/Warnings)
```bash
mill tool get_diagnostics '{"file_path": "src/app.ts"}'
```text
---

## Refactoring Tools

**All refactoring tools support `dryRun` option:**
- **`dryRun: true`** (default) - Preview changes without modifying files
- **`dryRun: false`** - Execute changes

### Rename File
```bash
# Preview
mill tool rename --target file:src/old.ts --new-name src/new.ts

# Execute
mill tool rename --target file:src/old.ts --new-name src/new.ts '{"options": {"dryRun": false}}'
```text
### Rename Directory
```bash
# Preview
mill tool rename --target directory:old-dir --new-name new-dir

# Execute
mill tool rename '{"target": {"kind": "directory", "path": "old-dir"}, "newName": "new-dir", "options": {"dryRun": false}}'
```text
### Rename Symbol (via AI assistant)
Ask Claude:
```text
"Rename the function fetchData to loadData"
```text
### Extract Function
```bash
mill tool extract '{"kind": "function", "source": {"file_path": "src/app.ts", "start_line": 10, "start_character": 5, "end_line": 15, "end_character": 10}, "name": "handleLogin"}'
```text
### Move Symbol
```bash
mill tool move '{"source": {"file_path": "src/app.ts", "line": 10, "character": 5}, "destination": {"file_path": "src/utils.ts"}}'
```text
### Inline Variable
```bash
mill tool inline '{"target": {"kind": "variable", "file_path": "src/app.ts", "line": 10, "character": 5}}'
```text
### Delete File
```bash
# Preview
mill tool delete --target file:src/unused.ts

# Execute
mill tool delete '{"target": {"kind": "file", "path": "src/unused.ts"}, "options": {"dryRun": false}}'
```text
---

## Analysis Tools

### Code Quality
```bash
# Analyze complexity
mill tool analyze.quality '{"kind": "complexity", "scope": "workspace"}'

# Check specific file
mill tool analyze.quality '{"kind": "complexity", "scope": "file:src/app.ts"}'
```text
### Find Dead Code
```bash
# Find unused imports
mill tool analyze.dead_code '{"kind": "unused_imports", "scope": "workspace"}'

# Find unused symbols
mill tool analyze.dead_code '{"kind": "unused_symbols", "scope": "workspace"}'
```text
### Dependency Analysis
```bash
mill tool analyze.dependencies '{"kind": "imports", "scope": "file:src/app.ts"}'
mill tool analyze.dependencies '{"kind": "circular", "scope": "workspace"}'
```text
### Batch Analysis (Multiple Files)
```bash
mill tool analyze.batch '{"scope": "workspace", "analyzers": ["complexity", "dead_code"]}'
```text
### Markdown Analysis
```bash
# Check for structural issues like heading hierarchy
mill tool analyze.quality '{"kind": "markdown_structure", "scope": "file:README.md"}'

# Check for formatting issues like missing alt text
mill tool analyze.quality '{"kind": "markdown_formatting", "scope": "file:docs/user-guide.md"}'
```text
---

## Workspace Tools

### Find and Replace
```bash
mill tool workspace.find_replace '{"pattern": "oldName", "replacement": "newName", "scope": "workspace"}'
```text
### Create Package (Rust)
```bash
mill tool workspace.create_package '{"name": "my-crate", "template": "minimal", "package_type": "library"}'
```text
### Extract Dependencies (Rust)
```bash
mill tool workspace.extract_dependencies '{"module_path": "src/utils.rs"}'
```text
---

## Bulk Operations

### Convert Naming Conventions
```bash
# Convert kebab-case to camelCase
mill convert-naming --from kebab-case --to camelCase --glob "src/**/*.ts"

# Dry run (preview)
mill convert-naming --from snake_case --to camelCase --glob "**/*.ts" --dry-run
```text
---

## Refactoring Patterns

### Safe Refactoring Workflow

1. **Preview** (default - always safe):
   ```bash
   mill tool rename --target file:old.ts --new-name new.ts
   ```

2. **Review** the plan output

3. **Execute** if satisfied:
   ```bash
   mill tool rename '{"target": {"kind": "file", "path": "old.ts"}, "newName": "new.ts", "options": {"dryRun": false}}'
   ```

### Scope Options (for rename operations)

Control what gets updated:

```bash
--scope code        # Code only (imports, string literals)
--scope standard    # Code + docs + configs (recommended)
--scope comments    # Standard + code comments
--scope everything  # Comments + markdown prose text
```text
Example:
```bash
mill tool rename --target directory:old-dir --new-name new-dir --scope standard
```text
---

## Common Workflows

### Setup New Project
```bash
cd /path/to/project
mill setup
mill start
# Configure Claude Desktop
```text
### Rename File Safely
```bash
# 1. Preview
mill tool rename --target file:src/old.ts --new-name src/new.ts

# 2. Execute
mill tool rename '{"target": {"kind": "file", "path": "src/old.ts"}, "newName": "src/new.ts", "options": {"dryRun": false}}'
```text
### Find and Fix Dead Code
```bash
# 1. Find unused imports
mill tool analyze.dead_code '{"kind": "unused_imports", "scope": "workspace"}'

# 2. Review the findings

# 3. Use Claude to remove them:
"Remove all unused imports from src/app.ts"
```text
### Refactor with AI
Ask Claude:
```text
"Extract the login logic from src/app.ts into a separate function"
"Move the Config type to src/types.ts"
"Rename all occurrences of oldName to newName"
```text
---

## Keyboard-Friendly Shortcuts

### Quick Status Check
```bash
alias mst="mill status"
```text
### Quick Tool List
```bash
alias mtools="mill tools --format names"
```text
### Quick Docs
```bash
alias mdocs="mill docs"
```text
Add these to your `~/.bashrc` or `~/.zshrc`.

---

## Troubleshooting

### LSP Not Working
```bash
mill doctor                              # Diagnose issues
mill install-lsp typescript              # Reinstall LSP
mill stop && mill start                  # Restart server
```text
### Configuration Reset
```bash
rm -rf .typemill/
mill setup
```text
### View Logs
```bash
# Server logs (if running as daemon)
tail -f ~/.mill/logs/mill.log

# Or run in foreground to see output
mill start
```text
### Search Documentation
```bash
mill docs --search "LSP"
mill docs --search "rename"
mill docs --search "dry run"
```text
---

## Tips

💡 **Always preview first**: Refactoring tools default to dry run mode (`dryRun: true`)
💡 **Use AI for complex refactoring**: Claude can chain multiple tool calls
💡 **Check status regularly**: `mill status` shows what's configured
💡 **Search docs**: `mill docs --search` finds what you need quickly
💡 **Use scope wisely**: `--scope standard` is recommended for most renames

---

## More Help

- **Full Documentation**: `mill docs`
- **Tool Reference**: `mill docs tools`
- **Quick Start**: `mill docs quickstart`
- **GitHub Issues**: https://github.com/goobits/typemill/issues
- **Search Docs**: `mill docs --search <keyword>`