use mill_server::handlers::plugin_dispatcher::create_test_dispatcher;

#[tokio::test]
async fn test_all_35_public_tools_are_registered() {
    let dispatcher = create_test_dispatcher().await;
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;
    let registered_tools = registry.list_tools();

    const EXPECTED_TOOLS: [&str; 35] = [
        // Navigation (8) - get_document_symbols moved to internal
        "find_definition",
        "find_references",
        "find_implementations",
        "find_type_definition",
        "search_symbols",
        "get_symbol_info",
        "get_diagnostics",
        "get_call_hierarchy",
        // Refactoring Plans (7)
        "rename.plan",
        "extract.plan",
        "inline.plan",
        "move.plan",
        "reorder.plan",
        "transform.plan",
        "delete.plan",
        // Quick Operations (7) - one-step plan + apply for each refactoring type
        "rename",
        "extract",
        "inline",
        "move",
        "reorder",
        "transform",
        "delete",
        // Workspace (4)
        "workspace.apply_edit",
        "workspace.create_package",
        "workspace.extract_dependencies",
        "workspace.update_members",
        // System (1)
        "health_check",
        // Analysis (8) - Unified Analysis API
        "analyze.quality",
        "analyze.dead_code",
        "analyze.dependencies",
        "analyze.documentation",
        "analyze.structure",
        "analyze.tests",
        "analyze.batch",
        "analyze.module_dependencies",
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

    assert_eq!(
        registered_tools.len(),
        EXPECTED_TOOLS.len(),
        "Expected {} tools, found {}.\nMissing: {:?}\nExtra: {:?}",
        EXPECTED_TOOLS.len(),
        registered_tools.len(),
        find_missing(&EXPECTED_TOOLS, &registered_tools),
        find_extra(&EXPECTED_TOOLS, &registered_tools)
    );

    let missing = find_missing(&EXPECTED_TOOLS, &registered_tools);
    assert!(
        missing.is_empty(),
        "The following tools are missing: {:?}",
        missing
    );
}

#[tokio::test]
async fn test_all_20_internal_tools_are_registered_and_hidden() {
    let dispatcher = create_test_dispatcher().await;
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;

    const EXPECTED_INTERNAL_TOOLS: [&str; 20] = [
        // Lifecycle (3)
        "notify_file_opened",
        "notify_file_saved",
        "notify_file_closed",
        // Internal Editing (1)
        "rename_symbol_with_imports",
        // Internal Workspace (1)
        "apply_workspace_edit",
        // Internal Intelligence (2)
        "get_completions",
        "get_signature_help",
        // Workspace Tools (3) - Made internal, replaced by Unified API
        "move_directory",
        "update_dependencies",
        "update_dependency",
        // File Operations (4) - Made internal, replaced by Unified API
        "create_file",
        "delete_file",
        "rename_file",
        "rename_directory",
        // File Utilities (3) - Made internal, backend use only
        "read_file",
        "write_file",
        "list_files",
        // Structure Analysis (1) - Made internal, replaced by analyze.structure
        "get_document_symbols",
        // Advanced (2) - Made internal, low-level plumbing
        "execute_edits",
        "execute_batch",
    ];

    // 1. Verify they are NOT in the public list
    let public_tools = registry.list_tools();
    for internal_tool in &EXPECTED_INTERNAL_TOOLS {
        assert!(
            !public_tools.contains(&internal_tool.to_string()),
            "Internal tool '{}' should not be in public tool list",
            internal_tool
        );
    }

    // 2. Verify they ARE in the internal list
    let internal_tools = registry.list_internal_tools();
    assert_eq!(
        internal_tools.len(),
        EXPECTED_INTERNAL_TOOLS.len(),
        "Expected {} internal tools, but found {}",
        EXPECTED_INTERNAL_TOOLS.len(),
        internal_tools.len()
    );
    for expected in &EXPECTED_INTERNAL_TOOLS {
        assert!(
            internal_tools.contains(&expected.to_string()),
            "Expected internal tool '{}' not found in internal tool list",
            expected
        );
    }

    // 3. Verify they are still registered in the main registry
    for tool_name in &EXPECTED_INTERNAL_TOOLS {
        assert!(
            registry.has_tool(tool_name),
            "Internal tool '{}' should still be registered in the system",
            tool_name
        );
    }
}