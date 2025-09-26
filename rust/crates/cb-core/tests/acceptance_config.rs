//! Acceptance tests for configuration management
//! These tests verify the public API contract for configuration loading

use cb_core::{AppConfig, CoreError};
use std::env;
use tempfile::TempDir;

fn clean_env() {
    // Remove all CODEFLOW_BUDDY environment variables
    for (key, _) in env::vars() {
        if key.starts_with("CODEFLOW_BUDDY") {
            env::remove_var(key);
        }
    }
}

#[test]
fn test_config_load_default() {
    // Clear any environment variables that might affect the test
    clean_env();

    // Should load with default values when no config file exists
    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = AppConfig::load().unwrap();

    // Verify default values
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 3040);
    assert_eq!(config.server.max_clients, Some(10));
    assert_eq!(config.server.timeout_ms, 30000);
    assert!(config.server.tls.is_none());
    assert!(config.server.auth.is_none());

    assert!(!config.lsp.servers.is_empty());
    assert_eq!(config.lsp.default_timeout_ms, 5000);
    assert!(config.lsp.enable_preload);

    assert!(config.fuse.is_none());

    assert_eq!(config.logging.level, "info");
    assert_eq!(config.logging.format, "pretty");
    assert!(config.logging.file.is_none());

    assert!(config.cache.enabled);
    assert_eq!(config.cache.max_size_bytes, 256 * 1024 * 1024);
    assert_eq!(config.cache.ttl_seconds, 3600);
    assert!(!config.cache.persistent);
    assert!(config.cache.cache_dir.is_none());

    // Clean up
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_config_load_from_json() {
    clean_env();

    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Create .codebuddy directory and config file
    std::fs::create_dir_all(".codebuddy").unwrap();
    let config_content = r#"
{
  "server": {
    "host": "0.0.0.0",
    "port": 3041,
    "maxClients": 50,
    "timeoutMs": 60000
  },
  "lsp": {
    "servers": [
      {
        "extensions": ["rs"],
        "command": ["rust-analyzer"],
        "restartInterval": 15
      }
    ],
    "defaultTimeoutMs": 10000,
    "enablePreload": false
  },
  "logging": {
    "level": "debug",
    "format": "json"
  },
  "cache": {
    "enabled": false,
    "maxSizeBytes": 1024,
    "ttlSeconds": 300,
    "persistent": true,
    "cacheDir": "/tmp/cache"
  }
}
"#;

    std::fs::write(".codebuddy/config.json", config_content).unwrap();

    let config = AppConfig::load().unwrap();

    // Verify loaded values
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 3041);
    assert_eq!(config.server.max_clients, Some(50));
    assert_eq!(config.server.timeout_ms, 60000);

    assert_eq!(config.lsp.servers.len(), 1);
    assert_eq!(config.lsp.servers[0].extensions, vec!["rs"]);
    assert_eq!(config.lsp.servers[0].command, vec!["rust-analyzer"]);
    assert_eq!(config.lsp.servers[0].restart_interval, Some(15));
    assert_eq!(config.lsp.default_timeout_ms, 10000);
    assert!(!config.lsp.enable_preload);

    assert_eq!(config.logging.level, "debug");
    assert_eq!(config.logging.format, "json");

    assert!(!config.cache.enabled);
    assert_eq!(config.cache.max_size_bytes, 1024);
    assert_eq!(config.cache.ttl_seconds, 300);
    assert!(config.cache.persistent);
    assert_eq!(config.cache.cache_dir.unwrap().to_str().unwrap(), "/tmp/cache");

    // Clean up
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_config_env_override() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Set environment variables
    env::set_var("CODEFLOW_BUDDY__SERVER__PORT", "9000");
    env::set_var("CODEFLOW_BUDDY__LOGGING__LEVEL", "error");
    env::set_var("CODEFLOW_BUDDY__CACHE__ENABLED", "false");

    let config = AppConfig::load().unwrap();

    // Environment should override defaults
    assert_eq!(config.server.port, 9000);
    assert_eq!(config.logging.level, "error");
    assert!(!config.cache.enabled);

    // Clean up
    env::remove_var("CODEFLOW_BUDDY__SERVER__PORT");
    env::remove_var("CODEFLOW_BUDDY__LOGGING__LEVEL");
    env::remove_var("CODEFLOW_BUDDY__CACHE__ENABLED");
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_config_validation_invalid_port() {
    clean_env();
    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    std::fs::create_dir_all(".codebuddy").unwrap();
    let config_content = r#"
{
  "server": {
    "host": "127.0.0.1",
    "port": 0,
    "maxClients": 10,
    "timeoutMs": 30000
  },
  "lsp": {
    "servers": [
      {
        "extensions": ["ts"],
        "command": ["typescript-language-server", "--stdio"],
        "restartInterval": 10
      }
    ],
    "defaultTimeoutMs": 5000,
    "enablePreload": true
  },
  "logging": {
    "level": "info",
    "format": "pretty"
  },
  "cache": {
    "enabled": true,
    "maxSizeBytes": 268435456,
    "ttlSeconds": 3600,
    "persistent": false
  }
}
"#;

    std::fs::write(".codebuddy/config.json", config_content).unwrap();

    let result = AppConfig::load();
    assert!(result.is_err());

    let error = result.unwrap_err();
    match error {
        CoreError::Config { message } => {
            assert!(message.contains("Server port cannot be 0"));
        }
        _ => panic!("Expected config error, got: {:?}", error),
    }

    // Clean up
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_config_validation_invalid_log_level() {
    clean_env();
    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    std::fs::create_dir_all(".codebuddy").unwrap();
    let config_content = r#"
{
  "server": {
    "host": "127.0.0.1",
    "port": 3042,
    "maxClients": 10,
    "timeoutMs": 30000
  },
  "lsp": {
    "servers": [
      {
        "extensions": ["ts"],
        "command": ["typescript-language-server", "--stdio"],
        "restartInterval": 10
      }
    ],
    "defaultTimeoutMs": 5000,
    "enablePreload": true
  },
  "logging": {
    "level": "invalid",
    "format": "pretty"
  },
  "cache": {
    "enabled": true,
    "maxSizeBytes": 268435456,
    "ttlSeconds": 3600,
    "persistent": false
  }
}
"#;

    std::fs::write(".codebuddy/config.json", config_content).unwrap();

    let result = AppConfig::load();
    assert!(result.is_err());

    let error = result.unwrap_err();
    match error {
        CoreError::Config { message } => {
            assert!(message.contains("Invalid log level"));
        }
        _ => panic!("Expected config error, got: {:?}", error),
    }

    // Clean up
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_config_validation_empty_lsp_servers() {
    clean_env();
    let temp_dir = TempDir::new().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    std::fs::create_dir_all(".codebuddy").unwrap();
    let config_content = r#"
{
  "server": {
    "host": "127.0.0.1",
    "port": 3042,
    "maxClients": 10,
    "timeoutMs": 30000
  },
  "lsp": {
    "servers": [],
    "defaultTimeoutMs": 5000,
    "enablePreload": true
  },
  "logging": {
    "level": "info",
    "format": "pretty"
  },
  "cache": {
    "enabled": true,
    "maxSizeBytes": 268435456,
    "ttlSeconds": 3600,
    "persistent": false
  }
}
"#;

    std::fs::write(".codebuddy/config.json", config_content).unwrap();

    let result = AppConfig::load();
    assert!(result.is_err());

    let error = result.unwrap_err();
    match error {
        CoreError::Config { message } => {
            assert!(message.contains("At least one LSP server must be configured"));
        }
        _ => panic!("Expected config error, got: {:?}", error),
    }

    // Clean up
    env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_config_serialization_round_trip() {
    let original_config = AppConfig::default();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original_config).unwrap();

    // Deserialize back
    let deserialized_config: AppConfig = serde_json::from_str(&json).unwrap();

    // Should be identical
    assert_eq!(original_config.server.host, deserialized_config.server.host);
    assert_eq!(original_config.server.port, deserialized_config.server.port);
    assert_eq!(original_config.lsp.servers.len(), deserialized_config.lsp.servers.len());
    assert_eq!(original_config.logging.level, deserialized_config.logging.level);
    assert_eq!(original_config.cache.enabled, deserialized_config.cache.enabled);
}