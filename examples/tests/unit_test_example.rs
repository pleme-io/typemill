// Example: Unit Test Pattern
// Location: crates/*/src/ (inline with #[test] or #[cfg(test)])
// Purpose: Test individual functions, structs, methods in isolation

// Example function
pub fn parse_symbol(input: &str) -> Symbol {
    // Implementation
    Symbol {
        kind: SymbolKind::Function,
        name: "main".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_symbol_name() {
        let input = "fn main()";
        let symbol = parse_symbol(input);
        assert_eq!(symbol.kind, SymbolKind::Function);
        assert_eq!(symbol.name, "main");
    }

    #[test]
    fn test_parse_symbol_empty() {
        let input = "";
        let symbol = parse_symbol(input);
        assert_eq!(symbol.name, "");
    }
}
