use futures::stream::StreamExt;
use mill_plugin_api::LanguagePlugin;
use std::path::PathBuf;
use tracing::debug;

pub(crate) async fn extract_dependencies(
    plugin: &dyn LanguagePlugin,
    located_files: &[PathBuf],
) -> Vec<String> {
    // Get ImportParser capability
    let import_parser = match plugin.import_parser() {
        Some(parser) => parser,
        None => {
            debug!("Plugin does not support import parsing");
            return Vec::new();
        }
    };

    let all_dependencies = futures::stream::iter(located_files)
        .map(|file_path| {
            let file_path = file_path.clone();
            async move {
                debug!(
                    file_path = %file_path.display(),
                    "Parsing dependencies from file"
                );

                // Read file and parse imports using ImportParser capability
                match tokio::fs::read_to_string(&file_path).await {
                    Ok(content) => {
                        let deps = import_parser.parse_imports(&content);
                        Some(deps)
                    }
                    Err(e) => {
                        debug!(
                            error = %e,
                            file_path = %file_path.display(),
                            "Failed to read file"
                        );
                        None
                    }
                }
            }
        })
        .buffer_unordered(50) // Process up to 50 files concurrently
        .fold(std::collections::HashSet::new(), |mut acc, deps_opt| async move {
            if let Some(deps) = deps_opt {
                for dep in deps {
                    acc.insert(dep);
                }
            }
            acc
        })
        .await;

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
