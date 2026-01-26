//! Language registry synchronization
//!
//! Reads languages.toml and generates feature flags across all crates.
//!
//! This tool surgically edits Cargo.toml files to add/update lang-* features
//! while preserving all other configuration (comments, formatting, non-language features).

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Array, DocumentMut, Formatted, InlineTable, Item, Value};

#[derive(Parser)]
pub struct SyncLanguagesArgs {
    /// Path to languages.toml registry (default: workspace root)
    #[arg(long, default_value = "languages.toml")]
    registry: PathBuf,

    /// Dry run - show what would be changed without modifying files
    #[arg(long)]
    dry_run: bool,

    /// Verbose output
    #[arg(long, short)]
    verbose: bool,
}

/// Language configuration from languages.toml
#[derive(Debug, Deserialize)]
struct LanguageRegistry {
    languages: BTreeMap<String, LanguageConfig>,
}

#[derive(Debug, Deserialize)]
struct LanguageConfig {
    #[allow(dead_code)] // Used for validation, may be used for path operations later
    path: String,
    plugin_struct: String,
    category: LanguageCategory,
    default: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum LanguageCategory {
    Full,   // Full programming language (propagates to all 7 crates)
    Config, // Config-only language (only plugin-bundle, server, app)
}

/// Crates that need language feature flags
const CRATES_WITH_FEATURES: &[CrateInfo] = &[
    CrateInfo {
        name: "mill-services",
        path: "crates/mill-services",
        needs_dependency: true,
        categories: &[LanguageCategory::Full],
    },
    CrateInfo {
        name: "mill-ast",
        path: "crates/mill-ast",
        needs_dependency: false, // Uses workspace deps
        categories: &[LanguageCategory::Full],
    },
    CrateInfo {
        name: "mill-plugin-system",
        path: "crates/mill-plugin-system",
        needs_dependency: false,
        categories: &[LanguageCategory::Full],
    },
    CrateInfo {
        name: "mill-transport",
        path: "crates/mill-transport",
        needs_dependency: false,
        categories: &[LanguageCategory::Full],
    },
    CrateInfo {
        name: "mill-plugin-bundle",
        path: "crates/mill-plugin-bundle",
        needs_dependency: true,
        categories: &[LanguageCategory::Full, LanguageCategory::Config],
    },
    CrateInfo {
        name: "mill-server",
        path: "crates/mill-server",
        needs_dependency: false,
        categories: &[LanguageCategory::Full, LanguageCategory::Config],
    },
    CrateInfo {
        name: "mill",
        path: "apps/mill",
        needs_dependency: false,
        categories: &[LanguageCategory::Full, LanguageCategory::Config],
    },
];

struct CrateInfo {
    name: &'static str,
    path: &'static str,
    needs_dependency: bool,
    categories: &'static [LanguageCategory],
}

pub fn run(args: SyncLanguagesArgs) -> Result<()> {
    println!("{}", "🔄 Synchronizing language features...".bold().cyan());

    // 1. Parse languages.toml
    let registry = parse_registry(&args.registry)?;
    println!(
        "✓ Loaded {} language definitions from {}",
        registry.languages.len(),
        args.registry.display()
    );

    if args.verbose {
        for (name, config) in &registry.languages {
            println!(
                "  - {} ({:?}, default: {}, struct: {})",
                name.green(),
                config.category,
                config.default,
                config.plugin_struct
            );
        }
    }

    // 2. Update each crate's Cargo.toml
    let mut total_features_added = 0;
    let mut total_features_updated = 0;

    for crate_info in CRATES_WITH_FEATURES {
        let cargo_path = PathBuf::from(crate_info.path).join("Cargo.toml");

        if !cargo_path.exists() {
            println!(
                "  {} Skipping {} (Cargo.toml not found)",
                "⚠".yellow(),
                crate_info.name
            );
            continue;
        }

        let (added, updated) = sync_crate_features(
            &cargo_path,
            crate_info,
            &registry,
            args.dry_run,
            args.verbose,
        )?;

        total_features_added += added;
        total_features_updated += updated;
    }

    // 3. Generate mill-plugin-bundle/src/lib.rs
    let bundle_code_path = PathBuf::from("crates/mill-plugin-bundle/src/lib.rs");
    generate_plugin_bundle_code(&bundle_code_path, &registry, args.dry_run, args.verbose)?;

    // 4. Summary
    println!();
    if args.dry_run {
        println!(
            "{}",
            "🔍 Dry run complete (no files modified)".yellow().bold()
        );
    } else {
        println!(
            "{}",
            "✅ Language feature synchronization complete!"
                .green()
                .bold()
        );
    }

    println!("   Features added: {}", total_features_added);
    println!("   Features updated: {}", total_features_updated);
    println!("   Crates modified: {}", CRATES_WITH_FEATURES.len() + 1);

    Ok(())
}

fn parse_registry(path: &Path) -> Result<LanguageRegistry> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read registry file: {}", path.display()))?;

