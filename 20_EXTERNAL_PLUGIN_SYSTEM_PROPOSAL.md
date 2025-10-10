# External Plugin System Proposal (REVISED)

**Status:** Phase 1 Complete ✅ | Phase 2+ In Progress
**Author:** Analysis of codebase architecture
**Goal:** Enable language plugins to be maintained in external Git repositories, installable as add-ons rather than compiled into the core binary.

**⚠️ CRITICAL DESIGN CHANGES:**
- **Rejected:** Native FFI with `*mut dyn Trait` (ABI-unstable)
- **Adopted:** Out-of-process RPC plugins OR incremental crates.io approach
- **Rationale:** Codex correctly identified trait object layout instability and toolchain version coupling

---

## Current Architecture

### Integration Points

1. **Compile-Time Registration**
   - `config/languages/languages.toml` is the single source of truth
   - `crates/cb-services/build.rs` reads TOML and generates:
     - `ProjectLanguage` enum variants (`cb-core/src/language.rs`)
     - `LanguageMetadata` constants (`cb-plugin-api/src/metadata.rs`)
     - Workspace dependencies (root `Cargo.toml`)
     - Registry registration code (`cb-services/src/services/registry_builder.rs`)

2. **Workspace Dependencies**
   ```toml
   # Root Cargo.toml (lines 88-94)
   cb-lang-rust = { path = "crates/cb-lang-rust" }
   cb-lang-go = { path = "crates/cb-lang-go" }
   cb-lang-typescript = { path = "crates/cb-lang-typescript" }
   # ... all plugins are workspace members
   ```

3. **Hard-Coded Instantiation**
   ```rust
   // Generated in registry_builder.rs
   registry.register(Arc::new(cb_lang_rust::RustPlugin::new()));
   ```

4. **Tight Coupling**
   - Language plugins are workspace members
   - Core binary must be rebuilt to add/remove languages
   - Plugins cannot version independently

### Plugin Interface (Well-Designed)

**Strengths:**
- Clean `LanguagePlugin` trait in `cb-plugin-api`
- Modular capabilities via `ImportSupport` and `WorkspaceSupport` traits
- Minimal dependencies: `cb-plugin-api`, `cb-types`, `cb-core`, `cb-protocol`
- Already uses trait objects (`Arc<dyn LanguagePlugin>`)

**Ready for externalization, but ABI stability requires protocol-based approach.**

---

## ❌ REJECTED APPROACH: Native FFI Dynamic Loading

### Why This Fails

**Problem 1: Trait Object Layout is NOT ABI-Stable**
```rust
// THIS IS UNSOUND - WILL SEGFAULT
#[no_mangle]
pub extern "C" fn _plugin_create() -> *mut dyn LanguagePlugin {
    Box::into_raw(Box::new(RustPlugin::new()))
}
```

**Why:**
- `*mut dyn Trait` fat pointer layout is Rust-internal, not guaranteed across:
  - Compiler versions (rustc 1.75 vs 1.76)
  - Optimization levels (-O0 vs -O3)
  - Linkage modes (static vs dynamic)
- Even with `#[repr(C)]`, trait object vtables are unstable

**Problem 2: String/Vec Across FFI Boundary**
```rust
// THIS IS UNSOUND - ALLOCATOR MISMATCH
#[derive(Debug, Deserialize)]
pub struct PluginInfo {
    pub name: String,  // ❌ Different allocator in plugin vs host
    pub extensions: Vec<String>,  // ❌ Memory layout not guaranteed
}
```

**Why:**
- Plugin and host may use different allocators
- `String` layout is not `#[repr(C)]`
- Passing by value = use-after-free if allocators differ

**Problem 3: Toolchain Version Coupling**
- Plugin built with Rust 1.75 + syn 2.0.48
- Host built with Rust 1.76 + syn 2.0.50
- Symbol mismatches, panic handler conflicts, allocator incompatibility

**Codex is correct: This approach is fundamentally broken in Rust.**

---

## ✅ RECOMMENDED APPROACH 1: Out-of-Process RPC Plugins

### Architecture: LSP-Style Plugin Workers

