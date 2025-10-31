//! go.mod manifest file handling
//!
//! This module provides functionality for parsing and manipulating go.mod
//! manifest files, extracting dependency information, and updating dependencies.

use mill_lang_common::read_manifest;
use mill_plugin_api::{Dependency, DependencySource, ManifestData, PluginError, PluginResult};
use serde::Serialize;
use std::path::Path;
use tracing::debug;

/// Represents a parsed go.mod file structure
#[derive(Debug, Clone, Serialize)]
struct GoMod {
    module: String,
    go_version: Option<String>,
    requires: Vec<GoRequire>,
    replaces: Vec<GoReplace>,
    excludes: Vec<GoExclude>,
}

/// A require directive
#[derive(Debug, Clone, Serialize)]
struct GoRequire {
    path: String,
    version: String,
    indirect: bool,
}

/// A replace directive
#[derive(Debug, Clone, Serialize)]
struct GoReplace {
    old_path: String,
    old_version: Option<String>,
    new_path: String,
    new_version: Option<String>,
}

/// An exclude directive
#[derive(Debug, Clone, Serialize)]
struct GoExclude {
    path: String,
    version: String,
}

/// Parse a go.mod file and extract manifest information
pub fn parse_go_mod(content: &str) -> PluginResult<ManifestData> {
    debug!("Parsing go.mod content");
    let go_mod = parse_go_mod_internal(content)?;
    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    for require in go_mod.requires.iter() {
        let dep = Dependency {
            name: require.path.clone(),
            source: DependencySource::Version(require.version.clone()),
        };
        if require.indirect {
            dev_dependencies.push(dep);
        } else {
            dependencies.push(dep);
        }
    }
    for replace in go_mod.replaces.iter() {
        apply_replacement(&mut dependencies, replace);
        apply_replacement(&mut dev_dependencies, replace);
    }
    debug!(
        module = % go_mod.module, dependencies_count = dependencies.len(),
        dev_dependencies_count = dev_dependencies.len(), "Parsed go.mod successfully"
    );
    Ok(ManifestData {
        name: go_mod.module.clone(),
        version: go_mod.go_version.clone().unwrap_or_default(),
        dependencies,
        dev_dependencies,
        raw_data: serde_json::to_value(&go_mod)?,
    })
}

/// Internal parser for go.mod files
fn parse_go_mod_internal(content: &str) -> PluginResult<GoMod> {
    let mut module = String::new();
    let mut go_version = None;
    let mut requires = Vec::new();
    let mut replaces = Vec::new();
    let mut excludes = Vec::new();
    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if line.starts_with("module ") {
            module = parse_module_directive(line)?;
        } else if line.starts_with("go ") {
            go_version = Some(parse_go_directive(line)?);
        } else if line.starts_with("require ") {
            if line.contains('(') {
                requires.extend(parse_require_block(&mut lines)?);
            } else {
                requires.push(parse_require_line(line)?);
            }
        } else if line.starts_with("replace ") {
            if line.contains('(') {
                replaces.extend(parse_replace_block(&mut lines)?);
            } else {
                replaces.push(parse_replace_line(line)?);
            }
        } else if line.starts_with("exclude ") {
            if line.contains('(') {
                excludes.extend(parse_exclude_block(&mut lines)?);
            } else {
                excludes.push(parse_exclude_line(line)?);
            }
        }
    }
    if module.is_empty() {
        return Err(PluginError::manifest("Missing module directive in go.mod"));
    }
    Ok(GoMod {
        module,
        go_version,
        requires,
        replaces,
        excludes,
    })
}

/// Parse module directive: "module example.com/mymodule"
fn parse_module_directive(line: &str) -> PluginResult<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(PluginError::manifest("Invalid module directive"));
    }
    Ok(parts[1].to_string())
}

/// Parse go directive: "go 1.21"
fn parse_go_directive(line: &str) -> PluginResult<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(PluginError::manifest("Invalid go directive"));
    }
    Ok(parts[1].to_string())
}

/// Parse a single require line: "require example.com/pkg v1.2.3"
fn parse_require_line(line: &str) -> PluginResult<GoRequire> {
    let line = line.trim_start_matches("require").trim();
    parse_require_entry(line)
}

