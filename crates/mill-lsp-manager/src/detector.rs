//! Language detection for LSP requirements

use crate::types::LspRegistry;
use std::path::Path;
use tracing::debug;

/// Detect languages used in a project
pub fn detect_languages(project_path: &Path) -> Vec<String> {
    let mut languages = Vec::new();

    // Check for manifest files
    if project_path.join("Cargo.toml").exists() {
        languages.push("rust".to_string());
    }

    if project_path.join("package.json").exists()
        || project_path.join("tsconfig.json").exists()
    {
        languages.push("typescript".to_string());
    }

    if project_path.join("requirements.txt").exists()
        || project_path.join("pyproject.toml").exists()
        || project_path.join("setup.py").exists()
    {
        languages.push("python".to_string());
    }

    // Scan for source files (limited depth to avoid performance issues)
    if let Ok(entries) = std::fs::read_dir(project_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let dir_name = entry.file_name();
                    if dir_name == "src" || dir_name == "lib" {
                        if let Some(langs) = scan_directory(&entry.path(), 2) {
                            for lang in langs {
                                if !languages.contains(&lang) {
                                    languages.push(lang);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    debug!("Detected languages: {:?}", languages);
    languages
}

/// Scan a directory for source files (recursive with depth limit)
fn scan_directory(path: &Path, max_depth: usize) -> Option<Vec<String>> {
    if max_depth == 0 {
        return None;
    }

    let mut languages = Vec::new();

    let entries = std::fs::read_dir(path).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if let Some(langs) = scan_directory(&path, max_depth - 1) {
                for lang in langs {
                    if !languages.contains(&lang) {
                        languages.push(lang);
                    }
                }
            }
        } else if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy();
            match ext.as_ref() {
                "rs" => {
                    if !languages.contains(&"rust".to_string()) {
                        languages.push("rust".to_string());
                    }
                }
                "ts" | "tsx" | "js" | "jsx" => {
                    if !languages.contains(&"typescript".to_string()) {
                        languages.push("typescript".to_string());
                    }
                }
                "py" => {
                    if !languages.contains(&"python".to_string()) {
                        languages.push("python".to_string());
                    }
                }
                _ => {}
            }
        }
    }

    Some(languages)
}

/// Get required LSP servers for detected languages
pub fn required_lsps(registry: &LspRegistry, project_path: &Path) -> Vec<String> {
    let languages = detect_languages(project_path);
    let mut lsps: Vec<String> = Vec::new();

    for language in languages {
        let found = registry.find_by_language(&language);
        for (lsp_name, _config) in found {
            if !lsps.contains(lsp_name) {
                lsps.push(lsp_name.clone());
            }
        }
    }

    debug!("Required LSPs: {:?}", lsps);
    lsps
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();

        let languages = detect_languages(dir.path());
        assert!(languages.contains(&"rust".to_string()));
    }

    #[test]
    fn test_detect_typescript_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let languages = detect_languages(dir.path());
        assert!(languages.contains(&"typescript".to_string()));
    }

    #[test]
    fn test_detect_python_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("requirements.txt"), "").unwrap();

        let languages = detect_languages(dir.path());
        assert!(languages.contains(&"python".to_string()));
    }

    #[test]
    fn test_detect_multiple_languages() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let languages = detect_languages(dir.path());
        assert!(languages.len() >= 2);
    }
}
