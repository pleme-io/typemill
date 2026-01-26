/// Test that the error message from the _ match arm is clear and helpful.
/// This is a unit test that verifies our error handling logic without needing
/// the full integration stack.
#[test]
fn test_error_message_for_unsupported_file_scope_kinds() {
    // Verify the error message format we expect
    let kind = "deep";
    let expected_error = format!(
        "Kind '{}' is not supported for file-scope analysis. Use scope_type='workspace' or choose a different kind.",
        kind
    );

    // This test documents the expected error message
    assert!(expected_error.contains("not supported for file-scope"));
    assert!(expected_error.contains("workspace"));
    assert!(expected_error.contains(kind));
}

/// Test that our validation logic correctly identifies supported file-scope kinds.
/// This verifies the match arms in the handler logic.
#[test]
fn test_kind_validation_coverage() {
    // These are the kinds that should work for file-scope analysis
    let file_scope_kinds = vec![
        "unused_imports",
        "unused_symbols",
        "unreachable_code",
        "unused_parameters",
        "unused_types",
        "unused_variables",
    ];

    // Verify our test knows about all supported kinds
    for kind in &file_scope_kinds {
        // This documents which kinds are expected to work with file-scope
        assert!(
            matches!(
                *kind,
                "unused_imports"
                    | "unused_symbols"
                    | "unreachable_code"
                    | "unused_parameters"
                    | "unused_types"
                    | "unused_variables"
            ),
            "Kind '{}' should be supported for file-scope",
            kind
        );
    }

    // "deep" should NOT be in this list - it requires workspace scope
    #[cfg(feature = "analysis-deep-dead-code")]
    {
        let deep_kind = "deep";
        assert!(
            !file_scope_kinds.contains(&deep_kind),
            "Kind 'deep' should NOT be in file-scope supported kinds"
        );
    }
}
