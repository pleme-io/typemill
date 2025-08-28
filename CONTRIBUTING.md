# Contributing to cclsp

First off, thank you for considering contributing to cclsp! It's people like you that make cclsp such a great tool.

## Code of Conduct

By participating in this project, you are expected to uphold our [Code of Conduct](CODE_OF_CONDUCT.md).

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior  
- Your environment (OS, Node version, etc.)

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. Please describe:

- The enhancement you'd like to see
- Why it would be useful
- Any relevant use cases

### Adding Language Support

One of the most valuable contributions is adding support for new language servers:

1. Find the LSP server for your language
2. Test the configuration locally  
3. Submit a PR with updated examples and documentation

### Your First Code Contribution

Unsure where to begin contributing? You can start by looking through these issues:

- Issues labeled with `good first issue` - these should be relatively simple to implement
- Issues labeled with `help wanted` - these are often more involved but are areas where we need help

## Development Process

### Prerequisites

- Node.js 18+ or Bun runtime
- Git
- Your favorite code editor

### Setting Up Your Development Environment

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/your-username/cclsp.git
   cd cclsp
   ```

3. Install dependencies:
   ```bash
   bun install
   ```

4. Create a branch for your feature or fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

### Development Workflow

1. Make your changes
2. Add or update tests as needed
3. Run the test suite:
   ```bash
   bun test
   ```

4. Run linting and formatting:
   ```bash
   bun run lint
   bun run format
   bun run typecheck
   ```

5. Test your changes manually:
   ```bash
   bun run dev
   ```

### Testing New Language Servers

When adding support for a new language server:

1. Install the language server locally
2. Add configuration to `cclsp.json`
3. Create test files in the target language
4. Test all three main features:
   - Go to definition
   - Find references
   - Rename symbol

### Commit Messages

We use conventional commits with gitmoji for better readability:

- ✨ `:sparkles:` feat: New feature
- 🐛 `:bug:` fix: Bug fix
- 📚 `:books:` docs: Documentation changes
- ♻️ `:recycle:` refactor: Code refactoring
- ✅ `:white_check_mark:` test: Adding tests
- 🎨 `:art:` style: Code style changes
- ⚡ `:zap:` perf: Performance improvements

Example:
```
✨ feat: add support for Ruby language server

- Add configuration for solargraph
- Test go to definition and find references
- Update README with Ruby examples
```

### Pull Request Process

1. Ensure all tests pass and there are no linting errors
2. Update the README.md with details of changes if applicable
3. Add yourself to the contributors list if this is your first contribution
4. Create a Pull Request with a clear title and description
5. Link any related issues

### Code Review Process

- All submissions require review from at least one maintainer
- We may suggest changes or improvements
- Please be patient as reviews may take time
- Once approved, a maintainer will merge your PR

## Project Structure

```
cclsp/
├── src/
│   ├── index.ts          # MCP server entry point
│   ├── lsp-client.ts     # LSP client implementation
│   └── *.test.ts         # Test files
├── dist/                 # Compiled output (gitignored)
├── .github/              # GitHub specific files
├── package.json          # Package configuration
└── tsconfig.json         # TypeScript configuration
```

## Style Guide

### TypeScript Style

- Use TypeScript strict mode
- Prefer `const` over `let`
- Use functional programming patterns where appropriate
- Add types to all function parameters and return values
- Use meaningful variable and function names

### Code Organization

- Keep files focused on a single responsibility
- Export types and interfaces separately
- Group related functionality together
- Add JSDoc comments for public APIs

## Recognition

Contributors will be recognized in the following ways:

- Added to the contributors section in README
- Mentioned in release notes for significant contributions
- Given credit in commit messages when their ideas are implemented

## Questions?

Feel free to open an issue with the `question` label or start a discussion in [GitHub Discussions](https://github.com/ktnyt/cclsp/discussions).

Thank you for contributing! 🎉