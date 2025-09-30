# CodeBuddy Examples

This directory contains test fixtures and sample code for demonstrating and testing CodeBuddy's LSP functionality across multiple programming languages.

## Directory Structure

### [`backend/`](./backend) - Python Backend Examples
**Purpose:** FastAPI application for testing Python LSP features

**Contents:**
- `main.py` - FastAPI REST API with user management, CRUD operations
- `requirements.txt` - Python dependencies (FastAPI, uvicorn, pydantic)

**Use Cases:**
- Testing Python symbol navigation (`find_definition`, `find_references`)
- Function refactoring and renaming
- Hover information and type inference
- Code completion testing

**Active:** ✅ Used in Python LSP integration tests

---

### [`database/`](./database) - SQL Schema Examples
**Purpose:** PostgreSQL database schemas for reference

**Contents:**
- `init.sql` - Database schema with users, projects, and relationships

**Use Cases:**
- SQL schema reference
- Future SQL LSP integration (if implemented)

**Active:** ⚠️ Reference only - not currently used in tests

---

### [`frontend/`](./frontend) - TypeScript Frontend Examples
**Purpose:** React TypeScript code for testing TS/JS LSP features

**Contents:**
- `src/index.ts` - React components, User interfaces, API client functions
- `package.json` - Node.js dependencies

**Use Cases:**
- TypeScript symbol navigation and renaming
- Interface and type definition testing
- Import statement refactoring
- Component refactoring scenarios

**Active:** ✅ Used in TypeScript LSP integration tests

---

### [`playground/`](./playground) - Multi-Language Test Workspace
**Purpose:** Comprehensive testing environment with diverse scenarios

**Contents:**
- `atomic-refactoring-test/` - Tests for atomic refactoring operations (rename with import updates)
- `python/` - Python examples with math utilities, data processors, helpers
- `rust/` - Rust examples with DataProcessor, utilities, library structure
- `src/` - TypeScript test files covering errors, utilities, components
- `test-workspace-symbols/` - Workspace-wide symbol search testing

**Use Cases:**
- Integration testing across multiple file types
- Cross-file refactoring validation
- LSP feature compatibility testing
- Workspace symbol search validation
- Import statement update testing

**Active:** ✅ Core testing infrastructure - heavily used in integration tests

---

### [`tenant-client.ts`](./tenant-client.ts) - MCP WebSocket Client Example
**Purpose:** Modern TypeScript client example for CodeBuddy's MCP WebSocket API

**Contents:**
- `CodeBuddyMcpClient` class with full MCP protocol support
- TypeScript interfaces matching Rust MCP types
- Helper methods for common LSP operations:
  - `findDefinition()` - Find symbol definitions
  - `findReferences()` - Find symbol references
  - `renameSymbol()` - Rename symbols with dry-run support
  - `getHover()` - Get hover information
  - `getDocumentSymbols()` - Get document outline
  - `searchWorkspaceSymbols()` - Search symbols across workspace
  - `formatDocument()` - Format code
  - `getDiagnostics()` - Get errors/warnings
- Complete usage examples demonstrating real-world scenarios

**Use Cases:**
- Building custom CodeBuddy clients
- Integration with external tools
- Testing WebSocket API
- Reference implementation for other languages

**Status:** ✅ **Active** - Up-to-date with current MCP protocol (2025-06-18)

---

## Usage

### Running Example Tests

The examples are primarily used as fixtures for integration tests:

```bash
# Run all tests (includes example-based tests)
cargo test

# Run specific LSP integration tests
cargo test --test e2e_refactoring

# Run tests with output
cargo test -- --nocapture
```

### Adding New Examples

When adding new example files:

1. **Place in appropriate directory** based on language
2. **Include realistic code patterns** that test LSP features
3. **Add documentation** explaining what LSP features the example tests
4. **Update this README** with the new example's purpose

### Example Requirements

Good test examples should:

- ✅ Use realistic code patterns (not trivial "hello world")
- ✅ Include multiple functions/classes for cross-reference testing
- ✅ Have clear symbol names for easy test validation
- ✅ Include imports/dependencies for testing reference resolution
- ✅ Contain some intentional patterns for refactoring (e.g., renameable symbols)

---

## Language Support

| Language   | LSP Server      | Example Directory | Status |
|------------|-----------------|-------------------|--------|
| TypeScript | typescript-language-server | `frontend/`, `playground/src/`, `playground/atomic-refactoring-test/` | ✅ Active |
| Python     | pylsp           | `backend/`, `playground/python/` | ✅ Active |
| Rust       | rust-analyzer   | `playground/rust/` | ✅ Active |
| Go         | gopls           | _(not yet added)_ | ⚠️ Planned |

---

## Contributing

When modifying examples:

1. Ensure changes don't break existing integration tests
2. Update this README if adding new directories or major changes
3. Run `cargo test` to verify all tests still pass
4. Document any new testing scenarios the examples enable

---

## Notes

- These examples are **test fixtures**, not production code
- Some examples intentionally contain patterns suitable for refactoring tests
- The `playground/` directory is the most actively used testing infrastructure
- Examples should be kept simple but realistic enough to test real-world LSP scenarios