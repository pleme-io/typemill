# 🚀 Codebuddy Quick Reference

This guide is for experienced developers who want to get productive with Codebuddy in under 15 minutes. It assumes you are familiar with AI assistants, LSP, and your command line.

---

## 1. Installation

**Recommended (macOS/Linux):**
```bash
curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash
```

**Alternative (Cargo):**
```bash
cargo install codebuddy --locked
```

---

## 2. Core Commands

**First, configure your project:**
```bash
codebuddy setup    # Auto-detects languages and creates .codebuddy/config.json
```

**Then, manage the server:**
```bash
codebuddy start    # Start the MCP server for your AI assistant
codebuddy status   # Check server status and loaded languages
codebuddy stop     # Stop the server
```

**Execute a tool directly:**
```bash
codebuddy tool find_definition '{"file_path":"src/app.ts","line":10,"character":5}'
```

---

## 3. Configuration (`.codebuddy/config.json`)

`codebuddy setup` handles this for you. For manual tweaking:

```json
{
  "servers": [
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"],
      "restartInterval": 30
    },
    {
      "extensions": ["rs"],
      "command": ["rust-analyzer"]
    }
  ]
}
```
- **`extensions`**: File types this server is responsible for.
- **`command`**: The command to start the LSP server.
- **`restartInterval`**: (Optional) Auto-restart interval in minutes to ensure stability.

---

## 4. Top 10 MCP Tools

These are the most common tools for daily work. See `API_REFERENCE.md` for the full list.

| Tool | Description | Example |
|------|-------------|---------|
| `find_definition` | Go to the definition of a symbol. | `{"file_path":"src/app.ts","line":10,"character":5}` |
| `find_references` | Find all references to a symbol. | `{"file_path":"src/app.ts","line":10,"character":5}` |
| `rename_symbol_strict` | Safely rename a symbol across the project. | `{"file_path":"src/app.ts","line":10,"character":5,"new_name":"MyNewName"}` |
| `get_document_symbols` | Get the hierarchical symbol structure of a file. | `{"file_path":"src/app.ts"}` |
| `search_workspace_symbols`| Search for symbols by name across the workspace. | `{"query":"MyComponent"}` |
| `format_document`| Format a file using the configured LSP server. | `{"file_path":"src/app.ts"}` |
| `organize_imports`| Sort and remove unused imports for a file. | `{"file_path":"src/app.ts"}` |
| `get_diagnostics`| Get all errors and warnings for a file. | `{"file_path":"src/app.ts"}` |
| `rename_file` | Rename a file and automatically update all imports. | `{"old_path":"src/old.ts","new_path":"src/new.ts"}` |
| `rename_directory` | Rename a directory and automatically update all imports. | `{"old_path":"src/components","new_path":"src/ui"}` |

---

## 5. Key Links

- **[API_REFERENCE.md](API_REFERENCE.md)**: The complete, detailed reference for all tools.
- **[CONTRIBUTING.md](CONTRIBUTING.md)**: For developers who want to build from source or contribute.
- **[docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md)**: A deep dive into the system architecture.
