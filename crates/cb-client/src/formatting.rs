use crate::error::ClientError;
use crate::websocket::MCPResponse;
use codebuddy_foundation::protocol::refactor_plan::RefactorPlan;
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::fmt::Write;
use std::time::Duration;

/// Emojis for visual feedback
static CHECKMARK: Emoji<'_, '_> = Emoji("✅ ", "");
static CROSS: Emoji<'_, '_> = Emoji("❌ ", "");
static WARNING: Emoji<'_, '_> = Emoji("⚠️ ", "");
static INFO: Emoji<'_, '_> = Emoji("ℹ️ ", "");
static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "");
static GEAR: Emoji<'_, '_> = Emoji("⚙️ ", "");

/// Output formatting utilities
#[derive(Debug, Clone)]
pub struct Formatter {
    pub use_colors: bool,
    pub use_emojis: bool,
}

impl Default for Formatter {
    fn default() -> Self {
        Self {
            use_colors: console::colors_enabled(),
            use_emojis: true,
        }
    }
}

impl Formatter {
    /// Create a new formatter
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a formatter with specific settings
    pub fn with_settings(use_colors: bool, use_emojis: bool) -> Self {
        Self {
            use_colors,
            use_emojis,
        }
    }

    /// Format a success message
    pub fn success(&self, message: &str) -> String {
        let emoji = if self.use_emojis {
            CHECKMARK.to_string()
        } else {
            String::new()
        };
        if self.use_colors {
            format!("{}{}", emoji, style(message).green().bold())
        } else {
            format!("{}{}", emoji, message)
        }
    }

    /// Format an informational message
    pub fn info(&self, message: &str) -> String {
        let emoji = if self.use_emojis {
            INFO.to_string()
        } else {
            String::new()
        };
        if self.use_colors {
            format!("{}{}", emoji, style(message).blue())
        } else {
            format!("{}{}", emoji, message)
        }
    }

    /// Format a title message
    pub fn title(&self, message: &str) -> String {
        let emoji = if self.use_emojis {
            ROCKET.to_string()
        } else {
            String::new()
        };
        if self.use_colors {
            format!("{}{}", emoji, style(message).bold().underlined())
        } else {
            format!("{}{}", emoji, message)
        }
    }

    /// Format a subtitle message
    pub fn subtitle(&self, message: &str) -> String {
        if self.use_colors {
            format!("{}", style(message).dim())
        } else {
            message.to_string()
        }
    }

    /// Format an error message
    pub fn error(&self, message: &str) -> String {
        let emoji = if self.use_emojis {
            CROSS.to_string()
        } else {
            String::new()
        };
        if self.use_colors {
            format!("{}{}", emoji, style(message).red().bold())
        } else {
            format!("ERROR: {}{}", emoji, message)
        }
    }

    /// Format a warning message
    pub fn warning(&self, message: &str) -> String {
        let emoji = if self.use_emojis {
            WARNING.to_string()
        } else {
            String::new()
        };
        if self.use_colors {
            format!("{}{}", emoji, style(message).yellow().bold())
        } else {
            format!("WARNING: {}{}", emoji, message)
        }
    }

    /// Format a header message
    pub fn header(&self, message: &str) -> String {
        let emoji = if self.use_emojis {
            ROCKET.to_string()
        } else {
            String::new()
        };
        if self.use_colors {
            format!("{}{}", emoji, style(message).blue().bold())
        } else {
            format!("{}{}", emoji, message)
        }
    }

    /// Format a configuration message
    pub fn config(&self, message: &str) -> String {
        let emoji = if self.use_emojis {
            GEAR.to_string()
        } else {
            String::new()
        };
        if self.use_colors {
            format!("{}{}", emoji, style(message).magenta())
        } else {
            format!("{}{}", emoji, message)
        }
    }

    /// Format a key-value pair
    pub fn key_value(&self, key: &str, value: &str) -> String {
        if self.use_colors {
            format!("{}: {}", style(key).bold(), style(value).dim())
        } else {
            format!("{}: {}", key, value)
        }
    }

