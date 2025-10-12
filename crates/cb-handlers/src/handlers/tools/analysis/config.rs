//! Configuration system for analysis customization
//!
//! This module provides a flexible configuration system that allows users to customize
//! analysis behavior through TOML configuration files, presets, and per-category overrides.
//!
//! # Configuration File Example
//!
//! Create `.codebuddy/analysis.toml` in your workspace root:
//!
//! ```toml
//! # .codebuddy/analysis.toml
//! preset = "default"
//!
//! [overrides.quality]
//! enabled = ["complexity", "smells", "maintainability"]
//! [overrides.quality.thresholds]
//! complexity_threshold = 15
//! maintainability_threshold = 70
//!
//! [overrides.dead_code]
//! enabled = ["unused_imports", "unused_symbols", "unreachable_code"]
//!
//! [overrides.documentation]
//! [overrides.documentation.thresholds]
//! coverage_threshold = 80
//!
//! [overrides.tests]
//! [overrides.tests.thresholds]
//! coverage_ratio_threshold = 0.9
//! ```
//!
//! # Presets
//!
//! Three presets are available out of the box:
//! - **"strict"**: Aggressive thresholds for high-quality codebases
//! - **"default"**: Balanced thresholds for most projects
//! - **"relaxed"**: Lenient thresholds for prototypes or legacy code
//!
//! # Usage
//!
//! ```no_run
//! use cb_handlers::handlers::tools::analysis::config::AnalysisConfig;
//! use std::path::Path;
//!
//! // Load from file
//! let config = AnalysisConfig::load(Path::new("/workspace")).unwrap_or_else(|_| {
//!     AnalysisConfig::default()
//! });
//!
//! // Check if a kind is enabled
//! if config.is_kind_enabled("quality", "complexity") {
//!     // Run complexity analysis
//! }
//!
//! // Get a threshold
//! if let Some(threshold) = config.get_threshold("quality", "complexity_threshold") {
//!     println!("Complexity threshold: {}", threshold);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Analysis configuration loaded from .codebuddy/analysis.toml
///
/// This is the root configuration structure that defines how analysis tools
/// should behave. It supports preset-based configuration and per-category overrides.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnalysisConfig {
    /// Preset name (e.g., "strict", "relaxed", "default")
    ///
    /// Presets provide a starting point for configuration with sensible defaults
    /// for different use cases. Can be overridden by category-specific settings.
    #[serde(default)]
    pub preset: Option<String>,

    /// Category-specific overrides
    ///
    /// Each key is a category name (e.g., "quality", "dead_code", "dependencies")
    /// and the value defines which detection kinds are enabled and what thresholds to use.
    #[serde(default)]
    pub overrides: HashMap<String, CategoryConfig>,
}

/// Configuration for a specific analysis category
///
/// Allows fine-grained control over which detection kinds run and what
/// thresholds they use within a category.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CategoryConfig {
    /// Enabled detection kinds for this category
    ///
    /// If specified, only these kinds will run. If not specified, all kinds
    /// for the category are enabled by default.
    ///
    /// Example: `["complexity", "smells", "maintainability"]`
    #[serde(default)]
    pub enabled: Option<Vec<String>>,

    /// Thresholds for this category
    ///
    /// Maps threshold names to their numeric values. The specific thresholds
    /// available depend on the category and detection kind.
    ///
    /// Example: `{"complexity_threshold": 15.0, "maintainability_threshold": 70.0}`
    #[serde(default)]
    pub thresholds: Option<HashMap<String, f64>>,

    /// Additional options
    ///
    /// Extensibility point for category-specific configuration that doesn't fit
    /// into enabled/thresholds. Currently unused but reserved for future enhancements.
    #[serde(default)]
    pub options: Option<HashMap<String, serde_json::Value>>,
}

