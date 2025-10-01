//! Test Python refactoring functionality

#[cfg(test)]
mod tests {
    use crate::refactoring::{
        plan_extract_function, plan_extract_variable, plan_inline_variable, CodeRange,
    };

    #[tokio::test]
    async fn test_python_extract_function_basic() {
        let source = r#"def calculate_total(price, tax_rate):
    tax_amount = price * tax_rate
    total = price + tax_amount
    return total"#;

        let range = CodeRange {
            start_line: 1,
            start_col: 4,
            end_line: 2,
            end_col: 30,
        };

        let result = plan_extract_function(source, &range, "calculate_tax", "/tmp/test.py", None).await;
        assert!(result.is_ok(), "Python extract_function should work");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 2); // Insert function + replace with call
        assert_eq!(plan.source_file, "/tmp/test.py");
    }

    #[tokio::test]
    async fn test_python_inline_variable_basic() {
        let source = r#"def test_function():
    multiplier = 2.5
    result = some_value * multiplier + 10
    return result"#;

        let result = plan_inline_variable(source, 1, 10, "/tmp/test.py", None).await;
        assert!(result.is_ok(), "Python inline_variable should work");

        let plan = result.unwrap();
        assert!(plan.edits.len() >= 2); // Replace usages + remove declaration
        assert_eq!(plan.source_file, "/tmp/test.py");
    }

    #[tokio::test]
    async fn test_python_extract_variable_basic() {
        let source = r#"def calculate_total(price, tax_rate):
    return price * tax_rate + price"#;

        let result = plan_extract_variable(
            source,
            1,
            11,
            1,
            27,
            Some("tax_amount".to_string()),
            "/tmp/test.py",
            None,
        ).await;
        assert!(result.is_ok(), "Python extract_variable should work");

        let plan = result.unwrap();
        assert_eq!(plan.edits.len(), 2); // Insert variable + replace expression
        assert_eq!(plan.source_file, "/tmp/test.py");
    }

    #[tokio::test]
    async fn test_python_language_detection() {
        let source = "x = 1 + 2";

        // Test that .py files are detected as Python
        let result = plan_extract_variable(source, 0, 4, 0, 9, None, "test.py", None).await;
        assert!(result.is_ok(), "Python file should be detected correctly");

        // Test that the edit plan uses Python syntax (no const)
        let plan = result.unwrap();
        let declaration_edit = &plan.edits[0];
        println!("Python variable declaration: {}", declaration_edit.new_text);

        // Check that it uses Python syntax (no const, proper assignment)
        assert!(
            declaration_edit.new_text.contains("result = 1 + 2")
                || declaration_edit.new_text.contains("extracted = 1 + 2")
                || declaration_edit.new_text.contains("value = 1 + 2"),
            "Should use Python variable assignment syntax: {}",
            declaration_edit.new_text
        );
        assert!(
            !declaration_edit.new_text.contains("const"),
            "Should not use TypeScript const keyword"
        );
    }
}