    /// Format a URL
    pub fn url(&self, url: &str) -> String {
        if self.use_colors {
            style(url).underlined().cyan().to_string()
        } else {
            url.to_string()
        }
    }

    /// Format a file path
    pub fn path(&self, path: &str) -> String {
        if self.use_colors {
            style(path).italic().to_string()
        } else {
            path.to_string()
        }
    }

    /// Format a duration
    pub fn duration(&self, duration: Duration) -> String {
        let ms = duration.as_millis();
        let formatted = if ms < 1000 {
            format!("{}ms", ms)
        } else {
            format!("{:.2}s", duration.as_secs_f64())
        };

        if self.use_colors {
            style(formatted).dim().to_string()
        } else {
            formatted
        }
    }

    /// Format JSON with syntax highlighting
    pub fn json(&self, value: &Value) -> Result<String, ClientError> {
        let pretty = serde_json::to_string_pretty(value).map_err(|e| {
            ClientError::SerializationError(format!("Failed to format JSON: {}", e))
        })?;

        if !self.use_colors {
            return Ok(pretty);
        }

        // Simple syntax highlighting for JSON
        let mut result = String::new();
        let mut in_string = false;
        let mut escape_next = false;
        let mut chars = pretty.chars().peekable();

        while let Some(ch) = chars.next() {
            if escape_next {
                result.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '"' if !escape_next => {
                    if in_string {
                        result.push_str(&style(format!("{}", ch)).green().to_string());
                        in_string = false;
                    } else {
                        result.push_str(&style(format!("{}", ch)).green().to_string());
                        in_string = true;
                    }
                }
                '\\' if in_string => {
                    result.push_str(&style(format!("{}", ch)).green().to_string());
                    escape_next = true;
                }
                c if in_string => {
                    result.push_str(&style(format!("{}", c)).green().to_string());
                }
                '{' | '}' | '[' | ']' => {
                    result.push_str(&style(format!("{}", ch)).yellow().to_string());
                }
                ':' => {
                    result.push_str(&style(format!("{}", ch)).cyan().to_string());
                }
                ',' => {
                    result.push_str(&style(format!("{}", ch)).dim().to_string());
                }
                c if c.is_numeric() || c == '.' || c == '-' => {
                    // Look ahead to capture full number
                    let mut number = String::new();
                    number.push(c);

                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_numeric()
                            || next_ch == '.'
                            || next_ch == 'e'
                            || next_ch == 'E'
                            || next_ch == '+'
                            || next_ch == '-'
                        {
                            if let Some(ch) = chars.next() {
                                number.push(ch);
                            }
                        } else {
                            break;
                        }
                    }

                    result.push_str(&style(number).blue().to_string());
                }
                't' => {
                    // Try to match "true"
                    let mut temp_chars = chars.clone();
                    if temp_chars.next() == Some('r')
                        && temp_chars.next() == Some('u')
                        && temp_chars.next() == Some('e')
                    {
                        result.push_str(&style("true").magenta().to_string());
                        chars.nth(2); // consume "rue"
                    } else {
                        result.push(ch);
                    }
                }
                'f' => {
                    // Try to match "false"
                    let mut temp_chars = chars.clone();
                    if temp_chars.next() == Some('a')
                        && temp_chars.next() == Some('l')
                        && temp_chars.next() == Some('s')
                        && temp_chars.next() == Some('e')
                    {
                        result.push_str(&style("false").magenta().to_string());
                        chars.nth(3); // consume "alse"
                    } else {
                        result.push(ch);
                    }
                }
                'n' => {
                    // Try to match "null"
                    let mut temp_chars = chars.clone();
                    if temp_chars.next() == Some('u')
                        && temp_chars.next() == Some('l')
                        && temp_chars.next() == Some('l')
                    {
                        result.push_str(&style("null").red().to_string());
                        chars.nth(2); // consume "ull"
                    } else {
                        result.push(ch);
                    }
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        Ok(result)
    }

    /// Format an MCP response
    pub fn mcp_response(&self, response: &MCPResponse) -> Result<String, ClientError> {
        let mut output = String::new();

        // Response header
        writeln!(
            output,
            "{}",
            self.header(&format!("Response (ID: {})", response.id))
        )
        .map_err(|e| ClientError::SerializationError(format!("Failed to write output: {}", e)))?;

        if let Some(ref error) = response.error {
            writeln!(
                output,
                "{}",
                self.error(&format!("Error {}: {}", error.code, error.message))
            )
            .map_err(|e| {
                ClientError::SerializationError(format!("Failed to write output: {}", e))
            })?;

            if let Some(ref data) = error.data {
                writeln!(output, "\nError details:").map_err(|e| {
                    ClientError::SerializationError(format!("Failed to write output: {}", e))
                })?;
                writeln!(output, "{}", self.json(data)?).map_err(|e| {
                    ClientError::SerializationError(format!("Failed to write output: {}", e))
                })?;
            }
        } else if let Some(ref result) = response.result {
            writeln!(output, "{}", self.success("Success")).map_err(|e| {
                ClientError::SerializationError(format!("Failed to write output: {}", e))
            })?;

            writeln!(output, "\nResult:").map_err(|e| {
                ClientError::SerializationError(format!("Failed to write output: {}", e))
            })?;
            writeln!(output, "{}", self.json(result)?).map_err(|e| {
                ClientError::SerializationError(format!("Failed to write output: {}", e))
            })?;
        } else {
            writeln!(output, "{}", self.success("Success (no result data)")).map_err(|e| {
                ClientError::SerializationError(format!("Failed to write output: {}", e))
            })?;
        }

        Ok(output)
    }

    /// Format a client error
    pub fn client_error(&self, error: &ClientError) -> String {
        match error {
            ClientError::ConfigError(msg) => self.error(&format!("Configuration error: {}", msg)),
            ClientError::ConnectionError(msg) => self.error(&format!("Connection error: {}", msg)),
            ClientError::AuthError(msg) => self.error(&format!("Authentication error: {}", msg)),
            ClientError::RequestError(msg) => self.error(&format!("Request error: {}", msg)),
            ClientError::TimeoutError(msg) => self.error(&format!("Timeout error: {}", msg)),
            ClientError::SerializationError(msg) => {
                self.error(&format!("Serialization error: {}", msg))
            }
            ClientError::IoError(msg) => self.error(&format!("I/O error: {}", msg)),
            ClientError::TransportError(msg) => self.error(&format!("Transport error: {}", msg)),
            ClientError::ProtocolError(msg) => self.error(&format!("Protocol error: {}", msg)),
            ClientError::Core(err) => self.error(&format!("Core error: {}", err)),
        }
    }

    /// Format a progress message
    pub fn progress_message(&self, message: &str) -> String {
        let emoji = if self.use_emojis {
            "⏳ ".to_string()
        } else {
            String::new()
        };
        if self.use_colors {
            format!("{}{}", emoji, style(message).cyan())
        } else {
            format!("{}{}", emoji, message)
        }
    }

    /// Format a success message (alias for consistency)
    pub fn success_message(&self, message: &str) -> String {
        self.success(message)
    }

    /// Format an error message (alias for consistency)
    pub fn error_message(&self, message: &str) -> String {
        self.error(message)
    }

    /// Create a progress bar
    pub fn progress_bar(&self, length: Option<u64>, message: &str) -> ProgressBar {
        let pb = if let Some(len) = length {
            ProgressBar::new(len)
        } else {
            ProgressBar::new_spinner()
        };

        if self.use_colors {
            if let Ok(style) = if length.is_some() {
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
                )
            } else {
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg}")
            } {
                pb.set_style(style);
            }
        } else if let Ok(style) = if length.is_some() {
            ProgressStyle::with_template("[{elapsed_precise}] {bar:40} {pos:>7}/{len:7} {msg}")
        } else {
            ProgressStyle::with_template("[{elapsed_precise}] {msg}")
        } {
            pb.set_style(style);
        }

        pb.set_message(message.to_string());
        pb
    }

