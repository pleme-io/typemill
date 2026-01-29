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
mill docs tools/refactor        # View refactor guide
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

### Inspect Code
```bash
# Find definition
mill tool inspect_code '{"filePath": "src/app.ts", "line": 9, "character": 5, "include": ["definition"]}'

# Find references
mill tool inspect_code '{"filePath": "src/app.ts", "line": 9, "character": 5, "include": ["references"]}'

# Get diagnostics
mill tool inspect_code '{"filePath": "src/app.ts", "include": ["diagnostics"]}'
```

### Search Code
```bash
mill tool search_code '{"query": "Config", "limit": 10}'
```
---

## Refactoring Tools

**All refactoring tools support `dryRun` option:**
- **`dryRun: true`** (default) - Preview changes without modifying files
- **`dryRun: false`** - Execute changes

### Rename File (rename_all)
```bash
# Preview
mill tool rename_all --target file:src/old.ts --new-name src/new.ts

# Execute
mill tool rename_all --target file:src/old.ts --new-name src/new.ts '{"options": {"dryRun": false}}'
```

### Rename Directory (rename_all)
```bash
# Preview
mill tool rename_all --target directory:old-dir --new-name new-dir

# Execute
mill tool rename_all '{"target": {"kind": "directory", "filePath": "old-dir"}, "newName": "new-dir", "options": {"dryRun": false}}'
```

### Rename Symbol (via AI assistant)
Ask Claude:
```
"Rename the function fetchData to loadData"
```

### Extract Function (refactor)
```bash
mill tool refactor '{"action": "extract", "params": {"kind": "function", "filePath": "src/app.ts", "range": {"startLine": 9, "startCharacter": 5, "endLine": 14, "endCharacter": 10}, "name": "handleLogin"}}'
```

### Move Symbol (relocate)
```bash
mill tool relocate '{"target": {"kind": "symbol", "filePath": "src/app.ts", "line": 9, "character": 5}, "destination": {"filePath": "src/utils.ts"}}'
```

### Inline Variable (refactor)
```bash
mill tool refactor '{"action": "inline", "params": {"kind": "variable", "filePath": "src/app.ts", "line": 9, "character": 5}}'
```

### Delete File (prune)
```bash
# Preview
mill tool prune --target file:src/unused.ts

# Execute
mill tool prune '{"target": {"kind": "file", "filePath": "src/unused.ts"}, "options": {"dryRun": false}}'
```
---

## Workspace Tools

### Find and Replace (workspace)
```bash
mill tool workspace '{"action": "find_replace", "pattern": "oldName", "replacement": "newName", "scope": "workspace"}'
```

### Create Package (workspace)
```bash
mill tool workspace '{"action": "create_package", "params": {"name": "my-crate", "packageType": "library"}, "options": {"template": "minimal"}}'
```

### Extract Dependencies (workspace)
```bash
mill tool workspace '{"action": "extract_dependencies", "params": {"sourceManifest": "source/Cargo.toml", "targetManifest": "target/Cargo.toml", "dependencies": ["tokio"]}}'
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
   mill tool rename_all --target file:old.ts --new-name new.ts
   ```

2. **Review** the plan output

3. **Execute** if satisfied:
   ```bash
   mill tool rename_all '{"target": {"kind": "file", "path": "old.ts"}, "newName": "new.ts", "options": {"dryRun": false}}'
   ```

### Scope Options (for rename_all operations)

Control what gets updated:

```bash
--scope code        # Code only (imports, string literals)
--scope standard    # Code + docs + configs (recommended)
--scope comments    # Standard + code comments
--scope everything  # Comments + markdown prose text
```
Example:
```bash
mill tool rename_all --target directory:old-dir --new-name new-dir --scope standard
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
mill tool rename_all --target file:src/old.ts --new-name src/new.ts

# 2. Execute
mill tool rename_all '{"target": {"kind": "file", "path": "src/old.ts"}, "newName": "src/new.ts", "options": {"dryRun": false}}'
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

### Navigation Tools (Magnificent Seven API)

| Tool | Required Parameters | Optional Parameters | Returns |
|------|-------------------|-------------------|---------|
| `inspect_code` | `filePath` | `line`, `character`, `include`, `detailLevel` | Varies by include fields |
| `search_code` | `query` | `scope`, `kind` | SymbolInfo[] |

**Include fields for inspect_code:**
- `"definition"` - Find symbol definition
- `"references"` - Find all references
- `"implementations"` - Find implementations
- `"typeInfo"` - Find type information
- `"callHierarchy"` - Get call hierarchy
- `"diagnostics"` - Get errors/warnings

**Detail levels:**
- `"basic"` - Definition + type info only
- `"deep"` - All available information

**Example:**
```bash
mill tool inspect_code '{"filePath": "src/app.ts", "line": 9, "character": 5, "include": ["definition", "typeInfo"]}'
```

---

### Refactoring Tools (Magnificent Seven API)

All refactoring tools support `options.dryRun` (default: `true`)

| Tool | Required Parameters | Optional Parameters | Returns |
|------|-------------------|-------------------|---------|
| `rename_all` | `target`, `newName` | `options` (dryRun, scope) | EditPlan/ApplyResult |
| `refactor` | `action`, varies by action | `options` (dryRun) | EditPlan/ApplyResult |
| `relocate` | `target`, `destination` | `options` (dryRun) | EditPlan/ApplyResult |
| `prune` | `target` | `options` (dryRun) | EditPlan/ApplyResult |

**Actions for refactor:**
- `extract` - Extract function, variable, constant
- `inline` - Inline variable, function, constant

**Target formats:**
- File: `{"kind": "file", "filePath": "src/app.ts"}`
- Directory: `{"kind": "directory", "filePath": "src/utils"}`
- Symbol: `{"kind": "symbol", "filePath": "src/app.ts", "line": 9, "character": 5}`

**Example:**
```bash
mill tool rename_all '{"target": {"kind": "file", "filePath": "old.ts"}, "newName": "new.ts", "options": {"dryRun": false}}'
```

---

### Workspace Tools (Magnificent Seven API)

| Tool | Required Parameters | Optional Parameters | Returns |
|------|-------------------|-------------------|---------|
| `workspace` | `action` | varies by action | Varies by action |

**Actions for workspace:**
- `create_package` - Create new package (requires: name, packageType, path)
- `extract_dependencies` - Extract dependencies (requires: source_crate, target_crate, module_path)
- `find_replace` - Find and replace (requires: pattern, replacement)
- `verify_project` - Health check (no additional params)

**Package types:**
- Rust: `"rust_library"`, `"rust_binary"`
- TypeScript: `"typescript_package"`, `"typescript_library"`
- Python: `"python_package"`

**Example:**
```bash
mill tool workspace '{"action": "create_package", "name": "my-crate", "packageType": "rust_library", "path": "crates/my-crate", "options": {"dryRun": false}}'
```

---

### System Tools (Magnificent Seven API)

| Tool | Required Parameters | Optional Parameters | Returns |
|------|-------------------|-------------------|---------|
| `workspace` | `action: "verify_project"` | - | HealthStatus |

**Example:**
```bash
mill tool workspace '{"action": "verify_project"}'
```

**Legacy (internal-only):**
```bash
mill tool health_check '{}'  # Now internal, use workspace instead
```

---

## More Help

- **Full Documentation**: `mill docs`
- **Tool Reference**: `mill docs tools`
- **Workflow Recipes**: `mill docs cookbook`
- **Quick Start**: `mill docs quickstart`
- **GitHub Issues**: https://github.com/goobits/typemill/issues
- **Search Docs**: `mill docs --search <keyword>`
