# Codebuddy Project – "Foundations First" Strategic Plan

## Executive Summary

This document outlines the "Foundations First" strategic plan for the Codebuddy project, synthesizing proposals from Gemini, Bob, and Wendy. It prioritizes immediate architectural unification to eliminate legacy code, followed by modernization of core systems and feature enhancements. This approach ensures all new development occurs on a clean, stable, and future-proof foundation.

---

## Phase 1: Foundational Cleanup & Safety (Weeks 1-2)

**Goal:** Eliminate all handler-related legacy code and establish a single, unified architectural pattern, secured by a comprehensive safety net.

### 1.1. Tool Registration Safety Net

- **Implement an integration test** (`crates/cb-server/tests/tool_registration_test.rs`).
- This test will assert that all 42 expected tools are registered, using the exact `EXPECTED_TOOLS` array provided.
- It will fail initially and serve as our guardrail for the unification process, ensuring no tools are lost during refactoring. Its final passing will signify the success of this phase.

**Full EXPECTED_TOOLS array and test code:**
```rust
#[tokio::test]
async fn test_all_42_tools_are_registered() {
    let dispatcher = create_test_dispatcher().await;
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;
    let registered_tools = registry.list_tools();

    const EXPECTED_TOOLS: [&str; 42] = [
        // Navigation (14)
        "find_definition", "find_references", "find_implementations",
        "find_type_definition", "get_document_symbols", "search_workspace_symbols",
        "get_hover", "get_completions", "get_signature_help", "get_diagnostics",
        "prepare_call_hierarchy", "get_call_hierarchy_incoming_calls",
        "get_call_hierarchy_outgoing_calls", "web_fetch",
        // Editing (10)
        "rename_symbol", "rename_symbol_strict", "organize_imports",
        "get_code_actions", "format_document", "extract_function",
        "inline_variable", "extract_variable", "fix_imports",
        "rename_symbol_with_imports",
        // File Operations (6)
        "create_file", "read_file", "write_file", "delete_file",
        "rename_file", "list_files",
        // Workspace (5)
        "rename_directory", "analyze_imports", "find_dead_code",
        "update_dependencies", "extract_module_to_package",
        // Advanced (2)
        "apply_edits", "batch_execute",
        // Lifecycle (3)
        "notify_file_opened", "notify_file_saved", "notify_file_closed",
        // System (2)
        "health_check", "system_status"
    ];

    fn find_missing(expected: &[&str], actual: &[String]) -> Vec<&str> {
        expected.iter().filter(|tool| !actual.contains(&tool.to_string())).copied().collect()
    }
    fn find_extra(expected: &[&str], actual: &[String]) -> Vec<String> {
        actual.iter().filter(|tool| !expected.contains(&tool.as_str())).cloned().collect()
    }

    assert_eq!(registered_tools.len(), 42,
        "Expected 42 tools, found {}
Missing: {:?}
Extra: {:?}",
        registered_tools.len(),
        find_missing(&EXPECTED_TOOLS, &registered_tools),
        find_extra(&EXPECTED_TOOLS, &registered_tools)
    );
}
```

### 1.2. Unify Tool Handler Architecture

- **Execute trait unification immediately.**
- **Eliminate `ToolHandlerAdapter` wrappers** and the legacy handler trait.
- **Update all existing handlers** (`FileOperationHandler`, `RefactoringHandler`, etc.) to use the single, modern, async-compatible trait. This resolves the primary source of technical debt upfront.

---

## Phase 2: Modernize the Machinery (Weeks 3-4)

**Goal:** Build new, ergonomic, and intelligent systems on the clean foundation laid in Phase 1.

### 2.1. Macro-Based Tool Registration

- **Implement a simplified, powerful Rust macro** (`register_handlers!`) in `crates/cb-server/src/handlers/macros.rs`.
- Built for the single unified trait, the macro will be cleaner and require no future rework.
- It will still wrap handlers in `Arc`, auto-count tools, and handle duplicate registration errors, but without the complexity of managing a `legacy` branch.

