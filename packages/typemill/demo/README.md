# TypeMill Demo

## Recording Tools

### Option 1: VHS (Recommended)
Generates deterministic GIFs from a script.

```bash
# Install VHS
brew install charmbracelet/tap/vhs

# Record
vhs demo.tape
```

### Option 2: asciinema
Records live terminal sessions.

```bash
# Install asciinema
pip install asciinema

# Record interactively
asciinema rec demo.cast

# Or run the script
asciinema rec demo.cast -c "./record.sh"

# Convert to GIF (optional)
npm install -g asciicast2gif
asciicast2gif demo.cast demo.gif
```

## Demo Script

The `record.sh` script demonstrates:
1. Version check
2. Available tools list
3. LSP server status
4. Live rename operation with dry-run

## Real Output Examples

### `mill status`
```
TypeMill Status

Server Status
  Not running
  Start with: mill start

Configuration
  Configuration loaded
  Path: .typemill/config.json
  Log level: info
  Log format: Pretty

LSP Servers
✅ typescript-language-server: Extensions: ts, tsx, js, jsx
✅ pylsp: Extensions: py
✅ rust-analyzer: Extensions: rs

Status check complete
```

### `mill tools`
```
Available MCP Tools

Tool Name           Handler
------------------  ----------------
inspect_code        InspectHandler
prune               PruneHandler
refactor            RefactorHandler
relocate            RelocateHandler
rename_all          RenameAllHandler
search_code         SearchHandler
workspace           WorkspaceHandler

Public tools: 7 across 7 handlers
```

### Rename Operation (dry-run)

**Before:**
```
src/
├── utils.ts      ← File to rename
└── app.ts        ← Imports from utils.ts
```

**Command:**
```bash
mill tool rename_all '{
  "target": {"kind": "file", "filePath": "src/utils.ts"},
  "newName": "src/helpers.ts",
  "options": {"dryRun": true}
}'
```

**Output:**
```json
{
  "status": "preview",
  "summary": "Preview: 2 file(s) will be affected",
  "changes": {
    "rename": "src/utils.ts → src/helpers.ts",
    "importUpdates": [
      {
        "file": "src/app.ts",
        "before": "import { formatDate } from './utils'",
        "after": "import { formatDate } from './helpers'"
      }
    ]
  }
}
```

**After (when dryRun: false):**
```
src/
├── helpers.ts    ← Renamed!
└── app.ts        ← Import automatically updated!
```
