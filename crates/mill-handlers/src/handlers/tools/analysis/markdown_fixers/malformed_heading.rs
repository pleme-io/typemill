use super::{FixOutcome, MarkdownContext, MarkdownFixer, TextEdit, generate_unified_diff, apply_edits};
use mill_foundation::protocol::analysis_result::{Position, Range};
use regex::Regex;
use serde_json::Value;

pub struct MalformedHeadingFixer;

impl MarkdownFixer for MalformedHeadingFixer {
    fn id(&self) -> &'static str {
        "malformed_heading"
    }

    fn apply(&self, ctx: &MarkdownContext, _config: &Value) -> FixOutcome {
        let lines: Vec<&str> = ctx.content.lines().collect();
        let mut edits = Vec::new();
        let re = Regex::new(r"^(#{1,6})([^\s#])").unwrap();

        for (line_num, line) in lines.iter().enumerate() {
            if let Some(captures) = re.captures(line) {
                let hashes = captures.get(1).unwrap().as_str();
                let rest = &line[hashes.len()..];

                // Found malformed heading (no space after #)
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
                    new_text: format!("{} {}", hashes, rest),
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
    fn test_malformed_heading_fixer_id() {
        let fixer = MalformedHeadingFixer;
        assert_eq!(fixer.id(), "malformed_heading");
    }

    #[test]
    fn test_malformed_heading_fixer_adds_space() {
        let content = "#Heading 1\n##Heading 2".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MalformedHeadingFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 2);
        assert_eq!(outcome.edits[0].new_text, "# Heading 1");
        assert_eq!(outcome.edits[1].new_text, "## Heading 2");
        assert!(outcome.preview.is_some());
    }

    #[test]
    fn test_malformed_heading_fixer_preserves_correct_headings() {
        let content = "# Heading 1\n## Heading 2".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MalformedHeadingFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 0);
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_malformed_heading_fixer_handles_various_levels() {
        let content = "#H1\n##H2\n###H3\n####H4\n#####H5\n######H6".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MalformedHeadingFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 6);
        assert_eq!(outcome.edits[0].new_text, "# H1");
        assert_eq!(outcome.edits[5].new_text, "###### H6");
    }

    #[test]
    fn test_malformed_heading_fixer_preview_mode() {
        let content = "#Test".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MalformedHeadingFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert!(outcome.preview.is_some());
        let preview = outcome.preview.unwrap();
        assert!(preview.contains("--- a/test.md"));
        assert!(preview.contains("-#Test"));
        assert!(preview.contains("+# Test"));
    }
}
