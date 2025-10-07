//! go.mod manifest file handling
//!
//! This module provides functionality for parsing and manipulating go.mod
//! manifest files, extracting dependency information, and updating dependencies.
//!
//! # go.mod Format
//!
//! A go.mod file contains:
//! - `module` directive: The module path
//! - `go` directive: Minimum Go version
//! - `require` directives: Direct dependencies
//! - `replace` directives: Module replacements
//! - `exclude` directives: Excluded versions
//! - `retract` directives: Retracted versions
//!
//! # Example
//!
//! ```text
//! module example.com/mymodule
//!
//! go 1.21
//!
//! require (
//!     example.com/dependency v1.2.3
//!     another.module/pkg v0.1.0
//! )
//!
//! replace example.com/dependency => ../local/path
//! ```
use cb_plugin_api::{
    Dependency, DependencySource, ManifestData, PluginError, PluginResult,
};
use cb_lang_common::read_manifest;
use std::path::Path;
use tracing::{debug, warn};
/// Represents a parsed go.mod file structure
#[derive(Debug, Clone)]
struct GoMod {
    module: String,
    go_version: Option<String>,
    requires: Vec<GoRequire>,
    replaces: Vec<GoReplace>,
    excludes: Vec<GoExclude>,
}
/// A require directive
#[derive(Debug, Clone)]
struct GoRequire {
    path: String,
    version: String,
    indirect: bool,
}
/// A replace directive
#[derive(Debug, Clone)]
struct GoReplace {
    old_path: String,
    old_version: Option<String>,
    new_path: String,
    new_version: Option<String>,
}
/// An exclude directive
#[derive(Debug, Clone)]
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
        version: go_mod.go_version.clone().unwrap_or_else(|| "1.0.0".to_string()),
        dependencies,
        dev_dependencies,
        raw_data: serde_json::json!(
            { "module" : go_mod.module, "go_version" : go_mod.go_version, "requires" :
            go_mod.requires.iter().map(| r | { serde_json::json!({ "path" : r.path,
            "version" : r.version, "indirect" : r.indirect }) }).collect::< Vec < _ >>
            (), "replaces" : go_mod.replaces.iter().map(| r | { serde_json::json!({
            "old_path" : r.old_path, "old_version" : r.old_version, "new_path" : r
            .new_path, "new_version" : r.new_version }) }).collect::< Vec < _ >> (),
            "excludes" : go_mod.excludes.iter().map(| e | { serde_json::json!({ "path" :
            e.path, "version" : e.version }) }).collect::< Vec < _ >> (), }
        ),
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
        return Err(PluginError::manifest(format!("Invalid require entry: {}", entry)));
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
fn parse_require_block<'a, I>(
    lines: &mut std::iter::Peekable<I>,
) -> PluginResult<Vec<GoRequire>>
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
        return Err(PluginError::manifest(format!("Invalid replace entry: {}", entry)));
    }
    let old_parts: Vec<&str> = parts[0].split_whitespace().collect();
    let new_parts: Vec<&str> = parts[1].split_whitespace().collect();
    if old_parts.is_empty() || new_parts.is_empty() {
        return Err(PluginError::manifest(format!("Invalid replace entry: {}", entry)));
    }
    Ok(GoReplace {
        old_path: old_parts[0].to_string(),
        old_version: old_parts.get(1).map(|s| s.to_string()),
        new_path: new_parts[0].to_string(),
        new_version: new_parts.get(1).map(|s| s.to_string()),
    })
}
/// Parse a multi-line replace block
fn parse_replace_block<'a, I>(
    lines: &mut std::iter::Peekable<I>,
) -> PluginResult<Vec<GoReplace>>
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
        return Err(PluginError::manifest(format!("Invalid exclude entry: {}", entry)));
    }
    Ok(GoExclude {
        path: parts[0].to_string(),
        version: parts[1].to_string(),
    })
}
/// Parse a multi-line exclude block
fn parse_exclude_block<'a, I>(
    lines: &mut std::iter::Peekable<I>,
) -> PluginResult<Vec<GoExclude>>
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
/// Update a dependency version in go.mod content
pub fn update_dependency(
    content: &str,
    dep_name: &str,
    new_version: &str,
) -> PluginResult<String> {
    debug!(
        dependency = % dep_name, version = % new_version, "Updating dependency in go.mod"
    );
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_require_block = false;
    let mut found = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("require (") {
            in_require_block = true;
            result.push(line.to_string());
            continue;
        }
        if in_require_block && trimmed == ")" {
            in_require_block = false;
            result.push(line.to_string());
            continue;
        }
        if trimmed.starts_with(&format!("{} ", dep_name))
            || (in_require_block && trimmed.starts_with(dep_name))
        {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                let indent = line.len() - line.trim_start().len();
                let prefix = " ".repeat(indent);
                let suffix = if parts.len() > 2 && parts[2].starts_with("//") {
                    format!(" {}", parts[2..].join(" "))
                } else {
                    String::new()
                };
                result.push(format!("{}{} {}{}", prefix, dep_name, new_version, suffix));
                found = true;
                continue;
            }
        }
        result.push(line.to_string());
    }
    if !found {
        warn!(dependency = % dep_name, "Dependency not found in go.mod");
        return Err(
            PluginError::manifest(format!("Dependency {} not found in go.mod", dep_name)),
        );
    }
    Ok(result.join("\n"))
}
/// Generate a new go.mod file
#[allow(dead_code)]
pub fn generate_manifest(module_name: &str, go_version: &str) -> String {
    format!("module {}\n\ngo {}\n", module_name, go_version)
}
/// Load and parse a go.mod file from a path
pub async fn load_go_mod(path: &Path) -> PluginResult<ManifestData> {
    let content = read_manifest(path).await?;
    parse_go_mod(&content)
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
        assert!(
            manifest.dependencies.iter().any(| d | { d.name == "example.com/dependency"
            && matches!(& d.source, DependencySource::Version(v) if v == "v1.2.3") })
        );
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
        assert!(
            matches!(& dep.source, DependencySource::Path(p) if p == "../local/path")
        );
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
    fn test_update_dependency() {
        let content = r#"module example.com/mymodule

go 1.21

require (
    example.com/dependency v1.2.3
    another.module/pkg v0.1.0
)
"#;
        let result = update_dependency(content, "example.com/dependency", "v1.3.0")
            .unwrap();
        assert!(result.contains("example.com/dependency v1.3.0"));
        assert!(! result.contains("v1.2.3"));
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
        assert!(raw["excludes"].as_array().unwrap().len() == 1);
    }
}
