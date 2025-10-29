use super::{apply_edits, generate_unified_diff, FixOutcome, MarkdownContext, MarkdownFixer, TextEdit};
use mill_foundation::protocol::analysis_result::{Position, Range};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Configuration options for AutoTocFixer
#[derive(Debug, Deserialize)]
struct AutoTocOptions {
    /// TOC marker to search for (default: "## Table of Contents")
    #[serde(default = "default_marker")]
    marker: String,

    /// Maximum heading depth to include (default: 3)
    #[serde(default = "default_max_depth")]
    max_depth: usize,

    /// Include H1 headings in TOC (default: false)
    #[serde(default)]
    include_h1: bool,

    /// Regex patterns to exclude from TOC (default: ["^TOC$", "^Contents$"])
    #[serde(default = "default_exclude_patterns")]
    exclude_patterns: Vec<String>,
}

fn default_marker() -> String {
    "## Table of Contents".to_string()
}

fn default_max_depth() -> usize {
    3
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "^TOC$".to_string(),
        "^Contents$".to_string(),
        "^Table of Contents$".to_string(),
    ]
}

impl Default for AutoTocOptions {
    fn default() -> Self {
        Self {
            marker: default_marker(),
            max_depth: default_max_depth(),
            include_h1: false,
            exclude_patterns: default_exclude_patterns(),
        }
    }
}

/// Heading extracted from markdown content
#[derive(Debug, Clone)]
struct Heading {
    level: usize,
    text: String,
    line: usize,
}

/// Auto Table of Contents fixer
///
/// Generates or updates a table of contents in markdown files based on headings.
/// Supports customizable markers, depth limits, and exclusion patterns.
pub struct AutoTocFixer;

impl AutoTocFixer {
    /// Generate GitHub-compatible anchor slug from heading text
    ///
    /// Algorithm:
    /// 1. Convert to lowercase
    /// 2. Replace spaces with hyphens
    /// 3. Remove special characters (keep alphanumeric, hyphens, underscores)
    /// 4. Handle duplicates by appending -1, -2, etc.
    fn generate_anchor(text: &str, anchor_counts: &mut HashMap<String, usize>) -> String {
        let base_anchor: String = text
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();

        let count = anchor_counts.entry(base_anchor.clone()).or_insert(0);
        let final_anchor = if *count == 0 {
            base_anchor.clone()
        } else {
            format!("{}-{}", base_anchor, count)
        };
        *count += 1;
        final_anchor
    }

    /// Extract headings from markdown content
    fn extract_headings(content: &str, options: &AutoTocOptions) -> Vec<Heading> {
        let lines: Vec<&str> = content.lines().collect();
        let mut headings = Vec::new();
        let exclude_regex: Vec<regex::Regex> = options
            .exclude_patterns
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect();

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('#') {
                continue;
            }

            // Count heading level
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            if level == 0 || level > 6 {
                continue;
            }

            // Extract heading text
            let rest = trimmed[level..].trim_start();
            if rest.is_empty() {
                continue;
            }

            // Skip if level is outside configured range
            if !options.include_h1 && level == 1 {
                continue;
            }
            if level > options.max_depth {
                continue;
            }

            // Skip if matches exclude pattern
            let should_exclude = exclude_regex.iter().any(|re| re.is_match(rest));
            if should_exclude {
                continue;
            }

            headings.push(Heading {
                level,
                text: rest.to_string(),
                line: line_num,
            });
        }

        headings
    }

    /// Find TOC marker location and extent
    ///
    /// Returns (marker_line, toc_end_line) or None if not found
    fn find_toc_marker(content: &str, marker: &str) -> Option<(usize, usize)> {
        let lines: Vec<&str> = content.lines().collect();

        // Find marker line
        let marker_line = lines.iter().position(|line| line.trim() == marker)?;

        // Find end of TOC section (next heading or --- separator)
        let mut toc_end = marker_line + 1;
        while toc_end < lines.len() {
            let line = lines[toc_end].trim();

            // End at next heading
            if line.starts_with('#') {
                break;
            }

            // End at separator
            if line == "---" || line.starts_with("---") {
                toc_end += 1; // Include separator line
                break;
            }

            toc_end += 1;
        }

        Some((marker_line, toc_end))
    }

    /// Generate TOC markdown from headings
    fn generate_toc(headings: &[Heading], marker: &str) -> String {
        let mut toc = String::new();
        toc.push_str(marker);
        toc.push('\n');
        toc.push('\n');

        let mut anchor_counts = HashMap::new();

        for heading in headings {
            // Skip the TOC marker itself if it appears in headings
            if heading.text == marker.trim_start_matches('#').trim() {
                continue;
            }

            let anchor = Self::generate_anchor(&heading.text, &mut anchor_counts);
            let indent = "  ".repeat(heading.level.saturating_sub(2));
            toc.push_str(&format!("{}- [{}](#{})\n", indent, heading.text, anchor));
        }

        toc
    }
}

