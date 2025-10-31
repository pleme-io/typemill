use crate::import_support::CImportSupport;
use mill_plugin_api::import_support::ImportParser;

#[test]
fn test_parse_imports() {
    let support = CImportSupport;
    let source = r#"
#include <stdio.h>
#include "my_header.h"
#include <other/header.h>

int main() {
    return 0;
}
"#;
    let imports = support.parse_imports(source);
    assert_eq!(imports.len(), 3);
    assert!(imports.contains(&"stdio.h".to_string()));
    assert!(imports.contains(&"my_header.h".to_string()));
    assert!(imports.contains(&"other/header.h".to_string()));
}

#[test]
fn test_contains_import() {
    let support = CImportSupport;
    let source = r#"
#include <stdio.h>
#include "my_header.h"
"#;
    assert!(support.contains_import(source, "stdio.h"));
    assert!(support.contains_import(source, "my_header.h"));
    assert!(!support.contains_import(source, "string.h"));
}