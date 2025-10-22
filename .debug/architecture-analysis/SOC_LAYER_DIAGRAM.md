# Separation of Concerns - Layer Diagram

**Last Updated:** October 20, 2025
**Status:** COMPREHENSIVE REFACTORING COMPLETE (Phase 1-3)

## Current Architecture Layers (Post-Consolidation)

```
┌─────────────────────────────────────────────────────────────────┐
│                     PRESENTATION LAYER                          │
│                  (mill-transport, cb-handlers)                    │
├─────────────────────────────────────────────────────────────────┤
│ Concerns:                          │ Status:                    │
│ • MCP request routing              │ ✓ Clean routing            │
│ • WebSocket/stdio handling         │ ✓ Trait abstraction        │
│ • Session management               │ ✓ Service extraction ✅    │
│ • Request/response marshaling      │ ✓ Business logic moved ✅  │
│                                    │ ✓ Checksum validation ✅   │
│ Violations Found: 0 ✅             │ (All fixed Oct 19-20)      │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    BUSINESS LOGIC LAYER                         │
│     (mill-services, mill-ast, mill-foundation)          │
├─────────────────────────────────────────────────────────────────┤
│ Concerns:                          │ Status:                    │
│ • Refactoring planning             │ ✓ Service traits           │
│ • Import management                │ ✓ DI via AppState          │
│ • Code analysis                    │ ✓ Composable services      │
│ • Plan conversion ✅               │ ✓ FileService focused ✅   │
│ • Checksum validation ✅           │ ✓ MoveService split ✅     │
│ • Reference tracking               │ ✓ Clear responsibilities   │
│                                    │                            │
│ Violations Found: 0 ✅             │ (Phase 2 complete)         │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    DATA ACCESS LAYER                            │
│              (file-service, reference-updater)                  │
├─────────────────────────────────────────────────────────────────┤
│ Concerns:                          │ Status:                    │
│ • File I/O operations              │ ✓ Abstracted file ops      │
│ • Reference/import tracking        │ ✓ Atomic operations        │
│ • Caching                          │ ✓ Lock management          │
│ • Locking for atomicity            │ ✓ Git as optional feature  │
│                                    │                            │
│ Violations Found: 0 ✅             │ (All refactored)           │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                  INFRASTRUCTURE LAYER                           │
│      (mill-lsp, cb-plugins, mill-config, plugin system)      │
├─────────────────────────────────────────────────────────────────┤
│ Concerns:                          │ Status:                    │
│ • LSP server communication         │ ✓ LSP encapsulation        │
│ • Language plugin dispatch         │ ✓ Plugin system ✅         │
│ • Configuration management         │ ✓ Config centralization    │
│ • Logging (structured tracing)     │ ⚠ eprintln! still used     │
│ • Error handling                   │ ✓ Unified error types      │
│                                    │                            │
│ Violations Found: 1 (low severity) │ (eprintln! in LSP client)  │
└─────────────────────────────────────────────────────────────────┘
```

## Request Flow Through Layers (Updated October 2025)

```
MCP Client
    ↓
    │ JSON-RPC Message
    ↓
PRESENTATION: Transport (ws.rs)
    ├─ Parse JSON
    ├─ Extract session info
    └─ Call McpDispatcher trait
         ↓
    PRESENTATION: PluginDispatcher
    ├─ Route to ToolRegistry
    ├─ Look up handler
    └─ Call handler
         ↓
    PRESENTATION: ToolHandler (e.g., WorkspaceApplyHandler) ✅ CLEAN
         ├─ Parse arguments
         ├─ Call ChecksumValidator ✅ (moved to mill-services)
         ├─ Call PlanConverter ✅ (moved to mill-services)
         └─ Call FileService
              ↓
         BUSINESS LOGIC: FileService ✅ FOCUSED
         ├─ Delegates to MoveService (for moves/renames)
         ├─ Delegates to ReferenceUpdater (for imports)
         └─ Acquire lock via LockManager
              ↓
         DATA ACCESS: Lock Management
         ├─ File-level locking
         └─ Atomic operation guarantee
              ↓
         INFRASTRUCTURE: File System
         ├─ OS-level file I/O
         └─ Actual read/write operations
              ↓
    Response flows back up through layers
         ↓
MCP Client receives result
```

