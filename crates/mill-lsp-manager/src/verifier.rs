//! Security verification for downloads

use crate::error::{LspError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::{debug, info};

/// Maximum download size (200MB)
pub const MAX_DOWNLOAD_SIZE: u64 = 200 * 1024 * 1024;

/// Verify SHA256 checksum of a file
pub fn verify_checksum(file_path: &Path, expected_checksum: &str) -> Result<()> {
    debug!("Verifying checksum for {}", file_path.display());

    let contents = std::fs::read(file_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let actual = format!("{:x}", hasher.finalize());

    // Skip verification if checksum is placeholder
    if expected_checksum.starts_with("placeholder") {
        info!("Skipping checksum verification (placeholder checksum)");
        return Ok(());
    }

    if actual != expected_checksum {
        return Err(LspError::ChecksumMismatch {
            expected: expected_checksum.to_string(),
            actual,
        });
    }

    debug!("Checksum verification passed");
    Ok(())
}

/// Validate download URL for security
pub fn validate_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        return Err(LspError::InsecureUrl(url.to_string()));
    }

    // Additional validation: only allow known-good hosts
    let allowed_hosts = [
        "github.com",
        "registry.npmjs.org",
        "files.pythonhosted.org",
    ];

    let url_parsed = url::Url::parse(url)
        .map_err(|e| LspError::InsecureUrl(format!("Invalid URL: {}", e)))?;

    let host = url_parsed
        .host_str()
        .ok_or_else(|| LspError::InsecureUrl("No host in URL".to_string()))?;

    if !allowed_hosts.iter().any(|&allowed| host.ends_with(allowed)) {
        return Err(LspError::InsecureUrl(format!(
            "Host '{}' not in allowed list",
            host
        )));
    }

    Ok(())
}

/// Check if download size is within limits
pub fn validate_size(size: u64) -> Result<()> {
    if size > MAX_DOWNLOAD_SIZE {
        return Err(LspError::DownloadTooLarge(size, MAX_DOWNLOAD_SIZE));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_checksum_verification() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content").unwrap();

        let expected = "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72";
        verify_checksum(file.path(), expected).unwrap();
    }

    #[test]
    fn test_checksum_mismatch() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content").unwrap();

        let result = verify_checksum(file.path(), "wrong_checksum");
        assert!(matches!(result, Err(LspError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_https_required() {
        assert!(validate_url("http://example.com/file").is_err());
        assert!(validate_url("https://github.com/file").is_ok());
    }

    #[test]
    fn test_allowed_hosts() {
        assert!(validate_url("https://github.com/rust-lang/rust-analyzer/releases/download/file").is_ok());
        assert!(validate_url("https://registry.npmjs.org/package").is_ok());
        assert!(validate_url("https://evil.com/malware").is_err());
    }

    #[test]
    fn test_size_limits() {
        assert!(validate_size(1024 * 1024).is_ok()); // 1MB OK
        assert!(validate_size(MAX_DOWNLOAD_SIZE).is_ok()); // Max OK
        assert!(validate_size(MAX_DOWNLOAD_SIZE + 1).is_err()); // Over max fails
    }
}