    toml::from_str(&content).with_context(|| "Failed to parse languages.toml")
}

fn sync_crate_features(
    cargo_path: &Path,
    crate_info: &CrateInfo,
    registry: &LanguageRegistry,
    dry_run: bool,
    verbose: bool,
) -> Result<(usize, usize)> {
    let content = fs::read_to_string(cargo_path)?;
    let mut doc = content
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse {}", cargo_path.display()))?;

    let mut added = 0;
    let mut updated = 0;

    // Get applicable languages for this crate
    let applicable_langs: Vec<(&String, &LanguageConfig)> = registry
        .languages
        .iter()
        .filter(|(_, config)| crate_info.categories.contains(&config.category))
        .collect();

    if verbose {
        println!(
            "\n  {} {} ({} languages)",
            "📦".blue(),
            crate_info.name.bold(),
            applicable_langs.len()
        );
    }

    // Update [features] section
    let features_table = doc
        .get_mut("features")
        .and_then(|item| item.as_table_mut())
        .context("No [features] section found")?;

    // Track which lang-* features should exist
    let expected_features: HashSet<String> = applicable_langs
        .iter()
        .map(|(name, _)| format!("lang-{}", name))
        .collect();

    // Update default feature
    if let Some(default_item) = features_table.get_mut("default") {
        if let Some(default_array) = default_item.as_array_mut() {
            // Remove all lang-* from default
            default_array.retain(|v| !v.as_str().map(|s| s.starts_with("lang-")).unwrap_or(false));

            // Add default languages
            for (name, config) in &applicable_langs {
                if config.default {
                    let feature_name = format!("lang-{}", name);
                    // Check if already present
                    let exists = default_array
                        .iter()
                        .any(|v| v.as_str() == Some(&feature_name));
                    if !exists {
                        default_array.push(feature_name);
                    }
                }
            }
        }
    }

    // Update/add lang-* features
    for (lang_name, config) in &applicable_langs {
        let feature_name = format!("lang-{}", lang_name);
        let feature_deps = generate_feature_dependencies(crate_info, lang_name, &config.category);

        if features_table.contains_key(&feature_name) {
            // Update existing
            features_table[&feature_name] = Item::Value(Value::Array(feature_deps));
            updated += 1;
        } else {
            // Add new
            features_table.insert(&feature_name, Item::Value(Value::Array(feature_deps)));
            added += 1;
        }
    }

    // Remove lang-* features that shouldn't exist (cleaned up languages)
    let mut to_remove: Vec<String> = Vec::new();
    for (key, _) in features_table.iter() {
        if key.starts_with("lang-") && !expected_features.contains(key) {
            to_remove.push(key.to_string());
        }
    }
    for key in to_remove {
        features_table.remove(&key);
        if verbose {
            println!("    {} Removed obsolete feature: {}", "🗑".yellow(), key);
        }
    }

    // Update [dependencies] if needed
    if crate_info.needs_dependency {
        let deps_table = doc
            .get_mut("dependencies")
            .and_then(|item| item.as_table_mut())
            .context("No [dependencies] section found")?;

        // Track expected dependencies
        let expected_deps: HashSet<String> = applicable_langs
            .iter()
            .map(|(name, _)| format!("mill-lang-{}", name))
            .collect();

        for (lang_name, _config) in &applicable_langs {
            let dep_name = format!("mill-lang-{}", lang_name);

            // Check if dependency exists
            if let Some(existing_dep) = deps_table.get_mut(&dep_name) {
                // Update existing dependency to ensure it's optional
                if let Some(inline_table) = existing_dep.as_inline_table_mut() {
                    // For mill-services, ensure optional = true
                    if crate_info.name == "mill-services" && !inline_table.contains_key("optional")
                    {
                        inline_table.insert("optional", Value::Boolean(Formatted::new(true)));
                        if verbose {
                            println!("    {} Updated {} to be optional", "✏️ ".yellow(), dep_name);
                        }
                    }
                }
            } else {
                // Add new dependency
                let mut dep_table = InlineTable::new();
                if crate_info.name == "mill-services" {
                    // mill-services uses workspace deps, but they must be optional
                    dep_table.insert("workspace", Value::Boolean(Formatted::new(true)));
                    dep_table.insert("optional", Value::Boolean(Formatted::new(true)));
                } else {
                    // mill-plugin-bundle uses path deps
                    dep_table.insert(
                        "path",
                        Value::String(Formatted::new(format!("../mill-lang-{}", lang_name))),
                    );
                    dep_table.insert("optional", Value::Boolean(Formatted::new(true)));
                    dep_table.insert("default-features", Value::Boolean(Formatted::new(false)));
                }

                deps_table.insert(&dep_name, Item::Value(Value::InlineTable(dep_table)));

                if verbose {
                    println!("    {} Added dependency: {}", "➕".green(), dep_name);
                }
            }
        }

        // Remove obsolete mill-lang-* dependencies
        let mut deps_to_remove: Vec<String> = Vec::new();
        for (key, _) in deps_table.iter() {
            if key.starts_with("mill-lang-") && !expected_deps.contains(key) {
                deps_to_remove.push(key.to_string());
            }
        }
        for key in deps_to_remove {
            deps_table.remove(&key);
            if verbose {
                println!("    {} Removed obsolete dependency: {}", "🗑".yellow(), key);
            }
        }
    }

    // Write back
    if !dry_run {
        fs::write(cargo_path, doc.to_string())?;
        println!("  ✓ Updated {}", crate_info.name);
    } else if verbose {
        println!("    [DRY RUN] Would write to {}", cargo_path.display());
    }

    Ok((added, updated))
}

