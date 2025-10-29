use mill_foundation::protocol::DependencyUpdate;
use mill_plugin_api::{
    import_support::{
        ImportAdvancedSupport, ImportMoveSupport, ImportMutationSupport, ImportParser,
        ImportRenameSupport,
    },
    PluginResult,
};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct CImportSupport;

use regex::Regex;
impl ImportParser for CImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        let re = Regex::new(r#"#include\s*[<"](.+)[>"]"#).unwrap();
        re.captures_iter(content)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        let imports = self.parse_imports(content);
        imports.contains(&module.to_string())
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