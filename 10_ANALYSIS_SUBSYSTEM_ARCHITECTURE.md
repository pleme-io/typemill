# Analysis Subsystem Architecture

**Status**: Design Proposal - Ready for Implementation
**Date**: 2025-10-09
**Related**: 60_ADVANCED_ANALYSIS_VISION.md

## Problem Statement

### Current Pain Points
- **27 crates in monorepo** → Touching `find_dead_code` triggers full workspace rebuild
- **Long test cycles** → ~80s for full suite, ~60s with LSP tests
- **Tight coupling** → Analysis code in `cb-handlers` depends on entire handler stack
- **Can't iterate fast** → Adding "deep dead code analysis" requires running all integration tests

### Desired State
- **Isolated development** → Work on analysis features with ~2s test cycles
- **Independent testing** → Unit tests without LSP/file I/O/handler dependencies
- **Fast CI** → Run analysis tests separately from LSP integration tests
- **Clear boundaries** → Analysis features don't pollute handler layer

---

## Architecture Design

### Directory Structure

```
workspace/
├── crates/                           # Existing core infrastructure
│   ├── cb-handlers/                  # MCP tool handlers (thin layer)
│   ├── cb-lsp/                       # LSP client management
│   ├── cb-plugin-api/                # Language plugin traits
│   └── ...
│
├── analysis/                         # NEW: Analysis subsystem (isolated)
│   ├── cb-analysis-common/           # Shared analysis infrastructure
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── traits.rs             # AnalysisEngine trait
│   │   │   ├── graph.rs              # Dependency graph builder
│   │   │   ├── cache.rs              # Analysis result caching
│   │   │   ├── config.rs             # Common analysis configuration
│   │   │   └── types.rs              # Shared result types
│   │   ├── tests/
│   │   │   └── graph_tests.rs        # Pure unit tests
│   │   └── Cargo.toml                # Minimal deps: cb-plugin-api, serde
│   │
│   ├── cb-analysis-dead-code/        # Dead code analysis (extracted from handlers)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── detector.rs           # Core algorithm (pure functions)
│   │   │   ├── config.rs             # AnalysisConfig
│   │   │   └── types.rs              # DeadSymbol, result types
│   │   ├── tests/
│   │   │   ├── unit_tests.rs         # Fast unit tests (mocked LSP)
│   │   │   └── fixtures.rs           # Test data
│   │   └── Cargo.toml                # Deps: cb-analysis-common
│   │
│   ├── cb-analysis-circular-deps/    # FUTURE: Circular dependency detection
│   ├── cb-analysis-security/         # FUTURE: Security pattern detection
│   └── cb-analysis-breaking/         # FUTURE: Breaking change impact
│
└── Cargo.toml                        # Workspace config with optional features
```

---

## API Design

### Core Trait: `AnalysisEngine`

```rust
// analysis/cb-analysis-common/src/traits.rs

use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// Abstraction for LSP communication (dependency inversion)
#[async_trait]
pub trait LspProvider: Send + Sync {
    /// Query LSP workspace/symbol
    async fn workspace_symbols(&self, query: &str) -> Result<Vec<Value>, AnalysisError>;

    /// Query LSP textDocument/references
    async fn find_references(&self, uri: &str, line: u32, character: u32)
        -> Result<Vec<Value>, AnalysisError>;

    /// Query LSP textDocument/documentSymbol
    async fn document_symbols(&self, uri: &str) -> Result<Vec<Value>, AnalysisError>;
}

/// Core analysis engine trait
#[async_trait]
pub trait AnalysisEngine: Send + Sync {
    type Config;
    type Result;

    /// Run analysis with the given configuration
    async fn analyze(
        &self,
        lsp: &dyn LspProvider,
        workspace_path: &Path,
        config: Self::Config,
    ) -> Result<Self::Result, AnalysisError>;

    /// Get analysis metadata (name, version, capabilities)
    fn metadata(&self) -> AnalysisMetadata;
}

/// Common error type for analysis operations
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("LSP communication failed: {0}")]
    LspError(String),

    #[error("File system error: {0}")]
    FileSystemError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Analysis timeout after {0}s")]
    Timeout(u64),
}

/// Metadata about an analysis engine
#[derive(Debug, Clone)]
pub struct AnalysisMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub symbol_kinds_supported: Vec<u64>,
}
```

