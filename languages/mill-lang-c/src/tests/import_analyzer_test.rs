use crate::CPlugin;
use mill_plugin_api::{ImportAnalyzer, LanguagePlugin};
use std::fs;
use tempfile::tempdir;

const SAMPLE_CODE: &str = r#"
#include <stdio.h>
#include "my_header.h"
"#;

#[test]
fn test_build_import_graph() {
    let plugin = CPlugin::default();
    let analyzer = plugin.import_analyzer().unwrap();

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("main.c");
    fs::write(&file_path, SAMPLE_CODE).unwrap();

    let graph = analyzer.build_import_graph(&file_path).unwrap();

    assert_eq!(graph.imports.len(), 2);
    assert_eq!(graph.imports[0].module_path, "stdio.h");
    assert_eq!(graph.imports[1].module_path, "my_header.h");
}