/// Parse a require entry (without the "require" keyword)
fn parse_require_entry(entry: &str) -> PluginResult<GoRequire> {
    let parts: Vec<&str> = entry.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(PluginError::manifest(format!(
            "Invalid require entry: {}",
            entry
        )));
    }
    let path = parts[0].to_string();
    let version = parts[1].to_string();
    let indirect = parts.get(2).is_some_and(|&s| s == "//indirect");
    Ok(GoRequire {
        path,
        version,
        indirect,
    })
}

/// Parse a multi-line require block
fn parse_require_block<'a, I>(lines: &mut std::iter::Peekable<I>) -> PluginResult<Vec<GoRequire>>
where
    I: Iterator<Item = &'a str>,
{
    let mut requires = Vec::new();
    while let Some(&line) = lines.peek() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            lines.next();
            continue;
        }
        if line == ")" {
            lines.next();
            break;
        }
        requires.push(parse_require_entry(line)?);
        lines.next();
    }
    Ok(requires)
}

/// Parse a single replace line: "replace old => new"
fn parse_replace_line(line: &str) -> PluginResult<GoReplace> {
    let line = line.trim_start_matches("replace").trim();
    parse_replace_entry(line)
}

/// Parse a replace entry (without the "replace" keyword)
fn parse_replace_entry(entry: &str) -> PluginResult<GoReplace> {
    let parts: Vec<&str> = entry.split("=>").collect();
    if parts.len() != 2 {
        return Err(PluginError::manifest(format!(
            "Invalid replace entry: {}",
            entry
        )));
    }
    let old_parts: Vec<&str> = parts[0].split_whitespace().collect();
    let new_parts: Vec<&str> = parts[1].split_whitespace().collect();
    if old_parts.is_empty() || new_parts.is_empty() {
        return Err(PluginError::manifest(format!(
            "Invalid replace entry: {}",
            entry
        )));
    }
    Ok(GoReplace {
        old_path: old_parts[0].to_string(),
        old_version: old_parts.get(1).map(|s| s.to_string()),
        new_path: new_parts[0].to_string(),
        new_version: new_parts.get(1).map(|s| s.to_string()),
    })
}

/// Parse a multi-line replace block
fn parse_replace_block<'a, I>(lines: &mut std::iter::Peekable<I>) -> PluginResult<Vec<GoReplace>>
where
    I: Iterator<Item = &'a str>,
{
    let mut replaces = Vec::new();
    while let Some(&line) = lines.peek() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            lines.next();
            continue;
        }
        if line == ")" {
            lines.next();
            break;
        }
        replaces.push(parse_replace_entry(line)?);
        lines.next();
    }
    Ok(replaces)
}

/// Parse a single exclude line: "exclude example.com/pkg v1.2.3"
fn parse_exclude_line(line: &str) -> PluginResult<GoExclude> {
    let line = line.trim_start_matches("exclude").trim();
    parse_exclude_entry(line)
}

/// Parse an exclude entry (without the "exclude" keyword)
fn parse_exclude_entry(entry: &str) -> PluginResult<GoExclude> {
    let parts: Vec<&str> = entry.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(PluginError::manifest(format!(
            "Invalid exclude entry: {}",
            entry
        )));
    }
    Ok(GoExclude {
        path: parts[0].to_string(),
        version: parts[1].to_string(),
    })
}

/// Parse a multi-line exclude block
fn parse_exclude_block<'a, I>(lines: &mut std::iter::Peekable<I>) -> PluginResult<Vec<GoExclude>>
where
    I: Iterator<Item = &'a str>,
{
    let mut excludes = Vec::new();
    while let Some(&line) = lines.peek() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            lines.next();
            continue;
        }
        if line == ")" {
            lines.next();
            break;
        }
        excludes.push(parse_exclude_entry(line)?);
        lines.next();
    }
    Ok(excludes)
}

