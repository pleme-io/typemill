//! Import rewrite support for Svelte files.

use crate::script_blocks::find_script_blocks;
use mill_lang_common::io::relative_path;
use mill_lang_typescript::path_alias_resolver::TypeScriptPathAliasResolver;
use mill_plugin_api::ImportParser;
use mill_plugin_api::PathAliasResolver;
use regex::Regex;
use std::path::{Component, Path, PathBuf};

pub struct SvelteImportSupport {
    es_import_re: Regex,
    require_re: Regex,
    dynamic_import_re: Regex,
}

impl SvelteImportSupport {
    pub fn new() -> Self {
        Self {
            es_import_re: Regex::new(r#"(?m)import\s+[^\n]*from\s+['"]([^'"]+)['"]"#)
                .expect("es import regex should be valid"),
            require_re: Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
                .expect("require regex should be valid"),
            dynamic_import_re: Regex::new(r#"import\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
                .expect("dynamic import regex should be valid"),
        }
    }

    pub fn collect_imports(&self, content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        let blocks = find_script_blocks(content);
        if blocks.is_empty() {
            self.collect_from_source(content, &mut imports);
        } else {
            for block in blocks {
                let block_content = &content[block.content_start..block.content_end];
                self.collect_from_source(block_content, &mut imports);
            }
        }

        imports
    }

    fn collect_from_source(&self, content: &str, imports: &mut Vec<String>) {
        for caps in self.es_import_re.captures_iter(content) {
            if let Some(module) = caps.get(1) {
                imports.push(module.as_str().to_string());
            }
        }
        for caps in self.require_re.captures_iter(content) {
            if let Some(module) = caps.get(1) {
                imports.push(module.as_str().to_string());
            }
        }
        for caps in self.dynamic_import_re.captures_iter(content) {
            if let Some(module) = caps.get(1) {
                imports.push(module.as_str().to_string());
            }
        }
    }
}

impl Default for SvelteImportSupport {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportParser for SvelteImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        self.collect_imports(content)
    }

    fn contains_import(&self, content: &str, module: &str) -> bool {
        self.collect_imports(content).iter().any(|m| m == module)
    }
}

pub fn rewrite_svelte_imports_for_move(
    content: &str,
    old_path: &Path,
    new_path: &Path,
    current_file: &Path,
    project_root: &Path,
    resolver: &TypeScriptPathAliasResolver,
) -> (String, usize) {
    let old_abs = if old_path.is_absolute() {
        old_path.to_path_buf()
    } else {
        project_root.join(old_path)
    };
    let new_abs = if new_path.is_absolute() {
        new_path.to_path_buf()
    } else {
        project_root.join(new_path)
    };

    let blocks = find_script_blocks(content);
    if blocks.is_empty() {
        return (content.to_string(), 0);
    }

    let mut updated = String::with_capacity(content.len());
    let mut last_index = 0;
    let mut changes = 0;

    for block in blocks {
        updated.push_str(&content[last_index..block.content_start]);

        let block_content = &content[block.content_start..block.content_end];
        let (new_block, block_changes) = rewrite_script_block_imports(
            block_content,
            &old_abs,
            &new_abs,
            current_file,
            project_root,
            resolver,
        );

        changes += block_changes;
        updated.push_str(&new_block);
        last_index = block.content_end;
    }

    updated.push_str(&content[last_index..]);

    (updated, changes)
}

fn rewrite_script_block_imports(
    content: &str,
    old_path: &Path,
    new_path: &Path,
    current_file: &Path,
    project_root: &Path,
    resolver: &TypeScriptPathAliasResolver,
) -> (String, usize) {
    let support = SvelteImportSupport::new();
    let mut new_content = content.to_string();
    let mut changes = 0;

    let is_directory_move =
        old_path.is_dir() || (old_path.extension().is_none() && !old_path.is_file());
    let normalized_old = normalize_path(old_path);

    for specifier in support.collect_imports(content) {
        if !resolver.is_potential_alias(&specifier) {
            continue;
        }

        let resolved = match resolver.resolve_alias(&specifier, current_file, project_root) {
            Some(path) => normalize_path(Path::new(&path)),
            None => continue,
        };

        let is_affected = if is_directory_move {
            resolved.starts_with(&normalized_old)
        } else {
            resolved == normalized_old
                || resolved.with_extension("") == normalized_old.with_extension("")
        };

        if !is_affected {
            continue;
        }

        let new_resolved_path = if is_directory_move {
            if let Ok(relative) = resolved.strip_prefix(&normalized_old) {
                normalize_path(&new_path.join(relative))
            } else {
                continue;
            }
        } else {
            new_path.to_path_buf()
        };

        let replacement = resolver
            .path_to_alias(&new_resolved_path, current_file, project_root)
            .unwrap_or_else(|| {
                let mut rel = relative_path(current_file, &new_resolved_path)
                    .to_string_lossy()
                    .replace('\\', "/");
                if !rel.starts_with('.') {
                    rel = format!("./{}", rel);
                }
                if let Some(ext) = new_resolved_path.extension().and_then(|e| e.to_str()) {
                    if rel.ends_with(&format!(".{}", ext)) {
                        let trim = rel.len() - ext.len() - 1;
                        rel.truncate(trim);
                    }
                }
                rel
            });
        let replacement = if resolver.is_potential_alias(&specifier) && replacement.starts_with('.')
        {
            alias_from_prefix(
                &specifier,
                &new_resolved_path,
                current_file,
                project_root,
                resolver,
            )
            .or_else(|| derive_alias_from_specifier(&specifier, &resolved, &new_resolved_path))
            .unwrap_or(replacement)
        } else {
            replacement
        };
        let replacement = if specifier.starts_with("$lib") {
            alias_from_svelte_lib_path(&new_resolved_path, current_file, project_root)
                .unwrap_or(replacement)
        } else {
            replacement
        };

        for quote_char in &['\'', '"'] {
            let old_str = format!("from {}{}{}", quote_char, specifier, quote_char);
            let new_str = format!("from {}{}{}", quote_char, replacement, quote_char);
            if new_content.contains(&old_str) {
                new_content = new_content.replace(&old_str, &new_str);
                changes += 1;
            }

            let old_str = format!("require({}{}{})", quote_char, specifier, quote_char);
            let new_str = format!("require({}{}{})", quote_char, replacement, quote_char);
            if new_content.contains(&old_str) {
                new_content = new_content.replace(&old_str, &new_str);
                changes += 1;
            }

            let old_str = format!("import({}{}{})", quote_char, specifier, quote_char);
            let new_str = format!("import({}{}{})", quote_char, replacement, quote_char);
            if new_content.contains(&old_str) {
                new_content = new_content.replace(&old_str, &new_str);
                changes += 1;
            }
        }
    }

    (new_content, changes)
}

fn derive_alias_from_specifier(
    specifier: &str,
    resolved_old: &Path,
    resolved_new: &Path,
) -> Option<String> {
    let alias_prefix = specifier.split('/').next().unwrap_or("");
    if !(alias_prefix.starts_with('$')
        || alias_prefix.starts_with('@')
        || alias_prefix.starts_with('~'))
    {
        return None;
    }

    let suffix = specifier
        .strip_prefix(alias_prefix)
        .unwrap_or("")
        .trim_start_matches('/');

    let mut normalized_old = resolved_old.to_path_buf();
    if let Some(ext) = normalized_old.extension().and_then(|e| e.to_str()) {
        if ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
            normalized_old = normalized_old.with_extension("");
        }
    }

