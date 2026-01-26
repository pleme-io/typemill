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
pub fn get_lsp_installer(plugin: &dyn LanguagePlugin) -> Option<&dyn LspInstaller> {
    plugin.lsp_installer()
}

/// Check if an LSP is installed
pub async fn check_lsp_installed(language: &str) -> Result<Option<PathBuf>, String> {
    let plugin = find_plugin_by_language(language)
        .ok_or_else(|| format!("No plugin found for language: {}", language))?;

    let installer = get_lsp_installer(&*plugin)
        .ok_or_else(|| format!("Plugin for {} does not support LSP installation", language))?;

    installer
        .check_installed()
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

    installer
        .ensure_installed(&cache_dir)
        .await
        .map_err(|e| format!("Installation failed: {}", e))
}

/// Get list of all languages with LSP installer support
pub fn list_supported_languages() -> Vec<(&'static str, String)> {
    iter_plugins()
        .filter_map(|desc| {
            // Create plugin instance to check for LSP installer
            let plugin = (desc.factory)();
            plugin
                .lsp_installer()
                .map(|installer| (desc.name, installer.lsp_name().to_string()))
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
        let handles_extension = desc
            .extensions
            .iter()
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

/// Detect TypeScript project root by finding tsconfig.json or package.json
/// Returns relative path from start_dir to the project root
///
/// # Arguments
/// * `start_dir` - Directory to start searching from (usually current directory)
///
/// # Returns
/// * `Some(PathBuf)` - Relative path to the directory containing tsconfig.json or package.json
/// * `None` - No TypeScript project found
pub fn detect_typescript_root(start_dir: &Path) -> Option<PathBuf> {
    use walkdir::WalkDir;

    debug!(path = ?start_dir, "Detecting TypeScript project root");

    for entry in WalkDir::new(start_dir)
        .max_depth(3) // Don't search too deep
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden directories, node_modules, and build artifacts
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
                && name != "node_modules"
                && name != "dist"
                && name != "build"
                && name != "target"
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let filename = entry.file_name().to_string_lossy();
            if filename == "tsconfig.json" || filename == "package.json" {
                if let Some(parent) = entry.path().parent() {
                    // Return relative path from start_dir
                    if let Ok(rel_path) = parent.strip_prefix(start_dir) {
                        info!(
                            path = ?rel_path,
                            file = %filename,
                            "Found TypeScript project"
                        );
                        return Some(rel_path.to_path_buf());
                    }
                }
            }
        }
    }

    debug!("No TypeScript project found");
    None
}

/// Detect all TypeScript project roots in workspace
/// Returns Vec of relative paths sorted by depth (shallowest first)
///
/// # Arguments
/// * `start_dir` - Directory to start searching from
///
/// # Returns
/// * `Vec<PathBuf>` - List of relative paths to directories containing TS projects
#[allow(dead_code)]
pub fn detect_all_typescript_roots(start_dir: &Path) -> Vec<PathBuf> {
    use std::collections::HashSet;
    use walkdir::WalkDir;

    debug!(path = ?start_dir, "Detecting all TypeScript project roots");

    let mut roots = HashSet::new();

    for entry in WalkDir::new(start_dir)
        .max_depth(5) // Search deeper for monorepos
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
                && name != "node_modules"
                && name != "dist"
                && name != "build"
                && name != "target"
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let filename = entry.file_name().to_string_lossy();
            if filename == "tsconfig.json" || filename == "package.json" {
                if let Some(parent) = entry.path().parent() {
                    if let Ok(rel_path) = parent.strip_prefix(start_dir) {
                        roots.insert(rel_path.to_path_buf());
                    }
                }
            }
        }
    }

    let mut roots_vec: Vec<PathBuf> = roots.into_iter().collect();
    // Sort by depth (shallowest first)
    roots_vec.sort_by_key(|p| p.components().count());

    info!(count = roots_vec.len(), roots = ?roots_vec, "Found TypeScript projects");
    roots_vec
}

