# TypeMill Strategic Review

## What TypeMill Is

TypeMill is a Rust MCP server providing 29 tools across 5 categories:
- **Navigation** (8 tools) - LSP bridge for find definition, references, etc.
- **Refactoring** (7 tools) - Rename, extract, inline, move, delete with dry-run
- **Analysis** (9 tools) - Dead code, quality, dependencies, cycles
- **Workspace** (4 tools) - Package creation, workspace management (part of refactoring workflow)
- **System** (1 tool) - Health check

---

## The Competitive Landscape

### MCP→LSP Bridges Already Exist

| Project | Status |
|---------|--------|
| [mcp-language-server](https://github.com/isaacphi/mcp-language-server) | Active, multi-language |
| [mcpls](https://github.com/bug-ops/mcpls) | Universal LSP bridge |
| [Serena](https://github.com/oraios/serena) | 30+ languages, symbol-level ops |
| [lsp-mcp](https://github.com/jonrad/lsp-mcp) | Active |

TypeMill's core value prop (MCP→LSP) is **not unique**.

### Analysis Tools Have Better Alternatives

| Analysis Type | Better Tool |
|---------------|-------------|
| JS/TS dead code | [Knip](https://knip.dev) - 100+ plugins, auto-fix |
| Rust dead code | `cargo clippy`, rust-analyzer |
| Python dead code | [vulture](https://github.com/jendrikseipp/vulture) |
| Code quality | ESLint, clippy, pylint |
| Dependencies | Madge, cargo-deps |

### IDE Refactoring via CLI

- [IdeaLS](https://github.com/SuduIDE/ideals) - IntelliJ as LSP server (community, not JetBrains)
- [Qodana](https://www.jetbrains.com/qodana/) - JetBrains analysis CLI (no refactoring)

---

## What Makes Serena Different

Serena uses **symbol-level** operations vs TypeMill's **file+line** approach:

| Serena | TypeMill |
|--------|----------|
| `find_symbol("processOrder")` | `find_definition(file, line, col)` |
| `insert_after_symbol("foo")` | `edit(file, line, content)` |
| Stable across edits | Breaks if lines shift |
| 30+ languages | ~8 languages |
| LSP or JetBrains backend | LSP only |

**But:** Serena lacks full refactoring:
- ❌ File/directory rename with import updates
- ❌ Extract function/variable
- ❌ Inline
- ❌ Move symbol between files
- ❌ Dry-run preview

---

## TypeMill's Actual Unique Value

### Strong (Keep & Improve)

| Feature | Why Unique |
|---------|------------|
| **Full refactoring suite** | Extract, inline, move - Serena doesn't have these |
| **File/directory rename** | Auto-updates all imports across languages |
| **Dry-run preview** | AI-safe - preview before executing |
| **Workspace operations** | Create packages, update workspace members (part of refactoring) |

### Weak (Consider Replacing)

| Feature | Better Approach |
|---------|-----------------|
| Dead code analysis | Call `clippy`, `knip`, `vulture` |
| Quality analysis | Call native linters |
| Dependency analysis | Call `madge`, `cargo-deps` |
| Navigation | Serena/others do this too |

---

## Key Insights

### 1. Analysis Should Orchestrate, Not Reimplement

Instead of custom dead code analysis:
```rust
// Call native tools, parse output, unified format
match lang {
    "rust" => run_clippy(),
    "typescript" => run_knip(),
    "python" => run_vulture(),
}
```

Benefits: Better accuracy, less maintenance, user trust.

### 2. Symbol-Level > File+Line for AI

Symbol-level operations are more robust:
- Don't break when code shifts
- Match how humans think
- Less context needed

TypeMill should add: `find_symbol()`, `insert_after_symbol()`, etc.

### 3. Workspace Tools = Refactoring Infrastructure

`create_package` and `update_members` aren't utilities - they're part of the refactoring workflow for splitting modules into crates.

### 4. Language Coverage Is Just LSP Configs

Serena's 30+ languages = more LSP server configurations. Not magic. TypeMill could add more.

---

## Strategic Recommendations

### Focus: Refactoring Excellence

TypeMill's defensible position is **comprehensive, AI-safe refactoring**:

| Operation | TypeMill | Serena | Others |
|-----------|----------|--------|--------|
| Rename symbol | ✅ | ✅ | ✅ |
| Rename file + update imports | ✅ | ❌ | ❌ |
| Extract function | ✅ | ❌ | ❌ |
| Inline | ✅ | ❌ | ❌ |
| Move symbol | ✅ | ❌ | ❌ |
| Dry-run preview | ✅ | ❌ | ❌ |
| Crate extraction | ✅ | ❌ | ❌ |

### Deprioritize: Custom Analysis

Let native tools do analysis. TypeMill provides:
1. Unified interface to call them
2. Consistent output format
3. Fallback if tools not installed

### Add: Symbol-Level Operations

Learn from Serena:
- `find_symbol(name)` → location
- `insert_after_symbol(name, code)`
- `replace_symbol_body(name, code)`

### Add: More Language Servers

Just configuration - add LSP configs for Java, C#, Go, etc.

---

## The Honest Pitch

> **TypeMill** is the most complete AI-safe refactoring toolkit for code agents. While other tools provide basic navigation and symbol operations, TypeMill offers **extract, inline, move, and file/directory rename with automatic import updates** - all with dry-run preview. For analysis, TypeMill orchestrates best-in-class native tools (clippy, knip, vulture) through a unified interface.

---

## Open Questions

1. Should TypeMill compete with Serena on navigation, or cede that ground?
2. Should analysis tools be removed entirely, or become thin wrappers around native tools?
3. Is symbol-level API worth the implementation effort?
4. Should TypeMill support JetBrains as an alternative backend (like Serena)?

---

## Background Research

### How Dead Code Analysis Works Elsewhere

| Approach | Used By | How It Works |
|----------|---------|--------------|
| **Mark-and-sweep** | Knip, tree shaking | Start from entry points, mark reachable, sweep unreachable |
| **Compiler integration** | rust-analyzer, clippy | Full type info from compiler |
| **Static + Dynamic** | Meta's SCARF | Compiler graphs + production logs |
| **Import/export graph** | ts-prune, Madge | Track exports with no matching imports |

Meta's SCARF (internal) deleted 100M+ lines over 5 years using static + dynamic analysis. Not open source.

### Tree-sitter vs Tree Shaking

- **Tree-sitter**: Parser generator library for building syntax trees (used by editors)
- **Tree shaking**: Dead code elimination technique in bundlers

Tree-sitter could unify AST parsing across languages but doesn't do analysis itself.

---

## References

- [Serena](https://github.com/oraios/serena) - Symbol-level AI coding toolkit
- [Knip](https://knip.dev) - JS/TS dead code + unused deps
- [mcp-language-server](https://github.com/isaacphi/mcp-language-server) - MCP→LSP bridge
- [mcpls](https://github.com/bug-ops/mcpls) - Universal MCP→LSP bridge
- [Meta SCARF](https://engineering.fb.com/2023/10/24/data-infrastructure/automating-dead-code-cleanup/) - Dead code at scale
- [IdeaLS](https://github.com/SuduIDE/ideals) - IntelliJ as LSP server
