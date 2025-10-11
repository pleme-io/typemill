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
    pub dry_run: Option<bool>,
    pub validate_checksums: Option<bool>,
    pub rollback_on_error: Option<bool>,
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

    /// Apply a preset to RefactorDefaults
    ///
    /// Merges preset values into defaults. Preset values override defaults.
    /// This allows users to define presets in .codebuddy/refactor.toml.
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
    /// validate_checksums = true
    /// rollback_on_error = true
    ///
    /// [presets.quick]
    /// validate_checksums = false
    /// rollback_on_error = false
    /// ```
    pub fn apply_preset_to_defaults(&self, preset_name: &str) -> Result<RefactorDefaults> {
        let preset = self
            .presets
            .get(preset_name)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_name))?;

        let mut defaults = self.defaults.clone();

        // Apply preset values to defaults (preset overrides defaults)
        if let Some(dry_run) = preset.dry_run {
            defaults.dry_run = dry_run;
        }
        if let Some(validate_checksums) = preset.validate_checksums {
            defaults.validate_checksums = validate_checksums;
        }
        if let Some(rollback_on_error) = preset.rollback_on_error {
            defaults.rollback_on_error = rollback_on_error;
        }

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
