use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mill_lang_common::manifest_common::TomlWorkspace;
use std::hint::black_box;

fn generate_toml_with_deps(dep_count: usize, start_index: usize) -> String {
    let mut s = String::from("[package]\nname = \"test\"\nversion = \"0.1.0\"\n\n[dependencies]\n");
    for i in 0..dep_count {
        s.push_str(&format!("dep_{} = \"1.0.{}\"\n", start_index + i, i));
    }
    s
}

fn bench_merge_dependencies(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_dependencies");

    for size in [10, 100, 500].iter() {
        // Source has 'size' dependencies.
        // Base has 0 dependencies.
        let base = generate_toml_with_deps(0, 0);
        let source = generate_toml_with_deps(*size, 0);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("merge_new_deps", size), size, |b, _| {
            b.iter(|| {
                TomlWorkspace::merge_dependencies(black_box(&base), black_box(&source)).unwrap()
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_merge_dependencies);
criterion_main!(benches);
