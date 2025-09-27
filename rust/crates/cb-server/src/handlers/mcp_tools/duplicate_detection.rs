//! Duplicate code detection MCP tools using jscpd subprocess

use crate::handlers::McpDispatcher;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;
use std::path::Path;

/// Arguments for find_code_duplicates tool
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FindCodeDuplicatesArgs {
    path: String,
    #[serde(default = "default_min_tokens")]
    min_tokens: u32,
    #[serde(default = "default_min_lines")]
    min_lines: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    languages: Option<Vec<String>>,
    #[serde(default)]
    include_content: bool,
}

fn default_min_tokens() -> u32 { 50 }
fn default_min_lines() -> u32 { 5 }

/// Duplicate instance information
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateInstance {
    file: String,
    start_line: u32,
    end_line: u32,
    token_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

/// Group of duplicate code instances
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateGroup {
    hash: String,
    instances: Vec<DuplicateInstance>,
    token_count: u32,
    line_count: u32,
}

/// Result of duplicate detection
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateDetectionResult {
    duplicates: Vec<DuplicateGroup>,
    statistics: DuplicateStatistics,
}

/// Statistics about duplicates found
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateStatistics {
    total_files: u32,
    files_with_duplicates: u32,
    duplicate_percentage: f64,
    total_duplicate_lines: u32,
}

/// Register duplicate detection tools with the dispatcher
pub fn register(dispatcher: &mut McpDispatcher) {
    dispatcher.register_tool("find_code_duplicates".to_string(), |app_state, args| async move {
        let params: FindCodeDuplicatesArgs = serde_json::from_value(args)
            .map_err(|e| crate::error::ServerError::InvalidRequest(format!("Invalid args: {}", e)))?;

        tracing::info!("Finding code duplicates in path: {}", params.path);

        // Resolve path relative to project root
        let full_path = if Path::new(&params.path).is_absolute() {
            params.path.clone()
        } else {
            app_state.project_root.join(&params.path).to_string_lossy().to_string()
        };

        // Validate path exists
        let path = Path::new(&full_path);
        if !path.exists() {
            return Err(crate::error::ServerError::runtime(
                format!("Path does not exist: {}", full_path)
            ));
        }

        // Build jscpd command
        let mut cmd = Command::new("npx");
        cmd.arg("jscpd")
            .arg("--reporters").arg("json")
            .arg("--min-tokens").arg(params.min_tokens.to_string())
            .arg("--min-lines").arg(params.min_lines.to_string())
            .arg("--silent")
            .arg("--absolute")
            .arg("--output").arg("/tmp"); // Output directory for JSON report

        // Add language filters if specified
        if let Some(langs) = &params.languages {
            for lang in langs {
                cmd.arg("--format").arg(lang);
            }
        }

        // Add the path to scan
        cmd.arg(&full_path);

        tracing::debug!("Executing jscpd command: {:?}", cmd);

        // Execute jscpd
        let output = cmd.output()
            .map_err(|e| crate::error::ServerError::runtime(
                format!("Failed to execute jscpd: {}. Make sure jscpd is installed (npm install -g jscpd)", e)
            ))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::error!("jscpd failed with stderr: {}, stdout: {}", stderr, stdout);
            return Err(crate::error::ServerError::runtime(
                format!("jscpd failed: {}", stderr)
            ));
        }

        // Read the JSON report from /tmp/jscpd-report.json
        let report_path = "/tmp/jscpd-report.json";
        let report_content = tokio::fs::read_to_string(report_path).await
            .map_err(|e| {
                // Fallback: try to parse stdout if report file not found
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::warn!("Could not read report file, trying stdout: {}", e);

                // Try parsing stdout as JSON
                if let Ok(json_data) = serde_json::from_str::<Value>(&stdout) {
                    return crate::error::ServerError::runtime(
                        format!("Report file not found, but got data from stdout")
                    );
                }

                crate::error::ServerError::runtime(
                    format!("Failed to read jscpd report: {}", e)
                )
            })?;

        // Parse jscpd JSON output
        let jscpd_result: Value = serde_json::from_str(&report_content)
            .map_err(|e| crate::error::ServerError::runtime(
                format!("Failed to parse jscpd output: {}", e)
            ))?;

        // Transform jscpd output to our format
        let result = transform_jscpd_output(jscpd_result, params.include_content).await?;

        // Clean up report file
        let _ = tokio::fs::remove_file(report_path).await;

        Ok(serde_json::to_value(result)?)
    });
}

