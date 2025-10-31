use async_trait::async_trait;
use lazy_static::lazy_static;
use mill_plugin_api::{
    ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
    ImportRenameSupport,
};
use regex::Regex;
use std::path::Path;

#[derive(Default)]
pub struct SwiftImportSupport;

lazy_static! {
    static ref IMPORT_REGEX: Regex =
        Regex::new(r"(?m)^\s*import\s+([a-zA-Z0-9_]+)").expect("Invalid regex for Swift import parsing");
}

#[async_trait]
impl ImportParser for SwiftImportSupport {
    fn parse_imports(&self, source: &str) -> Vec<String> {
        IMPORT_REGEX
            .captures_iter(source)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    fn contains_import(&self, source: &str, module: &str) -> bool {
        if let Ok(re) = Regex::new(&format!(r"^\s*import\s+{}\b", module)) {
            re.is_match(source)
        } else {
            false
        }
    }
}

#[async_trait]
impl ImportRenameSupport for SwiftImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        source: &str,
        old_module: &str,
        new_module: &str,
    ) -> (String, usize) {
        if let Ok(re) = Regex::new(&format!(r"\bimport\s+{}\b", old_module)) {
            let mut changes = 0;
            let result = re.replace_all(source, |_caps: &regex::Captures| {
                changes += 1;
                format!("import {}", new_module)
            });
            (result.to_string(), changes)
        } else {
            (source.to_string(), 0)
        }
    }
}

#[async_trait]
impl ImportMoveSupport for SwiftImportSupport {
    fn rewrite_imports_for_move(
        &self,
        source: &str,
        _old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        let new_module = new_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let old_module = _old_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        self.rewrite_imports_for_rename(source, old_module, &new_module)
    }
}

#[async_trait]
impl ImportMutationSupport for SwiftImportSupport {
    fn add_import(&self, source: &str, module: &str) -> String {
        let import_statement = format!("import {}\n", module);
        format!("{}{}", import_statement, source)
    }

    fn remove_import(&self, source: &str, module: &str) -> String {
        if let Ok(re) = Regex::new(&format!(r"(?m)^\s*import\s+{}\s*\n?", module)) {
            re.replace_all(source, "").to_string()
        } else {
            source.to_string()
        }
    }
}

#[async_trait]
impl ImportAdvancedSupport for SwiftImportSupport {}