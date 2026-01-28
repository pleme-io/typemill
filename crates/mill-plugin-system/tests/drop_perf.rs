use mill_plugin_system::mcp::ExternalMcpClient;
use std::time::Instant;

#[test]
fn measure_drop_latency_outside_runtime() {
    // 1. Setup client within a runtime
    let client = {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // "sleep 5" ensures process outlives the spawn block
            // We use "sleep" assuming it is available in the environment (standard unix)
            ExternalMcpClient::spawn(
                "test_server".to_string(),
                vec!["sleep".to_string(), "5".to_string()],
            )
            .await
            .unwrap()
        })
    };

    // 2. Runtime is dropped now. Handle::try_current() should fail.

    let start = Instant::now();
    drop(client);
    let duration = start.elapsed();

    println!("Drop took: {:.2?}", duration);

    assert!(
        duration.as_millis() < 10,
        "Expected drop to take less than 10ms with optimized implementation"
    );
}
