# Language Server Plugin Architecture Proposal

## Current Architecture Analysis

### Current State
The codebase currently has a **partially modular** architecture but falls short of a true plugin system:

#### Strengths
- ✅ Clean separation between MCP tools and LSP implementation
- ✅ Configuration-driven language server selection
- ✅ Tool registration pattern with `register_all_tools()`
- ✅ Centralized dispatcher pattern

#### Weaknesses (Spider Web Patterns)
- ❌ **Hard-coded MCP-to-LSP method mappings** in `manager.rs:101-116`
- ❌ **Tool operation types hard-coded** in `mcp_dispatcher.rs:56-86`
- ❌ **Direct LSP protocol knowledge** spread across multiple handlers
- ❌ **No language-specific behavior abstraction** - all languages treated identically
- ❌ **Tight coupling** between MCP tools and LSP protocol specifics

### Spider Web Examples

1. **Method Mapping Spider Web** (`manager.rs`):
```rust
// Hard-coded mapping - adding new methods requires modifying core code
match mcp_request.method.as_str() {
    "find_definition" => "textDocument/definition",
    "find_references" => "textDocument/references",
    // ... 6 more hard-coded mappings
}
```

2. **Operation Type Spider Web** (`mcp_dispatcher.rs`):
```rust
// 30+ lines of hard-coded tool categorization
self.tool_operations.insert("find_definition".to_string(), OperationType::Read);
self.tool_operations.insert("rename_symbol".to_string(), OperationType::Refactor);
// ... continues for all tools
```

3. **Cross-Module Dependencies**:
- Intelligence tools directly create LSP requests
- Handlers know about LSP protocol structure
- No abstraction layer between MCP and LSP

## Proposed Plugin Architecture

### Core Principles
1. **Language plugins are self-contained** - no core code modifications needed
2. **Protocol abstraction** - plugins don't need to know LSP details
3. **Capability-based discovery** - plugins declare what they support
4. **Hook-based extensibility** - plugins can extend behavior at key points

### Architecture Components

```
┌─────────────────────────────────────────────────────────────┐
│                      MCP Interface Layer                     │
├─────────────────────────────────────────────────────────────┤
│                    Plugin Manager (Core)                     │
│  - Plugin Discovery & Loading                               │
│  - Capability Registry                                      │
│  - Hook System                                              │
├─────────────────────────────────────────────────────────────┤
│                  Language Plugin Interface                   │
│  trait LanguagePlugin {                                     │
│    capabilities() -> Capabilities                           │
│    handle_request() -> Response                             │
│    configure() -> Config                                    │
│  }                                                          │
├─────────┬─────────┬─────────┬─────────┬───────────────────┤
│   Rust  │TypeScript│ Python │   Go    │   Custom...       │
│  Plugin │ Plugin  │ Plugin │ Plugin  │   Plugins         │
└─────────┴─────────┴─────────┴─────────┴───────────────────┘
```

### Ideal Plugin Structure

```rust
// rust/crates/cb-plugins/src/lib.rs
pub trait LanguagePlugin: Send + Sync {
    /// Plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Supported file extensions
    fn supported_extensions(&self) -> Vec<String>;

    /// Capabilities this plugin provides
    fn capabilities(&self) -> Capabilities;

    /// Handle a code intelligence request
    async fn handle_request(&self, request: PluginRequest) -> Result<PluginResponse>;

    /// Plugin-specific configuration
    fn configure(&self, config: Value) -> Result<()>;

    /// Lifecycle hooks
    fn on_file_open(&self, path: &Path) -> Result<()> { Ok(()) }
    fn on_file_save(&self, path: &Path) -> Result<()> { Ok(()) }
    fn on_file_close(&self, path: &Path) -> Result<()> { Ok(()) }
}

pub struct Capabilities {
    pub navigation: NavigationCapabilities,
    pub editing: EditingCapabilities,
    pub refactoring: RefactoringCapabilities,
    pub intelligence: IntelligenceCapabilities,
    pub custom: HashMap<String, Value>,
}

pub struct PluginRequest {
    pub method: String,
    pub file_path: PathBuf,
    pub params: Value,
}
```

### Example Plugin Implementation