impl AnalysisConfig {
    /// Load configuration from .codebuddy/analysis.toml
    ///
    /// Attempts to load and parse the configuration file from the workspace root.
    /// If the file doesn't exist or can't be parsed, returns an error.
    ///
    /// # Arguments
    /// - `workspace_root`: The root directory of the workspace
    ///
    /// # Returns
    /// - `Ok(AnalysisConfig)`: Successfully loaded configuration
    /// - `Err(ConfigError)`: File not found, parse error, or IO error
    ///
    /// # Example
    /// ```no_run
    /// use cb_handlers::handlers::tools::analysis::config::AnalysisConfig;
    /// use std::path::Path;
    ///
    /// let config = AnalysisConfig::load(Path::new("/workspace"))
    ///     .unwrap_or_else(|_| AnalysisConfig::default());
    /// ```
    pub fn load(workspace_root: &Path) -> Result<Self, ConfigError> {
        let config_path = workspace_root.join(".codebuddy").join("analysis.toml");

        // If file doesn't exist, return default config
        if !config_path.exists() {
            return Ok(Self::default());
        }

        // Read and parse TOML file
        let contents = std::fs::read_to_string(&config_path)?;
        let mut config: AnalysisConfig = toml::from_str(&contents)?;

        // Apply preset if specified
        if let Some(preset) = config.preset.clone() {
            config.apply_preset(&preset)?;
        }

        Ok(config)
    }

    /// Get default configuration
    ///
    /// Returns a configuration with the "default" preset applied and no overrides.
    /// This is the fallback when no configuration file is found.
    ///
    /// # Returns
    /// A new `AnalysisConfig` with default preset settings
    pub fn default() -> Self {
        let mut config = Self {
            preset: Some("default".to_string()),
            overrides: HashMap::new(),
        };

        // Apply default preset thresholds
        // This ensures we have sensible defaults even without a config file
        let _ = config.apply_preset("default");

        config
    }

    /// Apply preset (strict, relaxed, default)
    ///
    /// Loads predefined threshold values for all analysis categories based on
    /// the specified preset. Existing overrides are preserved and take precedence
    /// over preset values.
    ///
    /// # Arguments
    /// - `preset`: The preset name ("strict", "default", or "relaxed")
    ///
    /// # Returns
    /// - `Ok(())`: Preset successfully applied
    /// - `Err(ConfigError::InvalidPreset)`: Unknown preset name
    ///
    /// # Preset Descriptions
    ///
    /// ## "strict" - For high-quality, production codebases
    /// - Aggressive complexity limits (threshold=5)
    /// - High maintainability requirements (80%)
    /// - Low coupling tolerance (0.5)
    /// - Deep documentation requirements (90%)
    /// - Full test coverage expectations (100%)
    ///
    /// ## "default" - Balanced for most projects
    /// - Moderate complexity limits (threshold=10)
    /// - Standard maintainability (65%)
    /// - Normal coupling tolerance (0.7)
    /// - Standard documentation (70%)
    /// - Good test coverage (80%)
    ///
    /// ## "relaxed" - For prototypes and legacy code
    /// - Lenient complexity limits (threshold=20)
    /// - Lower maintainability bar (50%)
    /// - High coupling tolerance (0.9)
    /// - Minimal documentation (50%)
    /// - Basic test coverage (50%)
    pub fn apply_preset(&mut self, preset: &str) -> Result<(), ConfigError> {
        let thresholds = match preset {
            "strict" => get_strict_preset(),
            "default" => get_default_preset(),
            "relaxed" => get_relaxed_preset(),
            _ => {
                return Err(ConfigError::InvalidPreset(format!(
                    "Unknown preset '{}'. Available presets: strict, default, relaxed",
                    preset
                )))
            }
        };

        // Merge preset thresholds with existing config
        // Existing overrides take precedence over preset values
        for (category, preset_config) in thresholds {
            self.overrides
                .entry(category)
                .or_insert_with(|| preset_config.clone());
        }

        self.preset = Some(preset.to_string());
        Ok(())
    }

    /// Get threshold for a specific metric in a category
    ///
    /// Looks up a threshold value, first checking category overrides,
    /// then falling back to preset defaults if available.
    ///
    /// # Arguments
    /// - `category`: The analysis category (e.g., "quality", "dependencies")
    /// - `metric`: The threshold name (e.g., "complexity_threshold")
    ///
    /// # Returns
    /// - `Some(f64)`: The threshold value if found
    /// - `None`: Threshold not configured
    ///
    /// # Example
    /// ```no_run
    /// # use cb_handlers::handlers::tools::analysis::config::AnalysisConfig;
    /// let config = AnalysisConfig::default();
    /// if let Some(threshold) = config.get_threshold("quality", "complexity_threshold") {
    ///     println!("Complexity threshold: {}", threshold);
    /// }
    /// ```
    pub fn get_threshold(&self, category: &str, metric: &str) -> Option<f64> {
        self.overrides
            .get(category)
            .and_then(|cat_config| cat_config.thresholds.as_ref())
            .and_then(|thresholds| thresholds.get(metric))
            .copied()
    }

