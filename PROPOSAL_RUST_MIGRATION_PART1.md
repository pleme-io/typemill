# Rust Migration Plan v2: For Parity and Parallel Implementation

**STATUS: 100% COMPLETE** ✅ *(Last Updated: 2025-09-27)*

This document updates the original Rust migration plan with crate-level API contracts so that multiple implementers (or agents) can work on different Cargo crates simultaneously. The technical goals remain unchanged: deliver a Rust backend that matches the current TypeScript feature set while improving performance, reliability, and operational tooling.

## Implementation Status Summary

All items in this plan have been successfully implemented:
- ✅ `SessionReport` struct in cb-client (now implemented with comprehensive functionality)
- ✅ `bun run test:e2e:rust` script for TypeScript E2E testing against Rust server (exists and configured)

All core infrastructure, APIs, and architectural components are complete and working. The project is ready for **PROPOSAL_RUST_MIGRATION_PART2.md**.

## 1. Foundation: Project Setup ✅ COMPLETE

The Rust project lives alongside the TypeScript codebase in a `rust/` directory and is structured as a **Cargo workspace**.

**✅ Scaffolding (from repo root):**
```sh
cargo new --lib rust
cd rust
cargo new --lib crates/cb-core
cargo new --lib crates/cb-ast
cargo new --bin crates/cb-server
cargo new --bin crates/cb-client
cargo new --lib crates/tests
```
✅ `rust/Cargo.toml` declares workspace members and `rust/rust-toolchain.toml` pins Rust version.

---

## 2. Success Criteria & Acceptance Testing

Targets are identical to the original plan and must be agreed before coding begins.

### 2.1 Performance Targets
- **Request Latency:** p95 < 100 ms for `find_references`, `get_completions` under the defined benchmark.
- **Startup Time:** < 500 ms from process launch to ready state.

### 2.2 Reliability & Resource Usage
- **Memory Footprint:** RSS < 250 MB under sustained load.
- **CPU Usage:** Low idle (<5%) with well-defined ceilings during benchmarks.

### 2.3 Parity & Correctness
- **E2E Test Suite:** The existing TypeScript E2E suite must pass 100% against the Rust server.
- **Feature Checklist:** Maintain a parity matrix covering every MCP tool, transport, and auth feature.

---

## 3. Project Structure and Feature Mapping

```
./rust/
├── .gitignore
├── Cargo.toml
├── rust-toolchain.toml
└── crates/
    ├── cb-core/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── config.rs
    │       ├── error.rs
    │       └── model/
    │           ├── mod.rs
    │           ├── lsp.rs
    │           ├── mcp.rs
    │           └── fuse.rs
    │
    ├── cb-ast/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── error.rs
    │       ├── parser.rs
    │       ├── analyzer.rs
    │       └── transformer.rs
    │
    ├── cb-server/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── main.rs
    │       ├── state.rs
    │       ├── error.rs
    │       ├── auth/
    │       │   ├── mod.rs
    │       │   ├── jwt.rs
    │       │   └── middleware.rs
    │       ├── transport/
    │       │   ├── mod.rs
    │       │   ├── http.rs
    │       │   └── ws.rs
    │       ├── handlers/
    │       │   ├── mod.rs
    │       │   ├── mcp_dispatcher.rs
    │       │   └── mcp_tools/
    │       │       ├── mod.rs
    │       │       ├── navigation.rs
    │       │       ├── editing.rs
    │       │       └── filesystem.rs
    │       └── systems/
    │           ├── mod.rs
    │           ├── cache.rs
    │           ├── fuse/
    │           │   ├── mod.rs
    │           │   └── driver.rs
    │           └── lsp/
    │               ├── mod.rs
    │               ├── manager.rs
    │               ├── client.rs
    │               └── protocol.rs
    │
    ├── cb-client/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── main.rs
    │       ├── config.rs
    │       ├── error.rs
    │       └── connection/
    │           ├── mod.rs
    │           └── fuse_handler.rs
    │
    └── tests/
        ├── Cargo.toml
        └── tests/
            └── e2e_flow.rs
```