---

### Dead Code Analysis Implementation

```rust
// analysis/cb-analysis-dead-code/src/lib.rs

use cb_analysis_common::{AnalysisEngine, LspProvider, AnalysisError, AnalysisMetadata};
use async_trait::async_trait;

pub struct DeadCodeAnalyzer;

#[async_trait]
impl AnalysisEngine for DeadCodeAnalyzer {
    type Config = DeadCodeConfig;
    type Result = DeadCodeReport;

    async fn analyze(
        &self,
        lsp: &dyn LspProvider,
        workspace_path: &Path,
        config: Self::Config,
    ) -> Result<Self::Result, AnalysisError> {
        // Core algorithm extracted from analysis_handler.rs
        let symbols = collect_workspace_symbols(lsp, &config).await?;
        let dead_symbols = check_references(lsp, symbols, &config).await?;

        Ok(DeadCodeReport {
            workspace_path: workspace_path.to_path_buf(),
            dead_symbols,
            stats: compute_stats(&dead_symbols),
        })
    }

    fn metadata(&self) -> AnalysisMetadata {
        AnalysisMetadata {
            name: "dead-code",
            version: "1.0.0",
            description: "Find unused functions, classes, and variables",
            symbol_kinds_supported: vec![5, 6, 9, 10, 11, 12, 13, 14, 22, 23],
        }
    }
}

/// Configuration for dead code analysis
#[derive(Debug, Clone)]
pub struct DeadCodeConfig {
    pub symbol_kinds: Vec<u64>,
    pub max_concurrency: usize,
    pub min_reference_threshold: usize,
    pub include_exported: bool,
    pub file_types: Option<Vec<String>>,
    pub max_results: Option<usize>,
    pub timeout: Option<std::time::Duration>,
}

impl Default for DeadCodeConfig {
    fn default() -> Self {
        Self {
            symbol_kinds: vec![5, 6, 9, 10, 11, 12, 13, 14, 22, 23],
            max_concurrency: 20,
            min_reference_threshold: 1,
            include_exported: true,
            file_types: None,
            max_results: None,
            timeout: None,
        }
    }
}

/// Result of dead code analysis
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeadCodeReport {
    pub workspace_path: PathBuf,
    pub dead_symbols: Vec<DeadSymbol>,
    pub stats: AnalysisStats,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DeadSymbol {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub reference_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalysisStats {
    pub files_analyzed: usize,
    pub symbols_analyzed: usize,
    pub dead_symbols_found: usize,
    pub duration_ms: u128,
}
```

---

### Handler Integration (Thin Adapter Layer)

```rust
// crates/cb-handlers/src/handlers/analysis_handler.rs

use cb_analysis_dead_code::{DeadCodeAnalyzer, DeadCodeConfig};
use cb_analysis_common::{AnalysisEngine, LspProvider};

pub struct AnalysisHandler;

#[async_trait]
impl ToolHandler for AnalysisHandler {
    async fn handle_tool(&self, tool_call: ToolCall, context: &ToolContext) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "find_dead_code" => self.handle_find_dead_code(tool_call, context).await,
            _ => Err(ServerError::Unsupported(format!("Unknown analysis: {}", tool_call.name))),
        }
    }
}

impl AnalysisHandler {
    async fn handle_find_dead_code(&self, tool_call: ToolCall, context: &ToolContext)
        -> ServerResult<Value>
    {
        // Parse MCP tool call arguments into analysis config
        let config = parse_config_from_tool_call(&tool_call)?;
        let workspace_path = extract_workspace_path(&tool_call)?;

        // Get LSP adapter and wrap it in LspProvider trait
        let lsp_adapter = context.lsp_adapter.lock().await;
        let lsp_provider = DirectLspProviderAdapter::new(lsp_adapter.clone());

        // Run analysis engine
        let analyzer = DeadCodeAnalyzer;
        let report = analyzer.analyze(&lsp_provider, workspace_path, config).await
            .map_err(|e| ServerError::Internal(e.to_string()))?;

        // Convert report to MCP response format
        Ok(format_mcp_response(report))
    }
}

/// Adapter to make DirectLspAdapter compatible with LspProvider trait
struct DirectLspProviderAdapter {
    adapter: Arc<DirectLspAdapter>,
}

#[async_trait]
impl LspProvider for DirectLspProviderAdapter {
    async fn workspace_symbols(&self, query: &str) -> Result<Vec<Value>, AnalysisError> {
        self.adapter
            .request("workspace/symbol", json!({ "query": query }))
            .await
            .map(|v| v.as_array().cloned().unwrap_or_default())
            .map_err(|e| AnalysisError::LspError(e.to_string()))
    }

    async fn find_references(&self, uri: &str, line: u32, character: u32)
        -> Result<Vec<Value>, AnalysisError>
    {
        let params = json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        });

        self.adapter
            .request("textDocument/references", params)
            .await
            .map(|v| v.as_array().cloned().unwrap_or_default())
            .map_err(|e| AnalysisError::LspError(e.to_string()))
    }

    async fn document_symbols(&self, uri: &str) -> Result<Vec<Value>, AnalysisError> {
        self.adapter
            .request("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri } }))
            .await
            .map(|v| v.as_array().cloned().unwrap_or_default())
            .map_err(|e| AnalysisError::LspError(e.to_string()))
    }
}
```

