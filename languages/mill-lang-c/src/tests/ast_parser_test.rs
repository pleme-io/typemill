use crate::ast_parser::parse_source;

#[test]
fn test_parse_empty_source() {
    let source = "";
    let parsed_source = parse_source(source);
    assert!(parsed_source.symbols.is_empty());
}