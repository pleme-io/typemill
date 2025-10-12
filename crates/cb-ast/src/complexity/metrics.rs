use super::models::{CodeMetrics, ComplexityMetrics};

/// Language-specific decision point patterns
struct LanguagePatterns {
    decision_keywords: Vec<&'static str>,
    logical_operators: Vec<&'static str>,
}

impl LanguagePatterns {
    /// Get patterns for a specific language
    fn for_language(language: &str) -> Self {
        match language.to_lowercase().as_str() {
            "rust" | "go" | "java" => Self {
                decision_keywords: vec!["if", "else if", "for", "while", "match", "case", "catch"],
                logical_operators: vec!["&&", "||"],
            },
            "typescript" | "javascript" => Self {
                decision_keywords: vec![
                    "if", "else if", "for", "while", "do", "switch", "case", "catch",
                ],
                logical_operators: vec!["&&", "||", "?"],
            },
            "python" => Self {
                decision_keywords: vec![
                    "if", "elif", "for", "while", "except", "case", // case for Python 3.10+
                ],
                logical_operators: vec!["and", "or"],
            },
            _ => {
                // Fallback for unknown languages
                Self {
                    decision_keywords: vec!["if", "for", "while", "case", "catch"],
                    logical_operators: vec!["&&", "||"],
                }
            }
        }
    }
}

/// Calculate cyclomatic complexity for a function body
///
/// Uses a simplified algorithm that counts decision points in the source code.
/// This is not as accurate as building a full control flow graph, but it's fast
/// and works across all languages.
pub fn calculate_complexity(function_body: &str, language: &str) -> u32 {
    let patterns = LanguagePatterns::for_language(language);
    let mut complexity = 1; // Base complexity

    // Count decision keywords
    for keyword in &patterns.decision_keywords {
        complexity += count_keyword_occurrences(function_body, keyword);
    }

    // Count logical operators (each adds a branch)
    for operator in &patterns.logical_operators {
        complexity += count_operator_occurrences(function_body, operator);
    }

    complexity
}

/// Calculate comprehensive complexity metrics (cyclomatic + cognitive + nesting)
///
/// Cognitive complexity adds nesting penalties to better reflect human comprehension difficulty.
/// Unlike cyclomatic complexity, cognitive complexity penalizes nested structures more heavily.
pub fn calculate_complexity_metrics(function_body: &str, language: &str) -> ComplexityMetrics {
    let patterns = LanguagePatterns::for_language(language);

    let mut cyclomatic: u32 = 1; // Base complexity
    let mut cognitive: u32 = 0;
    let mut nesting_level: u32 = 0;
    let mut max_nesting: u32 = 0;

    let lines: Vec<&str> = function_body.lines().collect();

    for line in lines {
        let trimmed = line.trim();

        // Track nesting level by counting braces
        for ch in line.chars() {
            if ch == '{' {
                nesting_level += 1;
                max_nesting = max_nesting.max(nesting_level);
            } else if ch == '}' {
                nesting_level = nesting_level.saturating_sub(1);
            }
        }

        // Count decision keywords
        for keyword in &patterns.decision_keywords {
            let occurrences = count_keyword_occurrences(trimmed, keyword);
            if occurrences > 0 {
                // Cyclomatic: simple count
                cyclomatic += occurrences;

                // Cognitive: base increment + nesting penalty
                // Each decision point gets +1, plus +1 for each nesting level
                cognitive += occurrences + (occurrences * nesting_level);
            }
        }

        // Count logical operators
        for operator in &patterns.logical_operators {
            let occurrences = count_operator_occurrences(trimmed, operator);
            if occurrences > 0 {
                cyclomatic += occurrences;
                cognitive += occurrences + (occurrences * nesting_level);
            }
        }

        // Detect early returns (reduce cognitive complexity)
        if is_early_return(trimmed, language) && nesting_level == 0 {
            // Early returns at function level don't add cognitive complexity
            // (they're actually good for readability)
            cognitive = cognitive.saturating_sub(1);
        }
    }

    ComplexityMetrics {
        cyclomatic,
        cognitive,
        max_nesting_depth: max_nesting,
    }
}

/// Check if a line contains an early return/continue/break
fn is_early_return(line: &str, language: &str) -> bool {
    let line = line.trim();
    match language.to_lowercase().as_str() {
        "rust" | "go" | "java" | "typescript" | "javascript" => {
            line.starts_with("return") || line.starts_with("continue") || line.starts_with("break")
        }
        "python" => {
            line.starts_with("return") || line.starts_with("continue") || line.starts_with("break")
        }
        _ => line.starts_with("return"),
    }
}

/// Count occurrences of a keyword as a whole word (not part of another identifier)
fn count_keyword_occurrences(code: &str, keyword: &str) -> u32 {
    let mut count = 0;

    // Process line by line to skip comments
    for line in code.lines() {
        let trimmed = line.trim();

        // Skip obvious comment lines
        if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with('*')
            || trimmed.starts_with("/*")
        {
            continue;
        }

        // Strip inline comments (simplified - just handles // comments)
        let code_part = line.split("//").next().unwrap_or(line);

        let keyword_bytes = keyword.as_bytes();
        let line_bytes = code_part.as_bytes();

        for (i, window) in line_bytes.windows(keyword.len()).enumerate() {
            if window == keyword_bytes {
                // Check if it's a word boundary before
                let before_ok = i == 0
                    || !line_bytes[i - 1].is_ascii_alphanumeric() && line_bytes[i - 1] != b'_';

                // Check if it's a word boundary after
                let after_index = i + keyword.len();
                let after_ok = after_index >= line_bytes.len()
                    || !line_bytes[after_index].is_ascii_alphanumeric()
                        && line_bytes[after_index] != b'_';

                if before_ok && after_ok {
                    count += 1;
                }
            }
        }
    }

    count
}

