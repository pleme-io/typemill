# Separation of Concerns - Layer Diagram

## Current Architecture Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                     PRESENTATION LAYER                          │
│                  (cb-transport, cb-handlers)                    │
├─────────────────────────────────────────────────────────────────┤
│ Concerns:                          │ Status:                    │
│ • MCP request routing              │ ✓ Clean routing            │
│ • WebSocket/stdio handling         │ ✓ Trait abstraction        │
│ • Session management               │ ✗ Debug file I/O (BAD)     │
│ • Request/response marshaling      │ ✗ Plan conversion (BAD)    │
│                                    │ ✗ Checksum validation (BAD)│
│ Violations Found: 3                │                            │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    BUSINESS LOGIC LAYER                         │
│                   (cb-services, cb-ast)                         │
├─────────────────────────────────────────────────────────────────┤
│ Concerns:                          │ Status:                    │
│ • Refactoring planning             │ ✓ Service traits           │
│ • Import management                │ ✓ DI via AppState          │
│ • Code analysis                    │ ✓ Composable services      │
│ • Plan conversion (ideally)        │ ✗ FileService mixing (BAD) │
│ • Reference tracking               │ ✗ Plan conv in handler     │
│                                    │ ✗ Too many constructor args│
│ Violations Found: 3                │                            │
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
│ • Locking for atomicity            │ ✗ Some validation logic    │
│                                    │ ✗ Git coupling             │
│ Violations Found: 2                │                            │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                  INFRASTRUCTURE LAYER                           │
│            (cb-lsp, cb-plugins, cb-core)                        │
├─────────────────────────────────────────────────────────────────┤
│ Concerns:                          │ Status:                    │
│ • LSP server communication         │ ✓ LSP encapsulation        │
│ • Language plugin dispatch         │ ✓ Plugin system            │
│ • Configuration management         │ ✓ Config centralization    │
│ • Logging                          │ ✗ Debug output (eprintln)  │
│ • Error handling                   │ ✗ PATH logic in client     │
│                                    │                            │
│ Violations Found: 2                │                            │
└─────────────────────────────────────────────────────────────────┘
```

## Request Flow Through Layers

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
    PRESENTATION: ToolHandler (e.g., FileOperationHandler) ✗ BAD: Complex logic
         ├─ Parse arguments
         ├─ Convert to edit plan (✗ SHOULD BE IN BUSINESS LOGIC)
         ├─ Validate checksums (✗ SHOULD BE IN BUSINESS LOGIC)
         └─ Call FileService
              ↓
         BUSINESS LOGIC: FileService ✗ BAD: Mixed concerns
         ├─ Reference updater (business logic)
         ├─ Git service (infrastructure)
         ├─ Lock manager (infrastructure)
         └─ Acquire lock
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

## Concern Distribution

```
PRESENTATION      (10 violations)
  ├─ Debug file I/O           ✗ CRITICAL
  ├─ Plan conversion          ✗ MEDIUM
  └─ Checksum validation      ✗ MEDIUM

BUSINESS LOGIC    (3 violations)
  ├─ FileService mixing       ✗ MEDIUM
  ├─ Constructor params       ✗ LOW
  └─ Plugin dispatch logic    ✗ LOW

DATA ACCESS       (2 violations)
  ├─ Validation logic         ✗ LOW
  └─ Git coupling             ✗ LOW

INFRASTRUCTURE    (2 violations)
  ├─ Debug output (eprintln)  ✗ LOW
  └─ PATH logic in client     ✗ MEDIUM
```

## FileService Problem (the Biggest Violation)

```
Current Structure:
┌────────────────────────────────────────┐
│         FileService (TOO FAT)          │
├────────────────────────────────────────┤
│ • File I/O (DATA ACCESS LAYER)         │
│ • Reference updating (BUSINESS LOGIC)  │ ← Mixing concerns
│ • Git integration (INFRASTRUCTURE)     │
│ • Validation logic (BUSINESS LOGIC)    │
│ • Lock management (INFRASTRUCTURE)     │
└────────────────────────────────────────┘

