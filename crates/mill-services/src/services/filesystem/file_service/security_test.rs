use super::tests::tests::create_test_service;
use mill_foundation::validation::ValidationConfig;
use tempfile::TempDir;

#[tokio::test]
async fn test_command_injection_repro() {
    let temp_dir = TempDir::new().unwrap();
    let (mut service, _queue) = create_test_service(&temp_dir);

    // Enable validation with a malicious command
    // The command starts with a safe prefix "cargo check" but appends a malicious command
    // We use a command that should be fast.
    let malicious_command = "cargo check; echo 'pwned' > pwned.txt";

    service.validation_config = ValidationConfig {
        enabled: true,
        command: malicious_command.to_string(),
        on_failure: mill_foundation::validation::ValidationFailureAction::Report,
        ..ValidationConfig::default()
    };

    // Run validation
    let _ = service.run_validation().await;

    // Check if the exploit succeeded
    // If the vulnerability exists, "pwned.txt" will be created in the project root
    let exploited_path = temp_dir.path().join("pwned.txt");

    // We expect this to be TRUE before the fix (vulnerability exists)
    // After the fix, this should be FALSE
    assert!(
        !exploited_path.exists(),
        "Vulnerability still exists: pwned.txt WAS created"
    );
}

#[tokio::test]
async fn test_valid_command_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let (mut service, _queue) = create_test_service(&temp_dir);

    // "cargo check" is in the safe list.
    // We just want to ensure it passes the parser and attempts execution.
    service.validation_config = ValidationConfig {
        enabled: true,
        command: "cargo check".to_string(),
        on_failure: mill_foundation::validation::ValidationFailureAction::Report,
        ..ValidationConfig::default()
    };

    let result: Option<serde_json::Value> = service.run_validation().await;

    // If result is None, it means enabled=false or something.
    assert!(result.is_some());
    let val = result.unwrap();

    let status = val["validation_status"].as_str().unwrap();

    // "error" usually means security error or execution error (command not found).
    // If cargo is installed, it should be "failed" (exit code != 0) or "passed".
    // If cargo is NOT installed, it returns "error" with "Failed to execute command: No such file or directory".
    // We can check if it's NOT a Security Error.

    if status == "error" {
        let err_msg = val["validation_error"].as_str().unwrap();
        assert!(!err_msg.contains("Security Error"), "Should not be a security error: {}", err_msg);
    }
}