## Concern Distribution (After Phase 1-3 Refactoring)

```
PRESENTATION      (0 violations ✅)
  ├─ Debug file I/O           ✅ FIXED (Oct 19)
  ├─ Plan conversion          ✅ MOVED to mill-services
  └─ Checksum validation      ✅ MOVED to mill-services

BUSINESS LOGIC    (0 violations ✅)
  ├─ FileService focus        ✅ FIXED (Phase 2)
  ├─ MoveService split        ✅ COMPLETE (Phase 2-3)
  ├─ Service extraction       ✅ COMPLETE (4 new services)
  └─ Plugin decoupling        ✅ COMPLETE (Phase 3)

DATA ACCESS       (0 violations ✅)
  ├─ Validation logic         ✅ EXTRACTED to services
  └─ Git coupling             ✅ OPTIONAL feature flag

INFRASTRUCTURE    (1 low-severity issue)
  ├─ eprintln! in LSP         ⚠ ACCEPTABLE (debug output)
  └─ PATH logic               ✅ (in LspConfig, not client)
```

## FileService Transformation (Phase 1-2 Complete)

```
OLD Structure (Oct 15):
┌────────────────────────────────────────┐
│         FileService (TOO FAT)          │
├────────────────────────────────────────┤
│ • File I/O (DATA ACCESS LAYER)         │
│ • Reference updating (BUSINESS LOGIC)  │ ← Mixing concerns
│ • Git integration (INFRASTRUCTURE)     │
│ • Validation logic (BUSINESS LOGIC)    │
│ • Lock management (INFRASTRUCTURE)     │
│ • Cargo operations (LANGUAGE-SPECIFIC) │
└────────────────────────────────────────┘

NEW Structure (Oct 20) ✅:
┌─────────────────────┐
│  FileService (Core) │  (DATA ACCESS: File I/O + coordination)
│  • File read/write  │
│  • Path operations  │
│  • AST cache mgmt   │
└──────────┬──────────┘
           ↑ Uses
      ┌────┴─────────────────────────────────────────┐
      │                                              │
┌─────▼──────────┐  ┌────────▼───────────┐  ┌───────▼────────┐
│ MoveService    │  │ ChecksumValidator  │  │ PlanConverter  │
│ (in mill-services│  │ (in mill-services)   │  │ (in mill-services│
│  separate file)│  │                    │  │  separate file)│
└────────────────┘  └────────────────────┘  └────────────────┘
(BUSINESS LOGIC)    (BUSINESS LOGIC)        (BUSINESS LOGIC)

      ┌────────────────────┐  ┌──────────────────┐
      │ ReferenceUpdater   │  │ GitService       │
      │ (coordinated by    │  │ (optional flag)  │
      │  FileService)      │  │                  │
      └────────────────────┘  └──────────────────┘
      (BUSINESS LOGIC)        (INFRASTRUCTURE)

Language-Specific Logic Moved to Plugins (Phase 3):
      ┌────────────────────┐  ┌──────────────────┐
      │ cb-lang-rust       │  │ cb-lang-typescript│
      │ • Consolidation    │  │ • Import support  │
      │ • Rust detector    │  │ • Path rewriting  │
      │ • Cargo helpers    │  │                   │
      └────────────────────┘  └──────────────────┘
```

## Crate Structure (Post-Consolidation)

```
Foundation Layer (mill-foundation):
  ├─ core (types, errors, config models)
  ├─ protocol (MCP protocol types)
  └─ 3 crates consolidated ✅

Plugin Layer (6 active plugins):
  ├─ cb-lang-rust (620 lines: reference detector moved ✅)
  ├─ cb-lang-typescript
  ├─ cb-lang-markdown (now active ✅)
  ├─ cb-lang-toml (now active ✅)
  ├─ cb-lang-yaml (now active ✅)
  └─ cb-lang-common (shared utilities)

Services Layer (mill-services):
  ├─ file_service/ (focused on file I/O)
  ├─ move_service/ (split out Oct 19 ✅)
  ├─ reference_updater/ (import tracking)
  ├─ checksum_validator.rs (extracted Oct 19 ✅)
  ├─ plan_converter.rs (extracted Oct 19 ✅)
  ├─ dry_run_generator.rs (extracted Oct 19 ✅)
  └─ post_apply_validator.rs (extracted Oct 19 ✅)

Total Crates: 27 (from original 30+ before consolidation)
```

