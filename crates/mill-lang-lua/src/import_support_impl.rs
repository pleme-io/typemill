//! Lua import support — rewrites `require("path")` on file rename/move.

use mill_plugin_api::import_support::{ImportMoveSupport, ImportRenameSupport};
use mill_plugin_api::PluginResult;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

static REQUIRE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"require\s*[\(]\s*["']([^"']+)["']\s*\)"#).unwrap());
static REQUIRE_STRING_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"require\s+["']([^"']+)["']"#).unwrap());

pub struct LuaImportSupport;

impl LuaImportSupport {
    pub fn new() -> Self {
        Self
    }

    /// Rewrite `require("old.module")` to `require("new.module")` in source text.
    /// Lua uses dot-separated module paths: `require("foo.bar")` maps to `foo/bar.lua`.
    pub fn rewrite_lua_requires(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> PluginResult<(String, usize)> {
        // Convert filesystem paths to Lua module paths (foo/bar.lua -> foo.bar)
        let old_module = path_to_lua_module(old_path);
        let new_module = path_to_lua_module(new_path);

        if old_module.is_empty() {
            return Ok((content.to_string(), 0));
        }

        let mut changes = 0;
        let mut result = content.to_string();

        // Replace in require("module") form
        let replaced = REQUIRE_PATTERN.replace_all(&result, |caps: &regex::Captures| {
            let module_path = &caps[1];
            if module_path == old_module {
                changes += 1;
                caps[0].replace(module_path, &new_module)
            } else {
                caps[0].to_string()
            }
        });
        result = replaced.to_string();

        // Replace in require "module" form
        let replaced = REQUIRE_STRING_PATTERN.replace_all(&result, |caps: &regex::Captures| {
            let module_path = &caps[1];
            if module_path == old_module {
                changes += 1;
                caps[0].replace(module_path, &new_module)
            } else {
                caps[0].to_string()
            }
        });
        result = replaced.to_string();

        Ok((result, changes))
    }
}

/// Convert a filesystem path to a Lua module path.
/// `foo/bar.lua` -> `foo.bar`, `init.lua` -> just the parent module name.
fn path_to_lua_module(path: &Path) -> String {
    let path_str = path.with_extension("").to_string_lossy().to_string();
    // Strip trailing /init (Lua convention: foo/init.lua == require("foo"))
    let stripped = path_str.strip_suffix("/init").unwrap_or(&path_str);
    stripped.replace('/', ".")
}

impl ImportRenameSupport for LuaImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        let old_path = Path::new(old_name);
        let new_path = Path::new(new_name);
        self.rewrite_lua_requires(content, old_path, new_path)
            .unwrap_or_else(|_| (content.to_string(), 0))
    }
}

impl ImportMoveSupport for LuaImportSupport {
    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        self.rewrite_lua_requires(content, old_path, new_path)
            .unwrap_or_else(|_| (content.to_string(), 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_lua_module() {
        assert_eq!(path_to_lua_module(Path::new("foo/bar.lua")), "foo.bar");
        assert_eq!(path_to_lua_module(Path::new("utils.lua")), "utils");
        assert_eq!(
            path_to_lua_module(Path::new("foo/init.lua")),
            "foo"
        );
    }

    #[test]
    fn test_rewrite_require_parens() {
        let support = LuaImportSupport::new();
        let content = r#"local utils = require("utils")
local json = require("cjson")
"#;
        let (result, changes) = support
            .rewrite_lua_requires(content, Path::new("utils.lua"), Path::new("helpers.lua"))
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains(r#"require("helpers")"#));
        assert!(result.contains(r#"require("cjson")"#));
    }

    #[test]
    fn test_rewrite_nested_module() {
        let support = LuaImportSupport::new();
        let content = r#"local db = require("lib.database")
"#;
        let (result, changes) = support
            .rewrite_lua_requires(
                content,
                Path::new("lib/database.lua"),
                Path::new("lib/storage.lua"),
            )
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains(r#"require("lib.storage")"#));
    }

    #[test]
    fn test_no_false_positives() {
        let support = LuaImportSupport::new();
        let content = r#"local json = require("cjson")
"#;
        let (result, changes) = support
            .rewrite_lua_requires(content, Path::new("utils.lua"), Path::new("helpers.lua"))
            .unwrap();

        assert_eq!(changes, 0);
        assert_eq!(result, content);
    }
}
