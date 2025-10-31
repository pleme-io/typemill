//! Placeholder for C# import support.
use async_trait::async_trait;
use mill_plugin_api::{
    ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
    ImportRenameSupport,
};
use std::path::Path;

#[derive(Default)]
pub struct CsharpImportSupport;

impl ImportParser for CsharpImportSupport {
    fn parse_imports(&self, _source: &str) -> Vec<String> {
        vec![]
    }

    fn contains_import(&self, _source: &str, _import: &str) -> bool {
        false
    }
}

#[async_trait]
impl ImportRenameSupport for CsharpImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        source: &str,
        _old_name: &str,
        _new_name: &str,
    ) -> (String, usize) {
        (source.to_string(), 0)
    }
}

#[async_trait]
impl ImportMoveSupport for CsharpImportSupport {
    fn rewrite_imports_for_move(
        &self,
        source: &str,
        _old_path: &Path,
        _new_path: &Path,
    ) -> (String, usize) {
        (source.to_string(), 0)
    }
}

#[async_trait]
impl ImportMutationSupport for CsharpImportSupport {
    fn add_import(&self, source: &str, _import_to_add: &str) -> String {
        source.to_string()
    }

    fn remove_import(&self, source: &str, _import_to_remove: &str) -> String {
        source.to_string()
    }
}

#[async_trait]
impl ImportAdvancedSupport for CsharpImportSupport {}