fn generate_feature_dependencies(
    crate_info: &CrateInfo,
    lang_name: &str,
    category: &LanguageCategory,
) -> Array {
    let mut arr = Array::new();

    match crate_info.name {
        "mill-services" => {
            // Only Full languages reach here (filtered by applicable_langs)
            arr.push(format!("dep:mill-lang-{}", lang_name));
            arr.push(format!("mill-ast/lang-{}", lang_name));
            arr.push(format!("mill-plugin-system/lang-{}", lang_name));
        }
        "mill-ast" => {
            // Empty feature flag (used for conditional compilation)
            // Only Full languages reach here
        }
        "mill-plugin-system" => {
            // Only Full languages reach here
            arr.push("runtime");
            arr.push(format!("mill-ast/lang-{}", lang_name));
        }
        "mill-transport" => {
            // Only Full languages reach here
            arr.push(format!("mill-ast/lang-{}", lang_name));
        }
        "mill-plugin-bundle" => {
            // Both Full and Config languages
            arr.push(format!("dep:mill-lang-{}", lang_name));
        }
        "mill-server" => {
            // Both Full and Config languages reach here
            match category {
                LanguageCategory::Full => {
                    // Full languages need the complete dependency chain
                    arr.push(format!("mill-services/lang-{}", lang_name));
                    arr.push(format!("mill-ast/lang-{}", lang_name));
                    arr.push(format!("mill-plugin-bundle/lang-{}", lang_name));
                    arr.push(format!("mill-plugin-system/lang-{}", lang_name));
                    arr.push(format!("mill-transport/lang-{}", lang_name));
                }
                LanguageCategory::Config => {
                    // Config languages only need bundle (they don't have AST/services support)
                    arr.push(format!("mill-plugin-bundle/lang-{}", lang_name));
                }
            }
        }
        "mill" => {
            // Both Full and Config languages reach here
            match category {
                LanguageCategory::Full => {
                    // Full languages need the complete dependency chain
                    arr.push(format!("mill-server/lang-{}", lang_name));
                    arr.push(format!("mill-plugin-bundle/lang-{}", lang_name));
                    arr.push(format!("mill-ast/lang-{}", lang_name));
                    arr.push(format!("mill-plugin-system/lang-{}", lang_name));
                    arr.push(format!("mill-transport/lang-{}", lang_name));
                }
                LanguageCategory::Config => {
                    // Config languages only need bundle via server
                    arr.push(format!("mill-server/lang-{}", lang_name));
                    arr.push(format!("mill-plugin-bundle/lang-{}", lang_name));
                }
            }
        }
        _ => {}
    }

    arr
}