    /// Check if a detection kind is enabled
    ///
    /// Determines whether a specific detection kind should run based on the
    /// configuration. If no enabled list is specified for the category, all
    /// kinds are considered enabled by default.
    ///
    /// # Arguments
    /// - `category`: The analysis category (e.g., "quality")
    /// - `kind`: The detection kind (e.g., "complexity", "smells")
    ///
    /// # Returns
    /// - `true`: The kind should run
    /// - `false`: The kind is explicitly disabled
    ///
    /// # Example
    /// ```no_run
    /// # use cb_handlers::handlers::tools::analysis::config::AnalysisConfig;
    /// let config = AnalysisConfig::default();
    /// if config.is_kind_enabled("quality", "complexity") {
    ///     // Run complexity analysis
    /// }
    /// ```
    pub fn is_kind_enabled(&self, category: &str, kind: &str) -> bool {
        if let Some(cat_config) = self.overrides.get(category) {
            if let Some(enabled) = &cat_config.enabled {
                // If enabled list is specified, check if kind is in it
                return enabled.contains(&kind.to_string());
            }
        }
        // If no enabled list, all kinds are enabled by default
        true
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self::default()
    }
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    /// IO error reading configuration file
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parse error
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// Invalid preset name
    #[error("Invalid preset: {0}")]
    InvalidPreset(String),

    /// Feature not yet implemented (MVP stub)
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

// ============================================================================
// Preset Definitions
// ============================================================================

/// Get strict preset thresholds
///
/// Aggressive thresholds for high-quality, production codebases where
/// code quality is critical.
fn get_strict_preset() -> HashMap<String, CategoryConfig> {
    let mut presets = HashMap::new();

    // Quality - Aggressive complexity and maintainability requirements
    presets.insert(
        "quality".to_string(),
        CategoryConfig {
            enabled: None, // All kinds enabled by default
            thresholds: Some(HashMap::from([
                ("complexity_threshold".to_string(), 5.0),
                ("maintainability_threshold".to_string(), 80.0),
                ("readability_threshold".to_string(), 80.0),
            ])),
            options: None,
        },
    );

    // Dead Code - Flag all unused code immediately
    presets.insert(
        "dead_code".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coverage_threshold".to_string(), 0.0), // Flag all unused
            ])),
            options: None,
        },
    );

    // Dependencies - Low coupling tolerance, require high cohesion
    presets.insert(
        "dependencies".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coupling_threshold".to_string(), 0.5),
                ("cohesion_threshold".to_string(), 0.3),
                ("depth_threshold".to_string(), 3.0),
            ])),
            options: None,
        },
    );

    // Structure - Shallow hierarchies, small modules
    presets.insert(
        "structure".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("hierarchy_depth_threshold".to_string(), 3.0),
                ("inheritance_depth_threshold".to_string(), 2.0),
                ("module_size_threshold".to_string(), 30.0),
            ])),
            options: None,
        },
    );

    // Documentation - Comprehensive documentation required
    presets.insert(
        "documentation".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coverage_threshold".to_string(), 90.0),
                ("quality_threshold".to_string(), 0.9),
            ])),
            options: None,
        },
    );

    // Tests - Full test coverage expected
    presets.insert(
        "tests".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coverage_ratio_threshold".to_string(), 1.0),
                ("assertions_per_test_min".to_string(), 2.0),
            ])),
            options: None,
        },
    );

    presets
}

/// Get default preset thresholds
///
/// Balanced thresholds suitable for most production projects.
fn get_default_preset() -> HashMap<String, CategoryConfig> {
    let mut presets = HashMap::new();

    // Quality - Moderate complexity and maintainability standards
    presets.insert(
        "quality".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("complexity_threshold".to_string(), 10.0),
                ("maintainability_threshold".to_string(), 65.0),
                ("readability_threshold".to_string(), 65.0),
            ])),
            options: None,
        },
    );

    // Dead Code - Standard unused code detection
    presets.insert(
        "dead_code".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: None, // Use detection defaults
            options: None,
        },
    );

    // Dependencies - Standard coupling and cohesion expectations
    presets.insert(
        "dependencies".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coupling_threshold".to_string(), 0.7),
                ("cohesion_threshold".to_string(), 0.5),
                ("depth_threshold".to_string(), 5.0),
            ])),
            options: None,
        },
    );

    // Structure - Reasonable hierarchy limits
    presets.insert(
        "structure".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("hierarchy_depth_threshold".to_string(), 5.0),
                ("inheritance_depth_threshold".to_string(), 4.0),
                ("module_size_threshold".to_string(), 50.0),
            ])),
            options: None,
        },
    );

    // Documentation - Good documentation coverage
    presets.insert(
        "documentation".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coverage_threshold".to_string(), 70.0),
                ("quality_threshold".to_string(), 0.7),
            ])),
            options: None,
        },
    );

    // Tests - Good test coverage
    presets.insert(
        "tests".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coverage_ratio_threshold".to_string(), 0.8),
                ("assertions_per_test_min".to_string(), 1.0),
            ])),
            options: None,
        },
    );

    presets
}