```
┌────────────────────────────────────────────────┐
│  Codebuddy Core                                │
│  ┌──────────────────────────────────────────┐ │
│  │ Plugin Manager (process supervisor)      │ │
│  │ - spawn_plugin(name)                     │ │
│  │ - send_request(plugin, request)          │ │
│  │ - handle_crash(plugin)  ← Already exists│ │
│  └──────────────────────────────────────────┘ │
└────────┬───────────────────────────────────────┘
         │ stdio/JSON-RPC
         │ (Same pattern as LSP servers)
         │
    ┌────▼─────────────┐      ┌─────────────────┐
    │ Plugin Process   │      │ Plugin Process  │
    │ cb-lang-rust     │      │ cb-lang-python  │
    │ ├─ Standalone    │      │ ├─ Standalone   │
    │ │  binary        │      │ │  binary       │
    │ └─ JSON-RPC      │      │ └─ JSON-RPC     │
    │    server        │      │    server       │
    └──────────────────┘      └─────────────────┘
         Separate repo             Separate repo
         Own version               Own version
```

### Why This Works

**✅ Stable Protocol:** JSON-RPC is platform/language/version agnostic
- Same boundary codebuddy already uses for LSP servers
- Existing process management and crash recovery
- Well-tested pattern (LSP, DAP, MCP itself)

**✅ Already Have Tooling:**
```rust
// crates/cb-lsp/src/client.rs ALREADY DOES THIS
pub struct LspClient {
    processes: HashMap<String, Child>,  // Process supervisor
    // Send JSON-RPC, correlate responses
}
```

**✅ Zero ABI Concerns:**
- Protocol = JSON over stdio
- Each plugin is a separate process with its own memory space
- No shared allocators, no vtable layout issues
- Plugins can be written in ANY language (Rust, Go, Python)

### Protocol Definition

```rust
// crates/cb-protocol/src/plugin_protocol.rs
#[derive(Serialize, Deserialize)]
pub struct PluginRequest {
    pub id: u64,
    pub method: String,  // "parse", "analyze_manifest", "list_functions"
    pub params: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct PluginResponse {
    pub id: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<PluginError>,
}
```

**Example:**
```json
// Core → Plugin (parse request)
{
  "id": 42,
  "method": "parse",
  "params": { "source": "fn main() {}" }
}

// Plugin → Core (parsed result)
{
  "id": 42,
  "result": {
    "symbols": [{"name": "main", "kind": "Function", "line": 1}]
  }
}
```

### Implementation

#### 1. Plugin Process Manager

```rust
// New: crates/cb-plugins/src/process_manager.rs
pub struct PluginProcessManager {
    // Reuse LSP client infrastructure
    plugins: HashMap<String, PluginProcess>,
}

pub struct PluginProcess {
    name: String,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: AtomicU64,
    pending_requests: Arc<DashMap<u64, oneshot::Sender<PluginResponse>>>,
}

impl PluginProcessManager {
    pub async fn spawn_plugin(&mut self, name: &str, command: &[String]) -> Result<()> {
        let mut child = Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit)
            .spawn()?;

        // Same pattern as LSP client initialization
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        let process = PluginProcess {
            name: name.to_string(),
            child,
            stdin,
            stdout,
            request_id: AtomicU64::new(1),
            pending_requests: Arc::new(DashMap::new()),
        };

        // Spawn reader task (same as LSP client)
        tokio::spawn(read_responses(process.clone()));

        self.plugins.insert(name.to_string(), process);
        Ok(())
    }

    pub async fn call_plugin(&self, name: &str, method: &str, params: Value) -> Result<Value> {
        let plugin = self.plugins.get(name).ok_or(PluginError::NotFound)?;

        let id = plugin.request_id.fetch_add(1, Ordering::SeqCst);
        let request = PluginRequest { id, method: method.to_string(), params };

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();
        plugin.pending_requests.insert(id, tx);

        // Send request
        plugin.send_request(&request).await?;

        // Wait for response (with timeout)
        let response = tokio::time::timeout(Duration::from_secs(30), rx).await??;

        response.result.ok_or(PluginError::from(response.error))
    }
}
```

#### 2. Plugin Binary Structure

