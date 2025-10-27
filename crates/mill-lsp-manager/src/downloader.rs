//! Download LSP servers with progress tracking

use crate::error::{LspError, Result};
use crate::verifier::{validate_size, validate_url, verify_checksum, MAX_DOWNLOAD_SIZE};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

/// Download a file from a URL with progress tracking
pub async fn download_file(url: &str, dest_path: &Path, expected_checksum: &str) -> Result<()> {
    info!("Downloading {} to {}", url, dest_path.display());

    // Security: Validate URL
    validate_url(url)?;

    // Create parent directory if needed
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Start download
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| LspError::DownloadFailed(e.to_string()))?;

    if !response.status().is_success() {
        return Err(LspError::DownloadFailed(format!(
            "HTTP {}: {}",
            response.status(),
            response.status().canonical_reason().unwrap_or("Unknown")
        )));
    }

    // Get content length for progress bar
    let total_size = response.content_length();

    // Security: Check size limits
    if let Some(size) = total_size {
        validate_size(size)?;
    }

    // Setup progress bar
    let pb = if let Some(size) = total_size {
        let bar = ProgressBar::new(size);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(bar)
    } else {
        let bar = ProgressBar::new_spinner();
        bar.set_message("Downloading...");
        Some(bar)
    };

    // Download with streaming
    let mut file = File::create(dest_path).await?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| LspError::DownloadFailed(e.to_string()))?;

        // Security: Check cumulative size
        downloaded += chunk.len() as u64;
        if downloaded > MAX_DOWNLOAD_SIZE {
            return Err(LspError::DownloadTooLarge(downloaded, MAX_DOWNLOAD_SIZE));
        }

        file.write_all(&chunk).await?;

        if let Some(ref pb) = pb {
            pb.set_position(downloaded);
        }
    }

    file.flush().await?;

    if let Some(pb) = pb {
        pb.finish_with_message("Downloaded");
    }

    debug!("Download complete: {} bytes", downloaded);

    // Verify checksum
    verify_checksum(dest_path, expected_checksum)?;

    Ok(())
}

/// Decompress a file based on compression format
pub async fn decompress_file(
    compressed_path: &Path,
    output_path: &Path,
    format: &str,
) -> Result<()> {
    info!(
        "Decompressing {} -> {}",
        compressed_path.display(),
        output_path.display()
    );

    match format {
        "gzip" | "gz" => decompress_gzip(compressed_path, output_path).await?,
        "tar.gz" | "tgz" => decompress_tar_gz(compressed_path, output_path).await?,
        "zip" => {
            return Err(LspError::DecompressionFailed(
                "ZIP decompression not yet implemented".to_string(),
            ))
        }
        _ => {
            return Err(LspError::DecompressionFailed(format!(
                "Unknown compression format: {}",
                format
            )))
        }
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = tokio::fs::metadata(output_path).await?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755); // rwxr-xr-x
        tokio::fs::set_permissions(output_path, permissions).await?;
    }

    debug!("Decompression complete");
    Ok(())
}

/// Decompress gzip file
async fn decompress_gzip(input: &Path, output: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let input_file = std::fs::File::open(input)?;
    let mut decoder = GzDecoder::new(input_file);
    let mut buffer = Vec::new();
    decoder
        .read_to_end(&mut buffer)
        .map_err(|e| LspError::DecompressionFailed(e.to_string()))?;

    tokio::fs::write(output, buffer).await?;
    Ok(())
}

/// Decompress tar.gz file (extracts first file only)
async fn decompress_tar_gz(input: &Path, output_dir: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let input_file = std::fs::File::open(input)?;
    let decoder = GzDecoder::new(input_file);
    let mut archive = Archive::new(decoder);

    tokio::fs::create_dir_all(output_dir).await?;

    archive
        .unpack(output_dir)
        .map_err(|e| LspError::DecompressionFailed(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_download_validation() {
        // Should reject HTTP
        let result = download_file(
            "http://example.com/file",
            Path::new("/tmp/test"),
            "placeholder",
        )
        .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_formats() {
        assert!(matches!(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(decompress_file(
                    Path::new("/tmp/test.gz"),
                    Path::new("/tmp/test"),
                    "unknown"
                )),
            Err(LspError::DecompressionFailed(_))
        ));
    }
}
