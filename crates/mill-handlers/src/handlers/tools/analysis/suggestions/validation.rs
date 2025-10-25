use anyhow::{bail, Result};
use mill_foundation::protocol::analysis_result::{ RefactorCall , SafetyLevel , Suggestion };
use serde_json::Value;

/// Validates that a suggestion has all required metadata
pub fn validate_suggestion(suggestion: &Suggestion) -> Result<()> {
    // Check required fields
    if suggestion.description.is_empty() {
        bail!("Suggestion missing description");
    }

    // Check confidence range
    if !(0.0..=1.0).contains(&suggestion.confidence) {
        bail!("Confidence out of range: {}", suggestion.confidence);
    }

    // Check refactor_call for safe/requires_review suggestions
    if matches!(
        suggestion.safety,
        SafetyLevel::Safe | SafetyLevel::RequiresReview
    ) {
        if let Some(ref refactor_call) = suggestion.refactor_call {
            validate_refactor_call(refactor_call)?;
        } else {
            bail!(
                "Safe/RequiresReview suggestion missing refactor_call: {:?}",
                suggestion.safety
            );
        }
    }

    Ok(())
}

/// Validates refactor_call structure
fn validate_refactor_call(refactor_call: &RefactorCall) -> Result<()> {
    // Valid tool names (commands) - unified dryRun API
    let valid_commands = [
        "extract",
        "inline",
        "move",
        "rename",
        "transform",
        "delete",
        "reorder",
    ];

    if !valid_commands.contains(&refactor_call.command.as_str()) {
        bail!("Invalid command name: {}", refactor_call.command);
    }

    // Arguments must be an object
    if !refactor_call.arguments.is_object() {
        bail!("refactor_call.arguments must be an object");
    }

    // Tool-specific argument validation
    match refactor_call.command.as_str() {
        "delete" => validate_delete_args(&refactor_call.arguments)?,
        "extract" => validate_extract_args(&refactor_call.arguments)?,
        "inline" => validate_inline_args(&refactor_call.arguments)?,
        // ... other tools can be added here
        _ => {}
    }

    Ok(())
}

fn validate_delete_args(args: &Value) -> Result<()> {
    if args.get("filePath").is_none() {
        bail!("delete missing file_path");
    }
    if args.get("line").is_none() && args.get("start_line").is_none() {
        bail!("delete missing line or start_line");
    }
    Ok(())
}

fn validate_extract_args(args: &Value) -> Result<()> {
    if args.get("filePath").is_none() {
        bail!("extract missing file_path");
    }
    if args.get("start_line").is_none() || args.get("end_line").is_none() {
        bail!("extract missing start_line/end_line");
    }
    Ok(())
}

fn validate_inline_args(args: &Value) -> Result<()> {
    if args.get("filePath").is_none() {
        bail!("inline missing file_path");
    }
    if args.get("line").is_none() {
        bail!("inline missing line");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_foundation::protocol::analysis_result::{ RefactorCall , SafetyLevel , Suggestion };
    use serde_json::json;

    // Helper to create a default suggestion for testing
    fn mock_suggestion() -> Suggestion {
        Suggestion {
            action: "delete_line".to_string(),
            description: "Test suggestion".to_string(),
            target: None,
            estimated_impact: "low".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.85,
            reversible: true,
            refactor_call: Some(RefactorCall {
                command: "delete".to_string(),
                arguments: json!({
                    "filePath": "test.rs",
                    "line": 10,
                }),
            }),
        }
    }

    #[test]
    fn test_validate_complete_suggestion() {
        let suggestion = mock_suggestion();
        validate_suggestion(&suggestion).unwrap();
    }

    #[test]
    fn test_validate_missing_refactor_call_for_safe_suggestion() {
        let mut suggestion = mock_suggestion();
        suggestion.safety = SafetyLevel::Safe;
        suggestion.refactor_call = None; // Invalid for Safe suggestion
        assert!(validate_suggestion(&suggestion).is_err());
    }

    #[test]
    fn test_validate_missing_refactor_call_for_experimental_is_ok() {
        let mut suggestion = mock_suggestion();
        suggestion.safety = SafetyLevel::Experimental;
        suggestion.refactor_call = None; // This is OK for experimental
        assert!(validate_suggestion(&suggestion).is_ok());
    }

    #[test]
    fn test_validate_invalid_confidence() {
        let mut suggestion = mock_suggestion();
        suggestion.confidence = 1.5; // Out of range
        assert!(validate_suggestion(&suggestion).is_err());
    }

    #[test]
    fn test_validate_invalid_command() {
        let mut suggestion = mock_suggestion();
        if let Some(ref mut call) = suggestion.refactor_call {
            call.command = "invalid.command".to_string();
        }
        assert!(validate_suggestion(&suggestion).is_err());
    }

    #[test]
    fn test_validate_missing_delete_args() {
        let mut suggestion = mock_suggestion();
        if let Some(ref mut call) = suggestion.refactor_call {
            call.command = "delete".to_string();
            call.arguments = json!({}); // Missing args
        }
        assert!(validate_suggestion(&suggestion).is_err());
    }
}