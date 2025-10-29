//! Configuration system for analysis customization.
//!
//! This module provides a flexible configuration system that allows users to customize
//! analysis behavior through a TOML configuration file, environment variables, and
//! built-in presets.
//!
//! # Configuration Hierarchy
//!
//! Settings are resolved in the following order of precedence (highest to lowest):
//! 1. **Environment Variables**: `TYPEMILL_ANALYSIS_*` (e.g., `TYPEMILL_ANALYSIS_THRESHOLDS__MAX_COMPLEXITY=10`)
//! 2. **Local Configuration**: `.typemill/analysis.toml` file in the workspace root.
//! 3. **Preset**: A built-in configuration (`default`, `strict`, `relaxed`, `ci`).
//! 4. **Default Values**: Hardcoded defaults in the configuration structs.
//!
//! # Presets
//!
//! - **default**: A balanced set of rules for everyday development.
//! - **strict**: A more stringent ruleset for projects that require high code quality.
//! - **relaxed**: A lenient ruleset for projects that are in early development or have a lot of legacy code.
//! - **ci**: A ruleset optimized for CI/CD environments, with more comprehensive checks.
//!
//! # Configuration File Example
//!
//! Create `.typemill/analysis.toml` in your workspace root:
//!
//! ```toml
//! # .typemill/analysis.toml
//! preset = "strict" # Optional: Start with a preset
//!
//! [suggestions]
//! min_confidence = 0.8
//! include_safety_levels = ["safe", "requires_review"]
//! max_per_finding = 5
//! generate_refactor_calls = true
//!
//! [thresholds]
//! max_complexity = 12
//! max_nesting_depth = 3
//! max_function_lines = 80
//! max_parameters = 4
//!
//! [analysis]
//! enable_dead_code = true
//! enable_code_smells = true
//! enable_complexity = true
//! enable_maintainability = false
//! ```
//!
//! # Environment Variables
//!
//! Override any setting using environment variables with the prefix `TYPEMILL_ANALYSIS_`.
//! Use `__` as a separator for nested keys.
//!
//! ```bash
//! # Overrides [thresholds].max_complexity
//! export TYPEMILL_ANALYSIS_THRESHOLDS__MAX_COMPLEXITY=10
//!
//! # Overrides [suggestions].min_confidence
//! export TYPEMILL_ANALYSIS_SUGGESTIONS__MIN_CONFIDENCE=0.9
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use mill_handlers::handlers::tools::analysis::config::AnalysisConfig;
//! use std::path::Path;
//!
//! // Load from workspace, environment, and defaults
//! let config = AnalysisConfig::load(Path::new("/workspace"));
//!
//! // Access a value
//! println!("Max complexity: {}", config.thresholds.max_complexity);
//! ```

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Root configuration for all analysis tools.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AnalysisConfig {
    /// Optional preset to use as a base configuration.
    pub preset: Option<String>,
    /// Configuration for suggestion generation.
    pub suggestions: SuggestionConfig,
    /// Thresholds for various code quality metrics.
    pub thresholds: ThresholdConfig,
    /// Toggles for enabling/disabling major analysis categories.
    pub analysis: AnalysisSwitches,
}

/// Configuration for suggestion generation.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct SuggestionConfig {
    /// Minimum confidence level (0.0-1.0) for a suggestion to be shown.
    pub min_confidence: f64,
    /// List of safety levels to include in the output.
    pub include_safety_levels: Vec<String>,
    /// Maximum number of suggestions to generate per finding.
    pub max_per_finding: usize,
    /// Whether to generate actionable `refactor_call` objects.
    pub generate_refactor_calls: bool,
}

/// Thresholds for code quality and complexity metrics.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct ThresholdConfig {
    /// Maximum cyclomatic complexity allowed for a function.
    pub max_complexity: u32,
    /// Maximum nesting depth within a function.
    pub max_nesting_depth: u32,
    /// Maximum lines of code for a single function.
    pub max_function_lines: u32,
    /// Maximum number of parameters for a function.
    pub max_parameters: u32,
}

/// Toggles for enabling or disabling major analysis categories.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct AnalysisSwitches {
    pub enable_dead_code: bool,
    pub enable_code_smells: bool,
    pub enable_complexity: bool,
    pub enable_maintainability: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        get_default_preset()
    }
}

impl Default for SuggestionConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            include_safety_levels: vec!["safe".to_string(), "requires_review".to_string()],
            max_per_finding: 3,
            generate_refactor_calls: true,
        }
    }
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            max_complexity: 15,
            max_nesting_depth: 4,
            max_function_lines: 100,
            max_parameters: 5,
        }
    }
}

impl Default for AnalysisSwitches {
    fn default() -> Self {
        Self {
            enable_dead_code: true,
            enable_code_smells: true,
            enable_complexity: true,
            enable_maintainability: true,
        }
    }
}