impl MarkdownFixer for AutoTocFixer {
    fn id(&self) -> &'static str {
        "auto_toc"
    }

    fn apply(&self, ctx: &MarkdownContext, config: &Value) -> FixOutcome {
        let options: AutoTocOptions = serde_json::from_value(config.clone()).unwrap_or_default();

        // Extract headings
        let headings = Self::extract_headings(&ctx.content, &options);

        // Find TOC marker
        let toc_location = Self::find_toc_marker(&ctx.content, &options.marker);

        let edits = if let Some((marker_line, toc_end)) = toc_location {
            // Generate new TOC
            let new_toc = Self::generate_toc(&headings, &options.marker);
            let lines: Vec<&str> = ctx.content.lines().collect();

            // Replace old TOC section with new TOC
            vec![TextEdit {
                range: Range {
                    start: Position {
                        line: marker_line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: toc_end as u32,
                        character: if toc_end < lines.len() {
                            lines[toc_end - 1].len() as u32
                        } else {
                            0
                        },
                    },
                },
                old_text: lines[marker_line..toc_end].join("\n"),
                new_text: new_toc.trim_end().to_string(),
            }]
        } else {
            // No TOC marker found - no edits
            vec![]
        };

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

        let warnings = if toc_location.is_none() {
            vec![format!(
                "No TOC marker found ('{}') - skipping auto-TOC generation",
                options.marker
            )]
        } else {
            vec![]
        };

        FixOutcome {
            edits,
            preview,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_auto_toc_fixer_id() {
        let fixer = AutoTocFixer;
        assert_eq!(fixer.id(), "auto_toc");
    }

    #[test]
    fn test_generate_anchor_basic() {
        let mut counts = HashMap::new();
        assert_eq!(AutoTocFixer::generate_anchor("Hello World", &mut counts), "hello-world");
        assert_eq!(AutoTocFixer::generate_anchor("Test 123", &mut counts), "test-123");
    }

    #[test]
    fn test_generate_anchor_special_chars() {
        let mut counts = HashMap::new();
        assert_eq!(
            AutoTocFixer::generate_anchor("Hello, World!", &mut counts),
            "hello-world"
        );
        assert_eq!(
            AutoTocFixer::generate_anchor("Test & Example", &mut counts),
            "test--example"
        );
    }

    #[test]
    fn test_generate_anchor_duplicates() {
        let mut counts = HashMap::new();
        assert_eq!(AutoTocFixer::generate_anchor("Test", &mut counts), "test");
        assert_eq!(AutoTocFixer::generate_anchor("Test", &mut counts), "test-1");
        assert_eq!(AutoTocFixer::generate_anchor("Test", &mut counts), "test-2");
    }

    #[test]
    fn test_extract_headings_basic() {
        let content = "# Title\n## Section 1\n### Subsection\n## Section 2";
        let options = AutoTocOptions::default();
        let headings = AutoTocFixer::extract_headings(content, &options);

        assert_eq!(headings.len(), 3); // H1 excluded by default
        assert_eq!(headings[0].text, "Section 1");
        assert_eq!(headings[1].text, "Subsection");
        assert_eq!(headings[2].text, "Section 2");
    }

    #[test]
    fn test_extract_headings_with_h1() {
        let content = "# Title\n## Section 1";
        let mut options = AutoTocOptions::default();
        options.include_h1 = true;
        let headings = AutoTocFixer::extract_headings(content, &options);

        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].text, "Title");
    }

    #[test]
    fn test_extract_headings_max_depth() {
        let content = "## H2\n### H3\n#### H4\n##### H5";
        let mut options = AutoTocOptions::default();
        options.max_depth = 3;
        let headings = AutoTocFixer::extract_headings(content, &options);

        assert_eq!(headings.len(), 2); // Only H2 and H3
    }

    #[test]
    fn test_extract_headings_exclude_patterns() {
        let content = "## TOC\n## Section 1\n## Contents\n## Section 2";
        let options = AutoTocOptions::default();
        let headings = AutoTocFixer::extract_headings(content, &options);

        assert_eq!(headings.len(), 2); // Excludes TOC and Contents
        assert_eq!(headings[0].text, "Section 1");
        assert_eq!(headings[1].text, "Section 2");
    }

    #[test]
    fn test_find_toc_marker_basic() {
        let content = "# Title\n\n## Table of Contents\n\n- Item 1\n\n## Section 1";
        let result = AutoTocFixer::find_toc_marker(content, "## Table of Contents");

        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert_eq!(start, 2); // Line with marker
        assert_eq!(end, 6); // Line before next heading
    }

    #[test]
    fn test_find_toc_marker_with_separator() {
        let content = "## Table of Contents\n\n- Item 1\n\n---\n\n## Section 1";
        let result = AutoTocFixer::find_toc_marker(content, "## Table of Contents");

        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 5); // Includes separator
    }

    #[test]
    fn test_find_toc_marker_not_found() {
        let content = "# Title\n## Section 1";
        let result = AutoTocFixer::find_toc_marker(content, "## Table of Contents");

        assert!(result.is_none());
    }

    #[test]
    fn test_generate_toc_basic() {
        let headings = vec![
            Heading { level: 2, text: "Section 1".to_string(), line: 0 },
            Heading { level: 3, text: "Subsection".to_string(), line: 1 },
            Heading { level: 2, text: "Section 2".to_string(), line: 2 },
        ];
        let toc = AutoTocFixer::generate_toc(&headings, "## Table of Contents");

        assert!(toc.contains("## Table of Contents"));
        assert!(toc.contains("- [Section 1](#section-1)"));
        assert!(toc.contains("  - [Subsection](#subsection)"));
        assert!(toc.contains("- [Section 2](#section-2)"));
    }

    #[test]
    fn test_auto_toc_fixer_preview_mode() {
        let content = "# Title\n\n## Table of Contents\n\nOld content\n\n## Section 1\n### Subsection".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = AutoTocFixer;
        let config = serde_json::json!({});

        let outcome = fixer.apply(&ctx, &config);

        assert!(!outcome.edits.is_empty());
        assert!(outcome.preview.is_some());

        let preview = outcome.preview.unwrap();
        assert!(preview.contains("--- a/test.md"));
        assert!(preview.contains("+++ b/test.md"));
        assert!(preview.contains("Section 1"));
    }

    #[test]
    fn test_auto_toc_fixer_no_marker() {
        let content = "# Title\n## Section 1".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = AutoTocFixer;
        let config = serde_json::json!({});

        let outcome = fixer.apply(&ctx, &config);

        assert!(outcome.edits.is_empty());
        assert!(outcome.preview.is_none());
        assert_eq!(outcome.warnings.len(), 1);
        assert!(outcome.warnings[0].contains("No TOC marker found"));
    }

    #[test]
    fn test_auto_toc_fixer_custom_options() {
        let content = "# Title\n\n## Contents\n\nOld TOC\n\n## Section 1\n### Sub\n#### Deep".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = AutoTocFixer;
        let config = serde_json::json!({
            "marker": "## Contents",
            "max_depth": 4,
            "include_h1": false
        });

        let outcome = fixer.apply(&ctx, &config);

        assert!(!outcome.edits.is_empty());
        assert!(outcome.preview.is_some());
    }

    #[test]
    fn test_auto_toc_fixer_heading_with_special_chars() {
        let content = "## Table of Contents\n\nOld\n\n## Test & Example\n## Hello, World!".to_string();
        let ctx = MarkdownContext::new(content, PathBuf::from("test.md"));
        let fixer = AutoTocFixer;
        let config = serde_json::json!({});

        let outcome = fixer.apply(&ctx, &config);

        assert!(outcome.preview.is_some());
        let preview = outcome.preview.unwrap();
        assert!(preview.contains("test--example"));
        assert!(preview.contains("hello-world"));
    }
}
