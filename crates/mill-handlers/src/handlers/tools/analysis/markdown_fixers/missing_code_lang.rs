use super::{FixOutcome, MarkdownContext, MarkdownFixer, TextEdit, generate_unified_diff, apply_edits};
use mill_foundation::protocol::analysis_result::{Position, Range};
use regex::Regex;
use serde_json::Value;

pub struct MissingCodeLangFixer;

impl MarkdownFixer for MissingCodeLangFixer {
    fn id(&self) -> &'static str {
        "missing_code_language_tag"
    }

    fn apply(&self, ctx: &MarkdownContext, _config: &Value) -> FixOutcome {
        let lines: Vec<&str> = ctx.content.lines().collect();
        let mut edits = Vec::new();
        let re = Regex::new(r"^```\s*$").unwrap();
        let mut in_code_block = false;

        for (line_num, line) in lines.iter().enumerate() {
            if line.trim().starts_with("```") {
                if !in_code_block {
                    // Opening fence
                    if re.is_match(line) {
                        // Found code fence without language tag
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
                            new_text: "```text".to_string(),
                        });
                    }
                    in_code_block = true;
                } else {
                    // Closing fence
                    in_code_block = false;
                }
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
    fn test_missing_code_lang_fixer_id() {
        let fixer = MissingCodeLangFixer;
        assert_eq!(fixer.id(), "missing_code_language_tag");
    }

    #[test]
    fn test_missing_code_lang_fixer_adds_text_tag() {
        let content = "```\ncode here\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 1);
        assert_eq!(outcome.edits[0].new_text, "```text");
        assert!(outcome.preview.is_some());
    }

    #[test]
    fn test_missing_code_lang_fixer_preserves_existing_lang() {
        let content = "```rust\ncode here\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 0);
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_missing_code_lang_fixer_handles_multiple_blocks() {
        let content = "```\nblock1\n```\n\nSome text\n\n```javascript\nblock2\n```\n\n```\nblock3\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 2); // block1 and block3
    }

    #[test]
    fn test_missing_code_lang_fixer_preview_mode() {
        let content = "```\ncode\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert!(outcome.preview.is_some());
        let preview = outcome.preview.unwrap();
        assert!(preview.contains("--- a/test.md"));
        assert!(preview.contains("-```"));
        assert!(preview.contains("+```text"));
    }
}