impl AnalysisConfig {
    /// Loads configuration from files, environment, and presets.
    ///
    /// The final configuration is determined by merging sources in this order:
    /// 1. Preset values (lowest precedence).
    /// 2. `.typemill/analysis.toml` file.
    /// 3. Environment variables (highest precedence).
    ///
    /// # Arguments
    /// - `workspace_root`: The root directory of the workspace to search for the config file.
    pub fn load(workspace_root: &Path) -> Result<Self, ConfigError> {
        let config_path = workspace_root.join(".typemill").join("analysis.toml");

        // We need to determine the preset first, as it forms the base layer.
        // We do this by reading *only* the `preset` key from the config file.
        #[derive(Deserialize, Default)]
        struct PresetOnly {
            preset: Option<String>,
        }

        // If the file doesn't exist or is malformed, `extract` will fail.
        // `unwrap_or_default` handles this, giving us a default `PresetOnly` instance.
        let preset_name = Figment::new()
            .merge(Toml::file(&config_path))
            .extract::<PresetOnly>()
            .unwrap_or_default()
            .preset
            .unwrap_or_else(|| "default".to_string());

        // Get the configuration for the chosen preset.
        let preset_config = Self::from_preset(&preset_name)?;
        let preset_toml = toml::to_string(&preset_config)?;

        // Now, build the final configuration by layering sources.
        // 1. Base: Preset configuration.
        // 2. Override: .typemill/analysis.toml file.
        // 3. Override: Environment variables.
        Figment::new()
            .merge(Toml::string(&preset_toml))
            .merge(Toml::file(config_path))
            .merge(Env::prefixed("TYPEMILL_ANALYSIS_").split("__"))
            .extract()
            .map_err(ConfigError::from)
    }

    /// Checks if a specific analysis kind is enabled in the configuration.
    pub fn is_kind_enabled(&self, category: &str, kind: &str) -> bool {
        match category {
            "dead_code" => self.analysis.enable_dead_code,
            "quality" => match kind {
                "complexity" | "readability" => self.analysis.enable_complexity,
                "smells" => self.analysis.enable_code_smells,
                "maintainability" => self.analysis.enable_maintainability,
                // Markdown analysis is not controlled by these switches yet.
                "markdown_structure" | "markdown_formatting" => true,
                _ => true, // Default to enabled for unknown kinds within quality
            },
            // Enable other categories by default.
            _ => true,
        }
    }

    /// Creates a configuration from a named preset.
    pub fn from_preset(preset: &str) -> Result<Self, ConfigError> {
        match preset {
            "default" => Ok(get_default_preset()),
            "strict" => Ok(get_strict_preset()),
            "relaxed" => Ok(get_relaxed_preset()),
            "ci" => Ok(get_ci_preset()),
            _ => Err(ConfigError::InvalidPreset(preset.to_string())),
        }
    }
}

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Figment error: {0}")]
    Figment(#[from] figment::Error),

    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::ser::Error),

    #[error("Invalid preset '{0}'. Available presets: default, strict, relaxed, ci")]
    InvalidPreset(String),
}

impl From<ConfigError> for figment::Error {
    fn from(err: ConfigError) -> figment::Error {
        use figment::error::Kind;
        figment::Error::from(Kind::Message(err.to_string()))
    }
}

// --- Preset Definitions ---

pub fn get_default_preset() -> AnalysisConfig {
    AnalysisConfig {
        preset: Some("default".to_string()),
        suggestions: SuggestionConfig {
            min_confidence: 0.7,
            include_safety_levels: vec!["safe".to_string(), "requires_review".to_string()],
            max_per_finding: 3,
            generate_refactor_calls: true,
        },
        thresholds: ThresholdConfig {
            max_complexity: 15,
            max_nesting_depth: 4,
            max_function_lines: 100,
            max_parameters: 5,
        },
        analysis: AnalysisSwitches {
            enable_dead_code: true,
            enable_code_smells: true,
            enable_complexity: true,
            enable_maintainability: true,
        },
    }
}

pub fn get_strict_preset() -> AnalysisConfig {
    AnalysisConfig {
        preset: Some("strict".to_string()),
        suggestions: SuggestionConfig {
            min_confidence: 0.85,
            include_safety_levels: vec!["safe".to_string()],
            max_per_finding: 5,
            generate_refactor_calls: true,
        },
        thresholds: ThresholdConfig {
            max_complexity: 8,
            max_nesting_depth: 3,
            max_function_lines: 50,
            max_parameters: 4,
        },
        analysis: AnalysisSwitches {
            enable_dead_code: true,
            enable_code_smells: true,
            enable_complexity: true,
            enable_maintainability: true,
        },
    }
}

