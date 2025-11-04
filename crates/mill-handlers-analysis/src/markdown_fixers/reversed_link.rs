use super::{
    apply_edits, generate_unified_diff, FixOutcome, MarkdownContext, MarkdownFixer, TextEdit,
};
use mill_foundation::protocol::analysis_result::{Position, Range};
use regex::Regex;
use serde_json::Value;

pub struct ReversedLinkFixer;

impl MarkdownFixer for ReversedLinkFixer {
    fn id(&self) -> &'static str {
        "reversed_link_syntax"
    }

    fn apply(&self, ctx: &MarkdownContext, _config: &Value) -> FixOutcome {
        let lines: Vec<&str> = ctx.content.lines().collect();
        let mut edits = Vec::new();
        let re = Regex::new(r"\(([^)]+)\)\[([^\]]+)\]").unwrap();

        for (line_num, line) in lines.iter().enumerate() {
            if re.is_match(line) {
                // Found reversed link syntax
                let new_line = re.replace_all(line, "[$2]($1)").to_string();

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
                    new_text: new_line,
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
    fn test_reversed_link_fixer_id() {
        let fixer = ReversedLinkFixer;
        assert_eq!(fixer.id(), "reversed_link_syntax");
    }

    #[test]
    fn test_reversed_link_fixer_corrects_syntax() {
        let content = "Check out (https://example.com)[this link]".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = ReversedLinkFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 1);
        assert_eq!(
            outcome.edits[0].new_text,
            "Check out [this link](https://example.com)"
        );
        assert!(outcome.preview.is_some());
    }

    #[test]
    fn test_reversed_link_fixer_preserves_correct_syntax() {
        let content = "Check out [this link](https://example.com)".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = ReversedLinkFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 0);
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_reversed_link_fixer_handles_multiple_links() {
        let content = "(url1)[text1] and (url2)[text2]".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = ReversedLinkFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 1);
        assert!(outcome.edits[0].new_text.contains("[text1](url1)"));
        assert!(outcome.edits[0].new_text.contains("[text2](url2)"));
    }

    #[test]
    fn test_reversed_link_fixer_preview_mode() {
        let content = "(url)[text]".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = ReversedLinkFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert!(outcome.preview.is_some());
        let preview = outcome.preview.unwrap();
        assert!(preview.contains("--- a/test.md"));
        assert!(preview.contains("-(url)[text]"));
        assert!(preview.contains("+[text](url)"));
    }
}
