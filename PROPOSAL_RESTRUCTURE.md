# Project Restructure Proposal

## Executive Summary

This proposal outlines a comprehensive restructure of the Codebuddy project to improve maintainability, follow Rust ecosystem conventions, and enhance developer experience. The changes are based on analysis of current structure against industry best practices and address 12 identified organizational issues.

---

## BEFORE: Current Structure

```
/workspace/
├── CHANGELOG.md
├── CLAUDE.md
├── LICENSE
├── MCP_API.md
├── PROPOSAL_DOCKER_SHARED_VOLUMES.md
├── README.md
├── SUPPORT_MATRIX.md
├── TESTING_GUIDE.md
├── install.sh
├── vm.yaml
├── .gitignore
├── .gitattributes
├── .knipignore
├── .mcp.json
│
├── docker/
│   ├── config/
│   │   ├── development.json
│   │   └── production.json
│   ├── docker-compose.yml
│   ├── docker-compose.production.yml
│   ├── Dockerfile
│   ├── nginx.conf
│   └── README.md
│
├── docs/
│   └── features/
│       └── WORKFLOWS.md
│
├── examples/
│   ├── backend/
│   ├── database/
│   ├── frontend/
│   ├── playground/                    # Mixed: test fixtures + examples
│   │   ├── atomic-refactoring-test/
│   │   ├── python/
│   │   ├── rust/
│   │   │   └── target/                # Build artifacts in examples!
│   │   ├── src/
│   │   └── test-workspace-symbols/
│   ├── README.md
│   └── tenant-client.ts
│
└── rust/
    ├── Cargo.lock
    ├── Cargo.toml
    ├── CONTRIBUTING.md
    ├── README.md
    ├── ROADMAP.md
    ├── rust-toolchain.toml
    ├── justfile
    │
    ├── apps/
    │   └── server/                    # Production binary
    │       ├── Cargo.toml
    │       └── src/
    │           ├── main.rs
    │           └── cli.rs
    │
    ├── crates/
    │   ├── cb-api/
    │   ├── cb-ast/
    │   ├── cb-client/                 # Has main.rs - binary or library?
    │   │   └── src/
    │   │       ├── main.rs
    │   │       ├── lib.rs
    │   │       └── commands/
    │   ├── cb-core/
    │   ├── cb-mcp-proxy/
    │   ├── cb-plugins/
    │   ├── cb-server/                 # Has BOTH main.rs AND lib.rs
    │   │   ├── src/
    │   │   │   ├── main.rs            # Duplicate entry point!
    │   │   │   ├── lib.rs
    │   │   │   ├── auth/
    │   │   │   ├── handlers/
    │   │   │   ├── services/
    │   │   │   ├── systems/
    │   │   │   │   └── lsp/
    │   │   │   └── utils/
    │   │   └── tests/
    │   ├── cb-transport/
    │   ├── cb-vfs/
    │   └── tests/                     # Actually integration tests + harness
    │       ├── fixtures/
    │       ├── src/
    │       │   └── harness/
    │       └── tests/
    │
    ├── docs/
    │   ├── ARCHITECTURE.md
    │   ├── LOGGING_GUIDELINES.md
    │   ├── OPERATIONS.md
    │   ├── USAGE.md
    │   └── contracts.md
    │
    └── testing/
        └── benchmarks/
            ├── Cargo.toml
            └── benches/
```

---

## AFTER: Proposed Structure