/// Count occurrences of an operator
fn count_operator_occurrences(code: &str, operator: &str) -> u32 {
    let mut count = 0;

    // Check if operator is word-like (e.g., "and", "or", "not")
    let is_word_operator = operator.chars().all(|c| c.is_alphabetic());

    if is_word_operator {
        // Word-like operators need word boundary checking (like keywords)
        // Process line by line to skip comments
        for line in code.lines() {
            let trimmed = line.trim();

            // Skip comment lines
            if trimmed.starts_with("//")
                || trimmed.starts_with('#')
                || trimmed.starts_with('*')
                || trimmed.starts_with("/*")
            {
                continue;
            }

            // Strip inline comments
            let code_part = line.split("//").next().unwrap_or(line);

            let operator_bytes = operator.as_bytes();
            let line_bytes = code_part.as_bytes();

            for (i, window) in line_bytes.windows(operator.len()).enumerate() {
                if window == operator_bytes {
                    // Check word boundaries
                    let before_ok = i == 0
                        || !line_bytes[i - 1].is_ascii_alphanumeric() && line_bytes[i - 1] != b'_';

                    let after_index = i + operator.len();
                    let after_ok = after_index >= line_bytes.len()
                        || !line_bytes[after_index].is_ascii_alphanumeric()
                            && line_bytes[after_index] != b'_';

                    if before_ok && after_ok {
                        count += 1;
                    }
                }
            }
        }
    } else {
        // Symbol operators (&&, ||, ==) don't need word boundaries or comment filtering
        let mut start = 0;
        while let Some(pos) = code[start..].find(operator) {
            count += 1;
            start += pos + operator.len();
        }
    }

    count
}

/// Calculate code metrics for a function body
///
/// Analyzes SLOC, comment ratio, and other code quality metrics.
pub fn calculate_code_metrics(function_body: &str, language: &str) -> CodeMetrics {
    let lines: Vec<&str> = function_body.lines().collect();
    let total_lines = lines.len() as u32;

    let mut sloc = 0;
    let mut comment_lines = 0;

    // Language-specific comment patterns
    let (single_line_comment, multi_line_start, multi_line_end) =
        match language.to_lowercase().as_str() {
            "rust" | "go" | "java" | "typescript" | "javascript" => ("//", "/*", "*/"),
            "python" => ("#", "\"\"\"", "\"\"\""),
            _ => ("//", "/*", "*/"),
        };

    let mut in_multiline_comment = false;

    for line in &lines {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Check for multi-line comment boundaries
        if trimmed.contains(multi_line_start) {
            in_multiline_comment = true;
        }

        // Check if this is a comment line
        let is_comment = in_multiline_comment
            || trimmed.starts_with(single_line_comment)
            || trimmed.starts_with("*"); // Continuation of /** */ style

        if is_comment {
            comment_lines += 1;
        } else {
            sloc += 1;
        }

        if trimmed.contains(multi_line_end) {
            in_multiline_comment = false;
        }
    }

    let comment_ratio = if sloc > 0 {
        comment_lines as f64 / sloc as f64
    } else {
        0.0
    };

    CodeMetrics {
        sloc,
        total_lines,
        comment_lines,
        comment_ratio,
        parameters: 0, // Will be filled in by analyze_file_complexity
    }
}

/// Count function parameters from symbol or function signature
///
/// This is a heuristic that counts commas in the parameter list.
pub fn count_parameters(function_body: &str, language: &str) -> u32 {
    // Language-specific function declaration keywords
    let fn_keyword = match language.to_lowercase().as_str() {
        "rust" => "fn ",
        "python" => "def ",
        "typescript" | "javascript" => "function ",
        "go" => "func ",
        "java" => "public ", // Simplified - methods usually start with visibility
        _ => "fn ",
    };

    // Find the line with the function declaration (skip comments and empty lines)
    let fn_line = function_body
        .lines()
        .find(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with(fn_keyword)
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("const fn ")
        })
        .unwrap_or("");

    // Find parameter list between ( and )
    if let Some(paren_start) = fn_line.find('(') {
        if let Some(paren_end) = fn_line[paren_start..].find(')') {
            let params_str = &fn_line[paren_start + 1..paren_start + paren_end];

            // Empty parameter list
            if params_str.trim().is_empty() {
                return 0;
            }

            // Count parameters by counting commas + 1
            // Handle special cases like "self" in Python/Rust
            let param_count = params_str.matches(',').count() as u32 + 1;

            // Adjust for languages with implicit self/this
            match language.to_lowercase().as_str() {
                "python" => {
                    // Python methods have "self" as first param
                    if params_str.trim().starts_with("self,") || params_str.trim() == "self" {
                        param_count.saturating_sub(1)
                    } else {
                        param_count
                    }
                }
                "rust" => {
                    // Rust methods might have &self, &mut self, self
                    if params_str.trim().starts_with("&self")
                        || params_str.trim().starts_with("self")
                        || params_str.trim().starts_with("&mut self")
                    {
                        param_count.saturating_sub(1)
                    } else {
                        param_count
                    }
                }
                _ => param_count,
            }
        } else {
            0
        }
    } else {
        0
    }
}