pub fn get_relaxed_preset() -> AnalysisConfig {
    AnalysisConfig {
        preset: Some("relaxed".to_string()),
        suggestions: SuggestionConfig {
            min_confidence: 0.5,
            include_safety_levels: vec!["safe".to_string(), "requires_review".to_string(), "unsafe".to_string()],
            max_per_finding: 10,
            generate_refactor_calls: false,
        },
        thresholds: ThresholdConfig {
            max_complexity: 25,
            max_nesting_depth: 6,
            max_function_lines: 200,
            max_parameters: 7,
        },
        analysis: AnalysisSwitches {
            enable_dead_code: true,
            enable_code_smells: true,
            enable_complexity: true,
            enable_maintainability: true,
        },
    }
}

pub fn get_ci_preset() -> AnalysisConfig {
    AnalysisConfig {
        preset: Some("ci".to_string()),
        suggestions: SuggestionConfig {
            min_confidence: 0.8,
            include_safety_levels: vec!["safe".to_string(), "requires_review".to_string()],
            max_per_finding: 100, // Report more issues in CI
            generate_refactor_calls: false, // Not needed for CI reports
        },
        thresholds: ThresholdConfig {
            max_complexity: 10,
            max_nesting_depth: 4,
            max_function_lines: 75,
            max_parameters: 5,
        },
        analysis: AnalysisSwitches {
            enable_dead_code: true,
            enable_code_smells: true,
            enable_complexity: true,
            enable_maintainability: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment::Jail;

    #[test]
    fn test_default_config() {
        let config = AnalysisConfig::default();
        assert_eq!(config.thresholds.max_complexity, 15);
        assert_eq!(config.suggestions.min_confidence, 0.7);
        assert!(config.analysis.enable_dead_code);
    }

    #[test]
    fn test_load_from_preset() {
        let strict_config = AnalysisConfig::from_preset("strict").unwrap();
        assert_eq!(strict_config.thresholds.max_complexity, 8);
        assert_eq!(strict_config.suggestions.min_confidence, 0.85);

        let relaxed_config = AnalysisConfig::from_preset("relaxed").unwrap();
        assert_eq!(relaxed_config.thresholds.max_complexity, 25);
    }

    #[test]
    fn test_load_from_file() {
        Jail::expect_with(|jail| {
            jail.create_dir(".typemill")?;
            jail.create_file(
                ".typemill/analysis.toml",
                r#"
                [thresholds]
                max_complexity = 20
                "#,
            )?;

            let config = AnalysisConfig::load(jail.directory())?;
            assert_eq!(config.thresholds.max_complexity, 20);
            // Default value should still be present
            assert_eq!(config.thresholds.max_nesting_depth, 4);
            Ok(())
        });
    }

    #[test]
    fn test_file_overrides_preset() {
        Jail::expect_with(|jail| {
            jail.create_dir(".typemill")?;
            jail.create_file(
                ".typemill/analysis.toml",
                r#"
                preset = "strict"
                [thresholds]
                max_complexity = 22
                "#,
            )?;

            let config = AnalysisConfig::load(jail.directory())?;
            // The file's value of 22 should override the strict preset's value of 8.
            assert_eq!(config.thresholds.max_complexity, 22);
            // Other strict preset values should still apply.
            assert_eq!(config.suggestions.min_confidence, 0.85);
            Ok(())
        });
    }

    #[test]
    fn test_env_var_overrides_file_and_preset() {
        Jail::expect_with(|jail| {
            jail.create_dir(".typemill")?;
            jail.create_file(
                ".typemill/analysis.toml",
                r#"
                preset = "strict"
                [thresholds]
                max_complexity = 22
                [suggestions]
                min_confidence = 0.9
                "#,
            )?;

            // Set environment variables
            jail.set_env("TYPEMILL_ANALYSIS_THRESHOLDS__MAX_COMPLEXITY", "30");
            jail.set_env("TYPEMILL_ANALYSIS_ANALYSIS__ENABLE_DEAD_CODE", "false");

            let config = AnalysisConfig::load(jail.directory())?;

            // Env var overrides file
            assert_eq!(config.thresholds.max_complexity, 30);
            // File value is used when no env var
            assert_eq!(config.suggestions.min_confidence, 0.9);
            // Env var overrides preset/default
            assert!(!config.analysis.enable_dead_code);

            Ok(())
        });
    }

    #[test]
    fn test_load_no_file_uses_defaults() {
        Jail::expect_with(|jail| {
            // No config file created
            let config = AnalysisConfig::load(jail.directory())?;
            assert_eq!(config, AnalysisConfig::default());
            Ok(())
        });
    }

    #[test]
    fn test_invalid_preset_name() {
        let result = AnalysisConfig::from_preset("nonexistent");
        assert!(matches!(result, Err(ConfigError::InvalidPreset(_))));
    }
}