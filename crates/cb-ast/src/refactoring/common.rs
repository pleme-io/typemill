use super::{CodeRange, ExtractableFunction};
use crate::error::{AstError, AstResult};

/// Detect file language from file path
pub fn detect_language(file_path: &str) -> &str {
    if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
        "typescript"
    } else if file_path.ends_with(".js") || file_path.ends_with(".jsx") {
        "javascript"
    } else if file_path.ends_with(".py") {
        "python"
    } else if file_path.ends_with(".rs") {
        "rust"
    } else if file_path.ends_with(".go") {
        "go"
    } else if file_path.ends_with(".java") {
        "java"
    } else if file_path.ends_with(".swift") {
        "swift"
    } else if file_path.ends_with(".cs") {
        "csharp"
    } else {
        "unknown"
    }
}

/// Helper functions
pub fn extract_range_text(source: &str, range: &CodeRange) -> AstResult<String> {
    let lines: Vec<&str> = source.lines().collect();

    if range.start_line == range.end_line {
        // Single line
        let line = lines
            .get(range.start_line as usize)
            .ok_or_else(|| AstError::analysis("Invalid line number"))?;

        Ok(line[range.start_col as usize..range.end_col as usize].to_string())
    } else {
        // Multi-line
        let mut result = String::new();

        // First line
        if let Some(first_line) = lines.get(range.start_line as usize) {
            result.push_str(&first_line[range.start_col as usize..]);
            result.push('\n');
        }

        // Middle lines
        for line_idx in (range.start_line + 1)..range.end_line {
            if let Some(line) = lines.get(line_idx as usize) {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Last line
        if let Some(last_line) = lines.get(range.end_line as usize) {
            result.push_str(&last_line[..range.end_col as usize]);
        }

        Ok(result)
    }
}

#[allow(dead_code)]
pub fn generate_extracted_function(
    source: &str,
    analysis: &ExtractableFunction,
    function_name: &str,
) -> AstResult<String> {
    let params = analysis.required_parameters.join(", ");

    let return_statement = if analysis.return_variables.is_empty() {
        String::new()
    } else if analysis.return_variables.len() == 1 {
        format!("  return {};", analysis.return_variables[0])
    } else {
        format!("  return {{ {} }};", analysis.return_variables.join(", "))
    };

    // Extract the actual code lines from the selected range
    let lines: Vec<&str> = source.lines().collect();
    let range = &analysis.selected_range;
    let extracted_lines = if range.start_line == range.end_line {
        // Single line extraction
        let line = lines[range.start_line as usize];
        let start_col = range.start_col as usize;
        let end_col = range.end_col as usize;
        let extracted_text = &line[start_col..end_col.min(line.len())];
        vec![format!("  {}", extracted_text)]
    } else {
        // Multi-line extraction
        let mut result = Vec::new();
        for line_num in range.start_line..=range.end_line {
            if line_num >= lines.len() as u32 {
                break;
            }
            let line = lines[line_num as usize];
            if line_num == range.start_line {
                // First line - use from start_col to end
                let start_col = range.start_col as usize;
                if start_col < line.len() {
                    result.push(format!("  {}", &line[start_col..]));
                }
            } else if line_num == range.end_line {
                // Last line - use from start to end_col
                let end_col = range.end_col as usize;
                let extracted_text = &line[..end_col.min(line.len())];
                result.push(format!("  {}", extracted_text));
            } else {
                // Middle lines - use entire line with proper indentation
                result.push(format!("  {}", line));
            }
        }
        result
    };

    let extracted_code = extracted_lines.join("\n");

    Ok(format!(
        "function {}({}) {{\n{}\n{}\n}}",
        function_name, params, extracted_code, return_statement
    ))
}

#[allow(dead_code)]
pub fn generate_function_call(
    analysis: &ExtractableFunction,
    function_name: &str,
) -> AstResult<String> {
    let args = analysis.required_parameters.join(", ");

    if analysis.return_variables.is_empty() {
        Ok(format!("{}({});", function_name, args))
    } else if analysis.return_variables.len() == 1 {
        Ok(format!(
            "const {} = {}({});",
            analysis.return_variables[0], function_name, args
        ))
    } else {
        Ok(format!(
            "const {{ {} }} = {}({});",
            analysis.return_variables.join(", "),
            function_name,
            args
        ))
    }
}
