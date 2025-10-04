use cb_server::handlers::plugin_dispatcher::create_test_dispatcher;

#[tokio::test]
async fn test_all_39_public_tools_are_registered() {
    let dispatcher = create_test_dispatcher();
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;
    let registered_tools = registry.list_tools();

    // Note: This tests PUBLIC tools only (visible to AI agents via MCP).
    // Internal tools (lifecycle hooks, etc.) are tested separately.
    const EXPECTED_TOOLS: [&str; 39] = [
        // Navigation (14)
        "find_definition",
        "find_references",
        "find_implementations",
        "find_type_definition",
        "get_document_symbols",
        "search_workspace_symbols",
        "get_hover",
        "get_completions",
        "get_signature_help",
        "get_diagnostics",
        "prepare_call_hierarchy",
        "get_call_hierarchy_incoming_calls",
        "get_call_hierarchy_outgoing_calls",
        "web_fetch",
        // Editing (9) - rename_symbol_with_imports moved to internal
        "rename_symbol",
        "rename_symbol_strict",
        "organize_imports",
        "get_code_actions",
        "format_document",
        "extract_function",
        "inline_variable",
        "extract_variable",
        // File Operations (6)
        "create_file",
        "read_file",
        "write_file",
        "delete_file",
        "rename_file",
        "list_files",
        // Workspace (5) - apply_workspace_edit moved to internal, batch_update_dependencies removed
        "rename_directory",
        "analyze_imports",
        "find_dead_code",
        "update_dependencies",
        "extract_module_to_package",
        "update_dependency",
        // Advanced (2)
        "apply_edits",
        "batch_execute",
        // Lifecycle (0) - All lifecycle tools are now internal
        // System (2)
        "health_check",
        "system_status",
    ];

    fn find_missing<'a>(expected: &'a [&str], actual: &[String]) -> Vec<&'a str> {
        expected
            .iter()
            .filter(|tool| !actual.contains(&tool.to_string()))
            .copied()
            .collect()
    }
    fn find_extra(expected: &[&str], actual: &[String]) -> Vec<String> {
        actual
            .iter()
            .filter(|tool| !expected.contains(&tool.as_str()))
            .cloned()
            .collect()
    }

    // This assertion will fail until the refactoring is complete.
    assert_eq!(
        registered_tools.len(),
        EXPECTED_TOOLS.len(),
        "Expected {} tools, found {}.\nMissing: {:?}\nExtra: {:?}",
        EXPECTED_TOOLS.len(),
        registered_tools.len(),
        find_missing(&EXPECTED_TOOLS, &registered_tools),
        find_extra(&EXPECTED_TOOLS, &registered_tools)
    );

    // Also assert that all expected tools are present
    let missing = find_missing(&EXPECTED_TOOLS, &registered_tools);
    assert!(
        missing.is_empty(),
        "The following tools are missing: {:?}",
        missing
    );
}

#[tokio::test]
async fn test_internal_tools_are_hidden() {
    let dispatcher = create_test_dispatcher();
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;

    // Internal tools that should be hidden from MCP tool listings
    const EXPECTED_INTERNAL_TOOLS: [&str; 5] = [
        "notify_file_opened",
        "notify_file_saved",
        "notify_file_closed",
        "rename_symbol_with_imports",
        "apply_workspace_edit",
    ];

    // Get public tools (should NOT include internal tools)
    let public_tools = registry.list_tools();
    for internal_tool in &EXPECTED_INTERNAL_TOOLS {
        assert!(
            !public_tools.contains(&internal_tool.to_string()),
            "Internal tool '{}' should not be in public tool list",
            internal_tool
        );
    }

    // Get internal tools (should include all expected internal tools)
    let internal_tools = registry.list_internal_tools();
    for expected in &EXPECTED_INTERNAL_TOOLS {
        assert!(
            internal_tools.contains(&expected.to_string()),
            "Expected internal tool '{}' not found in internal tool list",
            expected
        );
    }

    // Verify internal tools are still registered (can be looked up)
    for tool_name in &EXPECTED_INTERNAL_TOOLS {
        assert!(
            registry.has_tool(tool_name),
            "Internal tool '{}' should still be registered in the system",
            tool_name
        );
    }
}
