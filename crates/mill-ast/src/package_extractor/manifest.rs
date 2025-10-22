use mill_plugin_api::LanguagePlugin;
use std::path::PathBuf;
use tracing::debug;

pub(crate) async fn extract_dependencies(
    plugin: &dyn LanguagePlugin,
    located_files: &[PathBuf],
) -> Vec<String> {
    let mut all_dependencies = std::collections::HashSet::new();

    // Get ImportParser capability
    let import_parser = match plugin.import_parser() {
        Some(parser) => parser,
        None => {
            debug!("Plugin does not support import parsing");
            return Vec::new();
        }
    };

    for file_path in located_files {
        debug!(
            file_path = %file_path.display(),
            "Parsing dependencies from file"
        );

        // Read file and parse imports using ImportParser capability
        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                let deps = import_parser.parse_imports(&content);
                for dep in deps {
                    all_dependencies.insert(dep);
                }
            }
            Err(e) => {
                debug!(
                    error = %e,
                    file_path = %file_path.display(),
                    "Failed to read file"
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
    manifest_updater: &dyn mill_plugin_api::ManifestUpdater,
    package_name: &str,
    dependencies: &[String],
) -> String {
    manifest_updater.generate_manifest(package_name, dependencies)
}