use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct NewLangArgs {
    /// Language name (e.g., "python", "go")
    language: String,

    /// Skip adding to workspace
    #[arg(long)]
    skip_workspace: bool,
}

pub fn run(args: NewLangArgs) -> Result<()> {
    let lang = args.language.to_lowercase();
    let crate_name = format!("cb-lang-{}", lang);
    let crate_dir = PathBuf::from("crates").join(&crate_name);

    if crate_dir.exists() {
        anyhow::bail!("Crate {} already exists", crate_name);
    }

    println!(
        "{} {}\n",
        "Creating language plugin:".bold(),
        crate_name.cyan()
    );

    // Create directory structure
    fs::create_dir_all(&crate_dir)?;
    fs::create_dir_all(crate_dir.join("src"))?;
    fs::create_dir_all(crate_dir.join("tests"))?;

    // Generate Cargo.toml
    let cargo_toml = generate_cargo_toml(&crate_name);
    fs::write(crate_dir.join("Cargo.toml"), cargo_toml)?;
    println!("{} Created Cargo.toml", "✓".green());

    // Generate lib.rs
    let lib_rs = generate_lib_rs(&lang);
    fs::write(crate_dir.join("src/lib.rs"), lib_rs)?;
    println!("{} Created src/lib.rs", "✓".green());

    // Generate basic test
    let test_rs = generate_test(&lang);
    fs::write(crate_dir.join("tests/integration_test.rs"), test_rs)?;
    println!("{} Created tests/integration_test.rs", "✓".green());

    // Add to workspace
    if !args.skip_workspace {
        println!(
            "\n{}",
            "⚠️  Please manually add to Cargo.toml workspace members:".yellow()
        );
        println!("  {}", format!("\"crates/{}\"", crate_name).cyan());
    }

    println!("\n{} Created {}", "✓".green(), crate_name.cyan());
    println!("\n{}", "Next steps:".bold());
    println!("  1. Implement LanguagePlugin trait in src/lib.rs");
    println!("  2. Add parser dependencies to Cargo.toml");
    println!(
        "  3. Run: {}",
        format!("cargo test -p {}", crate_name).cyan()
    );

    Ok(())
}

fn generate_cargo_toml(crate_name: &str) -> String {
    format!(
        r#"[package]
name = "{crate_name}"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true

[dependencies]
mill-plugin-api = {{ path = "../mill-plugin-api" }}
mill-lang-common = {{ path = "../mill-lang-common" }}
cb-protocol = {{ path = "../cb-protocol" }}

async-trait = {{ workspace = true }}
tokio = {{ workspace = true }}
serde = {{ workspace = true }}
serde_json = {{ workspace = true }}
tracing = {{ workspace = true }}
thiserror = {{ workspace = true }}

# TODO: Add language-specific parser dependencies

[dev-dependencies]
tokio-test = "0.4"
"#,
        crate_name = crate_name
    )
}

fn generate_lib_rs(lang: &str) -> String {
    let title_case = lang.chars().next().unwrap().to_uppercase().to_string() + &lang[1..];
    let lang_upper = lang.to_uppercase();

    let mut code = String::new();
    code.push_str(&format!(
        "//! {} language plugin for TypeMill\n\n",
        lang_upper
    ));
    code.push_str("use async_trait::async_trait;\n");
    code.push_str("use mill_plugin_api::{LanguagePlugin, LanguagePluginMetadata, ParsedSource};\n");
    code.push_str("use mill_foundation::protocol::ApiError;\n");
    code.push_str("use std::path::Path;\n\n");

    code.push_str(&format!("pub struct {}Plugin;\n\n", title_case));

    code.push_str(&format!("impl {}Plugin {{\n", title_case));
    code.push_str("    pub fn new() -> Self {\n");
    code.push_str("        Self\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str(&format!("impl Default for {}Plugin {{\n", title_case));
    code.push_str("    fn default() -> Self {\n");
    code.push_str("        Self::new()\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[async_trait]\n");
    code.push_str(&format!(
        "impl LanguagePlugin for {}Plugin {{\n",
        title_case
    ));
    code.push_str("    fn metadata(&self) -> LanguagePluginMetadata {\n");
    code.push_str("        LanguagePluginMetadata {\n");
    code.push_str(&format!("            name: \"{}\",\n", lang));
    code.push_str("            file_extensions: vec![\n");
    code.push_str(&format!(
        "                // TODO: Add file extensions for {}\n",
        lang
    ));
    code.push_str("            ],\n");
    code.push_str("            comment_styles: vec![\n");
    code.push_str("                // TODO: Add comment styles\n");
    code.push_str("            ],\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    code.push_str("    async fn parse(&self, _source: &str, _file_path: &Path) -> Result<ParsedSource, ApiError> {\n");
    code.push_str("        // TODO: Implement parsing logic\n");
    code.push_str("        Ok(ParsedSource {\n");
    code.push_str("            symbols: vec![],\n");
    code.push_str("            dependencies: vec![],\n");
    code.push_str("            errors: vec![],\n");
    code.push_str("        })\n");
    code.push_str("    }\n\n");

    code.push_str("    async fn find_symbol_at_position(\n");
    code.push_str("        &self,\n");
    code.push_str("        _parsed_source: &ParsedSource,\n");
    code.push_str("        _line: usize,\n");
    code.push_str("        _character: usize,\n");
    code.push_str("    ) -> Result<Option<mill_plugin_api::Symbol>, ApiError> {\n");
    code.push_str("        // TODO: Implement symbol lookup\n");
    code.push_str("        Ok(None)\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[cfg(test)]\n");
    code.push_str("mod tests {\n");
    code.push_str("    use super::*;\n\n");

    code.push_str("    #[tokio::test]\n");
    code.push_str("    async fn test_parse_empty() {\n");
    code.push_str(&format!(
        "        let plugin = {}Plugin::new();\n",
        title_case
    ));
    code.push_str("        let result = plugin.parse(\"\", Path::new(\"test\")).await;\n");
    code.push_str("        assert!(result.is_ok());\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

fn generate_test(lang: &str) -> String {
    format!(
        r#"//! Integration tests for {} plugin

use cb_lang_{lang}::{}Plugin;
use mill_plugin_api::LanguagePlugin;
use std::path::Path;

#[tokio::test]
async fn test_plugin_metadata() {{
    let plugin = {}Plugin::new();
    let metadata = plugin.metadata();

    assert_eq!(metadata.name, "{}");
    assert!(!metadata.file_extensions.is_empty());
}}

#[tokio::test]
async fn test_parse_integration() {{
    let plugin = {}Plugin::new();
    // TODO: Add real integration test with sample code
    let source = ""; // TODO: Add sample {} code
    let result = plugin.parse(source, Path::new("test")).await;
    assert!(result.is_ok());
}}
"#,
        lang,
        lang.chars().next().unwrap().to_uppercase().to_string() + &lang[1..],
        lang.chars().next().unwrap().to_uppercase().to_string() + &lang[1..],
        lang,
        lang.chars().next().unwrap().to_uppercase().to_string() + &lang[1..],
        lang,
    )
}
