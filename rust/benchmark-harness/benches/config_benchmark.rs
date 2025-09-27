use cb_client::config::{ClientConfig, ConnectionConfig, DisplayConfig};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::env;
use std::fs;
use tempfile::TempDir;

fn create_config_file(dir: &TempDir, config_size: usize) -> String {
    let config_path = dir.path().join("config.json");

    let mut extra_fields = String::new();
    for i in 0..config_size {
        extra_fields.push_str(&format!(r#", "field_{}": "value_{}""#, i, i));
    }

    let config_content = format!(
        r#"{{
            "url": "ws://localhost:3000",
            "auth_token": "benchmark-token-12345",
            "timeout_ms": 5000,
            "auto_reconnect": true,
            "no_color": false,
            "no_emoji": false{}
        }}"#,
        extra_fields
    );

    fs::write(&config_path, config_content).unwrap();
    config_path.to_string_lossy().into_owned()
}

fn setup_environment_variables() {
    env::set_var("CODEFLOW_BUDDY_URL", "ws://env-server:4000");
    env::set_var("CODEFLOW_BUDDY_TOKEN", "env-token-67890");
    env::set_var("CODEFLOW_BUDDY_TIMEOUT", "10000");
}

fn cleanup_environment_variables() {
    env::remove_var("CODEFLOW_BUDDY_URL");
    env::remove_var("CODEFLOW_BUDDY_TOKEN");
    env::remove_var("CODEFLOW_BUDDY_TIMEOUT");
}

fn bench_load_config_file_only(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir, 0);

    c.bench_function("load_config_file_only", |b| {
        b.iter(|| {
            let _ = ClientConfig::load_with_path(black_box(Some(&config_path)), false, false);
        });
    });
}

fn bench_load_config_with_env(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir, 0);
    setup_environment_variables();

    c.bench_function("load_config_with_env", |b| {
        b.iter(|| {
            let _ = ClientConfig::load_with_path(black_box(Some(&config_path)), false, false);
        });
    });

    cleanup_environment_variables();
}

fn bench_load_config_with_overrides(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir, 0);
    setup_environment_variables();

    c.bench_function("load_config_with_all_overrides", |b| {
        b.iter(|| {
            let mut config = ClientConfig::load_with_path(
                black_box(Some(&config_path)),
                false,
                false,
            )
            .unwrap();

            // Simulate command-line overrides
            config.connection.url = Some("ws://cli-override:5000".to_string());
            config.connection.auth_token = Some("cli-token-override".to_string());
            config.connection.timeout_ms = Some(15000);
            config.display.no_color = true;
            config.display.no_emoji = true;

            black_box(config);
        });
    });

    cleanup_environment_variables();
}

fn bench_load_config_various_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_config_sizes");

    for size in [0, 10, 50, 100, 500].iter() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_config_file(&temp_dir, *size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let _ = ClientConfig::load_with_path(
                    black_box(Some(&config_path)),
                    false,
                    false,
                );
            });
        });
    }

    group.finish();
}

fn bench_config_precedence_resolution(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir, 10);

    c.bench_function("config_precedence_full_chain", |b| {
        // Set up the full precedence chain
        setup_environment_variables();

        b.iter(|| {
            let mut config = ClientConfig::load_with_path(
                black_box(Some(&config_path)),
                false,
                false,
            )
            .unwrap();

            // Apply CLI overrides (highest precedence)
            if let Some(url) = black_box(Some("ws://final:9000")) {
                config.connection.url = Some(url.to_string());
            }
            if let Some(token) = black_box(Some("final-token")) {
                config.connection.auth_token = Some(token.to_string());
            }
            if let Some(timeout) = black_box(Some(20000u64)) {
                config.connection.timeout_ms = Some(timeout);
            }

            // Access the resolved values to ensure they're computed
            let _ = config.connection.resolve_url();
            let _ = config.connection.resolve_auth_token();
            let _ = config.connection.resolve_timeout();

            black_box(config);
        });

        cleanup_environment_variables();
    });
}

fn bench_config_serialization(c: &mut Criterion) {
    let config = ClientConfig {
        connection: ConnectionConfig {
            url: Some("ws://bench:3000".to_string()),
            auth_token: Some("bench-token".to_string()),
            timeout_ms: Some(5000),
            auto_reconnect: Some(true),
        },
        display: DisplayConfig {
            no_color: false,
            no_emoji: false,
        },
    };

    c.bench_function("config_to_json", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&config));
        });
    });

    c.bench_function("config_to_json_pretty", |b| {
        b.iter(|| {
            let _ = serde_json::to_string_pretty(black_box(&config));
        });
    });
}

fn bench_home_dir_resolution(c: &mut Criterion) {
    c.bench_function("resolve_home_config_path", |b| {
        b.iter(|| {
            // Simulate the path resolution that happens in ClientConfig::load()
            let home = dirs::home_dir();
            if let Some(home) = home {
                let config_dir = home.join(".codeflow-buddy");
                let config_path = config_dir.join("config.json");
                black_box(config_path);
            }
        });
    });
}

fn bench_config_validation(c: &mut Criterion) {
    c.bench_function("validate_config_values", |b| {
        let config = ClientConfig {
            connection: ConnectionConfig {
                url: Some("ws://localhost:3000".to_string()),
                auth_token: Some("valid-token-123".to_string()),
                timeout_ms: Some(5000),
                auto_reconnect: Some(true),
            },
            display: DisplayConfig {
                no_color: false,
                no_emoji: false,
            },
        };

        b.iter(|| {
            // Simulate validation checks
            let url = config.connection.resolve_url();
            let is_valid_url = url.map(|u| u.starts_with("ws://") || u.starts_with("wss://"));

            let timeout = config.connection.resolve_timeout();
            let is_valid_timeout = timeout > 0 && timeout <= 60000;

            let token = config.connection.resolve_auth_token();
            let has_token = token.is_some();

            black_box((is_valid_url, is_valid_timeout, has_token));
        });
    });
}

criterion_group!(
    benches,
    bench_load_config_file_only,
    bench_load_config_with_env,
    bench_load_config_with_overrides,
    bench_load_config_various_sizes,
    bench_config_precedence_resolution,
    bench_config_serialization,
    bench_home_dir_resolution,
    bench_config_validation
);
criterion_main!(benches);