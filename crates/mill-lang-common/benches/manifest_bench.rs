use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mill_lang_common::manifest_common::TomlWorkspace;
use std::hint::black_box;

fn generate_dependencies(count: usize) -> String {
    let mut s = String::from("[dependencies]\n");
    for i in 0..count {
        s.push_str(&format!("dep_{} = \"1.0.{}\"\n", i, i));
    }
    s
}

fn generate_dev_dependencies(count: usize) -> String {
    let mut s = String::from("[dev-dependencies]\n");
    for i in 0..count {
        s.push_str(&format!("dev_dep_{} = \"2.0.{}\"\n", i, i));
    }
    s
}

fn bench_merge_dependencies(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_dependencies");

    for size in [10, 100, 1_000].iter() {
        let base_toml = generate_dependencies(*size);
        // Source has overlapping dependencies and new ones
        let source_toml = format!(
            "{}\n{}",
            generate_dependencies(*size + size / 2), // Overlap + New
            generate_dev_dependencies(*size)
        );

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_deps", size)),
            size,
            |b, _| {
                b.iter(|| {
                    TomlWorkspace::merge_dependencies(
                        black_box(&base_toml),
                        black_box(&source_toml),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_merge_dependencies);
criterion_main!(benches);