```rust
// External repo: cb-lang-rust/src/main.rs
use cb_plugin_api::LanguagePlugin;
use cb_protocol::{PluginRequest, PluginResponse};

#[tokio::main]
async fn main() {
    let plugin = cb_lang_rust::RustPlugin::new();
    let server = PluginServer::new(plugin);
    server.run().await;  // Read stdin, write stdout
}

struct PluginServer<P: LanguagePlugin> {
    plugin: P,
}

impl<P: LanguagePlugin> PluginServer<P> {
    async fn run(&self) {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            if reader.read_line(&mut line).await? == 0 {
                break; // EOF
            }

            let request: PluginRequest = serde_json::from_str(&line)?;
            let response = self.handle_request(request).await;

            println!("{}", serde_json::to_string(&response)?);
        }
    }

    async fn handle_request(&self, req: PluginRequest) -> PluginResponse {
        let result = match req.method.as_str() {
            "parse" => {
                let source: String = serde_json::from_value(req.params)?;
                let parsed = self.plugin.parse(&source).await?;
                serde_json::to_value(parsed)?
            }
            "analyze_manifest" => {
                let path: PathBuf = serde_json::from_value(req.params)?;
                let manifest = self.plugin.analyze_manifest(&path).await?;
                serde_json::to_value(manifest)?
            }
            // ... other methods
            _ => return PluginResponse::error(req.id, "Unknown method"),
        };

        PluginResponse { id: req.id, result: Some(result), error: None }
    }
}
```

#### 3. Plugin Configuration

```toml
# ~/.codebuddy/config.json (extends existing LSP config)
{
  "servers": [
    {
      "extensions": ["ts", "tsx"],
      "command": ["typescript-language-server", "--stdio"]
    }
  ],
  "language_plugins": [
    {
      "name": "rust",
      "extensions": ["rs"],
      "command": ["cb-lang-rust-plugin"],  // Installed binary
      "manifest_filename": "Cargo.toml"
    },
    {
      "name": "python",
      "extensions": ["py"],
      "command": ["cb-lang-python-plugin"],
      "manifest_filename": "pyproject.toml"
    }
  ]
}
```

#### 4. CLI Plugin Management

```bash
# Install plugin (downloads pre-built binary)
codebuddy plugin install rust
# → Downloads cb-lang-rust-plugin from GitHub releases
# → Places in ~/.codebuddy/plugins/bin/
# → Updates config.json

# Alternative: Build from source
codebuddy plugin install rust --from-source
# → Clones https://github.com/goobits/cb-lang-rust
# → Runs cargo build --release
# → Copies binary to ~/.codebuddy/plugins/bin/

# List installed plugins
codebuddy plugin list
# Installed language plugins:
#   rust (1.2.0) - Extensions: rs
#   python (0.9.0) - Extensions: py, pyi
```

### Security Model

**Process Isolation:**
- Each plugin runs in separate process (standard OS process isolation)
- Crashed plugin cannot corrupt core
- Malicious plugin limited to user permissions (same as any installed binary)

**Verification:**
```bash
# Download plugin
curl -L https://github.com/goobits/cb-lang-rust/releases/download/v1.0.0/cb-lang-rust-plugin-linux-x64.tar.gz -o plugin.tar.gz

# Verify checksum (published in release notes)
echo "abc123... plugin.tar.gz" | sha256sum -c

# Extract and install
tar xzf plugin.tar.gz -C ~/.codebuddy/plugins/bin/
chmod +x ~/.codebuddy/plugins/bin/cb-lang-rust-plugin
```

**Sandboxing (Optional):**
- Run plugins in containers (Docker, podman)
- Use seccomp/AppArmor to restrict syscalls
- Limit filesystem access (read-only project dir)

### Migration Path

#### Phase 1: Protocol Definition ✅ COMPLETE

**Completed:** 2025-10-10

- ✅ Define JSON-RPC protocol for LanguagePlugin trait
- ✅ Add `PluginProcessManager` to `cb-plugins`
- ✅ Create `PluginServer` scaffolding in `cb-plugin-api`
- ✅ Create RPC adapter layer to bridge external plugins to LanguagePlugin trait
- ✅ Add configuration structures for external plugins
- ✅ Create dual registry builders (sync/async) for backward compatibility
- ✅ Fix all compilation errors and achieve clean build

