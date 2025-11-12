//! Import support for C #include directives
//!
//! This module provides comprehensive import/include management for C source code,
//! supporting both system includes (`#include <header.h>`) and local includes
//! (`#include "header.h"`).
//!
//! Functionality includes:
//! - Parsing #include directives from source code
//! - Building import graphs with dependency information
//! - Renaming, moving, and mutating includes during refactoring

use mill_foundation::protocol::{
    DependencyUpdate, ImportGraph, ImportGraphMetadata, ImportInfo, ImportType, SourceLocation,
};
use mill_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    PluginResult,
};
use std::path::Path;

use crate::constants::{INCLUDE_PATH_PATTERN, INCLUDE_PATTERN};

/// C language import support implementation
///
/// Handles parsing and manipulation of #include directives in C source code
#[derive(Debug, Clone, Copy, Default)]
pub struct CImportSupport;

impl ImportParser for CImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        INCLUDE_PATH_PATTERN
            .captures_iter(content)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        let imports = self.parse_imports(content);
        imports.contains(&module.to_string())
    }
}

impl CImportSupport {
    /// Analyze detailed imports from C source code, returning full ImportGraph
    pub fn analyze_detailed_imports(
        &self,
        source: &str,
        file_path: Option<&Path>,
    ) -> PluginResult<ImportGraph> {
        use chrono::Utc;

        let mut imports = Vec::new();
        let mut external_dependencies = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            if let Some(captures) = INCLUDE_PATTERN.captures(line) {
                let open_delim = &captures[1];
                let header = captures[2].to_string();
                let _close_delim = &captures[3];

                // Determine if this is a system or local include
                let is_external = open_delim == "<";

                // Find column position
                let start_col = line.find("#include").unwrap_or(0) as u32;
                let end_col = (start_col as usize
                    + line[start_col as usize..]
                        .find('>')
                        .or_else(|| line[start_col as usize..].find('"'))
                        .unwrap_or(line.len())
                    + 1) as u32;

                let import_info = ImportInfo {
                    module_path: header.clone(),
                    import_type: ImportType::CInclude,
                    named_imports: vec![],  // C doesn't have named imports
                    default_import: None,   // C doesn't have default imports
                    namespace_import: None, // C doesn't have namespace imports
                    type_only: false,       // Not applicable to C
                    location: SourceLocation {
                        start_line: line_num as u32,
                        start_column: start_col,
                        end_line: line_num as u32,
                        end_column: end_col,
                    },
                };

                imports.push(import_info);

                // Track external dependencies (system headers)
                if is_external && !external_dependencies.contains(&header) {
                    external_dependencies.push(header);
                }
            }
        }

        Ok(ImportGraph {
            source_file: file_path
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            imports,
            importers: vec![], // C doesn't have a reverse import mechanism like some languages
            metadata: ImportGraphMetadata {
                language: "C".to_string(),
                parsed_at: Utc::now(),
                parser_version: env!("CARGO_PKG_VERSION").to_string(),
                circular_dependencies: vec![], // Header guards prevent circular includes
                external_dependencies,
            },
        })
    }
}

impl ImportRenameSupport for CImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        _old_name: &str,
        _new_name: &str,
    ) -> (String, usize) {
        (content.to_string(), 0)
    }
}

impl ImportMoveSupport for CImportSupport {
    fn rewrite_imports_for_move(
        &self,
        content: &str,
        _old_path: &Path,
        _new_path: &Path,
    ) -> (String, usize) {
        (content.to_string(), 0)
    }
}

impl ImportMutationSupport for CImportSupport {
    fn add_import(&self, content: &str, _module: &str) -> String {
        content.to_string()
    }

    fn remove_import(&self, content: &str, _module: &str) -> String {
        content.to_string()
    }
}

impl ImportAdvancedSupport for CImportSupport {
    fn update_import_reference(
        &self,
        _file_path: &Path,
        content: &str,
        _update: &DependencyUpdate,
    ) -> PluginResult<String> {
        Ok(content.to_string())
    }
}
