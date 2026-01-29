//! Workspace utilities shared across handlers.

/// Controls how aggressively imports are updated during rename operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpdateMode {
    /// Only update top-level import/use statements (current default behavior).
    Conservative,
    /// Update all import/use statements including function-scoped ones.
    Standard,
    /// Update all imports and qualified paths (e.g., module::function, module.method).
    /// ⚠️ RISKY: May update code that shouldn't be changed. Use dry_run first!
    Aggressive,
    /// Update everything including string literals.
    /// ⚠️ VERY RISKY: Will update strings that may not be import paths. Always preview with dry_run!
    Full,
}

impl UpdateMode {
    /// Convert UpdateMode to mill_plugin_api::ScanScope.
    pub fn to_scan_scope(self) -> mill_plugin_api::ScanScope {
        use mill_plugin_api::ScanScope;
        match self {
            UpdateMode::Conservative => ScanScope::TopLevelOnly,
            UpdateMode::Standard => ScanScope::AllUseStatements,
            UpdateMode::Aggressive => ScanScope::QualifiedPaths,
            UpdateMode::Full => ScanScope::All,
        }
    }

    /// Returns true if this mode is risky and requires user confirmation.
    pub fn is_risky(self) -> bool {
        matches!(self, UpdateMode::Aggressive | UpdateMode::Full)
    }

    /// Returns a warning message for risky modes.
    pub fn warning_message(self) -> Option<&'static str> {
        match self {
            UpdateMode::Aggressive => Some(
                "⚠️ Aggressive mode updates qualified paths (e.g., module::function). This may modify code that shouldn't be changed. Review changes carefully before committing."
            ),
            UpdateMode::Full => Some(
                "⚠️ Full mode updates string literals containing the module name. This is VERY RISKY and may break unrelated code. Always use dry_run=true first to preview changes!"
            ),
            _ => None,
        }
    }
}
