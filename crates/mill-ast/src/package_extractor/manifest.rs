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

    // Read all files concurrently first, then parse synchronously
    // This avoids holding references to import_parser across await points
    let file_contents: Vec<(PathBuf, Option<String>)> = {
        let read_futures: Vec<_> = located_files
            .iter()
            .map(|file_path| {
                let file_path = file_path.clone();
                async move {
                    debug!(
                        file_path = %file_path.display(),
                        "Reading file for dependency extraction"
                    );
                    let content = tokio::fs::read_to_string(&file_path).await.ok();
                    (file_path, content)
                }
            })
            .collect();

        futures::future::join_all(read_futures).await
    };

    // Now parse all files synchronously (no await points, no Send issues)
    let mut all_dependencies = std::collections::HashSet::new();
    for (file_path, content_opt) in file_contents {
        if let Some(content) = content_opt {
            debug!(
                file_path = %file_path.display(),
                "Parsing dependencies from file"
            );
            let deps = import_parser.parse_imports(&content);
            for dep in deps {
                all_dependencies.insert(dep);
            }
        } else {
            debug!(
                file_path = %file_path.display(),
                "Failed to read file, skipping"
            );
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