```
/workspace/
├── README.md
├── CLAUDE.md
├── CONTRIBUTING.md
├── CHANGELOG.md
├── LICENSE
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── justfile
├── .gitignore
├── .gitattributes
├── .knipignore
├── .mcp.json
│
├── apps/
│   └── codebuddy/                     # Unified binary (server + client CLI)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── commands/
│               ├── mod.rs
│               ├── start.rs           # Server commands
│               ├── serve.rs
│               ├── setup.rs
│               ├── status.rs
│               ├── connect.rs         # Client commands
│               └── call.rs
│
├── crates/
│   ├── cb-api/
│   ├── cb-ast/
│   ├── cb-client/                     # Library only (WebSocket client)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── websocket.rs
│   │       ├── client_config.rs
│   │       └── formatting.rs
│   ├── cb-core/
│   ├── cb-mcp-proxy/
│   ├── cb-plugins/
│   ├── cb-server/                     # Library only (server logic)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── auth/
│   │   │   ├── handlers/
│   │   │   ├── services/
│   │   │   ├── systems/
│   │   │   │   └── lsp/
│   │   │   └── utils/
│   │   └── tests/
│   ├── cb-transport/
│   └── cb-vfs/
│
├── integration-tests/                 # Renamed from crates/tests
│   ├── Cargo.toml
│   ├── fixtures/
│   ├── src/
│   │   └── harness/                   # Shared test infrastructure
│   └── tests/
│       ├── e2e_*.rs
│       ├── integration_*.rs
│       ├── lsp_*.rs
│       └── mcp_*.rs
│
├── benchmarks/                        # Moved from rust/testing/benchmarks
│   ├── Cargo.toml
│   └── benches/
│
├── examples/                          # Clean user-facing examples
│   ├── README.md
│   ├── typescript-integration/
│   ├── python-integration/
│   └── rust-integration/
│
├── playground/                        # Developer workspace (gitignored)
│   └── .gitkeep
│
├── tests/
│   └── fixtures/                      # Test data (from examples/playground)
│       ├── typescript/
│       ├── python/
│       └── rust/
│
├── deployment/
│   ├── docker/
│   │   ├── config/
│   │   │   ├── development.json
│   │   │   └── production.json
│   │   ├── docker-compose.yml
│   │   ├── docker-compose.production.yml
│   │   ├── Dockerfile
│   │   └── nginx.conf
│   ├── vm.yaml
│   └── README.md
│
├── scripts/
│   └── install.sh
│
└── docs/
    ├── architecture/
    │   ├── ARCHITECTURE.md
    │   └── contracts.md
    ├── development/
    │   ├── TESTING_GUIDE.md
    │   ├── LOGGING_GUIDELINES.md
    │   └── ROADMAP.md
    ├── deployment/
    │   ├── OPERATIONS.md
    │   ├── docker.md
    │   └── PROPOSAL_DOCKER_SHARED_VOLUMES.md
    ├── features/
    │   └── WORKFLOWS.md
    ├── api/
    │   └── MCP_API.md
    └── support/
        └── SUPPORT_MATRIX.md
```

---

## Changes Required

### 1. Flatten Rust Directory Structure

**Changes:**
- Move `/workspace/rust/*` to `/workspace/`
- Move `/workspace/rust/Cargo.toml` → `/workspace/Cargo.toml`
- Move `/workspace/rust/Cargo.lock` → `/workspace/Cargo.lock`
- Move `/workspace/rust/rust-toolchain.toml` → `/workspace/rust-toolchain.toml`
- Move `/workspace/rust/justfile` → `/workspace/justfile`
- Move `/workspace/rust/apps/` → `/workspace/apps/`
- Move `/workspace/rust/crates/` → `/workspace/crates/`
- Delete `/workspace/rust/` directory

**Reason:** Rust workspace should be at repository root per ecosystem conventions. The project is fundamentally Rust (45,650+ lines), not multi-language. This eliminates redundant nesting and allows `cargo build` from root.

---

### 2. Consolidate Binary Architecture

**Changes:**
- Rename `/workspace/apps/server/` → `/workspace/apps/codebuddy/`
- Delete `/workspace/crates/cb-server/src/main.rs` (orphaned duplicate entry point)
- Delete `/workspace/crates/cb-client/src/main.rs`
- Move CLI commands from `apps/server/src/cli.rs` and `cb-client/src/commands/` into `apps/codebuddy/src/commands/`
- Update `apps/codebuddy/Cargo.toml` to set binary name as `codebuddy`
- Ensure `cb-server` and `cb-client` crates are library-only

