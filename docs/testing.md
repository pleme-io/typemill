# Testing Guide

Fast reference for test organization, execution, and best practices.

## Test Pyramid (4 Layers)

| Layer | Location | Purpose | Speed | Count | Feature Flag |
|-------|----------|---------|-------|-------|--------------|
| **Unit** | `crates/*/src/` | Individual functions, business logic | ⚡ <100ms | 100s | default |
| **Integration** | `tests/e2e/src/` | Tool handlers with mocks | ⚡ <5s | 81 | default |
| **E2E** | `apps/codebuddy/tests/e2e_*.rs` | Complete workflows | 🐌 <30s | 58 | default |
| **Smoke** | `apps/codebuddy/tests/smoke/` | Protocol connectivity | 🐌 <1min | 5 | `#[ignore]` |

**Total:** 244+ tests across 93+ files

## When to Use Each Layer

| Test Type | Use When | Don't Use When |
|-----------|----------|----------------|
| **Unit** | Testing pure functions, data transformations, algorithms | Testing with external dependencies |
| **Integration** | Testing tool handlers, service APIs, parameter validation | Testing individual functions or workflows |
| **E2E** | Testing critical multi-step workflows, error recovery | Testing simple functions or single tools |
| **Smoke** | Adding NEW protocol layer (WebSocket, gRPC) | Testing individual features (use integration) |

## Running Tests

| Command | Purpose | Time |
|---------|---------|------|
| `cargo nextest run --workspace` | Fast tests (unit + integration) | ~10s |
| `cargo nextest run --workspace --features lsp-tests` | + LSP server tests | ~60s |
| `cargo nextest run --workspace --all-features` | + Heavy tests (benchmarks) | ~80s |
| `cargo nextest run --workspace --ignored --features lsp-tests` | Smoke tests only | ~1min |
| `cargo nextest run -p codebuddy --test e2e_*` | E2E tests only | ~30s |
| `cargo nextest run --workspace --no-capture` | With test output | varies |

## Test Coverage by Category

### LSP Features (Data-Driven)

| Feature | Languages | Mock Tests | Real Tests |
|---------|-----------|------------|------------|
| Go to Definition | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Find References | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Hover | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Document Symbols | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Workspace Symbols | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Completion | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Rename | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |

### E2E Features

| Category | Tests | Coverage |
|----------|-------|----------|
| Analysis Features | 9 | analyze.dead_code, analyze.quality, find_complexity_hotspots |
| Workflow Execution | 10 | simple, complex, failure, dry-run, rollback, batch |
| File Operations | 6 | create, read, write, delete, rename, list |
| Refactoring | 8 | Cross-language, imports, symbols |
| Workspace Operations | 7 | Directory rename, consolidation, dependencies |
| Error Scenarios | 5 | Resilience, recovery, validation |
| Performance | 3 | Load testing, stress testing |
| Server Lifecycle | 10 | LSP management, restart, crash recovery |

## Data-Driven Test Architecture

### Three-Layer Pattern

| Layer | File | Purpose | Languages |
|-------|------|---------|-----------|
| **Fixtures** | `../crates/mill-test-support/src/harness/test_fixtures.rs` | Test data (code snippets, expected results) | TS, Py, Go, Rust |
| **Runners** | `apps/codebuddy/tests/lsp_feature_runners.rs` | Generic test logic (language-agnostic) | All |
| **Declarations** | `apps/codebuddy/tests/lsp_features.rs` | Test matrix (connects fixtures + runners) | All |

### Adding a New Language

| Step | Action | File | Lines Changed |
|------|--------|------|---------------|
| 1 | Add test case data | `test_fixtures.rs` | +10 lines |
| 2 | Run tests | - | - |

**That's it!** Test infrastructure automatically:
- Runs new cases for the new language
- Uses same logic as existing languages
- Generates mock + real LSP tests

## Feature Flags

| Flag | Purpose | Requirements | Enabled By |
|------|---------|--------------|------------|
| `fast-tests` | Unit + integration (mocks) | None | Default |
| `lsp-tests` | Real LSP server tests | LSP servers in PATH | `--features lsp-tests` |
| `e2e-tests` | End-to-end workflows | None | Default |
| `heavy-tests` | Benchmarks, property tests | None (slow) | `--features heavy-tests` |
| `all-features` | Complete suite | LSP servers | `--all-features` |

## Test Infrastructure

### Binary Architecture

| Binary | Location | PID Lock | Parallel Tests | Used By |
|--------|----------|----------|----------------|---------|
| `codebuddy` | `apps/codebuddy` | ✅ `/tmp/codebuddy.pid` | ❌ Conflicts | CLI, users |
| `mill-server` | `../crates/mill-server` | ❌ No lock | ✅ Isolated instances | TestClient, CI |

**Important:** TestClient uses `mill-server` (not `codebuddy`) to enable parallel test execution.

