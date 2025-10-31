use crate::CPlugin;
use mill_plugin_api::{LanguagePlugin, ModuleReferenceScanner, ReferenceKind, ScanScope};

const SAMPLE_CODE: &str = r#"
#include <stdio.h>
#include "my_header.h"

// #include <commented_out.h>

int main() {
    return 0;
}
"#;

#[test]
fn test_scan_references_all() {
    let plugin = CPlugin::default();
    let scanner = plugin.module_reference_scanner().unwrap();

    let references = scanner.scan_references(SAMPLE_CODE, "", ScanScope::All).unwrap();

    assert_eq!(references.len(), 3);
    assert_eq!(references[0].text, "stdio.h");
    assert_eq!(references[0].kind, ReferenceKind::Declaration);
    assert_eq!(references[1].text, "my_header.h");
    assert_eq!(references[1].kind, ReferenceKind::Declaration);
    assert_eq!(references[2].text, "commented_out.h");
    assert_eq!(references[2].kind, ReferenceKind::Declaration);
}

#[test]
fn test_scan_references_code_only() {
    let plugin = CPlugin::default();
    let scanner = plugin.module_reference_scanner().unwrap();

    let references = scanner.scan_references(SAMPLE_CODE, "", ScanScope::AllUseStatements).unwrap();

    assert_eq!(references.len(), 2);
    assert_eq!(references[0].text, "stdio.h");
    assert_eq!(references[1].text, "my_header.h");
}