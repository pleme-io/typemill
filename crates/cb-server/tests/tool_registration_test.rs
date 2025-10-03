use cb_server::handlers::plugin_dispatcher::create_test_dispatcher;

#[tokio::test]
async fn test_all_44_tools_are_registered() {
    let dispatcher = create_test_dispatcher();
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;
    let registered_tools = registry.list_tools();

    const EXPECTED_TOOLS: [&str; 44] = [
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
        // Editing (10)
        "rename_symbol",
        "rename_symbol_strict",
        "organize_imports",
        "get_code_actions",
        "format_document",
        "extract_function",
        "inline_variable",
        "extract_variable",
        "fix_imports",
        "rename_symbol_with_imports",
        // File Operations (6)
        "create_file",
        "read_file",
        "write_file",
        "delete_file",
        "rename_file",
        "list_files",
        // Workspace (7)
        "rename_directory",
        "analyze_imports",
        "find_dead_code",
        "update_dependencies",
        "extract_module_to_package",
        "update_dependency",
        "batch_update_dependencies",
        // Advanced (2)
        "apply_edits",
        "batch_execute",
        // Lifecycle (3)
        "notify_file_opened",
        "notify_file_saved",
        "notify_file_closed",
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
