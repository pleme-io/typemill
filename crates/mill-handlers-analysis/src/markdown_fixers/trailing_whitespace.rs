use super::{
    apply_edits, generate_unified_diff, FixOutcome, MarkdownContext, MarkdownFixer, TextEdit,
};
use mill_foundation::protocol::analysis_result::{Position, Range};
use serde_json::Value;

pub struct TrailingWhitespaceFixer;

impl MarkdownFixer for TrailingWhitespaceFixer {
    fn id(&self) -> &'static str {
        "trailing_whitespace"
    }

    fn apply(&self, ctx: &MarkdownContext, _config: &Value) -> FixOutcome {
        let lines: Vec<&str> = ctx.content.lines().collect();
        let mut edits = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim_end();
            if trimmed.len() != line.len() {
                // Found trailing whitespace
                edits.push(TextEdit {
                    range: Range {
                        start: Position {
                            line: line_num as u32,
                            character: 0,
                        },
                        end: Position {
                            line: line_num as u32,
                            character: line.len() as u32,
                        },
                    },
                    old_text: line.to_string(),
                    new_text: trimmed.to_string(),
                });
            }
        }

        let preview = if !edits.is_empty() {
            let new_content = apply_edits(&ctx.content, &edits);
            Some(generate_unified_diff(
                &ctx.file_path.to_string_lossy(),
                &ctx.content,
                &new_content,
            ))
        } else {
            None
        };

        FixOutcome {
            edits,
            preview,
            warnings: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_trailing_whitespace_fixer_id() {
        let fixer = TrailingWhitespaceFixer;
        assert_eq!(fixer.id(), "trailing_whitespace");
    }

    #[test]
    fn test_trailing_whitespace_fixer_removes_trailing_spaces() {
        let content = "line 1   \nline 2\nline 3\t\t".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = TrailingWhitespaceFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 2); // line 1 and line 3
        assert!(outcome.preview.is_some());
        assert!(outcome.preview.unwrap().contains("-line 1   "));
        assert_eq!(outcome.warnings.len(), 0);
    }

    #[test]
    fn test_trailing_whitespace_fixer_no_changes() {
        let content = "line 1\nline 2\nline 3".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = TrailingWhitespaceFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 0);
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_trailing_whitespace_fixer_preview_mode() {
        let content = "line 1  \nline 2".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = TrailingWhitespaceFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert!(outcome.preview.is_some());
        let preview = outcome.preview.unwrap();
        assert!(preview.contains("--- a/test.md"));
        assert!(preview.contains("+++ b/test.md"));
    }
}