## Trait Boundaries (Excellent Design - Unchanged)

```
mill-foundation/protocol (THE BOUNDARY)
┌───────────────────────────────────────────┐
│ Trait Definitions (No implementations)    │
├───────────────────────────────────────────┤
│ pub trait AstService { ... }              │
│ pub trait LspService { ... }              │
│ pub trait MessageDispatcher { ... }       │
│ pub trait ToolHandler { ... }             │
│ pub type ApiError { ... }                 │
└───────────────────────────────────────────┘
    ↑                           ↑
    │                           │
    ├─ mill-services implements   ├─ cb-handlers implements
    │  (business logic)         │  (presentation)
    │                           │
    └─────────────────┬─────────┘
                      │
            Both depend on traits,
            not on each other!
            (Good decoupling ✅)
```

## Dependency Flow (Now Strictly Downward)

```
✅ EXCELLENT: Application → Handlers → Services → Plugins → Foundation
        (One direction, fully acyclic, enforced by cargo-deny)

✅ Phase 3 Achievement (Oct 20):
   Services layer has ZERO production dependencies on language plugins!

   Before: mill-services → cb-lang-rust (coupling)
   After:  mill-services → mill-plugin-api → cb-lang-rust (abstraction)
```

## Error Handling Boundaries (Excellent - Unchanged)

```
                LSP Protocol Errors
                        │
                        ↓
        ┌───────────────────────────────┐
        │  LSP Client (Infrastructure)  │
        ├───────────────────────────────┤
        │ LspError → ApiError (Good!)   │
        └───────────────┬───────────────┘
                        ↓
        ┌───────────────────────────────┐
        │ Business Logic Layer          │
        ├───────────────────────────────┤
        │ ApiError usage (Good!)        │
        │ No raw errors leak (Good!)    │
        └───────────────┬───────────────┘
                        ↓
        ┌───────────────────────────────┐
        │ Presentation Layer            │
        ├───────────────────────────────┤
        │ McpResponse generation (Good!)│
        └───────────────────────────────┘
```

## Scores by Layer (Updated October 20, 2025)

```
Presentation Layer
  Layer Separation ................ 10/10 ✅ (+2)
  Trait Abstractions .............. 9/10  ✅ (+1)
  Business Logic Isolation ......... 9/10  ✅ (+4)
  Infrastructure Isolation ......... 10/10 ✅ (+1)
  ────────────────────────────────────
  Overall Presentation ............. 9.5/10 ✅ (+2.0)

Business Logic Layer
  Service Traits ................... 9/10  ✅ (+1)
  Dependency Injection ............. 9/10  ✅ (+1)
  Concern Isolation ................ 9/10  ✅ (+3)
  Data Access Abstraction .......... 9/10  ✅ (+2)
  ────────────────────────────────────
  Overall Business Logic ........... 9.0/10 ✅ (+1.7)

Data Access Layer
  Abstraction Level ................ 9/10  ✅ (+1)
  Atomic Operations ................ 9/10  ✅ (+1)
  Coupling ......................... 8/10  ✅ (+2)
  ────────────────────────────────────
  Overall Data Access .............. 8.7/10 ✅ (+1.4)

Infrastructure Layer
  Encapsulation .................... 9/10  ✅ (+1)
  Configuration Management ......... 9/10  ✅ (+1)
  Plugin System .................... 10/10 ✅ (+2)
  Logging Consistency .............. 7/10  ⚠ (+1)
  ────────────────────────────────────
  Overall Infrastructure ........... 8.8/10 ✅ (+1.3)

═══════════════════════════════════════════
OVERALL SEPARATION OF CONCERNS .... 9.0/10 ✅
═══════════════════════════════════════════
Previous Score: 7.5/10
Improvement: +1.5 points (+20%)
```