---

## Feature Flags

```toml
# Root Cargo.toml

[workspace]
members = [
    "crates/*",
    "analysis/cb-analysis-common",
    "analysis/cb-analysis-dead-code",
    # Future analysis crates
]

[features]
default = []

# Analysis features (opt-in during development)
analysis-dead-code = ["cb-analysis-dead-code"]
analysis-circular-deps = ["cb-analysis-circular-deps"]
analysis-security = ["cb-analysis-security"]
analysis-all = [
    "analysis-dead-code",
    "analysis-circular-deps",
    "analysis-security"
]

[dependencies]
# Analysis crates are optional
cb-analysis-common = { path = "analysis/cb-analysis-common" }
cb-analysis-dead-code = { path = "analysis/cb-analysis-dead-code", optional = true }
```

```toml
# analysis/cb-analysis-dead-code/Cargo.toml

[package]
name = "cb-analysis-dead-code"
version = "1.0.0-rc3"
edition = "2021"

[dependencies]
# Minimal dependencies (no handler layer, no LSP implementation)
cb-analysis-common = { path = "../cb-analysis-common" }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["sync", "time"] }
thiserror = "2.0"

[dev-dependencies]
mockall = "0.13"        # For mocking LspProvider in tests
tokio-test = "0.4"
tempfile = "3.8"
```

---

## Testing Strategy

### Fast Unit Tests (Analysis Crates)

```bash
# Work on dead code analysis in isolation (~2s)
cd analysis/cb-analysis-dead-code
cargo test

# Example test (no LSP, no file I/O)
#[tokio::test]
async fn test_detects_unused_function() {
    let mock_lsp = MockLspProvider::new()
        .with_workspace_symbols(vec![symbol("unused_fn", 12)])
        .with_references("unused_fn", vec![]); // 0 references

    let analyzer = DeadCodeAnalyzer;
    let config = DeadCodeConfig::default();
    let report = analyzer.analyze(&mock_lsp, Path::new("."), config).await.unwrap();

    assert_eq!(report.dead_symbols.len(), 1);
    assert_eq!(report.dead_symbols[0].name, "unused_fn");
}
```

### Integration Tests (Handler Layer)

```bash
# Test handler integration with mocked LSP (~10s)
cargo test -p cb-handlers --features analysis-dead-code -- dead_code

# Test with real LSP servers (~60s, CI only)
cargo test --workspace --features lsp-tests,analysis-dead-code -- --include-ignored
```

### CI Pipeline

```yaml
# .github/workflows/ci.yml

jobs:
  test-analysis-fast:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test -p cb-analysis-common
      - run: cargo test -p cb-analysis-dead-code
    # ~5s total

  test-handlers:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test -p cb-handlers --features analysis-dead-code
    # ~15s

  test-full:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --workspace --all-features
    # ~80s (only on main branch)
```

---

## Implementation Tasks

### Create Common Infrastructure