**Deliverables:**
- ✅ `crates/cb-protocol/src/plugin_protocol.rs` (56 LOC - request/response types)
- ✅ `crates/cb-plugins/src/process_manager.rs` (187 LOC - process supervisor)
- ✅ `crates/cb-plugin-api/src/server.rs` (112 LOC - plugin binary scaffolding)
- ✅ `crates/cb-plugins/src/rpc_adapter.rs` (74 LOC - RPC adapter)
- ✅ `crates/cb-lang-rust/src/main.rs` (28 LOC - proof-of-concept standalone plugin)
- ✅ `crates/cb-core/src/config.rs` - External plugin config structures
- ✅ `crates/cb-services/src/services/registry_builder.rs` - Dual sync/async builders

**Implementation Details:**

1. **Plugin Protocol** (`cb-protocol/src/plugin_protocol.rs`):
   ```rust
   #[derive(Debug, Serialize, Deserialize)]
   pub struct PluginRequest {
       pub id: u64,
       pub method: String,
       pub params: Value,
   }

   #[derive(Debug, Serialize, Deserialize)]
   pub struct PluginResponse {
       pub id: u64,
       pub result: Option<Value>,
       pub error: Option<PluginError>,
   }
   ```

2. **Process Manager** (`cb-plugins/src/process_manager.rs`):
   - Manages external plugin process lifecycle (spawn, communicate, cleanup)
   - Uses tokio async I/O for stdio communication
   - Implements request correlation with oneshot channels
   - Handles concurrent requests with DashMap

3. **Plugin Server** (`cb-plugin-api/src/server.rs`):
   - Wraps LanguagePlugin trait implementations as JSON-RPC servers
   - Reads requests from stdin, dispatches to plugin, writes responses to stdout
   - Provides scaffolding for standalone plugin binaries

4. **RPC Adapter** (`cb-plugins/src/rpc_adapter.rs`):
   - Bridges external RPC plugins to internal LanguagePlugin trait
   - Transparent translation between trait calls and JSON-RPC requests

5. **Registry Builder** (`cb-services/src/services/registry_builder.rs`):
   - Sync version: Built-in plugins only (for test compatibility)
   - Async version: Spawns external plugins from config
   - Uses `Box::leak` for static string conversion in external plugin metadata

6. **Configuration** (`cb-core/src/config.rs`):
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ExternalPluginConfig {
       pub name: String,
       pub extensions: Vec<String>,
       pub command: Vec<String>,
       pub manifest_filename: String,
   }
   ```

**Technical Decisions:**
- **Dual registry builders**: Created both sync and async versions to maintain backward compatibility with existing test infrastructure
- **Box::leak for static strings**: Used to convert owned strings to `&'static str` for LanguageMetadata (required by trait)
- **Runtime::block_on for static initialization**: Used in test support for synchronous static initialization
- **DashMap for request tracking**: Concurrent hashmap for tracking pending requests by ID
- **Tokio async runtime**: Used for plugin process management and async I/O

**Files Modified:**
- `crates/cb-plugin-api/Cargo.toml` - Added tokio and tracing dependencies
- `crates/cb-plugins/Cargo.toml` - Added dashmap dependency
- `crates/cb-plugin-api/src/lib.rs` - Added Serialize/Deserialize derives to protocol types
- `crates/cb-handlers/src/language_plugin_registry.rs` - Made new() async
- `crates/cb-server/src/test_helpers.rs` - Made test helpers async
- `crates/cb-test-support/src/harness/plugin_discovery.rs` - Used Runtime::block_on
- `crates/cb-services/src/services/app_state_factory.rs` - Uses async registry builder

**Merge Information:**
- Branch: `feat/plugin-system`
- Merged to: `main`
- Date: 2025-10-10
- Files changed: 22 files (+641 insertions, -82 deletions)
- Build status: ✅ Clean (cargo check --workspace passes)

#### Phase 2: Convert One Plugin
- Extract `cb-lang-rust` to separate repo
- Build as standalone binary with `PluginServer`
- Test via process manager in core

