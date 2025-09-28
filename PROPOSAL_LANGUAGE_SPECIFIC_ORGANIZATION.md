# Proposal: Language-Specific Tool Organization (Incremental Approach)

**Status:** Analysis Complete - Incremental Approach Recommended
**Target:** Restructure existing MCP tools by language instead of function
**Scope:** Evolutionary improvement to existing architecture

## Executive Summary

This proposal addresses the hard-coded mappings and poor organization in the current MCP tool system through a **language-specific reorganization** approach. This takes an incremental approach that delivers immediate value while preserving existing infrastructure.

## Current Architecture Problems

### Identified Issues

1. **Hard-coded mappings**: 27 hard-coded tool operation mappings in `mcp_dispatcher.rs:56-87`
2. **Static LSP method mappings**: Fixed method translations in `lsp/manager.rs:101-116`
3. **Function-based organization**: Tools organized by function (navigation, intelligence) rather than language
4. **Scalability challenges**: Adding new languages requires touching multiple files
5. **Inconsistent patterns**: Language-specific handlers exist in `dependency_handlers/` but core tools remain function-based

### Current Codebase Scale
- **20 non-test files** in `mcp_tools/` (~6,177 lines of code)
- **11 test files** requiring migration
- **27 hard-coded mappings** to replace

## Proposed Solution: Phased Language-Specific Reorganization

### Phase 1: Fix Mappings (Low Risk, High Value)
**Goal**: Eliminate hard-coded mappings while preserving current file structure

**Changes**:
```rust
// Create configurable language configuration system
pub struct LanguageConfig {
    pub mcp_to_lsp_mappings: HashMap<String, String>,
    pub operation_types: HashMap<String, OperationType>,
    pub language_specific_settings: Value,
}

// Replace hard-coded tool registry with dynamic system
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
    operation_mappings: HashMap<String, OperationType>,
}
```

**Files Modified**:
- `mcp_dispatcher.rs` - Replace `initialize_tool_operations()` with dynamic registry
- `lsp/manager.rs` - Replace `mcp_to_lsp_request()` with configurable mappings
- New file: `language_config.rs` - Configuration-driven method mappings

**Benefits**:
- ✅ Immediate elimination of hard-coded mappings
- ✅ Makes system fully configurable
- ✅ Zero reorganization risk
- ✅ Preserves all existing infrastructure

### Phase 2: Proof of Concept Migration (Medium Risk, Learning Phase)
**Goal**: Validate language-specific approach with one language

**Proposed Structure**:
```
rust/crates/cb-server/src/handlers/language_tools/
├── mod.rs                 # Language-based tool registration
├── typescript.rs          # TypeScript/JavaScript tools
├── common.rs             # Language-agnostic tools
└── tool_registry.rs      # Dynamic tool registration system
```

**TypeScript Module Implementation**:
```rust
// language_tools/typescript.rs
pub fn register_typescript_tools(dispatcher: &mut McpDispatcher) {
    // All TypeScript/JavaScript specific tools
    register_navigation_tools(dispatcher);  // find_definition, find_references
    register_intelligence_tools(dispatcher); // hover, completion, signature
    register_refactoring_tools(dispatcher);  // rename, extract, organize imports

    // TypeScript-specific optimizations
    register_typescript_specific_tools(dispatcher); // auto-imports, type inference
}

fn register_typescript_specific_tools(dispatcher: &mut McpDispatcher) {
    dispatcher.register_tool("typescript_infer_types".to_string(), |app_state, args| async move {
        // TypeScript-specific type inference implementation
        // Can leverage tsconfig.json, module resolution, etc.
    });
}
```

**Benefits**:
- ✅ Proves the concept works
- ✅ Limited scope - can be reverted if issues arise
- ✅ Identifies unforeseen challenges
- ✅ Enables TypeScript-specific optimizations

### Phase 3: Complete Reorganization (Higher Risk, Full Vision)
**Goal**: Apply learnings to complete the language-specific organization

**Full Structure**:
```
rust/crates/cb-server/src/handlers/language_tools/
├── mod.rs                 # Language-based tool registration
├── typescript.rs          # TypeScript/JavaScript tools + optimizations
├── python.rs             # Python tools + virtual env handling
├── golang.rs             # Go tools + module awareness
├── rust_lang.rs          # Rust tools + Cargo integration
├── common.rs             # Language-agnostic tools
└── tool_registry.rs      # Dynamic tool registration system
```

**Language-Specific Optimizations**:
- **TypeScript**: Auto-imports, type inference, TSConfig awareness
- **Python**: Virtual environment detection, type stub support
- **Go**: Module workspace management, interface suggestions
- **Rust**: Cargo integration, macro expansion support

## Migration Strategy

### File Changes Summary
- **🆕 8 new files**: Language tool modules and registry system
- **✏️ 5 modified files**: Update dispatcher, LSP manager, and module exports
- **🗑️ 6 deleted files**: Remove function-based tool modules

### Risk Assessment

**Benefits**:
- ✅ Zero hard-coded mappings - all method mappings become configurable
- ✅ Language-specific organization improves maintainability
- ✅ Adding new language = one new file
- ✅ Cleaner separation between language-agnostic and specific tools
- ✅ Preserves existing infrastructure (dispatcher, LSP manager, etc.)

**Risks**:
- ⚠️ Large refactoring scope (~6,177 lines across 20+ files)
- ⚠️ High regression risk during migration
- ⚠️ Complex test migration (11+ test files to update)
- ⚠️ "Big bang" approach inappropriate for production system

## Recommendation: Incremental Approach

**Start with Phase 1** immediately:
- Low risk, high value
- Fixes core architectural problems
- Enables future phases
- Delivers immediate configurability benefits

**Evaluate Phase 2** based on:
- Phase 1 implementation success
- Team capacity and priorities
- User feedback on configurability improvements

## Implementation Checklist

### Phase 1: Fix Mappings
- [ ] Create `LanguageConfig` system for configurable mappings
- [ ] Implement dynamic `ToolRegistry`
- [ ] Replace hard-coded mappings in `mcp_dispatcher.rs`
- [ ] Replace static LSP mappings in `lsp/manager.rs`
- [ ] Add configuration validation and error handling
- [ ] Update tests for new configuration system

### Phase 2: TypeScript Proof of Concept
- [ ] Create `language_tools/` module structure
- [ ] Implement `typescript.rs` with all TS/JS tools
- [ ] Add TypeScript-specific optimizations
- [ ] Migrate TypeScript tests
- [ ] Validate performance and functionality
- [ ] Gather feedback from TypeScript users

### Phase 3: Complete Migration
- [ ] Implement remaining language modules (Python, Go, Rust)
- [ ] Add language-specific optimizations for each
- [ ] Migrate all remaining tests
- [ ] Update documentation and examples
- [ ] Performance validation across all languages

## Success Metrics

- ✅ Zero hard-coded mappings remain in codebase
- ✅ Adding new TypeScript feature requires only `typescript.rs` changes
- ✅ All existing tests pass after each phase
- ✅ Performance impact < 5% per phase
- ✅ Configuration system enables user customization
- ✅ Language-specific optimizations demonstrate clear value

## Conclusion

The language-specific organization approach provides a practical path forward that:

1. **Solves immediate problems** - Eliminates hard-coded mappings in Phase 1
2. **Reduces risk** - Incremental approach with fallback options
3. **Enables learning** - Proves concepts before full commitment
4. **Preserves investment** - Builds on existing infrastructure
5. **Supports future evolution** - Compatible with eventual plugin architecture if needed

This approach balances architectural improvement with production system stability, delivering value at each phase while maintaining pragmatic engineering principles.