fn generate_plugin_bundle_code(
    target_path: &Path,
    registry: &LanguageRegistry,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let mut langs: Vec<(&String, &LanguageConfig)> = registry.languages.iter().collect();
    langs.sort_by_key(|(name, _)| *name); // Alphabetical order

    let mut code = String::new();

    // File header
    code.push_str("//! Language Plugin Bundle\n");
    code.push_str("//!\n");
    code.push_str("//! This file is AUTO-GENERATED by `cargo xtask sync-languages`.\n");
    code.push_str("//! Do not edit manually - changes will be overwritten.\n");
    code.push_str("//!\n");
    code.push_str("//! To add a new language:\n");
    code.push_str("//! 1. Add entry to languages.toml\n");
    code.push_str("//! 2. Run: cargo xtask sync-languages\n\n");

    code.push_str("use mill_plugin_api::{iter_plugins, LanguagePlugin};\n");
    code.push_str("use std::sync::Arc;\n\n");

    // Imports for each language
    code.push_str("// Force linker to include language plugins by actively using them.\n");
    code.push_str(
        "// This prevents linker dead code elimination from stripping the inventory submissions.\n",
    );
    code.push_str("// We reference each plugin's public type to ensure the crate is linked.\n");

    for (lang_name, config) in &langs {
        code.push_str(&format!("#[cfg(feature = \"lang-{}\")]\n", lang_name));
        code.push_str(&format!(
            "use mill_lang_{}::{};\n",
            lang_name, config.plugin_struct
        ));
    }

    code.push('\n');

    // Linkage function
    code.push_str(
        "// This function is never called but ensures the linker includes all plugin crates\n",
    );
    code.push_str("#[allow(dead_code)]\n");
    code.push_str("fn _force_plugin_linkage() {\n");
    code.push_str("    // These type references ensure the plugin crates are linked\n");
    code.push_str("    // The actual plugin instances will be discovered via inventory\n");

    for (lang_name, config) in &langs {
        code.push_str(&format!("    #[cfg(feature = \"lang-{}\")]\n", lang_name));
        code.push_str(&format!(
            "    let _: Option<{}> = None;\n",
            config.plugin_struct
        ));
    }

    code.push_str("}\n\n");

    // all_plugins function
    code.push_str("/// Returns all language plugins available in this bundle.\n");
    code.push_str("///\n");
    code.push_str("/// This function uses the plugin registry's auto-discovery mechanism\n");
    code.push_str(
        "/// to find all plugins that have self-registered using the `mill_plugin!` macro.\n",
    );
    code.push_str("pub fn all_plugins() -> Vec<Arc<dyn LanguagePlugin>> {\n");
    code.push_str("    let plugins: Vec<_> = iter_plugins()\n");
    code.push_str("        .map(|descriptor| {\n");
    code.push_str("            tracing::debug!(\n");
    code.push_str("                plugin_name = descriptor.name,\n");
    code.push_str("                extensions = ?descriptor.extensions,\n");
    code.push_str("                \"Discovered language plugin via inventory\"\n");
    code.push_str("            );\n");
    code.push_str("            let plugin = (descriptor.factory)();\n");
    code.push_str("            Arc::from(plugin) as Arc<dyn LanguagePlugin>\n");
    code.push_str("        })\n");
    code.push_str("        .collect();\n\n");
    code.push_str("    tracing::info!(\n");
    code.push_str("        plugin_count = plugins.len(),\n");
    code.push_str("        \"Language plugin bundle discovery complete\"\n");
    code.push_str("    );\n\n");
    code.push_str("    if plugins.is_empty() {\n");
    code.push_str("        tracing::warn!(\"No language plugins discovered - inventory system may be broken\");\n");
    code.push_str("    }\n\n");
    code.push_str("    plugins\n");
    code.push_str("}\n\n");

    // Tests
    code.push_str("#[cfg(test)]\n");
    code.push_str("mod tests {\n");
    code.push_str("    use super::*;\n\n");

    // Force linkage in tests
    code.push_str(
        "    // Force linker to include language plugins for inventory collection in tests\n",
    );
    for (lang_name, _) in &langs {
        code.push_str(&format!(
            "    #[cfg(all(test, feature = \"lang-{}\"))]\n",
            lang_name
        ));
        code.push_str(&format!("    extern crate mill_lang_{};\n", lang_name));
    }

    code.push('\n');
    code.push_str("    #[test]\n");
    code.push_str("    fn test_all_plugins_returns_plugins() {\n");
    code.push_str("        let plugins = all_plugins();\n");
    code.push_str("        assert!(\n");
    code.push_str("            plugins.len() >= 3,\n");
    code.push_str(
        "            \"Expected at least 3 plugins (Rust, TypeScript, Markdown), found {}\",\n",
    );
    code.push_str("            plugins.len()\n");
    code.push_str("        );\n");
    code.push_str("    }\n\n");

    code.push_str("    #[test]\n");
    code.push_str("    fn test_plugins_have_unique_names() {\n");
    code.push_str("        let plugins = all_plugins();\n");
    code.push_str("        let mut names = std::collections::HashSet::new();\n");
    code.push_str("        for plugin in plugins {\n");
    code.push_str("            let name = plugin.metadata().name;\n");
    code.push_str(
        "            assert!(names.insert(name), \"Duplicate plugin name found: {}\", name);\n",
    );
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    if !dry_run {
        fs::write(target_path, code)?;
        println!("  ✓ Generated {}", target_path.display());
    } else if verbose {
        println!("    [DRY RUN] Would generate {}", target_path.display());
        if verbose {
            println!("\n--- Generated code preview ---");
            println!("{}", &code[..code.len().min(1000)]);
            println!("...");
        }
    }

    Ok(())
}