```rust
// rust/crates/cb-plugins/typescript/src/lib.rs
pub struct TypeScriptPlugin {
    lsp_client: Option<LspClient>,
    config: TypeScriptConfig,
}

impl LanguagePlugin for TypeScriptPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "TypeScript Language Plugin",
            version: "1.0.0",
            author: "Codeflow Buddy Team",
        }
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["ts", "tsx", "js", "jsx", "mjs", "cjs"]
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            navigation: NavigationCapabilities {
                go_to_definition: true,
                find_references: true,
                find_implementations: true,
                ..Default::default()
            },
            refactoring: RefactoringCapabilities {
                rename: true,
                extract_function: true,
                organize_imports: true,
                ..Default::default()
            },
            // TypeScript-specific capabilities
            custom: hashmap! {
                "typescript.infer_types" => json!(true),
                "typescript.suggest_imports" => json!(true),
            },
        }
    }

    async fn handle_request(&self, request: PluginRequest) -> Result<PluginResponse> {
        // Plugin handles its own protocol translation
        match request.method.as_str() {
            "find_definition" => self.handle_definition(request).await,
            "typescript.infer_types" => self.handle_type_inference(request).await,
            _ => Err(Error::MethodNotSupported),
        }
    }
}
```

## Migration Plan

### Phase 1: Foundation (Week 1-2)
1. **Create plugin trait system**
   - Define `LanguagePlugin` trait
   - Define capability structures
   - Create plugin registry

2. **Build plugin manager**
   - Dynamic plugin loading
   - Capability-based routing
   - Hook system implementation

### Phase 2: Abstraction Layer (Week 2-3)
1. **Abstract protocol details**
   - Create protocol-agnostic request/response types
   - Move LSP-specific code into LSP adapter plugin
   - Remove hard-coded method mappings

2. **Refactor existing handlers**
   - Convert MCP tools to use plugin interface
   - Remove direct LSP dependencies
   - Use capability queries instead of hard-coded checks

### Phase 3: Plugin Migration (Week 3-4)
1. **Convert existing language support to plugins**
   - TypeScript/JavaScript plugin
   - Python plugin
   - Go plugin
   - Rust plugin

2. **Add plugin-specific features**
   - TypeScript: Type inference, auto-imports
   - Python: Virtual environment handling, type stubs
   - Rust: Cargo integration, macro expansion
   - Go: Module management, interface implementation

### Phase 4: Advanced Features (Week 4-5)
1. **Plugin marketplace infrastructure**
   - Plugin discovery mechanism
   - Version management
   - Dependency resolution

2. **Developer tooling**
   - Plugin template generator
   - Testing framework for plugins
   - Documentation generator

## Benefits of Plugin Architecture

### For Users
- **Easy language addition** - Drop in a plugin, no core changes needed
- **Language-specific features** - Plugins can provide unique capabilities
- **Better performance** - Load only needed plugins
- **Custom workflows** - Organization-specific plugins

### For Developers
- **Clean separation** - No more spider web dependencies
- **Parallel development** - Teams can work on plugins independently
- **Easier testing** - Plugins can be tested in isolation
- **Faster iteration** - Plugin changes don't affect core

### For Maintenance
- **Reduced coupling** - Core doesn't know about specific languages
- **Clear boundaries** - Well-defined plugin interface
- **Version independence** - Plugins can be updated separately
- **Better stability** - Core remains stable while plugins evolve

## Implementation Checklist

- [ ] Define plugin trait and capability structures
- [ ] Create plugin manager with registry
- [ ] Implement plugin loading mechanism
- [ ] Add configuration system for plugins
- [ ] Create protocol abstraction layer
- [ ] Refactor MCP dispatcher to use plugins
- [ ] Convert TypeScript support to plugin
- [ ] Convert Python support to plugin
- [ ] Convert Go support to plugin
- [ ] Add plugin discovery mechanism
- [ ] Create plugin development documentation
- [ ] Build plugin testing framework
- [ ] Implement plugin hooks system
- [ ] Add plugin marketplace infrastructure
- [ ] Create migration guide for existing code

## Risk Mitigation

1. **Backward Compatibility**
   - Keep existing API working during migration
   - Provide adapter for old configuration format
   - Gradual deprecation with clear timeline

2. **Performance**
   - Lazy plugin loading
   - Capability caching
   - Efficient message passing

3. **Security**
   - Plugin sandboxing options
   - Capability-based permissions
   - Signed plugin verification

## Success Metrics

- ✅ Adding a new language requires zero core code changes
- ✅ Plugin can be developed and tested independently
- ✅ Core test suite passes without any language plugins
- ✅ Performance remains within 5% of current implementation
- ✅ 90% reduction in cross-module dependencies

## Conclusion

The current architecture shows good modular design principles but stops short of true plugin architecture. The proposed changes would eliminate the "spider web" patterns by:

1. **Removing hard-coded mappings** - Plugins handle their own protocol translation
2. **Eliminating cross-module dependencies** - Clean plugin interface
3. **Enabling true extensibility** - New languages are just new plugins

This migration would transform the codebase from a modular monolith into a true plugin-based system, making it significantly easier to add new language support and maintain existing functionality.