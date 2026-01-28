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
```
---

## LSP Management

```bash
mill install-lsp typescript  # Install TypeScript LSP server
mill install-lsp rust        # Install Rust LSP server
mill install-lsp python      # Install Python LSP server
```
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
```
---

## Tool Usage

### List Tools
```bash
mill tools                  # Show all public tools (table format)
mill tools --format json    # Output as JSON
mill tools --format names   # List names only
```
### Call Tools Directly

**General syntax:**
```bash
mill tool <tool_name> '{"param": "value"}'     # JSON format
mill tool <tool_name> --param value            # Flag format (supported tools)
```
---

## Navigation Tools

### Find Definition
```bash
mill tool find_definition '{"file_path": "src/app.ts", "line": 10, "character": 5}'
```
### Find References
```bash
mill tool find_references '{"file_path": "src/app.ts", "line": 10, "character": 5}'
```
### Search Symbols
```bash
mill tool search_symbols '{"query": "Config", "scope": "workspace"}'
```
### Get Diagnostics (Errors/Warnings)
```bash
mill tool get_diagnostics '{"file_path": "src/app.ts"}'
```
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
```
### Rename Directory
```bash
# Preview
mill tool rename --target directory:old-dir --new-name new-dir

# Execute
mill tool rename '{"target": {"kind": "directory", "path": "old-dir"}, "newName": "new-dir", "options": {"dryRun": false}}'
```
### Rename Symbol (via AI assistant)
Ask Claude:
```
"Rename the function fetchData to loadData"
```
### Extract Function
```bash
mill tool extract '{"kind": "function", "source": {"file_path": "src/app.ts", "start_line": 10, "start_character": 5, "end_line": 15, "end_character": 10}, "name": "handleLogin"}'
```
### Move Symbol
```bash
mill tool move '{"source": {"file_path": "src/app.ts", "line": 10, "character": 5}, "destination": {"file_path": "src/utils.ts"}}'
```
### Inline Variable
```bash
mill tool inline '{"target": {"kind": "variable", "file_path": "src/app.ts", "line": 10, "character": 5}}'
```
### Delete File
```bash
# Preview
mill tool delete --target file:src/unused.ts

# Execute
mill tool delete '{"target": {"kind": "file", "path": "src/unused.ts"}, "options": {"dryRun": false}}'
```
---

## Workspace Tools

### Find and Replace
```bash
mill tool workspace.find_replace '{"pattern": "oldName", "replacement": "newName", "scope": "workspace"}'
```
### Create Package (Rust)
```bash
mill tool workspace.create_package '{"name": "my-crate", "template": "minimal", "package_type": "library"}'
```
### Extract Dependencies (Rust)
```bash
mill tool workspace.extract_dependencies '{"module_path": "src/utils.rs"}'
```
---

## Bulk Operations

### Convert Naming Conventions
```bash
# Convert kebab-case to camelCase
mill convert-naming --from kebab-case --to camelCase --glob "src/**/*.ts"

# Dry run (preview)
mill convert-naming --from snake_case --to camelCase --glob "**/*.ts" --dry-run
```
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
```
Example:
```bash
mill tool rename --target directory:old-dir --new-name new-dir --scope standard
```
---

## Common Workflows

### Setup New Project
```bash
cd /path/to/project
mill setup
mill start
# Configure Claude Desktop
```
### Rename File Safely
```bash
# 1. Preview
mill tool rename --target file:src/old.ts --new-name src/new.ts

# 2. Execute
mill tool rename '{"target": {"kind": "file", "path": "src/old.ts"}, "newName": "src/new.ts", "options": {"dryRun": false}}'
```
### Refactor with AI
Ask Claude:
```
"Extract the login logic from src/app.ts into a separate function"
"Move the Config type to src/types.ts"
"Rename all occurrences of oldName to newName"
```
---

## Keyboard-Friendly Shortcuts

### Quick Status Check
```bash
alias mst="mill status"
```
### Quick Tool List
```bash
alias mtools="mill tools --format names"
```
### Quick Docs
```bash
alias mdocs="mill docs"
```
Add these to your `~/.bashrc` or `~/.zshrc`.

---

## Troubleshooting

