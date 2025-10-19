//! Checksum validation service for refactoring plans
//!
//! Validates that files haven't changed since a plan was created by comparing
//! SHA-256 checksums. This prevents applying stale plans to modified files.

use codebuddy_foundation::protocol::{ ApiError , ApiResult as ServerResult , RefactorPlan , RefactorPlanExt };
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::{debug, info, warn};

use super::FileService;

/// Service for validating file checksums against refactoring plan checksums
///
/// This service ensures that files haven't been modified since a refactoring
/// plan was created. It's a critical safety mechanism to prevent applying
/// outdated plans to changed codebases.
pub struct ChecksumValidator {
    file_service: std::sync::Arc<FileService>,
}

impl ChecksumValidator {
    /// Create a new checksum validator
    pub fn new(file_service: std::sync::Arc<FileService>) -> Self {
        Self { file_service }
    }

    /// Validate all checksums in a refactoring plan
    ///
    /// Reads each file referenced in the plan's checksums and compares the
    /// actual SHA-256 checksum with the expected checksum from the plan.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A file cannot be read
    /// - A checksum mismatch is detected (file was modified)
    pub async fn validate_checksums(&self, plan: &RefactorPlan) -> ServerResult<()> {
        let checksums = plan.checksums();

        if checksums.is_empty() {
            debug!("No checksums to validate");
            return Ok(());
        }

        debug!(checksum_count = checksums.len(), "Validating checksums");

        for (file_path, expected_checksum) in checksums {
            let content = self
                .file_service
                .read_file(Path::new(&file_path))
                .await
                .map_err(|e| {
                    ApiError::InvalidRequest(format!(
                        "Cannot validate checksum for {}: {}",
                        file_path, e
                    ))
                })?;

            let actual_checksum = Self::calculate_checksum(&content);

            if &actual_checksum != expected_checksum {
                warn!(
                    file_path = %file_path,
                    expected = %expected_checksum,
                    actual = %actual_checksum,
                    "Checksum mismatch - file has changed since plan was created"
                );

                return Err(ApiError::InvalidRequest(format!(
                    "File '{}' has changed since plan was created. \
                     Expected checksum: {}, Actual: {}. \
                     Please regenerate the plan with current file contents.",
                    file_path, expected_checksum, actual_checksum
                )));
            }
        }

        info!(validated_files = checksums.len(), "All checksums valid");
        Ok(())
    }

    /// Calculate SHA-256 checksum of file content
    ///
    /// Returns a hex-encoded SHA-256 checksum string.
    pub fn calculate_checksum(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_checksum_deterministic() {
        let content = "hello world";
        let checksum1 = ChecksumValidator::calculate_checksum(content);
        let checksum2 = ChecksumValidator::calculate_checksum(content);
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_calculate_checksum_different_content() {
        let checksum1 = ChecksumValidator::calculate_checksum("hello");
        let checksum2 = ChecksumValidator::calculate_checksum("world");
        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_calculate_checksum_empty() {
        let checksum = ChecksumValidator::calculate_checksum("");
        assert!(!checksum.is_empty());
        // SHA-256 of empty string is known value
        assert_eq!(
            checksum,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_calculate_checksum_whitespace_sensitive() {
        let checksum1 = ChecksumValidator::calculate_checksum("hello world");
        let checksum2 = ChecksumValidator::calculate_checksum("hello  world");
        assert_ne!(checksum1, checksum2);
    }
}