### Test Helpers

| Helper | Purpose | Example Usage |
|--------|---------|---------------|
| `TestWorkspace` | Isolated temp workspace | `TestWorkspace::new()` |
| `TestClient` | MCP tool invocation | `client.call_tool("rename.plan", args)` |
| `test_fixtures.rs` | Language-specific test data | See data-driven architecture |

## Test Organization

```
workspace/
├── crates/*/src/           # Unit tests (inline #[test])
├── tests/
│   ├── src/harness/        # Test infrastructure
│   │   ├── test_fixtures.rs      # Language data
│   │   ├── test_helpers.rs       # Helper functions
│   │   └── test_builder.rs       # Workspace builder
│   └── tests/              # Integration tests
│       ├── lsp_features.rs       # Data-driven LSP tests
│       └── lsp_feature_runners.rs # Test runners
└── apps/codebuddy/tests/   # E2E and smoke tests
    ├── e2e_*.rs            # E2E workflows
    └── smoke/              # Protocol tests (#[ignore])
```

## Best Practices

### ✅ DO

| Practice | Rationale |
|----------|-----------|
| Follow test pyramid | Many unit tests, fewer integration, fewer E2E, minimal smoke |
| Test behavior, not implementation | Tests survive refactoring |
| Make tests fast | Fast feedback = happy developers |
| Use descriptive names | `test_rename_updates_all_references` not `test_1` |
| Test error cases | Happy path + errors = robust code |
| Keep tests isolated | Tests run in any order |
| Use test helpers | `TestWorkspace`, `TestClient` = cleaner tests |

### ❌ DON'T

| Anti-Pattern | Why Not | Alternative |
|-------------|---------|-------------|
| Test same logic twice | Wastes time, maintenance burden | Test once in lowest layer |
| Test implementation details | Breaks on refactoring | Test public API only |
| Tests depend on each other | Breaks parallel execution | Isolate tests |
| Use sleeps | Flaky tests | Use proper synchronization |
| Skip test layers | Wrong layer = slow tests | Use decision tree |
| Test third-party code | Not your responsibility | Trust LSP servers work |
| Add smoke tests for features | Wrong layer | Use integration tests |

## Timing Expectations

| Layer | Expected | Acceptable Max | Acceptable Timeout |
|-------|----------|----------------|-------------------|
| Unit | <100ms | 500ms | 1s |
| Integration | <5s | 30s | 1min |
| E2E | <30s | 2min | 5min |
| Smoke | <1min | 5min | 10min |

**Total suite:** ~2-3 minutes (without smoke tests)

## CI/CD Test Matrix

| Stage | Trigger | Tests | Duration |
|-------|---------|-------|----------|
| **Fast** | Every commit | Unit + Integration + Lint | ~30s |
| **Full** | PRs | Fast + E2E + Heavy | ~3min |
| **Smoke** | Manual | Protocol connectivity | ~5min |

## Troubleshooting

| Problem | Cause | Solution |
|---------|-------|----------|
| Tests timeout | Heavy tests running | Use `cargo nextest run --workspace` (no `--all-features`) |
| LSP tests fail | LSP servers not installed | Install: `npm i -g typescript-language-server`, `rustup component add rust-analyzer` |
| LSP servers not found | Not in PATH | Check: `which typescript-language-server` |
| Smoke tests skip | Marked `#[ignore]` | Run: `cargo nextest run --ignored --features lsp-tests` |
| Server already running | Using `codebuddy` not `mill-server` | TestClient should use `mill-server` |
| Tests fail intermittently | Race conditions | Fix synchronization, avoid sleeps |

## Warning Signs

| Sign | Problem | Action |
|------|---------|--------|
| Test suite >5min | Too many slow tests | Move to E2E or optimize |
| Many ignored tests | Broken or redundant | Fix or remove |
| Tests fail randomly | Race conditions | Fix sync |
| New tests wrong layer | Misunderstanding pyramid | Review this guide |
| Smoke tests test features | Testing wrong thing | Move to integration |

## Examples

See `examples/tests/` for code examples:
- Unit test patterns
- Integration test patterns
- E2E workflow patterns
- Data-driven test fixtures

## Next Steps to Expand Coverage

| Task | Effort | Impact |
|------|--------|--------|
| Add Java to LSP fixtures | Low | High (new language) |
| Add edge cases | Medium | Medium (robustness) |
| Add property-based tests | Medium | High (fuzzing) |
| Add performance benchmarks | Medium | Medium (regression detection) |
| Add workflow scenarios | High | High (user workflows) |

## Key References

- [Test Pyramid Pattern](https://martinfowler.com/articles/practical-test-pyramid.html)
- [contributing.md](../contributing.md) - Development setup
- [examples/tests/](../examples/tests/) - Test code examples
- [docs/archive/testing_guide-verbose.md](archive/testing_guide-verbose.md) - Full guide with explanations