    /// Format a table-like output
    pub fn table(&self, headers: &[&str], rows: &[Vec<String>]) -> String {
        if rows.is_empty() {
            return String::new();
        }

        // Calculate column widths
        let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        let mut output = String::new();

        // Header
        let header_line = headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                if self.use_colors {
                    format!("{:width$}", style(h).bold(), width = widths[i])
                } else {
                    format!("{:width$}", h, width = widths[i])
                }
            })
            .collect::<Vec<_>>()
            .join("  ");

        output.push_str(&header_line);
        output.push('\n');

        // Separator
        let separator = widths
            .iter()
            .map(|&w| "-".repeat(w))
            .collect::<Vec<_>>()
            .join("  ");
        output.push_str(&separator);
        output.push('\n');

        // Rows
        for row in rows {
            let row_line = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let width = widths.get(i).copied().unwrap_or(0);
                    format!("{:width$}", cell, width = width)
                })
                .collect::<Vec<_>>()
                .join("  ");

            output.push_str(&row_line);
            output.push('\n');
        }

        output
    }

    /// Format a status summary
    pub fn status_summary(&self, items: &[(String, String, bool)]) -> String {
        let mut output = String::new();

        for (key, value, is_ok) in items {
            let status_symbol = if *is_ok {
                if self.use_emojis {
                    "✅"
                } else {
                    "✓"
                }
            } else if self.use_emojis {
                "❌"
            } else {
                "✗"
            };

            let formatted_key = if self.use_colors {
                style(key).bold().to_string()
            } else {
                key.clone()
            };

            let formatted_value = if self.use_colors {
                if *is_ok {
                    style(value).green().to_string()
                } else {
                    style(value).red().to_string()
                }
            } else {
                value.clone()
            };

            output.push_str(&format!(
                "{} {}: {}\n",
                status_symbol, formatted_key, formatted_value
            ));
        }

        output
    }
}

