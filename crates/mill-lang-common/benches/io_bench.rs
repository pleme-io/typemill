use criterion::{criterion_group, criterion_main, Criterion};
use mill_lang_common::io::find_source_files;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_test_files(root: &Path, depth: usize, width: usize) {
    if depth == 0 {
        return;
    }

    for i in 0..width {
        let dir_path = root.join(format!("dir_{}", i));
        fs::create_dir(&dir_path).unwrap();

        for j in 0..width {
            fs::write(dir_path.join(format!("file_{}.rs", j)), "content").unwrap();
            fs::write(dir_path.join(format!("file_{}.txt", j)), "content").unwrap();
        }

        create_test_files(&dir_path, depth - 1, width);
    }
}

async fn run_find_source_files(dir: &Path) {
    let _ = find_source_files(dir, &["rs"]).await.unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    // Create a structure: depth 3, width 5
    // Level 1: 5 dirs
    // Level 2: 25 dirs
    // Level 3: 125 dirs
    // Total dirs: 155
    // Files per dir: 10 (5 .rs, 5 .txt)
    // Total files: 1550
    create_test_files(temp_dir.path(), 3, 5);

    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("find_source_files", |b| {
        b.to_async(&rt)
            .iter(|| run_find_source_files(temp_dir.path()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
