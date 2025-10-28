//! Plugin-based LSP installation helpers for CLI

use mill_lang_common::lsp::get_cache_dir;
use mill_plugin_api::{iter_plugins, LanguagePlugin, LspInstaller};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Find a language plugin by language name or file extension
pub fn find_plugin_by_language(language: &str) -> Option<Box<dyn LanguagePlugin>> {
    let lang_lower = language.to_lowercase();

    // Map common language names to extensions
    let extension = match lang_lower.as_str() {
        "rust" | "rs" => "rs",
        "typescript" | "ts" => "ts",
        "javascript" | "js" => "js",
        "python" | "py" => "py",
        _ => &lang_lower, // Try as extension directly
    };

    debug!(language, extension, "Looking for plugin");

    // Find descriptor and create plugin instance
    iter_plugins()
        .find(|desc| desc.extensions.contains(&extension))
        .map(|desc| (desc.factory)())
}

/// Get LSP installer from a plugin
pub fn get_lsp_installer<'a>(plugin: &'a dyn LanguagePlugin) -> Option<&'a dyn LspInstaller> {
    plugin.lsp_installer()
}

/// Check if an LSP is installed
pub async fn check_lsp_installed(language: &str) -> Result<Option<PathBuf>, String> {
    let plugin = find_plugin_by_language(language)
        .ok_or_else(|| format!("No plugin found for language: {}", language))?;

    let installer = get_lsp_installer(&*plugin)
        .ok_or_else(|| format!("Plugin for {} does not support LSP installation", language))?;

    installer.check_installed()
        .map_err(|e| format!("Failed to check LSP status: {}", e))
}

/// Install an LSP for a language
pub async fn install_lsp(language: &str) -> Result<PathBuf, String> {
    let plugin = find_plugin_by_language(language)
        .ok_or_else(|| format!("No plugin found for language: {}", language))?;

    let installer = get_lsp_installer(&*plugin)
        .ok_or_else(|| format!("Plugin for {} does not support LSP installation", language))?;

    let cache_dir = get_cache_dir();
    info!(language, lsp_name = installer.lsp_name(), "Installing LSP");

    installer.ensure_installed(&cache_dir)
        .await
        .map_err(|e| format!("Installation failed: {}", e))
}

/// Get list of all languages with LSP installer support
pub fn list_supported_languages() -> Vec<(&'static str, String)> {
    iter_plugins()
        .filter_map(|desc| {
            // Create plugin instance to check for LSP installer
            let plugin = (desc.factory)();
            if let Some(installer) = plugin.lsp_installer() {
                Some((desc.name, installer.lsp_name().to_string()))
            } else {
                None
            }
        })
        .collect()
}

/// Detect needed LSPs by scanning project files
pub fn detect_needed_lsps(project_root: &Path) -> Result<Vec<String>, String> {
    use std::collections::HashSet;
    use walkdir::WalkDir;

    debug!(path = ?project_root, "Detecting project languages");

    let mut detected_extensions: HashSet<String> = HashSet::new();

    // Scan project for file extensions
    for entry in WalkDir::new(project_root)
        .max_depth(5) // Scan deep enough for nested project structures (e.g., workspace/crates/*/src/*.rs)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Allow root directory
            if e.depth() == 0 {
                return true;
            }

            // Skip hidden directories and common build/dependency directories
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
                && name != "node_modules"
                && name != "target"
                && name != "dist"
                && name != "build"
                && name != "__pycache__"
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if let Some(ext_str) = ext.to_str() {
                    detected_extensions.insert(ext_str.to_lowercase());
                }
            }
        }
    }

    debug!(extensions = ?detected_extensions, "Detected file extensions");

    // Find plugins that handle these extensions and have LSP installers
    let mut needed_lsps = Vec::new();

    for desc in iter_plugins() {
        // Check if descriptor handles any detected extension
        let handles_extension = desc.extensions.iter()
            .any(|ext| detected_extensions.contains(*ext));

        if !handles_extension {
            continue;
        }

        // Create plugin instance to check for LSP installer
        let plugin = (desc.factory)();
        if plugin.lsp_installer().is_some() {
            let lsp_name = desc.name;
            if !needed_lsps.contains(&lsp_name.to_string()) {
                needed_lsps.push(lsp_name.to_string());
            }
        }
    }

    info!(lsps = ?needed_lsps, "Detected needed LSPs");
    Ok(needed_lsps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_plugin_by_language() {
        // Should find Rust plugin
        let plugin = find_plugin_by_language("rust");
        assert!(plugin.is_some());
        let plugin = plugin.unwrap();
        assert_eq!(plugin.metadata().name, "rust");

        // Should find TypeScript plugin
        let plugin = find_plugin_by_language("typescript");
        assert!(plugin.is_some());
        let plugin = plugin.unwrap();
        assert_eq!(plugin.metadata().name, "typescript");

        // Should find Python plugin
        let plugin = find_plugin_by_language("python");
        assert!(plugin.is_some());
        let plugin = plugin.unwrap();
        assert_eq!(plugin.metadata().name, "python");
    }

    #[test]
    fn test_list_supported_languages() {
        let supported = list_supported_languages();
        assert!(!supported.is_empty());

        // Should include at least Rust, TypeScript, Python
        let names: Vec<&str> = supported.iter().map(|(name, _lsp)| *name).collect();
        assert!(names.contains(&"rust"));
        assert!(names.contains(&"typescript"));
        assert!(names.contains(&"python"));
    }
}
