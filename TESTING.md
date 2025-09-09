# Testing Guide

This guide explains the optimized testing system for both fast and slow development machines.

## Test Architecture

### Shared Server Pattern
Tests use a shared MCP server instance to preserve TypeScript indexing across test runs:
```bash
TEST_SHARED_SERVER=true bun test
```

### Adaptive Timeouts
System detection automatically adjusts timeouts based on hardware:
- **Fast systems** (8+ CPUs, 8GB+ RAM): 60s timeout
- **Slow systems** (≤4 CPUs, <8GB RAM): 120s timeout

## Running Tests

### Standard Test Commands
```bash
# Quick unit tests
bun run test:fast

# Full integration tests 
bun run test:slow

# All tests for CI
bun run test:ci
```

### Adaptive Test Runner
The smart test runner automatically detects system capabilities:
```bash
node test-runner.cjs
```

Output example:
```
System Detection:
  CPUs: 4
  RAM: 7.8GB  
  Mode: SLOW

Running tests sequentially (slow system mode)...
```

### Manual Configuration
Override automatic detection:
```bash
# Force shared server mode
TEST_SHARED_SERVER=true bun test

# Force slow mode with extended timeouts
TEST_MODE=slow bun test

# Debug test execution
DEBUG_TESTS=1 bun test
```

## Test Optimization Features

### Progressive Warm-up
Tests warm up the TypeScript server by:
1. Opening all test files to trigger indexing
2. Waiting for workspace indexing (5-15s based on system)
3. Running tests with pre-indexed TypeScript context

### Resource Management
- **Sequential execution** on slow systems to prevent resource contention
- **Concurrent limits** based on CPU cores
- **Memory-aware** timeout multipliers

### Error Handling
- EPIPE error recovery with circuit breaker pattern
- Graceful degradation when LSP servers fail
- Automatic retry logic for network timeouts

## Troubleshooting

### Common Issues

**Tests timeout after 20-60s**
- Use `TEST_SHARED_SERVER=true` to preserve TypeScript indexing
- Run `node test-runner.cjs` for automatic system detection

**EPIPE errors during tests**
- Already handled with circuit breaker pattern
- Tests will retry with exponential backoff

**Out of memory on slow systems**
- Test runner automatically enables sequential mode
- Reduces concurrent LSP server instances

### Debug Output
Enable detailed logging:
```bash
DEBUG_TESTS=1 TEST_SHARED_SERVER=true bun test
```

This shows:
- System capability detection
- LSP server initialization
- TypeScript indexing progress
- Test warm-up timings