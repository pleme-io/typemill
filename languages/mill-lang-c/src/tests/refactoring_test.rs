use crate::{CPlugin};
use mill_plugin_api::RefactoringProvider;

const SIMPLE_SAMPLE_CODE: &str = r#"
#include <stdio.h>

void to_be_extracted() {
    printf("Hello from extracted function!\n");
}

int main() {
    to_be_extracted();
    return 0;
}
"#;

#[tokio::test]
async fn test_refactoring_extract_function() {
    let plugin = CPlugin::default();
    let result = plugin
        .plan_extract_function(SIMPLE_SAMPLE_CODE, 4, 4, "say_hello", "main.c")
        .await;

    assert!(result.is_ok(), "plan_extract_function failed: {:?}", result.err());
    let plan = result.unwrap();

    assert_eq!(plan.edits.len(), 2);

    let new_function_edit = &plan.edits[0];
    let replacement_edit = &plan.edits[1];

    assert!(new_function_edit.new_text.contains("void say_hello()"));
    assert!(new_function_edit.new_text.contains("printf(\"Hello from extracted function!\\n\");"));
    assert!(replacement_edit.new_text.contains("say_hello();"));
}

const SAMPLE_CODE: &str = r#"
#include <stdio.h>

int main() {
    int x = 10;
    int y = 20;
    int z = x + y;
    printf("The answer is %d\n", z);
    return 0;
}
"#;


#[tokio::test]
async fn test_refactoring_inline_variable() {
    let plugin = CPlugin::default();
    let result = plugin
        .plan_inline_variable(SAMPLE_CODE, 4, 8, "main.c")
        .await;

    assert!(result.is_ok(), "plan_inline_variable failed: {:?}", result.err());
    let plan = result.unwrap();

    assert_eq!(plan.edits.len(), 2);

    let declaration_removal_edit = &plan.edits[0];
    let replacement_edit = &plan.edits[1];

    assert_eq!(declaration_removal_edit.new_text, "");
    assert_eq!(replacement_edit.new_text.trim(), "int z = 10 + y;");
}

#[tokio::test]
async fn test_refactoring_extract_variable() {
    let plugin = CPlugin::default();
    let result = plugin
        .plan_extract_variable(SAMPLE_CODE, 6, 12, 6, 17, Some("sum".to_string()), "main.c")
        .await;

    assert!(result.is_ok(), "plan_extract_variable failed: {:?}", result.err());
    let plan = result.unwrap();

    println!("Generated new variable text: {}", plan.edits[0].new_text);
    println!("Generated replacement text: {}", plan.edits[1].new_text);

    assert_eq!(plan.edits.len(), 2);

    let new_variable_edit = &plan.edits[0];
    let replacement_edit = &plan.edits[1];

    assert!(new_variable_edit.new_text.contains("int sum = x + y;"));
    assert_eq!(replacement_edit.new_text.trim(), "int z = sum;");
}