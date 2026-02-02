# TypeMill

<div align="center">

**Pure Rust MCP server bridging Language Server Protocol functionality to AI coding assistants**

[![npm version](https://img.shields.io/npm/v/@goobits/typemill)](https://www.npmjs.com/package/@goobits/typemill)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

[Quick Start](#quick-start) • [Features](#features) • [Tools](#tools) • [Documentation](docs/)

![TypeMill Demo](demo/demo.svg)

</div>

---

## Quick Start

```bash
npx @goobits/typemill start
```

That's it. No installation required.

---

## What Can TypeMill Do?

### 🔄 Rename Across Your Entire Codebase

```
Before:                              After:
├── src/                             ├── src/
│   ├── utils.ts    ─────────────►   │   ├── helpers.ts
│   ├── app.ts                       │   ├── app.ts
│   │   import { foo } from './utils'│   │   import { foo } from './helpers'  ✓ Updated!
│   └── index.ts                     │   └── index.ts
│       import './utils'             │       import './helpers'               ✓ Updated!
```

### 📁 Move Files with Automatic Import Updates

```
Before:                              After:
├── src/                             ├── src/
│   ├── components/                  │   ├── components/
│   │   └── Button.tsx               │   │   └── ui/
│   └── App.tsx                      │   │       └── Button.tsx   ◄── Moved!
│       import { Button }            │   └── App.tsx
│         from './components/Button' │       import { Button }
│                                    │         from './components/ui/Button'  ✓ Fixed!
```

### 🔍 Understand Code Instantly

```
> inspect_code("src/server.ts", line=42, character=15)

{
  "definition": "src/types.ts:18",
  "type": "interface ServerConfig { port: number; host: string; }",
  "references": [
    "src/server.ts:42",
    "src/server.ts:67",
    "src/config.ts:12"
  ]
}
```

### 🔧 Extract, Inline, Transform

```typescript
// Before: Messy inline code
const result = items
  .filter(x => x.active)
  .map(x => x.value * 2)
  .reduce((a, b) => a + b, 0);

// After: refactor action="extract" kind="function" name="calculateActiveTotal"
function calculateActiveTotal(items: Item[]): number {
  return items
    .filter(x => x.active)
    .map(x => x.value * 2)
    .reduce((a, b) => a + b, 0);
}
const result = calculateActiveTotal(items);
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         AI Assistant                            │
│                  (Claude Code / Claude Desktop)                 │
└─────────────────────────────┬───────────────────────────────────┘
                              │ MCP Protocol (stdio/WebSocket)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                          TypeMill                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ inspect_code│  │ rename_all  │  │ workspace               │  │
│  │ search_code │  │ relocate    │  │ • find_replace          │  │
│  │             │  │ prune       │  │ • extract_dependencies  │  │
│  │             │  │ refactor    │  │ • verify                │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────┬───────────────────────────────────┘
                              │ Language Server Protocol
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
   │ typescript- │     │   rust-     │     │    pylsp    │
   │ language-   │     │  analyzer   │     │             │
   │   server    │     │             │     │             │
   └─────────────┘     └─────────────┘     └─────────────┘
        .ts .js             .rs                 .py
```

---

## Tools

| Tool | Description |
|------|-------------|
| `inspect_code` | Get definition, references, type info, diagnostics at a position |
| `search_code` | Search workspace symbols with fuzzy matching |
| `rename_all` | Rename symbols, files, or directories (updates all imports) |
| `relocate` | Move symbols, files, or directories |
| `prune` | Delete with cleanup (removes unused imports) |
| `refactor` | Extract functions, inline variables, reorder code |
| `workspace` | Find/replace, dependency extraction, project verification |

All refactoring tools support **dry-run mode** (default) for safe previews.

---

## Installation

### Option 1: npx (Recommended)
```bash
npx @goobits/typemill start
```

### Option 2: Global Install
```bash
npm install -g @goobits/typemill
typemill start
```

### Option 3: Build from Source
```bash
git clone https://github.com/goobits/typemill
cd typemill
cargo build --release
./target/release/mill start
```

---

## Configuration

Add to Claude Desktop (`~/.config/claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "typemill": {
      "command": "npx",
      "args": ["@goobits/typemill", "start"]
    }
  }
}
```

---

## Supported Languages

| Language | LSP Server | Extensions |
|----------|------------|------------|
| TypeScript/JavaScript | typescript-language-server | `.ts` `.tsx` `.js` `.jsx` |
| Rust | rust-analyzer | `.rs` |
| Python | pylsp | `.py` |

---

## License

MIT

