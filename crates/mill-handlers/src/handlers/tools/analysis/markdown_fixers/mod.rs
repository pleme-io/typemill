use mill_foundation::protocol::analysis_result::Range;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub mod trailing_whitespace;
pub mod missing_code_lang;
pub mod malformed_heading;
pub mod reversed_link;
pub mod auto_toc;

pub use trailing_whitespace::TrailingWhitespaceFixer;
pub use missing_code_lang::MissingCodeLangFixer;
pub use malformed_heading::MalformedHeadingFixer;
pub use reversed_link::ReversedLinkFixer;
pub use auto_toc::AutoTocFixer;

/// Text edit for a single replacement
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub old_text: String,
    pub new_text: String,
}

/// Outcome of applying a fixer
#[derive(Debug)]
pub struct FixOutcome {
    /// List of edits to apply
    pub edits: Vec<TextEdit>,
    /// Unified diff preview (optional, for dry-run mode)
    pub preview: Option<String>,
    /// Warnings or informational messages
    pub warnings: Vec<String>,
}

/// Context for a markdown file being fixed
pub struct MarkdownContext {
    /// File content
    pub content: String,
    /// File path
    pub file_path: PathBuf,
    /// SHA-256 hash of content (for optimistic locking)
    pub content_hash: String,
}

impl MarkdownContext {
    /// Create a new context from content and file path
    pub fn new(content: String, file_path: PathBuf) -> Self {
        let content_hash = compute_content_hash(&content);
        Self {
            content,
            file_path,
            content_hash,
        }
    }

    /// Verify that content hasn't changed
    pub fn verify_hash(&self, current_content: &str) -> bool {
        let current_hash = compute_content_hash(current_content);
        self.content_hash == current_hash
    }
}

/// Compute SHA-256 hash of content
fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Trait for markdown fixers
pub trait MarkdownFixer: Send + Sync {
    /// Unique ID for this fixer (matches finding kind)
    fn id(&self) -> &'static str;

    /// Apply the fix to the given context
    fn apply(&self, ctx: &MarkdownContext, config: &Value) -> FixOutcome;
}

/// Generate unified diff between old and new content
///
/// Format:
/// ```text
/// --- a/file.md
/// +++ b/file.md
/// @@ -line,count +line,count @@
/// -old line
/// +new line
/// ```
pub fn generate_unified_diff(
    file_path: &str,
    old_content: &str,
    new_content: &str,
) -> String {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let mut diff = String::new();
    diff.push_str(&format!("--- a/{}\n", file_path));
    diff.push_str(&format!("+++ b/{}\n", file_path));

    // Simple line-by-line diff (not optimized for minimal hunks)
    let mut i = 0;
    let mut j = 0;

    while i < old_lines.len() || j < new_lines.len() {
        // Find next difference
        let mut same_start = i;
        while i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            i += 1;
            j += 1;
            same_start = i;
        }

        if i >= old_lines.len() && j >= new_lines.len() {
            break; // No more differences
        }

        // Find end of difference
        let mut old_end = i;
        let mut new_end = j;

        // Scan ahead to find where they resync
        while old_end < old_lines.len() || new_end < new_lines.len() {
            if old_end < old_lines.len() && new_end < new_lines.len() && old_lines[old_end] == new_lines[new_end] {
                break; // Found resync point
            }
            if old_end < old_lines.len() {
                old_end += 1;
            }
            if new_end < new_lines.len() {
                new_end += 1;
            }
        }

        // Generate hunk header with 3 lines of context
        let context_lines = 3;
        let hunk_old_start = i.saturating_sub(context_lines).max(same_start.saturating_sub(context_lines));
        let hunk_new_start = j.saturating_sub(context_lines).max(same_start.saturating_sub(context_lines));

        let hunk_old_end = (old_end + context_lines).min(old_lines.len());
        let hunk_new_end = (new_end + context_lines).min(new_lines.len());

        let old_count = hunk_old_end - hunk_old_start;
        let new_count = hunk_new_end - hunk_new_start;

        diff.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk_old_start + 1,
            old_count,
            hunk_new_start + 1,
            new_count
        ));

        // Add context lines before change
        for k in hunk_old_start..i {
            if k < old_lines.len() {
                diff.push_str(&format!(" {}\n", old_lines[k]));
            }
        }

        // Add removed lines
        for k in i..old_end {
            if k < old_lines.len() {
                diff.push_str(&format!("-{}\n", old_lines[k]));
            }
        }

        // Add added lines
        for k in j..new_end {
            if k < new_lines.len() {
                diff.push_str(&format!("+{}\n", new_lines[k]));
            }
        }

        // Add context lines after change
        for k in old_end..(old_end + context_lines).min(old_lines.len()) {
            diff.push_str(&format!(" {}\n", old_lines[k]));
        }

        i = old_end;
        j = new_end;
    }

    diff
}

/// Apply edits to content
pub fn apply_edits(content: &str, edits: &[TextEdit]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = content.to_string();

    // Apply edits in reverse order to preserve line numbers
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by(|a, b| {
        b.range.start.line.cmp(&a.range.start.line)
            .then_with(|| b.range.start.character.cmp(&a.range.start.character))
    });

    for edit in sorted_edits {
        let start_line = edit.range.start.line as usize;
        let end_line = edit.range.end.line as usize;

        if start_line >= lines.len() {
            continue;
        }

        // Simple line-level replacement
        let before: String = lines[..start_line].join("\n");
        let after: String = if end_line + 1 < lines.len() {
            lines[end_line + 1..].join("\n")
        } else {
            String::new()
        };

        let mut new_content = before;
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str(&edit.new_text);
        if !after.is_empty() {
            new_content.push('\n');
            new_content.push_str(&after);
        }

        result = new_content;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_content_hash() {
        let content = "hello world";
        let hash1 = compute_content_hash(content);
        let hash2 = compute_content_hash(content);
        assert_eq!(hash1, hash2);

        let different_content = "hello world!";
        let hash3 = compute_content_hash(different_content);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_markdown_context_verify_hash() {
        let content = "test content".to_string();
        let ctx = MarkdownContext::new(content.clone(), PathBuf::from("test.md"));

        assert!(ctx.verify_hash(&content));
        assert!(!ctx.verify_hash("different content"));
    }

    #[test]
    fn test_generate_unified_diff() {
        let old = "line 1\nline 2\nline 3";
        let new = "line 1\nmodified line 2\nline 3";

        let diff = generate_unified_diff("test.md", old, new);

        assert!(diff.contains("--- a/test.md"));
        assert!(diff.contains("+++ b/test.md"));
        assert!(diff.contains("-line 2"));
        assert!(diff.contains("+modified line 2"));
    }
}
