//! MCP preset management commands

use anyhow::{bail, Result};
use mill_config::config::{AppConfig, ExternalMcpConfig, ExternalMcpServerConfig};
use std::path::Path;

#[cfg(feature = "mcp-proxy")]
use mill_plugin_system::mcp::presets;

/// List available MCP presets
pub fn list_presets() -> Result<()> {
    #[cfg(not(feature = "mcp-proxy"))]
    {
        bail!("MCP proxy feature not enabled. Rebuild with --features mcp-proxy");
    }

    #[cfg(feature = "mcp-proxy")]
    {
        let presets = presets::get_presets();

        println!("Available MCP Presets:\n");
        for (id, preset) in presets {
            println!("  {} - {}", id, preset.description);
        }
        println!("\nUsage: mill mcp add <preset>");

        Ok(())
    }
}

/// Add an MCP preset to config
pub fn add_preset(preset_id: &str) -> Result<()> {
    #[cfg(not(feature = "mcp-proxy"))]
    {
        bail!("MCP proxy feature not enabled. Rebuild with --features mcp-proxy");
    }

    #[cfg(feature = "mcp-proxy")]
    {
        // Get preset
        let preset = presets::get_preset(preset_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Preset '{}' not found. Run 'mill mcp list' to see available presets.",
                preset_id
            )
        })?;

        // Load config
        let config_path = Path::new(".typemill/config.json");
        let mut config = if config_path.exists() {
            AppConfig::load()?
        } else {
            AppConfig::default()
        };

        // Initialize external_mcp if needed
        let external_mcp = config
            .external_mcp
            .get_or_insert_with(|| ExternalMcpConfig { servers: vec![] });

        // Check if already exists
        if external_mcp.servers.iter().any(|s| s.name == preset.id) {
            println!("✓ {} is already configured", preset.name);
            return Ok(());
        }

        // Add preset
        external_mcp.servers.push(ExternalMcpServerConfig {
            name: preset.id.clone(),
            command: preset.command.clone(),
            env: if preset.env.is_empty() {
                None
            } else {
                Some(preset.env.clone())
            },
            auto_start: preset.auto_start,
        });

        // Save config
        config.save(config_path)?;

        println!("✓ Added {} to .typemill/config.json", preset.name);

        Ok(())
    }
}

/// Remove an MCP preset from config
pub fn remove_preset(preset_id: &str) -> Result<()> {
    #[cfg(not(feature = "mcp-proxy"))]
    {
        bail!("MCP proxy feature not enabled. Rebuild with --features mcp-proxy");
    }

    #[cfg(feature = "mcp-proxy")]
    {
        // Load config
        let config_path = Path::new(".typemill/config.json");
        if !config_path.exists() {
            bail!("No configuration file found at .typemill/config.json");
        }

        let mut config = AppConfig::load()?;

        // Check if external_mcp exists
        let external_mcp = config
            .external_mcp
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No MCP servers configured"))?;

        // Find and remove the server
        let initial_len = external_mcp.servers.len();
        external_mcp.servers.retain(|s| s.name != preset_id);

        if external_mcp.servers.len() == initial_len {
            bail!(
                "Preset '{}' is not configured. Run 'mill mcp list' to see available presets.",
                preset_id
            );
        }

        // Save config
        config.save(config_path)?;

        println!("✓ Removed {} from .typemill/config.json", preset_id);

        Ok(())
    }
}

/// Show detailed information about an MCP preset
pub fn info_preset(preset_id: &str) -> Result<()> {
    #[cfg(not(feature = "mcp-proxy"))]
    {
        bail!("MCP proxy feature not enabled. Rebuild with --features mcp-proxy");
    }

    #[cfg(feature = "mcp-proxy")]
    {
        // Get preset
        let preset = presets::get_preset(preset_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Preset '{}' not found. Run 'mill mcp list' to see available presets.",
                preset_id
            )
        })?;

        println!("{}", "=".repeat(60));
        println!("MCP Preset: {}", preset.name);
        println!("{}", "=".repeat(60));
        println!();
        println!("ID:          {}", preset.id);
        println!("Description: {}", preset.description);
        println!("Auto-start:  {}", preset.auto_start);
        println!();
        println!("Command:");
        println!("  {}", preset.command.join(" "));

        if !preset.env.is_empty() {
            println!();
            println!("Environment Variables:");
            for (key, value) in &preset.env {
                println!("  {}={}", key, value);
            }
        }

        println!();

        // Check if installed
        let config_path = Path::new(".typemill/config.json");
        if config_path.exists() {
            if let Ok(config) = AppConfig::load() {
                if let Some(external_mcp) = &config.external_mcp {
                    let is_installed = external_mcp.servers.iter().any(|s| s.name == preset.id);
                    println!(
                        "Status:      {}",
                        if is_installed {
                            "✓ Installed"
                        } else {
                            "✗ Not installed"
                        }
                    );
                    println!();
                    if !is_installed {
                        println!("To install: mill mcp add {}", preset.id);
                    } else {
                        println!("To remove:  mill mcp remove {}", preset.id);
                    }
                } else {
                    println!("Status:      ✗ Not installed");
                    println!();
                    println!("To install: mill mcp add {}", preset.id);
                }
            }
        } else {
            println!("Status:      ✗ Not installed");
            println!();
            println!("To install: mill mcp add {}", preset.id);
        }

        println!();

        Ok(())
    }
}
