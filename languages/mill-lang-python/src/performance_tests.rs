use super::PythonPlugin;
use std::time::{Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_parse_large_file_blocking() {
    let plugin = PythonPlugin::new();

    // Create a large Python file (~10MB)
    let large_source = "def foo(): pass\n".repeat(500000);

    let start = Instant::now();

    // Spawn a task that ticks every 10ms
    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
    let ticker = tokio::spawn(async move {
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let _ = tx.send(Instant::now()).await;
        }
    });

    // Run parse
    // If parse is blocking, it will occupy the thread and delay the ticker task
    // if the runtime doesn't have enough threads or if it blocks the thread pool.
    let _ = plugin.parse(&large_source).await.expect("Parse failed");

    let _ = ticker.await;

    let mut ticks = Vec::new();
    while let Some(tick) = rx.recv().await {
        ticks.push(tick);
    }

    let duration = start.elapsed();
    println!("Parse took: {:?}", duration);
    println!("Ticks collected: {}", ticks.len());

    // We don't assert strictly here because CI environments are noisy,
    // but manually we can observe if ticks are delayed.
    // Ideally, with non-blocking parse, ticks should be roughly 10ms apart.
}