```bash
mkdir -p analysis/cb-analysis-common/src
cd analysis/cb-analysis-common

# Create trait definitions
cat > src/traits.rs << 'EOF'
// LspProvider trait, AnalysisEngine trait
EOF

cat > src/lib.rs << 'EOF'
pub mod traits;
pub mod types;
pub mod error;

pub use traits::{AnalysisEngine, LspProvider};
pub use error::AnalysisError;
EOF

# Add to workspace
cd ../..
# Edit Cargo.toml to add analysis/cb-analysis-common
```

### Extract Dead Code Analysis

```bash
mkdir -p analysis/cb-analysis-dead-code/src

# Move core algorithm from cb-handlers/src/handlers/analysis_handler.rs
# to analysis/cb-analysis-dead-code/src/detector.rs

# Create tests with mocked LspProvider
cat > analysis/cb-analysis-dead-code/tests/unit_tests.rs

# Update cb-handlers to use the new crate with feature flag
```

### Add Feature Flag Integration

```rust
// crates/cb-handlers/src/handlers/analysis_handler.rs

#[cfg(feature = "analysis-dead-code")]
use cb_analysis_dead_code::DeadCodeAnalyzer;

impl AnalysisHandler {
    async fn handle_find_dead_code(...) -> ServerResult<Value> {
        #[cfg(feature = "analysis-dead-code")]
        {
            let analyzer = DeadCodeAnalyzer;
            // ... implementation
        }

        #[cfg(not(feature = "analysis-dead-code"))]
        Err(ServerError::Unsupported("Dead code analysis not enabled".into()))
    }
}
```

### Add Deep Dead Code Analysis (NEW FEATURE)

```bash
# Now you can iterate fast!
cd analysis
cargo new cb-analysis-deep-dead-code --lib

# Implement advanced dead code detection
# - Cross-crate unused exports
# - Transitive reference counting
# - Generic/trait usage tracking

# Test in isolation (~2s per test cycle)
cargo test -p cb-analysis-deep-dead-code
```

---

## Benefits Summary

### Development Speed
| Task | Before | After |
|------|--------|-------|
| Add analysis feature | ~80s test cycle | ~2s test cycle |
| Run analysis tests | 80s (full suite) | 5s (isolated) |
| Integration test | 60s (with LSP) | 15s (mocked) |
| CI feedback | 3-5 min | 30s (parallel) |

### Code Quality
- **Clear boundaries** → Analysis logic separate from handler plumbing
- **Testability** → Mock LSP provider, no file I/O in unit tests
- **Reusability** → Analysis engines can be called from CLI, API, or MCP
- **Maintainability** → Each analysis feature is self-contained

### Future Extensibility
- Add circular dependency detection: `cargo new analysis/cb-analysis-circular-deps`
- Add security scanner: `cargo new analysis/cb-analysis-security`
- Add breaking change detector: `cargo new analysis/cb-analysis-breaking`

**Each new feature is isolated and doesn't slow down existing tests.**

---

## Open Questions

1. **Should analysis engines be sync or async?**
   → Async preferred for LSP communication, but could have sync variants

2. **How to handle caching across analysis runs?**
   → Add `cb-analysis-common/src/cache.rs` with shared caching layer

3. **Should we support analysis plugins (dynamic loading)?**
   → Future: Could use `libloading` crate for dynamic analysis engines

4. **What about API-based analysis (for business model)?**
   → Analysis engines are already isolated, easy to wrap in HTTP API

---

## Appendix: Example Usage

### As MCP Tool (Current)
```json
{
  "method": "tools/call",
  "params": {
    "name": "find_dead_code",
    "arguments": {
      "workspace_path": ".",
      "symbol_kinds": ["function", "class"],
      "max_concurrency": 20
    }
  }
}
```

### As Standalone CLI (Future)
```bash
# Direct analysis without MCP overhead
codebuddy analyze dead-code --workspace . --symbol-kinds function,class

# Output JSON for CI integration
codebuddy analyze dead-code --format json > report.json
```

### As Rust API (Future)
```rust
use cb_analysis_dead_code::{DeadCodeAnalyzer, DeadCodeConfig};

let analyzer = DeadCodeAnalyzer;
let config = DeadCodeConfig::default();
let report = analyzer.analyze(&lsp, workspace_path, config).await?;

println!("Found {} dead symbols", report.dead_symbols.len());
```

---

**Key Insight:** The `LspProvider` trait is the abstraction that enables fast testing. By mocking LSP communication, analysis engines can be tested in milliseconds instead of minutes.
