use super::unused_imports::{extract_imported_symbols, is_symbol_used_in_code};

#[test]
fn test_extract_imported_symbols_typescript() {
    let content = r#"
import { useState, useEffect } from 'react';
import Button from './components/Button';
"#;

    let symbols = extract_imported_symbols(content, "react");
    assert!(symbols.contains(&"useState".to_string()));
    assert!(symbols.contains(&"useEffect".to_string()));

    let button_symbols = extract_imported_symbols(content, "./components/Button");
    assert!(button_symbols.contains(&"Button".to_string()));
}

#[test]
fn test_extract_imported_symbols_rust() {
    let content = r#"
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
"#;

    let symbols = extract_imported_symbols(content, "std::collections");
    assert!(symbols.contains(&"HashMap".to_string()));
    assert!(symbols.contains(&"HashSet".to_string()));

    let path_symbols = extract_imported_symbols(content, "std::path");
    assert!(path_symbols.contains(&"PathBuf".to_string()));
}

#[test]
fn test_is_symbol_used_in_code() {
    let content = r#"
import { useState, useEffect } from 'react';

function App() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}
"#;

    // useState is used
    assert!(is_symbol_used_in_code(content, "useState"));

    // useEffect is imported but not used
    assert!(!is_symbol_used_in_code(content, "useEffect"));
}

#[test]
fn test_is_symbol_used_multiple_occurrences() {
    let content = r#"
import { Button } from './components';

export function Page() {
    return <Button>Click</Button>;
}
"#;

    // Button appears twice (import + usage)
    assert!(is_symbol_used_in_code(content, "Button"));
}