/// Transform jscpd output to our standardized format
async fn transform_jscpd_output(
    jscpd_data: Value,
    include_content: bool
) -> Result<DuplicateDetectionResult, crate::error::ServerError> {
    // Parse duplicates from jscpd format
    let mut duplicate_groups = Vec::new();

    if let Some(duplicates) = jscpd_data["duplicates"].as_array() {
        for dup in duplicates {
            let mut instances = Vec::new();

            // Extract first file info
            if let (Some(first_file), Some(second_file)) =
                (dup["firstFile"].as_object(), dup["secondFile"].as_object()) {

                instances.push(DuplicateInstance {
                    file: first_file["name"].as_str().unwrap_or("")
                        .replace("file://", ""), // Remove file:// prefix
                    start_line: first_file["startLine"].as_u64().unwrap_or(1) as u32,
                    end_line: first_file["endLine"].as_u64().unwrap_or(1) as u32,
                    token_count: dup["tokens"].as_u64().unwrap_or(0) as u32,
                    content: if include_content {
                        // Read file content if requested
                        let file_path = first_file["name"].as_str().unwrap_or("")
                            .replace("file://", "");
                        read_file_lines(&file_path,
                                      first_file["startLine"].as_u64().unwrap_or(1) as u32,
                                      first_file["endLine"].as_u64().unwrap_or(1) as u32).await.ok()
                    } else {
                        None
                    },
                });

                instances.push(DuplicateInstance {
                    file: second_file["name"].as_str().unwrap_or("")
                        .replace("file://", ""), // Remove file:// prefix
                    start_line: second_file["startLine"].as_u64().unwrap_or(1) as u32,
                    end_line: second_file["endLine"].as_u64().unwrap_or(1) as u32,
                    token_count: dup["tokens"].as_u64().unwrap_or(0) as u32,
                    content: if include_content {
                        let file_path = second_file["name"].as_str().unwrap_or("")
                            .replace("file://", "");
                        read_file_lines(&file_path,
                                      second_file["startLine"].as_u64().unwrap_or(1) as u32,
                                      second_file["endLine"].as_u64().unwrap_or(1) as u32).await.ok()
                    } else {
                        None
                    },
                });
            }

            duplicate_groups.push(DuplicateGroup {
                hash: format!("{}_{}",
                    dup["tokens"].as_u64().unwrap_or(0),
                    dup["lines"].as_u64().unwrap_or(0)),
                instances,
                token_count: dup["tokens"].as_u64().unwrap_or(0) as u32,
                line_count: dup["lines"].as_u64().unwrap_or(0) as u32,
            });
        }
    }

    // Extract statistics
    let stats = if let Some(stats) = jscpd_data["statistics"].as_object() {
        let total = stats.get("total").and_then(|t| t.as_object());
        DuplicateStatistics {
            total_files: total.and_then(|t| t.get("files"))
                .and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            files_with_duplicates: total.and_then(|t| t.get("sources"))
                .and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            duplicate_percentage: total.and_then(|t| t.get("percentage"))
                .and_then(|v| v.as_f64()).unwrap_or(0.0),
            total_duplicate_lines: total.and_then(|t| t.get("duplicatedLines"))
                .and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        }
    } else {
        // Fallback statistics if structure is different
        DuplicateStatistics {
            total_files: 0,
            files_with_duplicates: duplicate_groups.len() as u32,
            duplicate_percentage: 0.0,
            total_duplicate_lines: duplicate_groups.iter()
                .map(|g| g.line_count * g.instances.len() as u32)
                .sum(),
        }
    };

    Ok(DuplicateDetectionResult {
        duplicates: duplicate_groups,
        statistics: stats,
    })
}

/// Read specific lines from a file
async fn read_file_lines(
    file_path: &str,
    start: u32,
    end: u32
) -> Result<String, crate::error::ServerError> {
    let content = tokio::fs::read_to_string(file_path).await
        .map_err(|e| crate::error::ServerError::runtime(format!("Failed to read file {}: {}", file_path, e)))?;

    let lines: Vec<&str> = content.lines().collect();
    let start_idx = (start as usize).saturating_sub(1);
    let end_idx = (end as usize).min(lines.len());

    if start_idx >= lines.len() {
        return Ok(String::new());
    }

    Ok(lines[start_idx..end_idx].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transform_empty_output() {
        let empty_json = json!({
            "duplicates": [],
            "statistics": {
                "total": {
                    "files": 0,
                    "sources": 0,
                    "percentage": 0.0,
                    "duplicatedLines": 0
                }
            }
        });

        let result = transform_jscpd_output(empty_json, false).await.unwrap();
        assert!(result.duplicates.is_empty());
        assert_eq!(result.statistics.total_files, 0);
    }

    #[tokio::test]
    async fn test_transform_with_duplicates() {
        let sample_json = json!({
            "duplicates": [{
                "firstFile": {
                    "name": "file://test1.rs",
                    "startLine": 10,
                    "endLine": 20
                },
                "secondFile": {
                    "name": "file://test2.rs",
                    "startLine": 30,
                    "endLine": 40
                },
                "tokens": 100,
                "lines": 10
            }],
            "statistics": {
                "total": {
                    "files": 2,
                    "sources": 2,
                    "percentage": 5.0,
                    "duplicatedLines": 20
                }
            }
        });

        let result = transform_jscpd_output(sample_json, false).await.unwrap();
        assert_eq!(result.duplicates.len(), 1);
        assert_eq!(result.duplicates[0].instances.len(), 2);
        assert_eq!(result.statistics.duplicate_percentage, 5.0);
    }
}