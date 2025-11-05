//! Tests for error handling
#![allow(deprecated)]

use mill_foundation::error::CoreError;
use mill_foundation::error::CoreResult;
use std::io;

#[test]
fn test_error_chain() {
    // Test that errors can be chained properly
    fn inner_function() -> Result<(), io::Error> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Access denied",
        ))
    }

    fn outer_function() -> CoreResult<()> {
        inner_function().map_err(CoreError::from)?;
        Ok(())
    }

    let result = outer_function();
    assert!(result.is_err());

    let error = result.unwrap_err();
    match error {
        CoreError::Io(io_error) => {
            assert_eq!(io_error.kind(), io::ErrorKind::PermissionDenied);
        }
        _ => panic!("Expected IO error"),
    }
}

#[test]
fn test_error_helpers() {
    let permission_denied = CoreError::permission_denied("read_file");
    match permission_denied {
        CoreError::PermissionDenied { operation } => {
            assert_eq!(operation, "read_file");
        }
        _ => panic!("Expected permission denied error"),
    }

    let not_found = CoreError::not_found("database.db");
    match not_found {
        CoreError::NotFound { resource } => {
            assert_eq!(resource, "database.db");
        }
        _ => panic!("Expected not found error"),
    }

    let timeout = CoreError::timeout("api_call");
    match timeout {
        CoreError::Timeout { operation } => {
            assert_eq!(operation, "api_call");
        }
        _ => panic!("Expected timeout error"),
    }
}

#[test]
fn test_error_implements_std_error() {
    let error = CoreError::internal("test");

    // Should implement std::error::Error
    let _: &dyn std::error::Error = &error;

    // Should have source method (even if it returns None for most variants)
    use std::error::Error;
    let source = error.source();
    assert!(source.is_none()); // Internal error doesn't have a source

    // IO error should have source
    let io_error = io::Error::new(io::ErrorKind::NotFound, "Not found");
    let core_error: CoreError = io_error.into();

    match core_error {
        CoreError::Io(ref _inner) => {
            let core_error_ref: &dyn std::error::Error = &core_error;
            let source = core_error_ref.source();
            assert!(source.is_some());
        }
        _ => panic!("Expected IO error"),
    }
}

#[test]
fn test_error_serialization() {
    // While CoreError doesn't implement Serialize (because std::error::Error can't be serialized),
    // we can test that error messages can be serialized for transport
    let error = CoreError::config("Invalid port number");
    let error_message = format!("{}", error);

    // Should be able to serialize the error message
    let json = serde_json::to_string(&error_message).unwrap();
    assert!(json.contains("Configuration error"));
    assert!(json.contains("Invalid port number"));

    // Should be able to deserialize back
    let deserialized: String = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, error_message);
}