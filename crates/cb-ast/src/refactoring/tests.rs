use super::common::*;

#[test]
fn test_extract_range_text_single_line() {
    let source = "const message = 'hello world';";
    let range = super::CodeRange {
        start_line: 0,
        start_col: 6,
        end_line: 0,
        end_col: 13,
    };

    let result = extract_range_text(source, &range).unwrap();
    assert_eq!(result, "message");
}

#[test]
fn test_extract_range_text_multi_line() {
    let source = "const x = 1;\nconst y = 2;\nconst z = 3;";
    let range = super::CodeRange {
        start_line: 0,
        start_col: 6,
        end_line: 1,
        end_col: 7,
    };

    let result = extract_range_text(source, &range).unwrap();
    assert_eq!(result, "x = 1;\nconst y");
}