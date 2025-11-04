use super::{
    apply_edits, generate_unified_diff, FixOutcome, MarkdownContext, MarkdownFixer, TextEdit,
};
use mill_foundation::protocol::analysis_result::{Position, Range};
use regex::Regex;
use serde_json::Value;

pub struct MissingCodeLangFixer;

impl MissingCodeLangFixer {
    /// Detect the programming language of a code block
    /// Returns Some(language_tag) if confident, None if unsure
    fn detect_language(content: &str) -> Option<&'static str> {
        if content.trim().is_empty() {
            return None;
        }

        // Skip non-code patterns
        if Self::is_non_code_pattern(content) {
            return None;
        }

        // Calculate scores for each language
        let rust_score = Self::rust_score(content);
        let js_score = Self::javascript_score(content);
        let python_score = Self::python_score(content);
        let json_score = Self::json_score(content);
        let bash_score = Self::bash_score(content);
        let toml_score = Self::toml_score(content);

        // Find the language with highest score
        let max_score = rust_score
            .max(js_score)
            .max(python_score)
            .max(json_score)
            .max(bash_score)
            .max(toml_score);

        // Only suggest if confidence is high enough (score >= 3)
        // Score of 3 means multiple strong signals, reducing false positives
        // Err on the side of caution - only tag when very confident
        if max_score < 3 {
            return None;
        }