**Simplified macro usage:**
```rust
// Context setup before macro
let handler_context = Arc::new(super::tools::ToolHandlerContext {
    app_state: self.app_state.clone(),
    plugin_manager: self.plugin_manager.clone(),
    lsp_adapter: self.lsp_adapter.clone(),
});

// NEW macro usage (single block)
register_handlers! {
    registry,
    handler_context => {
        FileOperationHandler,
        RefactoringHandler,
        SystemHandler,
        LifecycleHandler,
        WorkspaceHandler,
        AdvancedHandler,
        // ...all other handlers now conform to the same pattern
    }
}
```

### 2.2. Configurable, Priority-Based Plugin Selection

- **Modify `PluginMetadata`** to include a `priority: u32` field.
- **Extend `AppConfig`** to support `default_order` and `per_language` plugin preferences.
- **Rewrite `find_best_plugin` logic** to use the multi-tiered selection model (language config -> global order -> metadata priority).
- **Raise an `AmbiguousPluginSelection` error** on ties.

### 2.3. Generalize Tool Scope and Refactor Registry Logic

- **Add tool scope** (file-scoped vs. workspace-scoped) to plugin capabilities to eliminate hardcoded special cases in the registry.
- **Refactor `find_best_plugin`** to use this scope for more robust routing.

### 2.4. Modularize Large Functions

- **Break up long functions** in the plugin registry and manager into smaller, single-purpose helpers to improve readability and minimize lock contention.

---

## Phase 3: Validate and Extend (Weeks 5-6)

**Goal:** Prove the new architecture's performance and begin extending the system with new features.

### 3.1. Launch Full Benchmarking Suite

- **Reactivate and refactor** `dispatch_benchmark.rs`.
- **Add scenario-driven benchmarks** for plugin selection, dispatch latency, concurrency, and initialization overhead.
- **Integrate statistical analysis** and add a CI job to catch performance regressions of >5% automatically.

### 3.2. Integrate Workspace Extraction Enhancements

- With the core architecture now stable and unified, **integrate Wendy’s enhancements** to the `extract_module_to_package` tool.

---

## Phase 4: Documentation & Polish (Ongoing)

- **Update all documentation** (`ARCHITECTURE.md`, code comments) to reflect the unified architecture, new macro, and plugin selection system.
- **Add CLI diagnostics** (`codebuddy list-tools`) for listing all registered tools and their handlers.
- **Provide onboarding notes** for new contributors.

---

## Rollout & Implementation Order

| Phase         | Timeframe | Key Action Items                                                              |
|---------------|-----------|-------------------------------------------------------------------------------|
| **Phase 1**   | Weeks 1-2 | Implement safety net test; **Unify all handler traits.**                        |
| **Phase 2**   | Weeks 3-4 | Implement simplified macro; Implement full plugin selection & generalization. |
| **Phase 3**   | Weeks 5-6 | Launch full benchmark suite & CI integration; Integrate workspace extraction. |
| **Phase 4**   | Ongoing   | Update documentation and CLI diagnostics.                                     |

---

## Success Metrics

| Metric                   | Before | After Phase 1 (Immediate) | After Full Plan |
|--------------------------|--------|---------------------------|-----------------|
| Registration lines       | 50     | 50                        | 12              |
| **Handler traits**       | 2      | **1**                     | **1**           |
| **Adapter wrappers**     | 1      | **0**                     | **0**           |
| CI catches missing tools | ❌      | ✅                        | ✅              |
| Registry error on duplicate | ❌   | ✅                        | ✅              |
| Plugin selection logic   | Manual | Manual                    | Configurable    |
| Code quality score       | 6.5/10 | 8.5/10                    | 9.5/10          |
| Benchmark/CI coverage    | Minimal| Minimal                   | Full            |
| Tool listing CLI         | ❌      | ❌                        | ✅              |

---

## Strategic Recommendations

- **Adopt the "Foundations First" strategy:** Unify the core architecture immediately to eliminate technical debt.
- **Leverage the safety net:** Use the registration test as a guarantee of correctness during the initial refactoring phase.
- **Build modern systems on the unified core:** Implement the macro and plugin enhancements only after the foundation is clean.
- This approach minimizes risk, eliminates rework, and results in a higher-quality, legacy-free codebase faster.

---
(Sections 9, 11, and Appendix remain largely the same but are now interpreted in the context of this new, superior plan.)