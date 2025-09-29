//! JWT authentication utilities

use crate::error::ServerError;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// JWT Claims structure
#[derive(Debug, Deserialize, Serialize)]
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
}

/// Validate a JWT token and return true if valid
pub fn validate_token(token: &str, secret: &str) -> Result<bool, ServerError> {
    let key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::default();

    decode::<Claims>(token, &key, &validation)
        .map(|_| true)
        .map_err(|e| ServerError::Auth(e.to_string()))
}

/// Validate a JWT token with project ID verification
pub fn validate_token_with_project(
    token: &str,
    secret: &str,
    expected_project_id: &str,
) -> Result<bool, ServerError> {
    let key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::default();

    let token_data =
        decode::<Claims>(token, &key, &validation).map_err(|e| ServerError::Auth(e.to_string()))?;

    // Check if project_id claim matches expected value
    if let Some(project_id) = &token_data.claims.project_id {
        if project_id == expected_project_id {
            Ok(true)
        } else {
            Err(ServerError::Auth(format!(
                "Project ID mismatch: expected '{}', got '{}'",
                expected_project_id, project_id
            )))
        }
    } else {
        // No project_id claim, allow access (for backward compatibility)
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_token(secret: &str, project_id: Option<String>) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let claims = Claims {
            sub: Some("test_user".to_string()),
            exp: Some(now + 3600), // 1 hour from now
            iat: Some(now),
            iss: Some("codebuddy".to_string()),
            aud: Some("codeflow-clients".to_string()),
            project_id,
        };

        let header = Header::default();
        let key = EncodingKey::from_secret(secret.as_ref());
        encode(&header, &claims, &key).unwrap()
    }

    #[test]
    fn test_validate_token_success() {
        let secret = "test_secret";
        let token = create_test_token(secret, None);

        assert!(validate_token(&token, secret).unwrap());
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
        assert!(validate_token_with_project(&token, secret, "any_project").unwrap());
    }
}