**Deliverables:**
- [ ] External repo: `cb-lang-rust` with `main.rs` entry point
- [ ] GitHub Actions: build for Linux/macOS/Windows
- [ ] Published to GitHub Releases

#### Phase 3: Registry Integration
- Modify `registry_builder.rs` to spawn plugin processes
- Adapt existing `LanguagePlugin` calls to RPC
- Add fallback to built-in plugins if process spawn fails

**Deliverables:**
- [ ] `registry_builder.rs` spawns external plugins
- [ ] Adapter layer: `LanguagePlugin` trait → RPC calls
- [ ] Graceful fallback to built-in plugins

#### Phase 4: CLI Plugin Management
- Add `codebuddy plugin install/list/remove`
- Download pre-built binaries from GitHub releases
- Update config.json automatically

**Deliverables:**
- [ ] `apps/codebuddy/src/commands/plugin.rs`
- [ ] Binary download + verification
- [ ] Config management

#### Phase 5: Migrate Remaining Plugins
- Extract remaining plugins to separate repos
- Retire built-in plugins (optional: keep Rust/Go as fallback)
- Update documentation

**Deliverables:**
- [ ] All plugins externalized
- [ ] Core binary shrinks from ~30MB to <10MB
- [ ] Plugin ecosystem documentation

---

## ✅ RECOMMENDED APPROACH 2: Incremental crates.io + Feature Flags

### Architecture: Gradual Decoupling

**If RPC is too complex, start simpler:**

```
Phase 1: Publish Plugins to crates.io
Phase 2: CLI Manages Cargo Features
Phase 3: Eventually: Dynamic Loading (when ABI story matures)
```

### Implementation

#### 1. Publish Language Plugins

```toml
# External repo: cb-lang-rust/Cargo.toml
[package]
name = "cb-lang-rust"
version = "1.0.0"
description = "Rust language plugin for Codebuddy"
repository = "https://github.com/goobits/cb-lang-rust"

[dependencies]
cb-plugin-api = "1.0"  # Published to crates.io
syn = "2.0"
quote = "1.0"
```

**Publish:**
```bash
cd cb-lang-rust
cargo publish
```

#### 2. Core Depends on Published Crates

```toml
# crates/cb-services/Cargo.toml
[dependencies]
cb-lang-rust = { version = "1.0", optional = true }
cb-lang-go = { version = "1.0", optional = true }
cb-lang-typescript = { version = "1.0", optional = true }

[features]
default = ["lang-rust", "lang-go", "lang-typescript"]
lang-rust = ["dep:cb-lang-rust"]
lang-go = ["dep:cb-lang-go"]
lang-typescript = ["dep:cb-lang-typescript"]
all-languages = ["lang-rust", "lang-go", "lang-typescript", "lang-python", "lang-java"]
```

#### 3. CLI Manages Dependencies

```bash
# User wants to add Python support
codebuddy plugin add python

# CLI runs:
# 1. cargo add cb-lang-python --optional
# 2. Update Cargo.toml features: lang-python = ["dep:cb-lang-python"]
# 3. cargo build --features lang-python
# 4. Restart codebuddy

# User wants to remove Java support
codebuddy plugin remove java
# → cargo remove cb-lang-java
# → Remove from features
# → cargo build
```

### Benefits Over RPC

**Pros:**
- No process management complexity
- Native performance (in-process)
- Simpler mental model (standard Cargo workflow)
- Familiar to Rust developers

**Cons:**
- Still requires recompiling core
- Not true "install and use" UX
- All plugins must use same Rust toolchain version
- Cannot mix Rust versions (e.g., plugin on 1.75, core on 1.76)

### Comparison Table

| Aspect | RPC Plugins | crates.io + Features |
|--------|-------------|----------------------|
| **Runtime flexibility** | ✅ Add without recompile | ❌ Must recompile |
| **ABI stability** | ✅ JSON protocol | ⚠️ Same toolchain required |
| **Performance** | ⚠️ IPC overhead (~1-5ms/call) | ✅ Native (zero overhead) |
| **Crash isolation** | ✅ Separate process | ❌ Shared address space |
| **Complexity** | ⚠️ Process manager, protocol | ✅ Standard Cargo |
| **Multi-language** | ✅ Any language | ❌ Rust only |
| **User experience** | ✅ `plugin install` works instantly | ⚠️ `plugin add` triggers rebuild |
| **Ecosystem** | ✅ True external plugins | ⚠️ Still coupled via cargo |

