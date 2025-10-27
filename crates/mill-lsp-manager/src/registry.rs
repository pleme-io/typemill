//! LSP registry loading and querying

use crate::error::{LspError, Result};
use crate::types::{LspConfig, LspRegistry, Platform, PlatformConfig};
use tracing::debug;

/// Load the LSP registry from the embedded TOML file
pub fn load_registry() -> Result<LspRegistry> {
    // Registry is embedded at compile time
    const REGISTRY_TOML: &str = include_str!("../lsp-registry.toml");

    let registry: LspRegistry = toml::from_str(REGISTRY_TOML)
        .map_err(|e| LspError::RegistryLoadFailed(e.to_string()))?;

    debug!(
        "Loaded registry with {} LSP servers",
        registry.lsp.len()
    );

    Ok(registry)
}

impl LspRegistry {
    /// Get LSP configuration by name
    pub fn get(&self, name: &str) -> Option<&LspConfig> {
        self.lsp.get(name)
    }

    /// Find LSP server for a given language
    pub fn find_by_language(&self, language: &str) -> Vec<(&String, &LspConfig)> {
        self.lsp
            .iter()
            .filter(|(_, config)| {
                config
                    .languages
                    .iter()
                    .any(|lang| lang.eq_ignore_ascii_case(language))
            })
            .collect()
    }

    /// Get all available LSP server names
    pub fn list_all(&self) -> Vec<&String> {
        self.lsp.keys().collect()
    }
}

impl LspConfig {
    /// Get platform-specific configuration for the current platform
    pub fn get_platform_config(&self, platform: &Platform) -> Result<&PlatformConfig> {
        self.platform
            .iter()
            .find(|p| p.os == platform.os && p.arch == platform.arch)
            .ok_or_else(|| LspError::PlatformNotSupported {
                os: platform.os.clone(),
                arch: platform.arch.clone(),
            })
    }

    /// Check if runtime dependency is available
    pub fn check_runtime(&self) -> Result<()> {
        if let Some(runtime) = &self.runtime_required {
            if !command_exists(runtime) {
                return Err(LspError::RuntimeNotFound(runtime.clone()));
            }
        }
        Ok(())
    }

    /// Test if LSP is already installed in system PATH
    pub fn test_system_install(&self) -> bool {
        if !command_exists(&self.command) {
            return false;
        }

        // Try running with test args
        if !self.test_args.is_empty() {
            match std::process::Command::new(&self.command)
                .args(&self.test_args)
                .output()
            {
                Ok(output) => output.status.success(),
                Err(_) => false,
            }
        } else {
            true
        }
    }
}

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_registry() {
        let registry = load_registry().expect("Failed to load registry");
        assert!(!registry.lsp.is_empty(), "Registry should not be empty");
    }

    #[test]
    fn test_find_rust_analyzer() {
        let registry = load_registry().unwrap();
        let rust_lsps = registry.find_by_language("rust");
        assert!(!rust_lsps.is_empty(), "Should find rust-analyzer");
    }

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();
        assert!(!platform.os.is_empty());
        assert!(!platform.arch.is_empty());
    }
}
