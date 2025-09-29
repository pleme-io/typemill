use cb_core::config::AppConfig;
use std::fs;

#[test]
fn test_app_config_contract() {
    let fixture_path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/app_config.json");
    let fixture_content =
        fs::read_to_string(fixture_path).expect("Failed to read app_config.json fixture");

    let config: AppConfig = serde_json::from_str(&fixture_content)
        .expect("Failed to deserialize app_config.json into AppConfig");

    // Verify key fields
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.max_clients, Some(10));
    assert_eq!(config.server.timeout_ms, 120000);

    // Verify auth config
    let auth = config.server.auth.as_ref().expect("auth should be present");
    assert_eq!(auth.jwt_secret, "test-secret");
    assert_eq!(auth.jwt_expiry_seconds, 86400);
    assert_eq!(auth.jwt_issuer, "codebuddy");
    assert_eq!(auth.jwt_audience, "codeflow-clients");

    // Verify LSP config
    assert_eq!(config.lsp.servers.len(), 2);
    assert_eq!(config.lsp.default_timeout_ms, 30000);
    assert!(config.lsp.enable_preload);

    // Verify first LSP server
    let ts_server = &config.lsp.servers[0];
    assert_eq!(ts_server.extensions, vec!["ts", "tsx", "js", "jsx"]);
    assert_eq!(
        ts_server.command,
        vec!["typescript-language-server", "--stdio"]
    );
    assert_eq!(ts_server.restart_interval, Some(5));

    // Verify second LSP server
    let py_server = &config.lsp.servers[1];
    assert_eq!(py_server.extensions, vec!["py"]);
    assert_eq!(py_server.command, vec!["pylsp"]);
    assert_eq!(py_server.restart_interval, Some(5));

    // Verify optional FUSE config
    if let Some(fuse) = &config.fuse {
        assert_eq!(fuse.mount_point.to_str().unwrap(), "/tmp/codeflow");
        assert!(!fuse.read_only);
        assert_eq!(fuse.cache_timeout_seconds, 60);
        assert_eq!(fuse.max_file_size_bytes, 10485760);
    }

    // Verify logging config
    assert_eq!(config.logging.level, "info");
    assert_eq!(config.logging.format, "json");
    if let Some(file) = &config.logging.file {
        assert_eq!(file.path.to_str().unwrap(), "/tmp/codeflow.log");
        assert_eq!(file.max_size_bytes, 10485760);
        assert_eq!(file.max_files, 3);
    }

    // Verify cache config
    assert!(config.cache.enabled);
    assert_eq!(config.cache.ttl_seconds, 3600);
    assert_eq!(config.cache.max_size_bytes, 104857600);
}