/// Update config after LSP installation
/// Prompts user for configuration choices if interactive
///
/// # Arguments
/// * `language` - Language name (e.g., "typescript", "rust")
/// * `installed_path` - Full path to the installed LSP binary
/// * `interactive` - Whether to prompt the user for choices
///
/// # Returns
/// * `Ok(())` - Config updated successfully
/// * `Err(String)` - Error message
pub async fn update_config_after_install(
    language: &str,
    installed_path: &Path,
    interactive: bool,
) -> Result<(), String> {
    use super::user_input;
    use mill_config::config::AppConfig;

    info!(language, path = ?installed_path, "Updating config after LSP install");

    // Load current config
    let mut config = AppConfig::load().map_err(|e| format!("Failed to load config: {}", e))?;

    // Get extension for this language
    let extension = match language {
        "rust" | "rs" => "rs",
        "typescript" | "ts" => "ts",
        "javascript" | "js" => "js",
        "python" | "py" => "py",
        _ => language,
    };

    // Prompt for path type if interactive
    let (command, show_path_warning) = if interactive {
        println!("\n🔧 Update configuration:");
        println!("  [1] Use relative path (portable - requires PATH)");
        println!("  [2] Use absolute path (works immediately)");
        println!("  [3] Don't update config");

        let choice = user_input::read_choice(
            "\nChoice:",
            &["Relative path", "Absolute path", "Skip"],
            0, // Default to relative
        )
        .map_err(|e| format!("Failed to read choice: {}", e))?;

        match choice {
            0 => {
                // Relative path
                let bin_name = installed_path
                    .file_name()
                    .ok_or("Invalid path")?
                    .to_string_lossy()
                    .to_string();
                (vec![bin_name], true)
            }
            1 => {
                // Absolute path
                (vec![installed_path.to_string_lossy().to_string()], false)
            }
            2 => {
                // Skip
                println!("⏭️  Skipped config update");
                return Ok(());
            }
            _ => unreachable!(),
        }
    } else {
        // Non-interactive: use relative path by default
        let bin_name = installed_path
            .file_name()
            .ok_or("Invalid path")?
            .to_string_lossy()
            .to_string();
        (vec![bin_name], false) // Don't show warning in non-interactive mode
    };

    // Detect rootDir for TypeScript
    let root_dir = if extension == "ts" {
        detect_typescript_root(std::path::Path::new("."))
    } else {
        None
    };

    // Add "--stdio" for typescript-language-server
    let mut full_command = command;
    if extension == "ts" {
        full_command.push("--stdio".to_string());
    }

    // Update config
    config
        .update_lsp_command(language, full_command.clone(), root_dir.clone())
        .map_err(|e| format!("Failed to update config: {}", e))?;

    // Save config
    config
        .save(std::path::Path::new(".typemill/config.json"))
        .map_err(|e| format!("Failed to save config: {}", e))?;

    println!("✅ Updated .typemill/config.json");

    // Show PATH warning if needed
    if show_path_warning {
        if let Some(parent) = installed_path.parent() {
            println!("\n⚠️  Add to PATH for portability:");
            println!("   export PATH=\"{}:$PATH\"", parent.display());
        }
    }

    // Show rootDir if set
    if let Some(ref dir) = root_dir {
        println!("📁 Set rootDir: {}", dir.display());
    }

    Ok(())
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

    #[test]
    fn test_detect_typescript_root() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a fake TypeScript project
        let web_dir = root.join("web");
        fs::create_dir_all(&web_dir).unwrap();
        fs::write(web_dir.join("tsconfig.json"), "{}").unwrap();

        // Should find web/
        let result = detect_typescript_root(root);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("web"));
    }

    #[test]
    fn test_detect_typescript_root_not_found() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Empty directory
        let result = detect_typescript_root(root);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_all_typescript_roots() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create multiple TS projects
        let web_dir = root.join("web");
        fs::create_dir_all(&web_dir).unwrap();
        fs::write(web_dir.join("tsconfig.json"), "{}").unwrap();

        let api_dir = root.join("packages").join("api");
        fs::create_dir_all(&api_dir).unwrap();
        fs::write(api_dir.join("package.json"), r#"{"name":"api"}"#).unwrap();

        // Should find both
        let results = detect_all_typescript_roots(root);
        assert_eq!(results.len(), 2);

        // Should be sorted by depth (web before packages/api)
        assert!(results[0].components().count() <= results[1].components().count());
    }
}