/// Get relaxed preset thresholds
///
/// Lenient thresholds for prototypes, legacy code, or early-stage projects.
fn get_relaxed_preset() -> HashMap<String, CategoryConfig> {
    let mut presets = HashMap::new();

    // Quality - Lenient complexity and maintainability
    presets.insert(
        "quality".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("complexity_threshold".to_string(), 20.0),
                ("maintainability_threshold".to_string(), 50.0),
                ("readability_threshold".to_string(), 50.0),
            ])),
            options: None,
        },
    );

    // Dead Code - Relaxed unused code detection
    presets.insert(
        "dead_code".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: None,
            options: None,
        },
    );

    // Dependencies - High tolerance for coupling
    presets.insert(
        "dependencies".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coupling_threshold".to_string(), 0.9),
                ("cohesion_threshold".to_string(), 0.7),
                ("depth_threshold".to_string(), 8.0),
            ])),
            options: None,
        },
    );

    // Structure - Deep hierarchies allowed
    presets.insert(
        "structure".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("hierarchy_depth_threshold".to_string(), 8.0),
                ("inheritance_depth_threshold".to_string(), 6.0),
                ("module_size_threshold".to_string(), 100.0),
            ])),
            options: None,
        },
    );

    // Documentation - Minimal documentation requirements
    presets.insert(
        "documentation".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coverage_threshold".to_string(), 50.0),
                ("quality_threshold".to_string(), 0.5),
            ])),
            options: None,
        },
    );

    // Tests - Basic test coverage
    presets.insert(
        "tests".to_string(),
        CategoryConfig {
            enabled: None,
            thresholds: Some(HashMap::from([
                ("coverage_ratio_threshold".to_string(), 0.5),
                ("assertions_per_test_min".to_string(), 1.0),
            ])),
            options: None,
        },
    );

    presets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_default_preset() {
        let config = AnalysisConfig::default();
        assert_eq!(config.preset, Some("default".to_string()));
        assert!(!config.overrides.is_empty());
    }

    #[test]
    fn test_apply_strict_preset() {
        let mut config = AnalysisConfig {
            preset: None,
            overrides: HashMap::new(),
        };

        config.apply_preset("strict").unwrap();
        assert_eq!(config.preset, Some("strict".to_string()));

        // Check quality thresholds
        let quality_threshold = config.get_threshold("quality", "complexity_threshold");
        assert_eq!(quality_threshold, Some(5.0));

        let maintainability = config.get_threshold("quality", "maintainability_threshold");
        assert_eq!(maintainability, Some(80.0));
    }

    #[test]
    fn test_apply_default_preset() {
        let mut config = AnalysisConfig {
            preset: None,
            overrides: HashMap::new(),
        };

        config.apply_preset("default").unwrap();
        assert_eq!(config.preset, Some("default".to_string()));

        let complexity_threshold = config.get_threshold("quality", "complexity_threshold");
        assert_eq!(complexity_threshold, Some(10.0));
    }

    #[test]
    fn test_apply_relaxed_preset() {
        let mut config = AnalysisConfig {
            preset: None,
            overrides: HashMap::new(),
        };

        config.apply_preset("relaxed").unwrap();
        assert_eq!(config.preset, Some("relaxed".to_string()));

        let complexity_threshold = config.get_threshold("quality", "complexity_threshold");
        assert_eq!(complexity_threshold, Some(20.0));
    }

    #[test]
    fn test_apply_invalid_preset() {
        let mut config = AnalysisConfig {
            preset: None,
            overrides: HashMap::new(),
        };

        let result = config.apply_preset("invalid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::InvalidPreset(_)));
    }

    #[test]
    fn test_get_threshold_from_override() {
        let mut config = AnalysisConfig::default();

        // Add custom override
        config.overrides.insert(
            "quality".to_string(),
            CategoryConfig {
                enabled: None,
                thresholds: Some(HashMap::from([("complexity_threshold".to_string(), 15.0)])),
                options: None,
            },
        );

        let threshold = config.get_threshold("quality", "complexity_threshold");
        assert_eq!(threshold, Some(15.0));
    }

    #[test]
    fn test_get_threshold_missing() {
        let config = AnalysisConfig::default();
        let threshold = config.get_threshold("nonexistent", "some_threshold");
        assert_eq!(threshold, None);
    }

    #[test]
    fn test_is_kind_enabled_default() {
        let config = AnalysisConfig::default();

        // No enabled list means all kinds are enabled
        assert!(config.is_kind_enabled("quality", "complexity"));
        assert!(config.is_kind_enabled("quality", "smells"));
        assert!(config.is_kind_enabled("dead_code", "unused_imports"));
    }

    #[test]
    fn test_is_kind_enabled_with_filter() {
        let mut config = AnalysisConfig::default();

        // Add enabled filter for quality category
        config.overrides.insert(
            "quality".to_string(),
            CategoryConfig {
                enabled: Some(vec!["complexity".to_string(), "smells".to_string()]),
                thresholds: None,
                options: None,
            },
        );

        assert!(config.is_kind_enabled("quality", "complexity"));
        assert!(config.is_kind_enabled("quality", "smells"));
        assert!(!config.is_kind_enabled("quality", "maintainability"));
    }

    #[test]
    fn test_preset_preserves_existing_overrides() {
        let mut config = AnalysisConfig {
            preset: None,
            overrides: HashMap::new(),
        };

        // Add custom override before applying preset
        config.overrides.insert(
            "quality".to_string(),
            CategoryConfig {
                enabled: Some(vec!["complexity".to_string()]),
                thresholds: Some(HashMap::from([("complexity_threshold".to_string(), 99.0)])),
                options: None,
            },
        );

        // Apply preset
        config.apply_preset("default").unwrap();

        // Custom override should be preserved (not overwritten by preset)
        let threshold = config.get_threshold("quality", "complexity_threshold");
        assert_eq!(threshold, Some(99.0));

        let enabled = &config.overrides.get("quality").unwrap().enabled;
        assert_eq!(enabled.as_ref().unwrap(), &vec!["complexity".to_string()]);
    }

    #[test]
    fn test_all_presets_have_all_categories() {
        let strict = get_strict_preset();
        let default = get_default_preset();
        let relaxed = get_relaxed_preset();

        let categories = vec![
            "quality",
            "dead_code",
            "dependencies",
            "structure",
            "documentation",
            "tests",
        ];

        for category in categories {
            assert!(
                strict.contains_key(category),
                "strict preset missing {}",
                category
            );
            assert!(
                default.contains_key(category),
                "default preset missing {}",
                category
            );
            assert!(
                relaxed.contains_key(category),
                "relaxed preset missing {}",
                category
            );
        }
    }

    #[test]
    fn test_load_from_toml_file() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create .codebuddy directory
        let config_dir = workspace_root.join(".codebuddy");
        std::fs::create_dir_all(&config_dir).unwrap();

        // Write test config
        let config_path = config_dir.join("analysis.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "preset = \"strict\"").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "[overrides.quality]").unwrap();
        writeln!(file, "enabled = [\"complexity\"]").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "[overrides.quality.thresholds]").unwrap();
        writeln!(file, "complexity_threshold = 25.0").unwrap();

        // Load config
        let config = AnalysisConfig::load(workspace_root).unwrap();

        // Verify loaded correctly
        assert_eq!(config.preset, Some("strict".to_string()));
        assert!(config.overrides.contains_key("quality"));

        let quality_config = config.overrides.get("quality").unwrap();
        assert_eq!(
            quality_config.enabled.as_ref().unwrap(),
            &vec!["complexity".to_string()]
        );

        let threshold = config.get_threshold("quality", "complexity_threshold");
        assert_eq!(threshold, Some(25.0));
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Don't create config file
        let config = AnalysisConfig::load(workspace_root).unwrap();

        // Should return default config
        assert_eq!(config.preset, Some("default".to_string()));
    }
}