        // Return the language with highest score
        if rust_score == max_score {
            Some("rust")
        } else if js_score == max_score {
            Some("javascript")
        } else if python_score == max_score {
            Some("python")
        } else if json_score == max_score {
            Some("json")
        } else if bash_score == max_score {
            Some("bash")
        } else if toml_score == max_score {
            Some("toml")
        } else {
            None
        }
    }

    /// Check if content is a non-code pattern (directory tree, ASCII art, CLI output, quoted strings, etc.)
    fn is_non_code_pattern(content: &str) -> bool {
        let trimmed = content.trim();

        // Numbered lists like [0], [1], [2] (not TOML sections)
        let lines: Vec<&str> = trimmed.lines().collect();
        if lines.len() >= 2 {
            let has_numbered_brackets = lines
                .iter()
                .filter(|l| {
                    let t = l.trim();
                    // Match [0], [1], [2] etc. at start of line
                    t.starts_with('[')
                        && t.len() >= 3
                        && t.chars().nth(1).map_or(false, |c| c.is_ascii_digit())
                })
                .count()
                >= 2;
            if has_numbered_brackets {
                return true;
            }
        }

        // Error messages (often contain keywords but aren't code)
        // Common patterns: "error:", "Error:", "ERROR:", "failed to"
        if trimmed.starts_with("error:")
            || trimmed.starts_with("Error:")
            || trimmed.starts_with("ERROR:")
            || trimmed.contains("failed to")
            || trimmed.contains("Failed to")
            || trimmed.contains("error[E")
        {
            // Rust error codes
            return true;
        }

        // Test assertion patterns (contain code but aren't actual code)
        if trimmed.contains("should contain")
            || trimmed.contains("Should contain")
            || trimmed.contains("Actual:")
            || trimmed.contains("Expected:")
            || trimmed.contains("should be")
            || trimmed.contains("Should be")
        {
            return true;
        }

        // Relative path imports (JavaScript/TypeScript, not Python)
        // Python uses module paths like "foo.bar", not "./foo" or "../bar"
        if (trimmed.contains("from './")
            || trimmed.contains("from \"./")
            || trimmed.contains("from '../")
            || trimmed.contains("from \"../"))
            && trimmed.contains("import")
        {
            return true;
        }

        // Numbered prose (1., 2., 3. or 1), 2), 3))
        let has_numbered_prose = lines
            .iter()
            .filter(|l| {
                let t = l.trim();
                // Match "1. ", "2. ", or "1) ", "2) "
                (t.starts_with("1.")
                    || t.starts_with("2.")
                    || t.starts_with("3.")
                    || t.starts_with("1)")
                    || t.starts_with("2)")
                    || t.starts_with("3)"))
                    && t.len() > 2
                    && t.chars().nth(2) == Some(' ')
            })
            .count()
            >= 2;
        if has_numbered_prose {
            return true;
        }

        // Quoted strings (user instructions, not code)
        // Check if all non-empty lines start and end with quotes
        let non_empty_lines: Vec<&str> = trimmed.lines().filter(|l| !l.trim().is_empty()).collect();
        if !non_empty_lines.is_empty()
            && non_empty_lines.iter().all(|line| {
                let l = line.trim();
                l.starts_with('"') && l.ends_with('"')
            })
        {
            return true;
        }

        // Directory tree patterns
        if content.contains("├")
            || content.contains("│")
            || content.contains("└")
            || content.contains("─")
        {
            return true;
        }

        // ASCII art / box drawing
        if content.contains("┌")
            || content.contains("┐")
            || content.contains("┘")
            || content.contains("┬")
            || content.contains("┴")
            || content.contains("┤")
        {
            return true;
        }

        // CLI output with prompts (but not shebangs)
        let first_line = content.lines().next().unwrap_or("").trim();
        if first_line.starts_with("#!/") {
            // Shebang - not CLI output, likely bash/shell script
            return false;
        }
        if first_line.starts_with('$')
            || first_line.starts_with('#')
            || first_line.starts_with('>')
            || first_line.starts_with('%')
            || first_line.starts_with("λ")
        {
            return true;
        }

        false
    }

    /// Score content for Rust patterns
    fn rust_score(content: &str) -> usize {
        let mut score = 0;
        if content.contains("fn ") {
            score += 2;
        }
        if content.contains("impl ") {
            score += 2;
        }
        if content.contains("pub ") {
            score += 1;
        }
        if content.contains("use ") {
            score += 1;
        }
        if content.contains("::") {
            score += 1;
        }
        if content.contains("->") {
            score += 1;
        }
        if content.contains("let ") {
            score += 1;
        }
        if content.contains("struct ") {
            score += 1;
        }
        if content.contains("enum ") {
            score += 1;
        }
        if content.contains("mod ") {
            score += 1;
        }
        score
    }

    /// Score content for JavaScript/TypeScript patterns
    fn javascript_score(content: &str) -> usize {
        let mut score = 0;
        if content.contains("function ") {
            score += 2;
        }
        if content.contains("const ") {
            score += 1;
        }
        if content.contains("let ") {
            score += 1;
        }
        if content.contains("var ") {
            score += 1;
        }
        if content.contains(" => ") {
            score += 2;
        }
        if content.contains("interface ") {
            score += 1;
        }
        if content.contains("type ") {
            score += 1;
        }
        if content.contains("import ") {
            score += 1;
        }
        if content.contains("export ") {
            score += 1;
        }
        if content.contains("console.log") {
            score += 2;
        }
        score
    }

    /// Score content for Python patterns
    fn python_score(content: &str) -> usize {
        let mut score = 0;
        if content.contains("def ") {
            score += 2;
        }
        if content.contains("class ") {
            score += 1;
        }
        // Be conservative with "import" and "from" - common in prose
        // Only count if followed by actual module patterns
        if content.contains("import ") && (content.contains(".") || content.contains(" as ")) {
            score += 1;
        }
        if content.contains("from ") && content.contains(" import ") && content.contains(".") {
            score += 2;
        }
        if content.contains("    ") {
            score += 1;
        } // Indentation
        if content.contains("self.") {
            score += 1;
        }
        if content.contains("print(") {
            score += 1;
        }
        if content.contains("__init__") {
            score += 2;
        }
        score
    }

    /// Score content for JSON patterns
    fn json_score(content: &str) -> usize {
        let trimmed = content.trim();
        if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            return 0;
        }

        let mut score = 0;
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            score += 2;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            score += 2;
        }
        if content.contains("\"") && content.contains(":") {
            score += 2;
        }

        // Try parsing as JSON
        if serde_json::from_str::<Value>(content).is_ok() {
            score += 5; // Very confident if valid JSON
        }
        score
    }

    /// Score content for Bash/Shell patterns
    fn bash_score(content: &str) -> usize {
        let mut score = 0;
        if content.contains("#!/bin/bash") || content.contains("#!/bin/sh") {
            score += 3;
        }
        if content.contains("cargo ") {
            score += 1;
        }
        if content.contains("npm ") {
            score += 1;
        }
        if content.contains("git ") {
            score += 1;
        }
        if content.contains(" && ") {
            score += 1;
        }
        if content.contains("export ") {
            score += 1;
        }
        if content.contains("echo ") {
            score += 1;
        }
        score
    }

    /// Score content for TOML patterns
    fn toml_score(content: &str) -> usize {
        let mut score = 0;
        // TOML sections: [section.name] or [dependencies]
        // Exclude numbered lists [0], [1]
        let section_re = Regex::new(r"^\[[\w\.-]+\]").unwrap();
        if section_re.is_match(content) {
            // Check it's not a numbered list like [0], [1]
            let first_line = content.lines().next().unwrap_or("");
            if !Regex::new(r"^\[\d+\]").unwrap().is_match(first_line) {
                score += 2;
            }
        }
        // TOML key-value: key = value
        if content.contains(" = ") && !content.contains("==") {
            score += 2;
        }
        score
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
                        // Found code fence without language tag - try to detect the language
                        let block_content = Self::extract_block_content(&lines, line_num);

                        if let Some(detected_lang) = Self::detect_language(&block_content) {
                            // Detected a language - suggest the appropriate tag
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
                                new_text: format!("```{}", detected_lang),
                            });
                        }
                        // If no language detected, skip (don't add anything)
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
    fn test_missing_code_lang_fixer_detects_rust() {
        let content =
            "```\npub fn main() {\n    let x = 42;\n    println!(\"hello\");\n}\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 1);
        assert_eq!(outcome.edits[0].new_text, "```rust");
        assert!(outcome.preview.is_some());
    }

    #[test]
    fn test_missing_code_lang_fixer_detects_javascript() {
        let content = "```\nfunction hello() {\n  console.log(\"world\");\n}\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 1);
        assert_eq!(outcome.edits[0].new_text, "```javascript");
    }

    #[test]
    fn test_missing_code_lang_fixer_detects_json() {
        let content = "```\n{\n  \"name\": \"test\",\n  \"version\": \"1.0.0\"\n}\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 1);
        assert_eq!(outcome.edits[0].new_text, "```json");
    }

    #[test]
    fn test_missing_code_lang_fixer_detects_bash() {
        let content = "```\n#!/bin/bash\necho \"hello\"\ncargo build\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert_eq!(outcome.edits.len(), 1);
        assert_eq!(outcome.edits[0].new_text, "```bash");
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
        let content = "```\npub fn test1() {\n    let x = 1;\n}\n```\n\nSome text\n\n```javascript\nblock2\n```\n\n```\nimpl MyStruct {\n    pub fn test3() {}\n}\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        // Should detect rust for both untagged blocks
        assert_eq!(outcome.edits.len(), 2);
        assert_eq!(outcome.edits[0].new_text, "```rust");
        assert_eq!(outcome.edits[1].new_text, "```rust");
    }

    #[test]
    fn test_missing_code_lang_fixer_preview_mode() {
        let content = "```\nuse std::io;\npub fn example() {\n    let x = 1;\n}\n```".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        assert!(outcome.preview.is_some());
        let preview = outcome.preview.unwrap();
        assert!(preview.contains("--- a/test.md"));
        assert!(preview.contains("-```"));
        assert!(preview.contains("+```rust"));
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
    fn test_missing_code_lang_fixer_skips_undetectable_prose() {
        let content =
            "```\nThis is some regular text content.\nIt doesn't match any language pattern.\n```"
                .to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        // Should skip since we can't confidently detect a language
        assert_eq!(
            outcome.edits.len(),
            0,
            "Should skip prose without clear language"
        );
        assert!(outcome.preview.is_none());
    }

    #[test]
    fn test_missing_code_lang_fixer_mixed_blocks() {
        let content = concat!(
            "```\n",
            "pub fn rust_code() {\n",
            "    let x: i32 = 42;\n",
            "}\n",
            "```\n\n",
            "```\n",
            "├── src/\n",
            "└── test/\n",
            "```\n\n",
            "```\n",
            "function jsCode() {\n",
            "  console.log(\"hello\");\n",
            "}\n",
            "```"
        )
        .to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = MissingCodeLangFixer;

        let outcome = fixer.apply(&ctx, &Value::Null);

        // Should detect rust and js, skip the directory tree
        assert_eq!(
            outcome.edits.len(),
            2,
            "Should detect languages in code blocks"
        );
        assert_eq!(outcome.edits[0].new_text, "```rust");
        assert_eq!(outcome.edits[1].new_text, "```javascript");
    }
}
