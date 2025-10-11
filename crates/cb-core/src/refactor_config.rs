use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RefactorConfig {
    #[serde(default)]
    pub presets: HashMap<String, RefactorPreset>,
    #[serde(default)]
    pub defaults: RefactorDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RefactorPreset {
    pub strict: Option<bool>,
    pub validate_scope: Option<bool>,
    pub update_imports: Option<bool>,
    // ... other options
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefactorDefaults {
    pub dry_run: bool,
    pub rollback_on_error: bool,
    pub validate_checksums: bool,
}

impl Default for RefactorDefaults {
    fn default() -> Self {
        Self {
            dry_run: false,
            rollback_on_error: true,
            validate_checksums: true,
        }
    }
}

impl RefactorConfig {
    pub fn load(project_root: &PathBuf) -> Result<Self> {
        let config_path = project_root.join(".codebuddy/refactor.toml");
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(config_path)?;
        let config: RefactorConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Apply a preset to ApplyOptions (from workspace_apply_handler)
    ///
    /// Merges preset values into options, with explicit option values taking precedence.
    /// This allows users to define presets in .codebuddy/refactor.toml and apply them
    /// via the preset parameter.
    ///
    /// # Example
    ///
    /// .codebuddy/refactor.toml:
    /// ```toml
    /// [defaults]
    /// dry_run = false
    /// rollback_on_error = true
    /// validate_checksums = true
    ///
    /// [presets.strict]
    /// strict = true
    /// validate_scope = true
    /// update_imports = true
    ///
    /// [presets.quick]
    /// strict = false
    /// validate_scope = false
    /// ```
    ///
    /// Usage in workspace.apply_edit:
    /// ```json
    /// {
    ///   "plan": { ... },
    ///   "options": {
    ///     "preset": "strict",
    ///     "dry_run": true  // Overrides preset if present
    ///   }
    /// }
    /// ```
    pub fn apply_preset_to_defaults(&self, preset_name: &str) -> Result<RefactorDefaults> {
        let _preset = self
            .presets
            .get(preset_name)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_name))?;

        let defaults = self.defaults.clone();

        // Apply preset values to defaults
        // Note: Currently RefactorPreset has different fields than RefactorDefaults
        // This is a starting point - extend as needed when the types converge

        // For now, we just return the defaults since preset fields don't overlap
        // In a full implementation, you'd have fields like:
        // - if let Some(dry_run) = preset.dry_run { defaults.dry_run = dry_run; }
        // - if let Some(validate) = preset.validate_checksums { defaults.validate_checksums = validate; }

        Ok(defaults)
    }

    /// Get preset by name
    pub fn get_preset(&self, name: &str) -> Option<&RefactorPreset> {
        self.presets.get(name)
    }

    /// List all available preset names
    pub fn list_presets(&self) -> Vec<String> {
        self.presets.keys().cloned().collect()
    }
}