---

## RECOMMENDATION (Revised)

### Primary: Out-of-Process RPC Plugins

**Why:**
- **Battle-tested pattern:** Codebuddy already manages LSP server processes
- **Solves ABI problem:** JSON-RPC is stable across all versions/toolchains
- **Codex is right:** "Reuses battle-tested patterns" from existing LSP client
- **Minimal new code:** ~500 LOC (protocol + process manager)

**Start with:**
1. Define plugin JSON-RPC protocol (~100 LOC)
2. Extract `PluginProcessManager` from LSP client (~200 LOC)
3. Create `PluginServer` scaffolding (~100 LOC)
4. Convert Rust plugin as proof-of-concept (~100 LOC)
5. Add CLI commands (~300 LOC)

**Total: ~800 LOC, 90% reusing existing patterns**

### Fallback: crates.io + CLI-Managed Features

**Use if:**
- Team wants gradual migration
- RPC overhead is unacceptable for performance
- Prefer in-process plugins despite toolchain coupling

**Start with:**
1. Publish `cb-plugin-api` to crates.io
2. Publish one plugin (e.g., `cb-lang-rust`) to crates.io
3. Add `codebuddy plugin add/remove` CLI (~200 LOC)
4. CLI modifies Cargo.toml and triggers rebuild
5. Iterate on remaining plugins

**Total: ~400 LOC, but still requires user recompilation**

---

## Addressing Codex's Concerns

### 1. ABI Stability ✅ FIXED

**Original Problem:**
```rust
// UNSOUND - trait object layout is unstable
extern "C" fn _plugin_create() -> *mut dyn LanguagePlugin
```

**Solution (RPC):**
```rust
// SOUND - JSON over stdio, no ABI coupling
let response = plugin_process.call("parse", json!({"source": source})).await?;
```

**Solution (crates.io):**
- Still in-process, but versioned via crates.io
- Users must use same toolchain (acceptable tradeoff)

### 2. String/Vec FFI ✅ FIXED

**Original Problem:**
```rust
// UNSOUND - allocator mismatch
pub struct PluginInfo {
    pub name: String,  // ❌ Different allocators
}
```

**Solution (RPC):**
```json
// SOUND - serialized as JSON
{"name": "Rust", "version": "1.0.0"}
```

**Solution (crates.io):**
- No FFI boundary, problem doesn't exist

### 3. Security / Trusted Hashes ✅ ADDRESSED

**Original Gap:** "No story for trusted checksums"

**Solution:**
```bash
# Plugin releases include checksums.txt (signed by GitHub Actions)
# Download and verify before install

# Example: .github/workflows/release.yml
- name: Generate checksums
  run: |
    sha256sum cb-lang-rust-* > checksums.txt
    gpg --detach-sign checksums.txt  # Optional: GPG signing
```

**Installation flow:**
1. Download binary from GitHub releases
2. Download `checksums.txt` (same release)
3. Verify checksum matches: `sha256sum -c checksums.txt`
4. Optional: Verify GPG signature
5. Install to `~/.codebuddy/plugins/bin/`

**Revocation:**
- Plugin registry (future) maintains allowlist/blocklist
- CLI checks registry before installation
- Users can pin versions in config: `"version": "=1.2.0"`

### 4. Offline / Bundled Plugins ✅ ADDRESSED

**Original Gap:** "No story for bundling if dynamic loading slips"

**Solution:**
```toml
# Keep essential plugins as optional built-ins
[features]
default = ["builtin-rust", "builtin-go"]  # Core languages bundled
builtin-rust = ["cb-lang-rust"]  # Compiled in
builtin-go = ["cb-lang-go"]

# At runtime:
if external_plugin_exists("rust") {
    use_external_plugin();
} else {
    use_builtin_plugin();  // Fallback for offline
}
```

**Benefits:**
- Offline users get Rust/Go out-of-box
- Online users can upgrade to latest external plugin
- Gradual migration: built-ins → external over time

