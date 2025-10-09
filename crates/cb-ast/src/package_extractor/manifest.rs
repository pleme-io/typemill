use cb_lang_rust::RustPlugin;
use std::path::PathBuf;
use tracing::debug;

pub(crate) async fn extract_dependencies(
    rust_plugin: &RustPlugin,
    located_files: &[PathBuf],
) -> Vec<String> {
    let mut all_dependencies = std::collections::HashSet::new();

    for file_path in located_files {
        debug!(
            file_path = %file_path.display(),
            "Parsing dependencies from file"
        );

        match rust_plugin.parse_imports(file_path).await {
            Ok(deps) => {
                for dep in deps {
                    all_dependencies.insert(dep);
                }
            }
            Err(e) => {
                // Log error but continue with other files
                debug!(
                    error = %e,
                    file_path = %file_path.display(),
                    "Failed to parse imports from file"
                );
            }
        }
    }

    // Convert to sorted vector for consistent output
    let mut dependencies: Vec<String> = all_dependencies.into_iter().collect();
    dependencies.sort();
    dependencies
}

pub(crate) fn generate_manifest_for_plugin(
    rust_plugin: &RustPlugin,
    package_name: &str,
    dependencies: &[String],
) -> String {
    rust_plugin.generate_manifest(package_name, dependencies)
}