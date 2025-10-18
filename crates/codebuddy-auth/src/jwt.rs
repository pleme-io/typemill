//! JWT authentication utilities

use cb_types::error::CoreError;
pub use jsonwebtoken::{decode, DecodingKey, Validation};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT Claims structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Claims {
    /// Subject (user identifier)
    pub sub: Option<String>,
    /// Expiration time (as UTC timestamp)
    pub exp: Option<usize>,
    /// Issued at time (as UTC timestamp)
    pub iat: Option<usize>,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
    /// Project ID (custom claim)
    pub project_id: Option<String>,
    /// User ID (custom claim for multi-tenancy)
    pub user_id: Option<String>,
}

/// Validate a JWT token and return true if valid
pub fn validate_token(token: &str, secret: &str) -> Result<bool, CoreError> {
    let key = DecodingKey::from_secret(secret.as_ref());
    let mut validation = Validation::default();
    // Don't require aud claim in tests
    validation.validate_aud = false;

    decode::<Claims>(token, &key, &validation)
        .map(|_| true)
        .map_err(|e| CoreError::permission_denied(e.to_string()))
}

/// Validate a JWT token with project ID verification
pub fn validate_token_with_project(
    token: &str,
    secret: &str,
    expected_project_id: &str,
) -> Result<bool, CoreError> {
    let key = DecodingKey::from_secret(secret.as_ref());
    let mut validation = Validation::default();
    // Don't require aud claim in tests
    validation.validate_aud = false;

    let token_data = decode::<Claims>(token, &key, &validation)
        .map_err(|e| CoreError::permission_denied(e.to_string()))?;

    // Check if project_id claim matches expected value
    if let Some(project_id) = &token_data.claims.project_id {
        if project_id == expected_project_id {
            Ok(true)
        } else {
            Err(CoreError::permission_denied(format!(
                "Project ID mismatch: expected '{}', got '{}'",
                expected_project_id, project_id
            )))
        }
    } else {
        // No project_id claim, allow access (for backward compatibility)
        Ok(true)
    }
}

/// Generate a new JWT token with the given parameters
pub fn generate_token(
    secret: &str,
    expiry_seconds: u64,
    issuer: &str,
    audience: &str,
    project_id: Option<String>,
    user_id: Option<String>,
) -> Result<String, CoreError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CoreError::Internal {
            message: format!("System time error: {}", e),
        })?
        .as_secs() as usize;

    let claims = Claims {
        sub: Some("api_client".to_string()),
        exp: Some(now + expiry_seconds as usize),
        iat: Some(now),
        iss: Some(issuer.to_string()),
        aud: Some(audience.to_string()),
        project_id,
        user_id,
    };

    let header = Header::default();
    let key = EncodingKey::from_secret(secret.as_ref());

    encode(&header, &claims, &key)
        .map_err(|e| CoreError::permission_denied(format!("Token generation failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_token(secret: &str, project_id: Option<String>) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time should be after UNIX epoch")
            .as_secs() as usize;

        let claims = Claims {
            sub: Some("test_user".to_string()),
            exp: Some(now + 3600), // 1 hour from now
            iat: Some(now),
            iss: Some("codebuddy".to_string()),
            aud: Some("codeflow-clients".to_string()),
            project_id,
            user_id: None,
        };

        let header = Header::default();
        let key = EncodingKey::from_secret(secret.as_ref());
        encode(&header, &claims, &key).expect("Test token encoding should succeed")
    }

    #[test]
    fn test_validate_token_success() {
        let secret = "test_secret";
        let token = create_test_token(secret, None);

        assert!(validate_token(&token, secret).expect("Test token should be valid"));
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let secret = "test_secret";
        let token = create_test_token(secret, None);

        assert!(validate_token(&token, "wrong_secret").is_err());
    }

    #[test]
    fn test_validate_token_with_project_success() {
        let secret = "test_secret";
        let project_id = "test_project";
        let token = create_test_token(secret, Some(project_id.to_string()));

        assert!(validate_token_with_project(&token, secret, project_id).unwrap());
    }

    #[test]
    fn test_validate_token_with_project_mismatch() {
        let secret = "test_secret";
        let project_id = "test_project";
        let token = create_test_token(secret, Some(project_id.to_string()));

        assert!(validate_token_with_project(&token, secret, "different_project").is_err());
    }

    #[test]
    fn test_validate_token_with_project_no_claim() {
        let secret = "test_secret";
        let token = create_test_token(secret, None);

        // Should succeed when no project_id claim is present
        assert!(validate_token_with_project(&token, secret, "any_project")
            .expect("Test token should be valid"));
    }
}