    let alias_root = if suffix.is_empty() {
        normalized_old
    } else {
        strip_path_suffix(&normalized_old, Path::new(suffix))?
    };

    let mut new_relative = resolved_new.strip_prefix(&alias_root).ok()?.to_path_buf();
    if let Some(ext) = new_relative.extension().and_then(|e| e.to_str()) {
        if ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
            new_relative = new_relative.with_extension("");
        }
    }

    let relative_str = new_relative.to_string_lossy().replace('\\', "/");
    if relative_str.is_empty() {
        Some(alias_prefix.to_string())
    } else {
        Some(format!(
            "{}/{}",
            alias_prefix.trim_end_matches('/'),
            relative_str
        ))
    }
}

fn alias_from_prefix(
    specifier: &str,
    resolved_new: &Path,
    current_file: &Path,
    project_root: &Path,
    resolver: &TypeScriptPathAliasResolver,
) -> Option<String> {
    let alias_prefix = specifier.split('/').next().unwrap_or("");
    if alias_prefix.is_empty() {
        return None;
    }

    let alias_root = if let Some(root) = resolver
        .resolve_alias(alias_prefix, current_file, project_root)
        .map(PathBuf::from)
    {
        normalize_path(&root)
    } else if alias_prefix == "$lib" {
        let svelte_root = find_nearest_svelte_root(current_file, project_root)?;
        normalize_path(&svelte_root.join("src").join("lib"))
    } else {
        return None;
    };

    let resolved_new = if resolved_new.is_absolute() {
        resolved_new.to_path_buf()
    } else {
        project_root.join(resolved_new)
    };
    let mut new_relative = resolved_new.strip_prefix(&alias_root).ok()?.to_path_buf();
    if let Some(ext) = new_relative.extension().and_then(|e| e.to_str()) {
        if ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
            new_relative = new_relative.with_extension("");
        }
    }

    let relative_str = new_relative.to_string_lossy().replace('\\', "/");
    if relative_str.is_empty() {
        Some(alias_prefix.to_string())
    } else {
        Some(format!(
            "{}/{}",
            alias_prefix.trim_end_matches('/'),
            relative_str
        ))
    }
}

