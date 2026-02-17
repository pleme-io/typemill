use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::fs;
use tempfile::tempdir;
use tokio::runtime::Runtime;

fn bench_file_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_write");
    let rt = Runtime::new().unwrap();
    let data = vec![0u8; 1024 * 1024]; // 1MB
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("std_fs_write", |b| {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_std.bin");
        b.to_async(&rt).iter(|| async {
            fs::write(&path, &data).unwrap();
        });
    });

    group.bench_function("tokio_fs_write", |b| {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_tokio.bin");
        b.to_async(&rt).iter(|| async {
            tokio::fs::write(&path, &data).await.unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_file_write);
criterion_main!(benches);