/// Apply a replacement to a dependency list
fn apply_replacement(dependencies: &mut [Dependency], replace: &GoReplace) {
    for dep in dependencies.iter_mut() {
        if dep.name == replace.old_path {
            if let Some(ref old_version) = replace.old_version {
                if let DependencySource::Version(ref version) = dep.source {
                    if version != old_version {
                        continue;
                    }
                }
            }
            if replace.new_path.starts_with('.') || replace.new_path.starts_with('/') {
                dep.source = DependencySource::Path(replace.new_path.clone());
            } else if let Some(ref new_version) = replace.new_version {
                dep.name = replace.new_path.clone();
                dep.source = DependencySource::Version(new_version.clone());
            } else {
                dep.name = replace.new_path.clone();
            }
        }
    }
}

/// Generate a new go.mod file
pub fn generate_manifest(module_name: &str, go_version: &str) -> String {
    format!("module {}\n\ngo {}\n", module_name, go_version)
}

/// Load and parse a go.mod file from a path
pub async fn load_go_mod(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    parse_go_mod(&content)
}

/// Update a dependency in a go.mod file
pub fn update_dependency(
    content: &str,
    old_name: &str,
    new_name: &str,
    new_version: Option<&str>,
) -> PluginResult<String> {
    let pattern = format!(r"({})\s+v?[\d\.]+", regex::escape(old_name));
    let re = regex::Regex::new(&pattern).map_err(|e| PluginError::internal(e.to_string()))?;

    let replacement = if let Some(version) = new_version {
        format!("{} {}", new_name, version)
    } else {
        new_name.to_string()
    };

    let result = re.replace(content, &replacement);
    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_go_mod() {
        let content = r#"
module example.com/mymodule

go 1.21

require (
    example.com/dependency v1.2.3
    another.module/pkg v0.1.0
)
"#;
        let manifest = parse_go_mod(content).unwrap();
        assert_eq!(manifest.name, "example.com/mymodule");
        assert_eq!(manifest.version, "1.21");
        assert_eq!(manifest.dependencies.len(), 2);
        assert!(manifest.dependencies.iter().any(|d| {
            d.name == "example.com/dependency"
                && matches!(& d.source, DependencySource::Version(v) if v == "v1.2.3")
        }));
    }

    #[test]
    fn test_parse_go_mod_with_replace() {
        let content = r#"
module example.com/mymodule

go 1.21

require (
    example.com/dependency v1.2.3
)

replace example.com/dependency => ../local/path
"#;
        let manifest = parse_go_mod(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
        let dep = &manifest.dependencies[0];
        assert_eq!(dep.name, "example.com/dependency");
        assert!(matches!(& dep.source, DependencySource::Path(p) if p == "../local/path"));
    }

    #[test]
    fn test_parse_go_mod_with_indirect() {
        let content = r#"
module example.com/mymodule

go 1.21

require (
    example.com/direct v1.0.0
    example.com/indirect v0.1.0 //indirect
)
"#;
        let manifest = parse_go_mod(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dev_dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].name, "example.com/direct");
        assert_eq!(manifest.dev_dependencies[0].name, "example.com/indirect");
    }


    #[test]
    fn test_generate_manifest() {
        let result = generate_manifest("example.com/mymodule", "1.21");
        assert!(result.contains("module example.com/mymodule"));
        assert!(result.contains("go 1.21"));
    }

    #[test]
    fn test_parse_single_line_require() {
        let content = r#"
module example.com/mymodule

go 1.21

require example.com/dependency v1.2.3
"#;
        let manifest = parse_go_mod(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].name, "example.com/dependency");
    }

    #[test]
    fn test_parse_replace_with_version() {
        let content = r#"
module example.com/mymodule

go 1.21

require (
    example.com/dependency v1.2.3
)

replace example.com/dependency v1.2.3 => example.com/fork v1.2.4
"#;
        let manifest = parse_go_mod(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
        let dep = &manifest.dependencies[0];
        assert_eq!(dep.name, "example.com/fork");
        assert!(matches!(& dep.source, DependencySource::Version(v) if v == "v1.2.4"));
    }

    #[test]
    fn test_parse_exclude() {
        let content = r#"
module example.com/mymodule

go 1.21

require (
    example.com/dependency v1.2.3
)

exclude example.com/dependency v1.2.0
"#;
        let manifest = parse_go_mod(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
        let raw = &manifest.raw_data;
        assert_eq!(raw["excludes"].as_array().map(|a| a.len()), Some(1));
    }
}