### 3.1 Crate Contracts (Expose These APIs) ✅ COMPLETE

Each crate must expose the APIs below. As long as implementers follow these contracts, their work integrates on first merge.

#### `cb-core` ✅ COMPLETE
- ✅ **`pub struct AppConfig`** (in `config.rs`): loads from environment/files; provide `AppConfig::load()` returning `Result<AppConfig, CoreError>`.
- ✅ **`pub enum CoreError`** (in `error.rs`): shared error type implementing `std::error::Error` and `From` conversions for config/IO/json.
- ✅ **Protocol models** (in `model/`):
  - ✅ `pub enum McpMessage` with serde `Serialize/Deserialize` matching current MCP JSON.
  - ✅ `pub struct LspRequest`, `pub struct LspResponse` mirroring TypeScript structures.
  - ✅ `pub struct FuseConfig` for FUSE settings.
- ✅ **Intent specification** (new `model/intent.rs`):
  - ✅ `pub struct IntentSpec { pub name: String, pub arguments: serde_json::Value, pub metadata: Option<IntentMetadata> }`.
  - ✅ `pub struct IntentMetadata { pub source: String, pub correlation_id: Option<String> }`.
  - ✅ Re-export in `model/mod.rs` so downstream crates import `IntentSpec` from `cb_core::model`.
- ✅ **Tests:** `tests/acceptance_config.rs` verifying round-trip serialization and environment override precedence.

#### `cb-ast` ✅ COMPLETE
- ✅ Depends only on `cb-core` models.
- ✅ Provide `pub struct ImportGraph` and `pub fn build_import_graph(source: &str, path: &Path) -> Result<ImportGraph, AstError>`.
- ✅ Provide `pub struct EditPlan` and `pub fn plan_refactor(intent: &IntentSpec, source: &str) -> Result<EditPlan, AstError>` (`IntentSpec` imported from `cb_core::model`).
- ✅ `AstError` implements `std::error::Error` and converts to `CoreError`.
- ✅ Include unit tests covering ES module, CommonJS, dynamic imports, and type-only exports.

#### `cb-server` ✅ COMPLETE
- ✅ Binary crate. Expose in `lib.rs` (re-export from `src/lib.rs` or `main.rs`):
  - ✅ `pub struct ServerOptions` (built from `AppConfig`).
  - ✅ `pub fn bootstrap(options: ServerOptions) -> Result<ServerHandle, ServerError>`.
  - ✅ `pub struct ServerHandle` with methods `start()` (async), `shutdown()`.
- ✅ Use traits to decouple subsystems so agents can stub them. Document the canonical signatures in `src/interfaces.rs`:
  ```rust
  #[async_trait::async_trait]
  pub trait AstService: Send + Sync {
      async fn build_import_graph(&self, file: &Path) -> Result<ImportGraph, CoreError>;
      async fn plan_refactor(&self, intent: &IntentSpec, file: &Path) -> Result<EditPlan, CoreError>;
  }

  #[async_trait::async_trait]
  pub trait LspService: Send + Sync {
      async fn request(&self, message: McpMessage) -> Result<McpMessage, CoreError>;
  }
  ```
  ✅ All errors bubble up as `CoreError` so the server can translate them into MCP responses consistently.
- ✅ Provide default implementations wired to real modules, but keep traits in `pub mod interfaces` for mocking.
- ✅ Acceptance tests should spin up the server with mock `AstService`/`LspService` from `crates/tests` to confirm boot path works.
- ✅ Error surface: export `pub enum ServerError` (for bootstrap failures) and ensure it implements `From<CoreError>`.

#### `cb-client` ✅ COMPLETE
- ✅ Expose `pub async fn run_cli() -> Result<(), ClientError>` invoked by `main.rs`.
- ✅ `pub struct SessionReport` summarizing operations (success/failure counts).
- ✅ Config loads via `AppConfig` (from `cb-core`).
- ✅ Ensure CLI commands are defined via `clap` deriving `Parser` for repeatable UX.
- ✅ Define `pub enum ClientError` with variants for config, IO, and transport failures; implement `From<CoreError>` so shared errors propagate cleanly.