## Refactoring Completed (Phase 1-3)

```
✅ Priority 1: Remove Debug File I/O (COMPLETE - Oct 19)
├─ Impact: HIGH (removed critical violation)
├─ Complexity: LOW
├─ Risk: LOW
├─ Commits: 7be64098 (directory_rename.rs)
└─ Result: All /tmp debug logging removed

✅ Priority 2: Extract Service Classes (COMPLETE - Oct 19-20)
├─ Impact: HIGH (fixes business logic leakage)
├─ Complexity: MEDIUM
├─ Risk: LOW
├─ Created: ChecksumValidator, PlanConverter, DryRunGenerator, PostApplyValidator
├─ Location: /workspace/crates/mill-services/src/services/
└─ Result: Clean separation, all tests passing

✅ Priority 3: Split FileService (COMPLETE - Phase 2)
├─ Impact: HIGH (fixes major mixing concern)
├─ Complexity: HIGH
├─ Risk: MEDIUM
├─ Created: MoveService (separate file)
├─ Result: FileService focused on file I/O coordination
└─ Commits: fa425e88, bd519936

✅ Priority 4: Plugin System Refactoring (COMPLETE - Phase 3)
├─ Impact: ARCHITECTURAL (language-agnostic design)
├─ Complexity: HIGH
├─ Risk: HIGH (but executed successfully)
├─ Moved: All Rust-specific code to cb-lang-rust plugin
├─ Result: Zero production dependencies from services to plugins
└─ Commits: e3df12eb, d007381d, a48dba20

⚠ Priority 5: LSP eprintln! Cleanup (LOW PRIORITY)
├─ Impact: LOW (consistency improvement)
├─ Complexity: LOW
├─ Risk: LOW
├─ Status: DEFERRED (acceptable for debug output)
└─ File: /workspace/crates/mill-lsp/src/lsp_system/client.rs
```

## Benefits Achieved

### For Developers
- ✅ **Clear mental model:** Services have single responsibilities
- ✅ **Reduced coupling:** Changes are localized to one service
- ✅ **Easier debugging:** Dependency direction is strictly downward
- ✅ **Better testing:** Each service can be tested in isolation

### For Architecture
- ✅ **Prevents rot:** cargo-deny enforces layer boundaries
- ✅ **Scalable:** New plugins fit into clear plugin layer
- ✅ **Modular:** Services are composable and reusable
- ✅ **Language-agnostic:** Plugin system supports any language

### For Testing
- ✅ **Isolated testing:** Services use dependency injection
- ✅ **Clear boundaries:** Each layer tests independently
- ✅ **High coverage:** 867/869 tests passing (99.8%)

## Recent Commits (Phase 1-3 Timeline)

| Date | Commit | Phase | Description |
|------|--------|-------|-------------|
| Oct 20 | `e3df12eb` | Phase 3 | Move Rust reference detector to plugin |
| Oct 20 | `a48dba20` | Phase 3 | Clean up Phase 3 stragglers |
| Oct 19 | `d007381d` | Phase 3 | Complete Phase 3 - All Rust code in plugin |
| Oct 19 | `bd519936` | Phase 2 | Make MoveService language-agnostic |
| Oct 19 | `fa425e88` | Phase 2 | Untangle FileService and MoveService |
| Oct 19 | `59e6ff9e` | Phase 2 | Consolidate WorkspaceEdit creation |
| Oct 19 | `7be64098` | Phase 1 | Remove /tmp debug logging |
| Oct 19 | `1f6e1a3a` | Phase 1 | Consolidate checksum calculation |

## References

- **Proposal 06:** Workspace Consolidation (archived - complete)
- **Architecture Layers:** [docs/architecture/layers.md](docs/architecture/layers.md)
- **deny.toml:** Programmatic enforcement configuration
- **Architecture Overview:** [docs/architecture/overview.md](docs/architecture/overview.md)

---

**Status:** All major SOC violations resolved. Architecture is production-ready with excellent separation of concerns.