/// Format a refactor plan into a human-readable sentence
///
/// Produces sentences like:
/// - "Renames function 'old' to 'new' across 3 files"
/// - "Extracts function 'helper' into a new declaration in 2 files"
/// - "Moves code to 'target.rs' affecting 1 file"
pub fn format_plan(plan: &RefactorPlan) -> String {
    use codebuddy_foundation::protocol::refactor_plan::*;

    match plan {
        RefactorPlan::RenamePlan(p) => {
            let kind = &p.metadata.kind;
            let files = p.summary.affected_files;
            let file_text = if files == 1 { "file" } else { "files" };
            format!("Renames {} across {} {}", kind, files, file_text)
        }
        RefactorPlan::ExtractPlan(p) => {
            let kind = &p.metadata.kind;
            let files = p.summary.affected_files;
            let created = p.summary.created_files;
            let file_text = if files == 1 { "file" } else { "files" };
            if created > 0 {
                format!(
                    "Extracts {} into a new declaration in {} {}",
                    kind, files, file_text
                )
            } else {
                format!("Extracts {} in {} {}", kind, files, file_text)
            }
        }
        RefactorPlan::InlinePlan(p) => {
            let kind = &p.metadata.kind;
            let files = p.summary.affected_files;
            let file_text = if files == 1 { "file" } else { "files" };
            format!("Inlines {} in {} {}", kind, files, file_text)
        }
        RefactorPlan::MovePlan(p) => {
            let kind = &p.metadata.kind;
            let files = p.summary.affected_files;
            let file_text = if files == 1 { "file" } else { "files" };
            format!("Moves {} affecting {} {}", kind, files, file_text)
        }
        RefactorPlan::ReorderPlan(p) => {
            let kind = &p.metadata.kind;
            let files = p.summary.affected_files;
            let file_text = if files == 1 { "file" } else { "files" };
            format!("Reorders {} in {} {}", kind, files, file_text)
        }
        RefactorPlan::TransformPlan(p) => {
            let kind = &p.metadata.kind;
            let files = p.summary.affected_files;
            let file_text = if files == 1 { "file" } else { "files" };
            format!("Transforms code ({}) in {} {}", kind, files, file_text)
        }
        RefactorPlan::DeletePlan(p) => {
            let kind = &p.metadata.kind;
            let files = p.summary.affected_files;
            let deleted = p.summary.deleted_files;
            let file_text = if files == 1 { "file" } else { "files" };
            if deleted > 0 {
                format!(
                    "Deletes {} from {} {} ({} files removed)",
                    kind, files, file_text, deleted
                )
            } else {
                format!("Deletes {} from {} {}", kind, files, file_text)
            }
        }
    }
}