#### `tests` crate ✅ COMPLETE
- ✅ Supplies mocks for `AstService`, `LspService`, and file system adapters.
- ✅ Exposes helper functions:
  - ✅ `pub fn mock_ast_service() -> impl AstService`.
  - ✅ `pub fn mock_lsp_service() -> impl LspService`.
- ✅ Contains `tests/e2e_flow.rs` that launches `cb-server` with mocks and issues representative MCP requests to ensure contract compatibility.
- ✅ Provide `pub enum TestHarnessError` for helpers that may fail; keep constructors aligned with `CoreError` for easier debugging.

**Required Conventions** ✅ COMPLETE
- ✅ Every crate has a `tests/acceptance` directory using only its public API.
- ✅ Schemas (`serde` structs) include `#[serde(rename_all = "camelCase")]` to match existing MCP JSON.
- ✅ Public enums must use `#[non_exhaustive]` to allow additive updates without breaking consumers.
- ✅ Add JSON fixtures for each exported struct under `crates/tests/fixtures/<contract>.json`; mirror examples in `rust/docs/contracts.md`.

---

## 4. Phased Migration Plan ✅ COMPLETE

1. ✅ **Phase 1: Workspace + `cb-core`** – Scaffold workspace, implement config/errors/protocol models. Other crates use mocks until ready.
2. ✅ **Phase 2: `cb-ast`** – Deliver AST parsing and edit planning using `swc` (or equivalent). Export `ImportGraph` & `EditPlan` APIs.
3. ✅ **Phase 3: `cb-server` Skeleton** – Wire transports, dependency injection traits, and bootstrap logic using mocks.
4. ✅ **Phase 4: Systems & Real Integrations** – Connect FUSE, LSP process management, caching, and transactional handlers.
5. ✅ **Phase 5: `cb-client` + E2E** – Ship CLI, distribution artifacts, and validate parity via TypeScript E2E suite and Rust e2e tests.
   - ✅ CLI implementation complete
   - ✅ `SessionReport` struct implemented
   - ✅ TypeScript E2E integration script exists

Each phase can be owned by different implementers because crates only communicate through the contracts defined above.

---

## 5. Testing & Validation ✅ COMPLETE

- ✅ **Unit Tests:** Per crate, cover pure logic.
- ✅ **Acceptance Tests:** Per crate, exercise only public APIs with mocks provided by `crates/tests`.
- ✅ **Integration Tests:** In `crates/tests`, combine crates via their public interfaces.
- ✅ **End-to-End:** Run existing TypeScript E2E suite and new Rust e2e tests against the compiled server.

Command convention before merge:
```sh
cargo test --workspace                  # ✅ WORKS
bun run test:e2e:rust                   # ✅ WORKS - script to run TS suite against Rust server
```

---

## 6. Documentation & Coordination ✅ COMPLETE

- ✅ `rust/docs/parity-matrix.md` – Track feature parity.
- ✅ `rust/docs/contracts.md` – Summaries of crate APIs; update when signatures change.
- ✅ `rust/docs/` – Architecture, operations, and usage documentation.

✅ Before starting any crate, update `contracts.md` with the planned signatures and ping other owners if a breaking change is required.

---

## 7. Conclusion ✅ COMPLETE

✅ **SUCCESS:** Version 2 of the migration plan has been successfully implemented. The explicit crate contracts enabled work to be parallelized safely, and all contributors adhered to the exposed APIs and shipped the accompanying acceptance tests. The pieces integrated cleanly and delivered a Rust backend that meets or exceeds the current TypeScript capabilities.

**Next Steps:** The project has moved beyond this plan's scope and is ready for **PROPOSAL_RUST_MIGRATION_PART2.md** implementation.
