//! Import path resolution and updating functionality

pub mod edit_builder;
pub mod file_scanner;
pub mod path_resolver;
pub mod reference_finder;

#[cfg(test)]
mod tests;

use crate::error::AstResult;
use std::path::Path;

pub use file_scanner::find_project_files;
pub use path_resolver::ImportPathResolver;

/// Update import paths in all affected files after a file/directory rename
///
/// Returns an EditPlan that can be applied via FileService.apply_edit_plan()
pub async fn update_imports_for_rename(
    old_path: &Path,
    new_path: &Path,
    project_root: &Path,
    plugins: &[std::sync::Arc<dyn cb_plugin_api::LanguagePlugin>],
    rename_info: Option<&serde_json::Value>,
    dry_run: bool,
    scan_scope: Option<cb_plugin_api::ScanScope>,
) -> AstResult<cb_protocol::EditPlan> {
    edit_builder::build_import_update_plan(
        old_path,
        new_path,
        project_root,
        plugins,
        rename_info,
        dry_run,
        scan_scope,
    )
    .await
}