fn find_nearest_svelte_root(current_file: &Path, project_root: &Path) -> Option<PathBuf> {
    let absolute_file = if current_file.is_absolute() {
        current_file.to_path_buf()
    } else {
        project_root.join(current_file)
    };
    let mut current = absolute_file.parent()?;
    loop {
        let js = current.join("svelte.config.js");
        let ts = current.join("svelte.config.ts");
        let mjs = current.join("svelte.config.mjs");
        let cjs = current.join("svelte.config.cjs");
        if js.exists() || ts.exists() || mjs.exists() || cjs.exists() {
            return Some(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    Some(project_root.to_path_buf())
}

fn alias_from_svelte_lib_path(
    resolved_new: &Path,
    current_file: &Path,
    project_root: &Path,
) -> Option<String> {
    let svelte_root = find_nearest_svelte_root(current_file, project_root)?;
    let lib_root = svelte_root.join("src").join("lib");
    let resolved_new = if resolved_new.is_absolute() {
        resolved_new.to_path_buf()
    } else {
        project_root.join(resolved_new)
    };
    let mut relative = resolved_new.strip_prefix(&lib_root).ok()?.to_path_buf();
    if let Some(ext) = relative.extension().and_then(|e| e.to_str()) {
        if ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
            relative = relative.with_extension("");
        }
    }
    let relative_str = relative.to_string_lossy().replace('\\', "/");
    if relative_str.is_empty() {
        Some("$lib".to_string())
    } else {
        Some(format!("$lib/{}", relative_str))
    }
}

fn strip_path_suffix(path: &Path, suffix: &Path) -> Option<PathBuf> {
    let path_components: Vec<_> = path.components().collect();
    let suffix_components: Vec<_> = suffix.components().collect();
    if suffix_components.len() > path_components.len() {
        return None;
    }

    let start = path_components.len() - suffix_components.len();
    if path_components[start..] != suffix_components[..] {
        return None;
    }

    let mut result = PathBuf::new();
    for component in &path_components[..start] {
        result.push(component.as_os_str());
    }
    Some(result)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(component.as_os_str()),
        }
    }
    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}