**Reason:** Currently two `main.rs` files exist in `cb-server` (duplicate server initialization) and `cb-client` (small wrapper). This creates confusion about entry points and package distribution. A unified binary with subcommands matches industry patterns (git, docker, cargo) and simplifies user experience.

---

### 3. Reorganize Test Structure

**Changes:**
- Rename `/workspace/crates/tests/` → `/workspace/integration-tests/`
- Move integration tests to `integration-tests/tests/`
- Keep unit tests in per-crate `tests/` directories (e.g., `cb-core/tests/`)
- Move test fixtures from `examples/playground/` to `/workspace/tests/fixtures/`
- Keep shared test harness in `integration-tests/src/harness/`

**Reason:** Current "tests" crate name is confusing (it's a full crate with lib.rs, not just tests). Separating unit tests (fast iteration per crate) from integration tests (system-wide validation) follows Rust best practices and matches patterns in large projects like tokio and serde.

---

### 4. Split Examples and Playground

**Changes:**
- Create `/workspace/playground/` directory
- Add `/playground/` to `.gitignore` (except `.gitkeep`)
- Move `/workspace/examples/playground/` → `/workspace/tests/fixtures/` (test data)
- Keep clean user-facing examples in `/workspace/examples/`:
  - `typescript-integration/` (reorganized from `backend/`, `frontend/`)
  - `python-integration/` (reorganized from `backend/`)
  - `rust-integration/` (sample project)
- Remove `examples/playground/rust/target/` (build artifacts)

**Reason:** `examples/` directory mixes user-facing examples with test fixtures and development playgrounds. Users browsing examples shouldn't see test data or build artifacts. Clean examples improve discoverability and project professionalism.

---

### 5. Consolidate Documentation

**Changes:**
- Create organized `/workspace/docs/` structure:
  - `docs/architecture/` (ARCHITECTURE.md, contracts.md)
  - `docs/development/` (TESTING_GUIDE.md, LOGGING_GUIDELINES.md, ROADMAP.md)
  - `docs/deployment/` (OPERATIONS.md, docker.md, PROPOSAL_DOCKER_SHARED_VOLUMES.md)
  - `docs/features/` (WORKFLOWS.md)
  - `docs/api/` (MCP_API.md)
  - `docs/support/` (SUPPORT_MATRIX.md)
- Move `/workspace/rust/CONTRIBUTING.md` → `/workspace/CONTRIBUTING.md`
- Keep at root: README.md, CLAUDE.md, CONTRIBUTING.md, CHANGELOG.md, LICENSE
- Delete redundant `/workspace/rust/README.md`

**Reason:** Documentation currently scattered across 4 locations (root, `docs/`, `rust/`, `rust/docs/`). Consolidating into organized `/docs/` with clear categories improves discoverability while keeping essential files (README, CONTRIBUTING, LICENSE) at root per GitHub conventions.

---

### 6. Organize Infrastructure Files

**Changes:**
- Create `/workspace/deployment/` directory
- Move `/workspace/docker/` → `/workspace/deployment/docker/`
- Move `/workspace/vm.yaml` → `/workspace/deployment/vm.yaml`
- Create `/workspace/scripts/` directory
- Move `/workspace/install.sh` → `/workspace/scripts/install.sh`
- Consolidate deployment docs into `deployment/README.md`

**Reason:** Infrastructure and deployment concerns currently mixed with source code at root. Grouping deployment configurations and scripts improves organization and separates operational concerns from development workflow.

---

### 7. Move Benchmarks to Standard Location

**Changes:**
- Move `/workspace/rust/testing/benchmarks/` → `/workspace/benchmarks/`
- Update workspace `Cargo.toml` members: change `"testing/benchmarks"` to `"benchmarks"`
- Delete empty `/workspace/testing/` directory

**Reason:** Directory named "testing" contains only benchmarks, causing confusion. Moving to `/workspace/benchmarks/` clarifies purpose while maintaining benchmarks as separate crate for independent compilation and dependencies.

---

### 8. Standardize Naming

**Changes:**
- Search and replace "Codeflow Buddy" → "Codebuddy" in all documentation
- Update package descriptions in all `Cargo.toml` files
- Document crate naming convention in `CONTRIBUTING.md`:
  - User-facing: "Codebuddy" (binary, docs, branding)
  - Internal: `cb-*` crate prefixes (concise, standard Rust practice)

**Reason:** Documentation inconsistently uses "Codebuddy", "Codeflow Buddy", and "cb-" prefixes. Standardizing on "Codebuddy" for user-facing content while keeping `cb-` for crate names follows Rust ecosystem patterns (tokio-*, serde-*) and improves brand clarity.

---

### 9. Fix File Permissions

**Changes:**
```bash
find /workspace -type d -exec chmod 755 {} \;
find /workspace -type f -exec chmod 644 {} \;
find /workspace -type f -name "*.sh" -exec chmod 755 {} \;
```

**Reason:** Some directories have 700 permissions (no group/other read), causing issues in Docker builds, CI/CD systems, and collaborative development. Standard permissions (755 for dirs, 644 for files) ensure compatibility.

---

### 10. Update Workspace Configuration

**Changes:**
- Update `/workspace/Cargo.toml` workspace members:
  ```toml
  members = [
      "apps/codebuddy",
      "crates/cb-api",
      "crates/cb-ast",
      "crates/cb-client",
      "crates/cb-core",
      "crates/cb-mcp-proxy",
      "crates/cb-plugins",
      "crates/cb-server",
      "crates/cb-transport",
      "crates/cb-vfs",
      "integration-tests",
      "benchmarks",
  ]
  ```
- Update all internal import paths after restructure
- Update CI/CD workflows to reference new paths

**Reason:** Workspace configuration must reflect new structure for Cargo to correctly resolve dependencies and build targets.

---

## Implementation Priority

### Phase 1: Quick Wins (Immediate)
1. Fix file permissions (5 minutes)
2. Standardize naming in docs (30 minutes)
3. Delete `cb-server/src/main.rs` (1 hour with testing)
4. Organize root directory clutter (1 hour)

### Phase 2: Structural Changes (Next Sprint)
5. Consolidate documentation (2-3 hours)
6. Split examples and playground (1-2 hours)
7. Reorganize test structure (2-3 hours)
8. Move benchmarks (30 minutes)

### Phase 3: Major Refactoring (Requires Planning)
9. Flatten Rust directory structure (4-8 hours with testing)
10. Consolidate binary architecture (4-6 hours with testing)
11. Organize infrastructure files (1-2 hours)
12. Update all references and CI/CD (2-3 hours)

---

## Benefits

1. **Follows Rust Conventions**: Workspace at root, standard directory layout
2. **Improved Discoverability**: Clear hierarchy, logical organization
3. **Better Developer Experience**: Obvious where to find/add code
4. **Professional Appearance**: Clean root directory, organized docs
5. **Simplified Tooling**: Standard paths work with Rust tools out-of-box
6. **Scalability**: Structure supports project growth without refactoring
7. **Clearer Responsibilities**: Library vs binary, unit vs integration tests
8. **Reduced Confusion**: Single entry point, clear naming conventions

---

## Risks & Mitigation

**Risk**: Breaking existing integrations/workflows
**Mitigation**: Implement incrementally, test after each phase

**Risk**: Import path updates across 45,650 lines
**Mitigation**: Use automated search-replace, comprehensive test suite validation

**Risk**: Documentation becomes outdated during transition
**Mitigation**: Update docs as part of each change, not after

**Risk**: CI/CD failures during restructure
**Mitigation**: Update CI/CD configs first, test in feature branch

---

## Validation Checklist

- [ ] All tests pass after restructure
- [ ] `cargo build --release` succeeds from root
- [ ] `cargo test --workspace` succeeds
- [ ] `cargo clippy --workspace` passes
- [ ] Documentation links work (no 404s)
- [ ] CI/CD pipeline succeeds
- [ ] Docker build succeeds with new paths
- [ ] Installation script works from new location
- [ ] Binary runs with all subcommands
- [ ] IDE/tooling continues to work (rust-analyzer, etc.)