### LSP Not Working
```bash
mill doctor                              # Diagnose issues
mill install-lsp typescript              # Reinstall LSP
mill stop && mill start                  # Restart server
```
### Configuration Reset
```bash
rm -rf .typemill/
mill setup
```
### View Logs
```bash
# Server logs (if running as daemon)
tail -f ~/.mill/logs/mill.log

# Or run in foreground to see output
mill start
```
### Search Documentation
```bash
mill docs --search "LSP"
mill docs --search "rename"
mill docs --search "dry run"
```
---

## Tips

💡 **Always preview first**: Refactoring tools default to dry run mode (`dryRun: true`)
💡 **Use AI for complex refactoring**: Claude can chain multiple tool calls
💡 **Check status regularly**: `mill status` shows what's configured
💡 **Search docs**: `mill docs --search` finds what you need quickly
💡 **Use scope wisely**: `--scope standard` is recommended for most renames

---

## Tool Parameter Quick Reference

> **Quick lookup for tool parameters** - See [tools/](../tools/) for complete documentation

### Navigation Tools

| Tool | Required Parameters | Optional Parameters | Returns |
|------|-------------------|-------------------|---------|
| `find_definition` | `file_path`, `line`, `character` | - | Location |
| `find_references` | `file_path`, `line`, `character` | `include_declaration` | Location[] |
| `find_implementations` | `file_path`, `line`, `character` | - | Location[] |
| `find_type_definition` | `file_path`, `line`, `character` | - | Location |
| `search_symbols` | `query` | `scope`, `kind` | SymbolInfo[] |
| `get_symbol_info` | `file_path`, `line`, `character` | - | SymbolInfo |
| `get_diagnostics` | `file_path` | `severity` | Diagnostic[] |
| `get_call_hierarchy` | `file_path`, `line`, `character` | `direction` | CallHierarchy |

**Example:**
```bash
mill tool find_definition '{"file_path": "src/app.ts", "line": 10, "character": 5}'
```

---

### Refactoring Tools

All refactoring tools support `options.dryRun` (default: `true`)

| Tool | Required Parameters | Optional Parameters | Returns |
|------|-------------------|-------------------|---------|
| `rename` | `target`, `newName` | `options` (dryRun, scope) | EditPlan/ApplyResult |
| `extract` | `kind`, `source`, `name` | `options` (dryRun) | EditPlan/ApplyResult |
| `inline` | `target` | `options` (dryRun) | EditPlan/ApplyResult |
| `move` | `source`, `destination` | `options` (dryRun) | EditPlan/ApplyResult |
| `delete` | `target` | `options` (dryRun) | EditPlan/ApplyResult |

**Target formats:**
- File: `{"kind": "file", "path": "src/app.ts"}`
- Directory: `{"kind": "directory", "path": "src/utils"}`
- Symbol: `{"file": "src/app.ts", "line": 10, "character": 5}`

**Example:**
```bash
mill tool rename '{"target": {"kind": "file", "path": "old.ts"}, "newName": "new.ts", "options": {"dryRun": false}}'
```

---

### Workspace Tools

| Tool | Required Parameters | Optional Parameters | Returns |
|------|-------------------|-------------------|---------|
| `workspace.create_package` | `name`, `packageType`, `path` | `options` (dryRun) | CreateResult |
| `workspace.extract_dependencies` | `source_crate`, `target_crate`, `module_path` | `options` (dryRun) | ExtractResult |
| `workspace.find_replace` | `pattern`, `replacement` | `mode`, `scope`, `dryRun` | ReplaceResult |

**Package types:**
- Rust: `"rust_library"`, `"rust_binary"`
- TypeScript: `"typescript_package"`, `"typescript_library"`
- Python: `"python_package"`

**Example:**
```bash
mill tool workspace.create_package '{"name": "my-crate", "packageType": "rust_library", "path": "crates/my-crate", "options": {"dryRun": false}}'
```

---

### System Tools

| Tool | Required Parameters | Optional Parameters | Returns |
|------|-------------------|-------------------|---------|
| `health_check` | - | - | HealthStatus |

**Example:**
```bash
mill tool health_check '{}'
```

---

## More Help

- **Full Documentation**: `mill docs`
- **Tool Reference**: `mill docs tools`
- **Workflow Recipes**: `mill docs cookbook`
- **Quick Start**: `mill docs quickstart`
- **GitHub Issues**: https://github.com/goobits/typemill/issues
- **Search Docs**: `mill docs --search <keyword>`
