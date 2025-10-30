use super::{FixOutcome, MarkdownContext, MarkdownFixer, TextEdit, generate_unified_diff, apply_edits};
use mill_foundation::protocol::analysis_result::{Position, Range};
use regex::Regex;
use serde_json::Value;

pub struct MissingCodeLangFixer;

impl MissingCodeLangFixer {
    /// Check if block content looks like something that shouldn't get a `text` tag
    /// Returns true if we should skip adding a language tag
    fn should_skip_block(block_content: &str) -> bool {
        // Empty blocks - skip
        if block_content.trim().is_empty() {
            return true;
        }

        // Directory tree patterns
        if block_content.contains("├")
            || block_content.contains("│")
            || block_content.contains("└")
            || block_content.contains("─") {
            return true;
        }

        // ASCII art / box drawing characters
        if block_content.contains("┌")
            || block_content.contains("┐")
            || block_content.contains("└")
            || block_content.contains("┘")
            || block_content.contains("┬")
            || block_content.contains("┴")
            || block_content.contains("├")
            || block_content.contains("┤") {
            return true;
        }

        // CLI output patterns (common shell prompts)
        let first_line = block_content.lines().next().unwrap_or("");
        let trimmed = first_line.trim();
        if trimmed.starts_with('$')
            || trimmed.starts_with('#')
            || trimmed.starts_with('>')
            || trimmed.starts_with('%')
            || trimmed.starts_with("λ") {
            return true;
        }

        // Common CLI output patterns
        if block_content.contains("$ ")
            || block_content.contains("# ")
            || trimmed.contains(" → ") {
            return true;
        }

        false
    }

    /// Extract the content of a code block starting at the given line
    fn extract_block_content(lines: &[&str], start_line: usize) -> String {
        let mut content = String::new();
        for i in (start_line + 1)..lines.len() {
            if lines[i].trim().starts_with("```") {
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(lines[i]);
        }
        content
    }
}

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
                        // Found code fence without language tag - check if we should skip it
                        let block_content = Self::extract_block_content(&lines, line_num);

                        if !Self::should_skip_block(&block_content) {
                            // Only add text tag if it's not a special pattern
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

    #[test]
    fn test_missing_code_lang_fixer_skips_directory_trees() {
        let content = "```\nsrc/\n├── lib.rs\n└── main.rs\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 0, "Should skip directory tree");
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_missing_code_lang_fixer_skips_ascii_diagrams() {
        let content = "```\n┌─────────┐\n│  Box    │\n└─────────┘\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 0, "Should skip ASCII diagram");
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_missing_code_lang_fixer_skips_cli_output() {
        let content = "```\n$ mill setup\n✅ Detected: TypeScript\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 0, "Should skip CLI output");
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_missing_code_lang_fixer_skips_empty_blocks() {
        let content = "```\n\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 0, "Should skip empty blocks");
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_missing_code_lang_fixer_adds_text_to_prose() {
        let content = "```\nThis is some regular text content.\nIt should get a text tag.\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 1, "Should add text tag to prose");
        assert_eq!(outcome.edits[0].new_text, "```text");
    }

    #[test]
    fn test_missing_code_lang_fixer_mixed_blocks() {
        let content = concat!(
            "```\n",
            "Regular prose here\n",
            "```\n\n",
            "```\n",
            "├── src/\n",
            "└── test/\n",
            "```\n\n",
            "```\n",
            "More prose\n",
            "```"
        ).to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        // Should only add text to the 2 prose blocks, skip the directory tree
        assert_eq!(outcome.edits.len(), 2, "Should add text to prose blocks only");
    }
}
