# CCLSP Test Suite

This directory contains the organized test suite for CCLSP's MCP (Model Context Protocol) server implementation.

## 📁 Test Structure

```
tests/
├── core/                 # Core functionality tests
│   ├── quick.test.ts     # Quick validation (5 tools, ~10s)
│   ├── comprehensive.test.ts   # All 23 tools test (~60s)
│   ├── intelligence.test.ts    # Intelligence features focus
│   └── playground.test.ts      # Playground validation
├── unit/                 # Unit and integration tests
│   ├── handlers.test.ts   # Direct handler testing
│   ├── lsp-client.test.ts # LSP client integration
│   └── restart-server.test.ts  # Server restart timing
└── helpers/
    └── mcp-test-client.ts  # Shared MCP testing utilities
```

## 🚀 Running Tests

### Quick Validation
```bash
npm run test:mcp:quick      # Run 5 core tools (fastest)
npm run test:mcp            # Alias for quick test
```

### Comprehensive Testing
```bash
npm run test:mcp:full       # Test all 23 MCP tools
npm run test:mcp:intelligence  # Test 5 intelligence features
npm run test:mcp:playground    # Validate playground functionality
```

### Unit Testing
```bash
npm run test:mcp:unit       # Run handler and client tests
npm run test:mcp:restart    # Test server restart functionality
```

## 📊 Test Coverage

### Core Tests (`/core`)

#### `quick.test.ts` ⚡
- **Purpose**: Fast validation of core functionality
- **Coverage**: 5 essential tools
- **Duration**: ~10 seconds
- **Tools tested**:
  - `find_definition` - Navigate to definitions
  - `find_references` - Find all references
  - `get_diagnostics` - TypeScript errors/warnings
  - `get_hover` - Type information on hover
  - `rename_symbol` - Refactor across codebase

#### `comprehensive.test.ts` 🔬
- **Purpose**: Complete validation of all MCP tools
- **Coverage**: All 23 MCP tools
- **Duration**: ~60 seconds
- **Categories tested**:
  - Core Tools (4/4)
  - Document Tools (7/7)
  - Intelligence Tools (5/5)
  - Hierarchy Tools (3/3)
  - File Operations (3/3)
  - Server Management (1/1)

#### `intelligence.test.ts` 🧠
- **Purpose**: Validate TypeScript intelligence features
- **Coverage**: 5 intelligence tools
- **Duration**: ~20 seconds
- **Tools tested**:
  - `get_hover` - Type information
  - `get_completions` - Code suggestions
  - `get_signature_help` - Function signatures
  - `get_inlay_hints` - Parameter hints
  - `get_semantic_tokens` - Syntax highlighting

#### `playground.test.ts` 🎮
- **Purpose**: Validate playground test environment
- **Coverage**: Key playground features
- **Duration**: ~15 seconds
- **Validates**:
  - Diagnostics on test files
  - Hover information
  - Symbol references
  - Document outline

### Unit Tests (`/unit`)

#### `handlers.test.ts` 🔧
- **Purpose**: Direct MCP handler testing
- **Type**: Unit test (bypasses MCP protocol)
- **Tests**: Handler functions directly
- **Coverage**: File operations, workspace edits, folding

#### `lsp-client.test.ts` 🔗
- **Purpose**: LSP client integration testing
- **Type**: Integration test
- **Tests**: Direct LSP client functionality
- **Coverage**: Folding ranges, document links, symbols

#### `restart-server.test.ts` 🔄
- **Purpose**: Server restart functionality and timing
- **Type**: Specialized test
- **Tests**: Server restart with detailed timing
- **Duration**: Measures restart performance (~700ms typical)

## 🎯 Test Philosophy

### Test Categories

1. **Core Tests** - End-to-end MCP protocol testing
   - Tests through full MCP server
   - Validates actual tool responses
   - Ensures protocol compliance

2. **Unit Tests** - Component-level testing
   - Direct function testing
   - Faster execution
   - Easier debugging

### Coverage Goals

- ✅ **100% Tool Coverage**: All 23 MCP tools tested
- ✅ **Real Data Validation**: Tests verify actual TypeScript LSP data
- ✅ **Performance Monitoring**: Timing tracked for all operations
- ✅ **Error Handling**: Tests include error scenarios

## 📈 Expected Results

### Success Criteria

| Test | Expected Result |
|------|-----------------|
| `quick.test.ts` | 5/5 passed |
| `comprehensive.test.ts` | 22-23/23 passed* |
| `intelligence.test.ts` | 5/5 with real TS data |
| `playground.test.ts` | All features operational |
| `handlers.test.ts` | All handlers functional |
| `lsp-client.test.ts` | Client operations working |
| `restart-server.test.ts` | ~700ms restart time |

*Note: `restart_server` may timeout in comprehensive test due to sequencing but works individually.

## 🔧 Troubleshooting

### Common Issues

1. **"No LSP server available"**
   - Ensure TypeScript Language Server is installed: `npx typescript-language-server --version`
   - Check config file exists: `/workspace/plugins/cclsp/cclsp.json`

2. **Test timeouts**
   - Individual tests have different timeout settings
   - Intelligence features may take 2-4s (normal)
   - Run individual tests for debugging

3. **Build issues**
   - Ensure project is built: `npx esbuild index.ts --bundle --platform=node --target=node16 --format=esm --outdir=dist`
   - Or use bun: `bun run build`

## 🏗️ Test Maintenance

### Adding New Tests
1. Determine if it's a core or unit test
2. Follow existing naming convention: `[feature].test.ts`
3. Update this README with test details
4. Add npm script to package.json if needed

### Test Data Location
- Playground files: `/workspace/plugins/cclsp/playground/src/`
- Config: `/workspace/plugins/cclsp/cclsp.json`
- Test fixtures: `/workspace/plugins/cclsp/tests/fixtures/`

## 📝 Historical Note

This test suite was reorganized in December 2024, consolidating 33+ scattered test files into 7 well-organized, purposeful tests. The reorganization removed:
- 15 debug artifacts from troubleshooting sessions
- 11 duplicate or superseded tests
- Various experimental and one-off test scripts

The current structure represents the minimal, complete test coverage needed for CCLSP validation.