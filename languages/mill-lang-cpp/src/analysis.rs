//! Analysis capabilities for C++ code
//!
//! Provides stub implementations for code analysis operations.

use mill_foundation::protocol::{ImportGraph, ImportGraphMetadata, ImportInfo, ImportType, SourceLocation};
use mill_plugin_api::{
    capabilities::{ImportAnalyzer, ModuleReferenceScanner},
    ModuleReference, PluginResult, ScanScope, ReferenceKind,
};
use regex::Regex;
use std::path::Path;

pub struct CppAnalysisProvider;

impl ModuleReferenceScanner for CppAnalysisProvider {
    fn scan_references(
        &self,
        content: &str,
        module_name: &str,
        _scope: ScanScope,
    ) -> PluginResult<Vec<ModuleReference>> {
        let re = Regex::new(&format!("#include\\s*[<\"]({}[^>\"]*)[>\"]", regex::escape(module_name))).unwrap();
        let references = re.captures_iter(content).map(|caps| {
            let m = caps.get(0).unwrap();
            let line = content[..m.start()].lines().count();
            let column = m.start() - content.lines().take(line - 1).map(|l| l.len() + 1).sum::<usize>();
            ModuleReference {
                line,
                column,
                length: m.len(),
                text: caps.get(1).unwrap().as_str().to_string(),
                kind: ReferenceKind::Declaration,
            }
        }).collect();
        Ok(references)
    }
}

impl ImportAnalyzer for CppAnalysisProvider {
    fn build_import_graph(&self, file_path: &Path) -> PluginResult<ImportGraph> {
        let content = std::fs::read_to_string(file_path).map_err(|e| mill_plugin_api::PluginError::internal(format!("Failed to read file: {}", e)))?;
        let re = Regex::new(r#"#include\s*[<"]([^>"]+)[>"]"#).unwrap();
        let imports = re.captures_iter(&content).map(|caps| {
            let m = caps.get(0).unwrap();
            let start_byte = m.start();
            let mut line_number = 0;
            let mut last_line_start = 0;
            for (i, line) in content.lines().enumerate() {
                if last_line_start + line.len() >= start_byte {
                    line_number = i;
                    break;
                }
                last_line_start += line.len() + 1;
            }
            let start_column = start_byte - last_line_start;

            ImportInfo {
                module_path: caps.get(1).unwrap().as_str().to_string(),
                import_type: ImportType::CInclude,
                named_imports: vec![],
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_number as u32,
                    start_column: start_column as u32,
                    end_line: line_number as u32,
                    end_column: (start_column + m.len()) as u32,
                },
            }
        }).collect();

        Ok(ImportGraph {
            source_file: file_path.to_string_lossy().to_string(),
            imports,
            importers: vec![],
            metadata: ImportGraphMetadata {
                language: "C++".to_string(),
                parsed_at: chrono::Utc::now(),
                parser_version: "0.1.0".to_string(),
                circular_dependencies: vec![],
                external_dependencies: vec![],
            },
        })
    }
}