/// Convenience functions for common formatting
pub fn format_success(message: &str) -> String {
    Formatter::default().success(message)
}

pub fn format_error(message: &str) -> String {
    Formatter::default().error(message)
}

pub fn format_warning(message: &str) -> String {
    Formatter::default().warning(message)
}

pub fn format_info(message: &str) -> String {
    Formatter::default().info(message)
}

pub fn format_json(value: &Value) -> Result<String, ClientError> {
    Formatter::default().json(value)
}

pub fn format_mcp_response(response: &MCPResponse) -> Result<String, ClientError> {
    Formatter::default().mcp_response(response)
}

pub fn format_client_error(error: &ClientError) -> String {
    Formatter::default().client_error(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codebuddy_foundation::protocol::refactor_plan::*;
    use lsp_types::WorkspaceEdit;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_formatter_basic_messages() {
        let formatter = Formatter::with_settings(false, false);

        assert_eq!(formatter.success("test"), "test");
        assert_eq!(formatter.error("test"), "ERROR: test");
        assert_eq!(formatter.warning("test"), "WARNING: test");
        assert_eq!(formatter.info("test"), "test");
    }

    #[test]
    fn test_json_formatting() {
        let formatter = Formatter::with_settings(false, false);
        let value = json!({
            "name": "test",
            "value": 42,
            "active": true,
            "data": null
        });

        let result = formatter.json(&value).unwrap();
        assert!(result.contains("\"name\": \"test\""));
        assert!(result.contains("\"value\": 42"));
        assert!(result.contains("\"active\": true"));
        assert!(result.contains("\"data\": null"));
    }

    #[test]
    fn test_key_value_formatting() {
        let formatter = Formatter::with_settings(false, false);
        let result = formatter.key_value("URL", "ws://localhost:3000");
        assert_eq!(result, "URL: ws://localhost:3000");
    }

    #[test]
    fn test_table_formatting() {
        let formatter = Formatter::with_settings(false, false);
        let headers = vec!["Name", "Status", "Value"];
        let rows = vec![
            vec!["Item 1".to_string(), "OK".to_string(), "100".to_string()],
            vec!["Item 2".to_string(), "ERROR".to_string(), "0".to_string()],
        ];

        let result = formatter.table(&headers, &rows);
        assert!(result.contains("Name"));
        assert!(result.contains("Status"));
        assert!(result.contains("Value"));
        assert!(result.contains("Item 1"));
        assert!(result.contains("Item 2"));
    }

    // Helper to create test plan metadata
    fn create_test_metadata(kind: &str) -> PlanMetadata {
        PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: kind.to_string(),
            language: "rust".to_string(),
            estimated_impact: "low".to_string(),
            created_at: "2025-10-11T12:00:00Z".to_string(),
        }
    }

    // Helper to create test summary
    fn create_test_summary(affected: usize, created: usize, deleted: usize) -> PlanSummary {
        PlanSummary {
            affected_files: affected,
            created_files: created,
            deleted_files: deleted,
        }
    }

    #[test]
    fn test_format_plan_rename_single_file() {
        let plan = RefactorPlan::RenamePlan(RenamePlan {
            edits: WorkspaceEdit::default(),
            summary: create_test_summary(1, 0, 0),
            warnings: vec![],
            metadata: create_test_metadata("function"),
            file_checksums: HashMap::new(),
            is_consolidation: false,
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Renames function across 1 file");
    }

    #[test]
    fn test_format_plan_rename_multiple_files() {
        let plan = RefactorPlan::RenamePlan(RenamePlan {
            edits: WorkspaceEdit::default(),
            summary: create_test_summary(3, 0, 0),
            warnings: vec![],
            metadata: create_test_metadata("variable"),
            file_checksums: HashMap::new(),
            is_consolidation: false,
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Renames variable across 3 files");
    }

    #[test]
    fn test_format_plan_extract_with_creation() {
        let plan = RefactorPlan::ExtractPlan(ExtractPlan {
            edits: WorkspaceEdit::default(),
            summary: create_test_summary(2, 1, 0),
            warnings: vec![],
            metadata: create_test_metadata("function"),
            file_checksums: HashMap::new(),
        });

        let result = format_plan(&plan);
        assert_eq!(
            result,
            "Extracts function into a new declaration in 2 files"
        );
    }

    #[test]
    fn test_format_plan_extract_without_creation() {
        let plan = RefactorPlan::ExtractPlan(ExtractPlan {
            edits: WorkspaceEdit::default(),
            summary: create_test_summary(1, 0, 0),
            warnings: vec![],
            metadata: create_test_metadata("variable"),
            file_checksums: HashMap::new(),
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Extracts variable in 1 file");
    }

    #[test]
    fn test_format_plan_inline() {
        let plan = RefactorPlan::InlinePlan(InlinePlan {
            edits: WorkspaceEdit::default(),
            summary: create_test_summary(2, 0, 0),
            warnings: vec![],
            metadata: create_test_metadata("constant"),
            file_checksums: HashMap::new(),
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Inlines constant in 2 files");
    }

    #[test]
    fn test_format_plan_move() {
        let plan = RefactorPlan::MovePlan(MovePlan {
            edits: WorkspaceEdit::default(),
            summary: create_test_summary(3, 0, 0),
            warnings: vec![],
            metadata: create_test_metadata("symbol"),
            file_checksums: HashMap::new(),
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Moves symbol affecting 3 files");
    }

    #[test]
    fn test_format_plan_reorder() {
        let plan = RefactorPlan::ReorderPlan(ReorderPlan {
            edits: WorkspaceEdit::default(),
            summary: create_test_summary(1, 0, 0),
            warnings: vec![],
            metadata: create_test_metadata("parameters"),
            file_checksums: HashMap::new(),
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Reorders parameters in 1 file");
    }

    #[test]
    fn test_format_plan_transform() {
        let plan = RefactorPlan::TransformPlan(TransformPlan {
            edits: WorkspaceEdit::default(),
            summary: create_test_summary(2, 0, 0),
            warnings: vec![],
            metadata: create_test_metadata("to_async"),
            file_checksums: HashMap::new(),
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Transforms code (to_async) in 2 files");
    }

    #[test]
    fn test_format_plan_delete_with_file_removal() {
        let plan = RefactorPlan::DeletePlan(DeletePlan {
            deletions: vec![],
            summary: create_test_summary(3, 0, 2),
            warnings: vec![],
            metadata: create_test_metadata("dead_code"),
            file_checksums: HashMap::new(),
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Deletes dead_code from 3 files (2 files removed)");
    }

    #[test]
    fn test_format_plan_delete_without_file_removal() {
        let plan = RefactorPlan::DeletePlan(DeletePlan {
            deletions: vec![],
            summary: create_test_summary(1, 0, 0),
            warnings: vec![],
            metadata: create_test_metadata("unused_imports"),
            file_checksums: HashMap::new(),
        });

        let result = format_plan(&plan);
        assert_eq!(result, "Deletes unused_imports from 1 file");
    }
}
