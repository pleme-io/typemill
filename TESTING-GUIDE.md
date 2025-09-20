# 🧪 Testing Guide for CodeFlow-Buddy

## 🚀 Quick Start

```bash
# Clone and setup
git clone <repo>
cd codeflow-buddy
bun install
bun run build

# Run tests
bun test:fast
```

That's it! No setup script needed. The system:
- Uses smart defaults when no config exists
- Auto-installs language servers via npx when needed
- Tests work immediately

## 🎯 Test Commands

```bash
bun test              # Unit tests
bun test:fast         # Quick test suite (recommended)
bun test:minimal      # For slower systems
bun test:all          # Everything
```

## 📊 How It Works

1. **`bun install`** - Installs all npm dependencies (including TypeScript language server)
2. **`bun run build`** - Builds the TypeScript project to dist/
3. **Tests run** - System auto-configures with sensible defaults

No config needed! The system uses `npx` to run language servers, so they're installed on-demand from your node_modules.

## 🔧 Troubleshooting

### Tests fail on first run
Run again - LSP servers need a moment to warm up.

### "Command not found: bun"
Install Bun: `curl -fsSL https://bun.sh/install | bash`

### Custom configuration needed?
Create `.codebuddy/config.json` or run `node dist/index.js init`

But honestly, you probably don't need it - the defaults work great!

## 💡 Why So Simple?

- **Bun** handles all package management
- **NPX** auto-runs language servers from node_modules
- **Smart defaults** mean no config needed
- **TypeScript** is the only language in this codebase

No complex setup scripts, no global installs, no manual configuration. Just `bun install`, `bun run build`, and go!