---

## Code Size Estimate (Revised)

### RPC Approach

**Core Implementation:**
- `cb-protocol/src/plugin_protocol.rs`: ~100 LOC (request/response types)
- `cb-plugins/src/process_manager.rs`: ~200 LOC (process supervisor, extracted from LSP client)
- `cb-plugin-api/src/server.rs`: ~100 LOC (plugin binary scaffolding)
- `registry_builder.rs` modifications: ~100 LOC (spawn processes instead of in-process)
- CLI plugin commands: ~300 LOC (install/list/remove)

**Total: ~800 lines of code** (mostly reusing existing LSP client patterns)

**Per-Plugin Changes:**
- Add `main.rs` with `PluginServer`: ~50 LOC
- Update `Cargo.toml`: ~5 LOC

**Total per plugin: ~55 LOC**

### crates.io Approach

**Core Implementation:**
- Publish `cb-plugin-api` to crates.io: ~0 LOC (just `cargo publish`)
- CLI commands (`plugin add/remove`): ~200 LOC (cargo commands + Cargo.toml editing)

**Total: ~200 lines of code**

**Per-Plugin Changes:**
- Publish to crates.io: ~0 LOC (just `cargo publish`)
- Update docs: ~10 LOC

**Total per plugin: ~10 LOC**

---

## Migration Risk Assessment

### RPC Approach

**Risks:**
- IPC overhead may be noticeable for high-frequency operations (parse every keystroke)
- Process spawn latency (~50ms) on first use
- More complex debugging (multi-process)

**Mitigations:**
- Pre-spawn plugins on startup (like LSP servers)
- Cache parsed results (already done for AST)
- Batch requests where possible

**Likelihood:** Low risk (LSP servers prove this pattern works)

### crates.io Approach

**Risks:**
- User must rebuild core after adding plugin
- Toolchain version coupling (plugin on 1.75 won't build with core on 1.76)
- Not "true" external plugins (still cargo workspace semantics)

**Mitigations:**
- Good UX in CLI (`codebuddy plugin add` handles rebuild)
- Document required Rust version in plugin manifest
- Provide pre-built binaries for core + common plugins

**Likelihood:** Medium risk (user frustration with rebuilds)

---

## Success Metrics

### RPC Approach

1. **Plugin isolation:** Crashed plugin doesn't crash core (test with intentional panic)
2. **Installation UX:** `codebuddy plugin install rust` → works in <30s (no rebuild)
3. **Performance:** IPC overhead <5ms per plugin call (99th percentile)
4. **Binary size:** Core binary <10MB (vs ~30MB today)
5. **Ecosystem:** 3+ community plugins within 6 months

### crates.io Approach

1. **Installation UX:** `codebuddy plugin add rust` → rebuild completes in <2min
2. **Version independence:** Plugin releases don't block core releases
3. **Binary size:** Same as today (~30MB) but user can customize
4. **Ecosystem:** All existing plugins published to crates.io

---

## References

**Out-of-Process Plugin Systems:**
- **Language Server Protocol:** JSON-RPC over stdio, proven at scale (VSCode, Neovim, etc.)
- **Neovim RPC:** msgpack-rpc for plugins, handles crashes gracefully
- **Tailscale:** Go subprocess plugins with JSON-RPC
- **HashiCorp:** go-plugin framework (gRPC-based subprocess plugins)

**Rust crates.io Ecosystem:**
- **rustfmt, clippy:** Distributed as separate crates, invoked by cargo
- **cargo-make, cargo-nextest:** Cargo extensions, installed via `cargo install`
- **rustls, hyper:** Core dependencies versioned independently

**Rust RPC/IPC:**
- `serde_json` - JSON serialization (already in use)
- `tokio::process` - Process management (already in use)
- `tonic` / `tarpc` - gRPC/RPC frameworks (if JSON-RPC insufficient)

**Documentation:**
- [LSP Specification](https://microsoft.github.io/language-server-protocol/) - JSON-RPC protocol design
- [Process Isolation in Rust](https://blog.yoshuawuyts.com/rust-streams/) - Process supervision patterns
- [Cargo Book: Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html) - crates.io workflow
