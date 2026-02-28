//! Zig import support — rewrites `@import("path")` on file rename/move.

use mill_plugin_api::import_support::{ImportMoveSupport, ImportRenameSupport};
use mill_plugin_api::PluginResult;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

static IMPORT_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"@import\("([^"]+)"\)"#).unwrap());

pub struct ZigImportSupport;

impl ZigImportSupport {
    pub fn new() -> Self {
        Self
    }

    /// Rewrite `@import("old/path.zig")` to `@import("new/path.zig")` in source text.
    pub fn rewrite_zig_imports(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> PluginResult<(String, usize)> {
        let old_str = old_path.to_string_lossy();
        let new_str = new_path.to_string_lossy();
        let mut changes = 0;

        let result = IMPORT_PATTERN
            .replace_all(content, |caps: &regex::Captures| {
                let import_path = &caps[1];
                if import_path == old_str.as_ref()
                    || import_path.ends_with(&format!("/{}", old_str))
                {
                    changes += 1;
                    format!("@import(\"{}\")", import_path.replace(&*old_str, &*new_str))
                } else {
                    caps[0].to_string()
                }
            })
            .to_string();

        Ok((result, changes))
    }
}

impl ImportRenameSupport for ZigImportSupport {
    fn rewrite_imports_for_rename(
        &self,
        content: &str,
        old_name: &str,
        new_name: &str,
    ) -> (String, usize) {
        let old_path = Path::new(old_name);
        let new_path = Path::new(new_name);
        self.rewrite_zig_imports(content, old_path, new_path)
            .unwrap_or_else(|_| (content.to_string(), 0))
    }
}

impl ImportMoveSupport for ZigImportSupport {
    fn rewrite_imports_for_move(
        &self,
        content: &str,
        old_path: &Path,
        new_path: &Path,
    ) -> (String, usize) {
        self.rewrite_zig_imports(content, old_path, new_path)
            .unwrap_or_else(|_| (content.to_string(), 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_import() {
        let support = ZigImportSupport::new();
        let content = r#"const std = @import("std");
const utils = @import("utils.zig");
"#;
        let (result, changes) = support
            .rewrite_zig_imports(content, Path::new("utils.zig"), Path::new("helpers.zig"))
            .unwrap();

        assert_eq!(changes, 1);
        assert!(result.contains(r#"@import("helpers.zig")"#));
        assert!(result.contains(r#"@import("std")"#));
    }

    #[test]
    fn test_no_false_positives() {
        let support = ZigImportSupport::new();
        let content = r#"const std = @import("std");
const math = @import("math.zig");
"#;
        let (result, changes) = support
            .rewrite_zig_imports(content, Path::new("utils.zig"), Path::new("helpers.zig"))
            .unwrap();

        assert_eq!(changes, 0);
        assert_eq!(result, content);
    }
}
