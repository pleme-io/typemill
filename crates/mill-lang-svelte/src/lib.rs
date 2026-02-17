//! Svelte Language Plugin
//!
//! Provides safe import rewrite support for .svelte files by targeting
//! <script> blocks only and reusing TypeScript path alias logic.

use async_trait::async_trait;
use mill_plugin_api::mill_plugin;
use mill_plugin_api::{
    LanguageMetadata, LanguagePlugin, ManifestData, ParsedSource, PluginCapabilities, PluginResult,
};
use serde_json::json;
use std::path::Path;

mod import_support;
mod script_blocks;

use import_support::{rewrite_svelte_imports_for_move, SvelteImportSupport};
use mill_lang_typescript::path_alias_resolver::TypeScriptPathAliasResolver;

// Self-register the plugin with the TypeMill system.
mill_plugin! {
    name: "svelte",
    extensions: ["svelte"],
    manifest: "svelte.config.js",
    capabilities: SveltePlugin::CAPABILITIES,
    factory: SveltePlugin::boxed,
    lsp: None
}

pub struct SveltePlugin {
    metadata: LanguageMetadata,
    path_alias_resolver: TypeScriptPathAliasResolver,
    import_support: SvelteImportSupport,
}

impl SveltePlugin {
    pub const CAPABILITIES: PluginCapabilities = PluginCapabilities::none()
        .with_imports()
        .with_path_alias_resolver();

    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata {
                name: "svelte",
                extensions: &["svelte"],
                manifest_filename: "svelte.config.js",
                source_dir: "src",
                entry_point: "index.svelte",
                module_separator: "/",
            },
            path_alias_resolver: TypeScriptPathAliasResolver::new(),
            import_support: SvelteImportSupport::new(),
        }
    }

    pub fn boxed() -> Box<dyn LanguagePlugin> {
        Box::new(Self::new())
    }
}

impl Default for SveltePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguagePlugin for SveltePlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    async fn parse(&self, _source: &str) -> PluginResult<ParsedSource> {
        Ok(ParsedSource {
            data: json!({ "language": "svelte" }),
            symbols: vec![],
        })
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        Err(mill_plugin_api::PluginApiError::invalid_input(format!(
            "Svelte manifests are not supported (got: {:?})",
            path
        )))
    }

    fn capabilities(&self) -> PluginCapabilities {
        Self::CAPABILITIES
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn path_alias_resolver(&self) -> Option<&dyn mill_plugin_api::PathAliasResolver> {
        Some(&self.path_alias_resolver)
    }

    fn rewrite_file_references(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
        current_file: &Path,
        project_root: &Path,
        _rename_info: Option<&serde_json::Value>,
    ) -> Option<(String, usize)> {
        let (updated, changes) = rewrite_svelte_imports_for_move(
            content,
            old_path,
            new_path,
            current_file,
            project_root,
            &self.path_alias_resolver,
        );

        if changes > 0 && updated != content {
            Some((updated, changes))
        } else {
            None
        }
    }

    fn import_parser(&self) -> Option<&dyn mill_plugin_api::ImportParser> {
        Some(&self.import_support)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mill_plugin_api::path_alias_resolver::PathAliasResolver;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn rewrites_svelte_imports_inside_script() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create tsconfig mapping for $lib
        let tsconfig = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "$lib/*": ["src/lib/*"]
    }
  }
}"#;
        std::fs::write(project_root.join("tsconfig.json"), tsconfig).unwrap();

        std::fs::create_dir_all(project_root.join("src/lib/utils")).unwrap();
        std::fs::create_dir_all(project_root.join("src/routes")).unwrap();
        std::fs::write(
            project_root.join("src/lib/utils/text.ts"),
            "export const format = () => '';\n",
        )
        .unwrap();

        let content = r#"<script lang=\"ts\">
import { format } from '$lib/utils/text';
</script>

<div>{format('x')}</div>
"#;

        let plugin = SveltePlugin::new();
        let old_path = project_root.join("src/lib/utils/text.ts");
        let new_path = project_root.join("src/lib/utils/text-format.ts");
        let current_file = project_root.join("src/routes/page.svelte");

        assert!(
            plugin
                .path_alias_resolver
                .resolve_alias("$lib/utils/text", &current_file, project_root)
                .is_some(),
            "expected $lib alias to resolve via tsconfig"
        );

        let (updated, changes) = rewrite_svelte_imports_for_move(
            content,
            &old_path,
            &new_path,
            &current_file,
            project_root,
            &plugin.path_alias_resolver,
        );

        assert!(changes >= 1, "should update at least one import");
        assert!(updated.contains("$lib/utils/text-format"));
        assert_eq!(updated.contains("$lib/utils/text'"), false);
    }

    #[test]
    fn rewrites_sveltekit_extends_alias() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let web_dir = project_root.join("web");
        let svelte_kit_dir = web_dir.join(".svelte-kit");
        std::fs::create_dir_all(&svelte_kit_dir).unwrap();

        let svelte_tsconfig = r#"
{
  "compilerOptions": {
    "paths": {
      "$lib/*": ["../src/lib/*"]
    }
  }
}
"#;
        std::fs::write(svelte_kit_dir.join("tsconfig.json"), svelte_tsconfig).unwrap();

        let root_tsconfig = r#"
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "moduleResolution": "bundler"
  }
}
"#;
        std::fs::write(web_dir.join("tsconfig.json"), root_tsconfig).unwrap();

        std::fs::create_dir_all(web_dir.join("src/lib/utils")).unwrap();
        std::fs::create_dir_all(web_dir.join("src/routes")).unwrap();
        std::fs::write(
            web_dir.join("src/lib/utils/text.ts"),
            "export const format = () => '';\n",
        )
        .unwrap();

        let content = r#"<script lang=\"ts\">
import { format } from '$lib/utils/text';
</script>
"#;

        let plugin = SveltePlugin::new();
        let old_path = web_dir.join("src/lib/utils/text.ts");
        let new_path = web_dir.join("src/lib/utils/text-format.ts");
        let current_file = web_dir.join("src/routes/+page.svelte");

        let (updated, changes) = rewrite_svelte_imports_for_move(
            content,
            &old_path,
            &new_path,
            &current_file,
            project_root,
            &plugin.path_alias_resolver,
        );

        assert!(changes >= 1, "should update at least one import");
        assert!(updated.contains("$lib/utils/text-format"));
    }

    #[test]
    fn rewrites_repo_svelte_lib_alias_if_present() {
        let project_root = PathBuf::from("/workspace");
        let old_path = project_root.join("web/src/lib/utils/text.ts");
        let new_path = project_root.join("web/src/lib/utils/text-format.ts");
        let current_file = project_root.join("web/src/routes/+page.svelte");

        if !old_path.exists() || !current_file.exists() {
            return;
        }

        let content = std::fs::read_to_string(&current_file).unwrap();
        let plugin = SveltePlugin::new();

        let (updated, changes) = rewrite_svelte_imports_for_move(
            &content,
            &old_path,
            &new_path,
            &current_file,
            &project_root,
            &plugin.path_alias_resolver,
        );

        assert!(changes >= 1, "should update at least one import");
        assert!(updated.contains("$lib/utils/text-format"));
    }
}