Recommended Structure:
┌─────────────────────┐
│  CoreFileService    │  (DATA ACCESS: Pure I/O)
└──────────┬──────────┘
           ↑
      ┌────┴─────────────────┬────────────────────┐
      │                      │                    │
┌─────▼──────────┐  ┌────────▼─────────┐  ┌──────▼──────────┐
│ ReferenceUpdate│  │ GitAwareFileService│  │ ValidationService│
│   Service      │  │                    │  │                  │
└────────────────┘  └────────────────────┘  └──────────────────┘
(BUSINESS LOGIC)    (INFRASTRUCTURE)        (BUSINESS LOGIC)

These wrap CoreFileService:
FileService = Facade for backward compatibility
```

## Trait Boundaries (Good Design)

```
cb-protocol/lib.rs (THE BOUNDARY)
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
    ├─ cb-services implements   ├─ cb-handlers implements
    │  (business logic)         │  (presentation)
    │                           │
    └─────────────────┬─────────┘
                      │
            Both depend on traits,
            not on each other!
            (Good decoupling)
```

## Dependency Flow (Should Be Downward)

```
✓ GOOD: Application → Core → Services → Infrastructure
        (One direction, acyclic)

✗ BAD: (currently happening)
FileService (BUSINESS LOGIC) depends on:
  - ReferenceUpdater (its own level)
  - LockManager (INFRASTRUCTURE)
  - OperationQueue (INFRASTRUCTURE)
  - GitService (INFRASTRUCTURE)

Should be:
FileService (BUSINESS LOGIC) should be:
  - Pure, depends only on abstractions
  - Composed by higher layers
  - Tested in isolation
```

## Error Handling Boundaries

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

## Scores by Layer

```
Presentation Layer
  Layer Separation ................ 8/10
  Trait Abstractions .............. 8/10
  Business Logic Isolation ......... 5/10  ← Has conversions
  Infrastructure Isolation ......... 9/10
  ────────────────────────────────────
  Overall Presentation ............. 7.5/10

Business Logic Layer
  Service Traits ................... 8/10
  Dependency Injection ............. 8/10
  Concern Isolation ................ 6/10  ← FileService mixing
  Data Access Abstraction .......... 7/10
  ────────────────────────────────────
  Overall Business Logic ........... 7.3/10

Data Access Layer
  Abstraction Level ................ 8/10
  Atomic Operations ................ 8/10
  Coupling ......................... 6/10  ← Git coupling
  ────────────────────────────────────
  Overall Data Access .............. 7.3/10

Infrastructure Layer
  Encapsulation .................... 8/10
  Configuration Management ......... 8/10
  Plugin System .................... 8/10
  Logging Consistency .............. 6/10  ← eprintln!
  ────────────────────────────────────
  Overall Infrastructure ........... 7.5/10

═══════════════════════════════════════════
OVERALL SEPARATION OF CONCERNS .... 7.5/10
═══════════════════════════════════════════
```

## Refactoring Impact Map

```
Priority 1: Remove Debug File I/O (30 min)
├─ Impact: HIGH (removes critical violation)
├─ Complexity: LOW
├─ Risk: LOW
└─ File: workspace_apply_handler.rs

Priority 2: Extract PlanConverter (1-2 hours)
├─ Impact: HIGH (fixes business logic leakage)
├─ Complexity: MEDIUM
├─ Risk: LOW
├─ Creates: crates/cb-services/src/services/plan_converter.rs
└─ Moves: convert_to_edit_plan, extract_workspace_edit, validate_checksums

Priority 3: Split FileService (2-4 hours)
├─ Impact: HIGH (fixes major mixing concern)
├─ Complexity: HIGH
├─ Risk: MEDIUM (affects many tests)
├─ Creates: CoreFileService, ReferenceUpdateService
└─ Keeps: FileService as facade for backward compat

Priority 4: Move PATH Logic (30 min)
├─ Impact: MEDIUM (improves testability)
├─ Complexity: LOW
├─ Risk: LOW
└─ File: crates/cb-lsp/src/lsp_system/client.rs

Priority 5: Fix Debug Output (30 min)
├─ Impact: LOW (consistency improvement)
├─ Complexity: LOW
├─ Risk: LOW
└─ File: crates/cb-lsp/src/lsp_system/client.rs
```
