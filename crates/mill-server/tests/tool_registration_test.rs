use mill_handlers::handlers::tool_definitions::{is_public_tool, PUBLIC_TOOLS};
use mill_server::handlers::plugin_dispatcher::create_test_dispatcher;

#[tokio::test]
async fn test_magnificent_seven_tools_registered() {
    let dispatcher = create_test_dispatcher().await;
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;
    let registered_tools = registry.list_tools();

    // System tools - utilities not part of code intelligence
    const SYSTEM_TOOLS: [&str; 4] = [
        "health_check",
        "notify_file_opened",
        "notify_file_saved",
        "notify_file_closed",
    ];

    // Verify Magnificent Seven tools are present (public API)
    for tool in PUBLIC_TOOLS {
        assert!(
            registered_tools.contains(&tool.to_string()),
            "Missing public tool: {}. Registered: {:?}",
            tool,
            registered_tools
        );
    }

    // Verify system tools are present
    for tool in &SYSTEM_TOOLS {
        assert!(
            registered_tools.contains(&tool.to_string()),
            "Missing system tool: {}. Registered: {:?}",
            tool,
            registered_tools
        );
    }
}

#[tokio::test]
async fn test_no_legacy_tools_registered() {
    let dispatcher = create_test_dispatcher().await;
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;

    // Legacy tools that should NOT be registered (replaced by M7)
    const LEGACY_TOOLS: [&str; 18] = [
        // Navigation (replaced by inspect_code)
        "find_definition",
        "find_references",
        "find_implementations",
        "find_type_definition",
        "search_symbols",
        "find_symbol",
        "get_symbol_info",
        "get_diagnostics",
        "get_call_hierarchy",
        // Refactoring (replaced by rename_all, relocate, prune, refactor)
        "rename",
        "extract",
        "inline",
        "move",
        "delete",
        // Workspace (replaced by workspace tool with action parameter)
        "workspace.create_package",
        "workspace.extract_dependencies",
        "workspace.find_replace",
        // File ops (not needed in M7 API)
        "read_file",
    ];

    for tool in &LEGACY_TOOLS {
        assert!(
            !is_public_tool(tool),
            "Legacy tool '{}' should NOT be part of the public API.",
            tool
        );
    }

    // Ensure no legacy tools are exposed as public tools
    let public_registered: Vec<String> = registry
        .list_tools()
        .into_iter()
        .filter(|t| is_public_tool(t))
        .collect();
    for tool in &LEGACY_TOOLS {
        assert!(
            !public_registered.contains(&tool.to_string()),
            "Legacy tool '{}' should NOT be in public tool list. Registered public tools: {:?}",
            tool,
            public_registered
        );
    }
}
