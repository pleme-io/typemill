//! Performance benchmarks for import helper primitives
//!
//! These benchmarks measure the performance characteristics of core import
//! manipulation functions across different input sizes and patterns.
//!
//! # Benchmark Groups
//!
//! 1. **find_last_matching_line** - Search performance (100, 1K, 10K, 100K lines)
//! 2. **insert_line_at** - Insertion performance (beginning, middle, end)
//! 3. **remove_lines_matching** - Removal performance (various match rates)
//! 4. **replace_in_lines** - Replacement performance (various densities)
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench -p cb-lang-common
//! ```
//!
//! # Performance Targets
//!
//! - **find_last_matching_line**: < 1ms for 10K lines
//! - **insert_line_at**: < 100us for 1K lines
//! - **remove_lines_matching**: < 500us for 1K lines
//! - **replace_in_lines**: < 1ms for 1K lines with 100 replacements

use cb_lang_common::import_helpers::*;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// ============================================================================
// Test Data Generation
// ============================================================================

/// Generate file content with specified number of lines
fn generate_file(lines: usize, pattern: &str) -> String {
    (0..lines)
        .map(|i| format!("{} line_{}", pattern, i))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate import-heavy file (realistic for refactoring scenarios)
fn generate_import_file(total_lines: usize, import_ratio: f32) -> String {
    (0..total_lines)
        .map(|i| {
            if (i as f32) / (total_lines as f32) < import_ratio {
                format!("import module_{}", i)
            } else {
                format!("class Class_{} {{}}", i)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate mixed content with realistic patterns
fn generate_realistic_file(lines: usize) -> String {
    let mut content = Vec::new();

    // Header comments
    for i in 0..3 {
        content.push(format!("// Header comment {}", i));
    }

    // Imports (10% of file)
    let import_count = lines / 10;
    for i in 0..import_count {
        content.push(format!("import module_{};", i));
    }

    content.push(String::new()); // Blank line

    // Code (rest of file)
    let code_count = lines - import_count - 4;
    for i in 0..code_count {
        if i % 5 == 0 {
            content.push(format!("class Class_{} {{", i));
        } else if i % 5 == 4 {
            content.push("}".to_string());
        } else {
            content.push(format!("    fn method_{}() {{}}", i));
        }
    }

    content.join("\n")
}

// ============================================================================
// Benchmark: find_last_matching_line
// ============================================================================

fn bench_find_last_matching_line(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_last_matching_line");

    // Benchmark across different file sizes
    for size in [100, 1_000, 10_000, 100_000].iter() {
        let content = generate_file(*size, "import");

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_lines", size)),
            size,
            |b, _| {
                b.iter(|| {
                    find_last_matching_line(black_box(&content), |line| line.starts_with("import"))
                });
            },
        );
    }

    // Benchmark with different match densities
    let content_sparse = generate_import_file(10_000, 0.1); // 10% matches
    let content_dense = generate_import_file(10_000, 0.5); // 50% matches

    group.bench_function("10K_lines_10%_matches", |b| {
        b.iter(|| {
            find_last_matching_line(black_box(&content_sparse), |line| {
                line.starts_with("import")
            })
        });
    });

    group.bench_function("10K_lines_50%_matches", |b| {
        b.iter(|| {
            find_last_matching_line(black_box(&content_dense), |line| line.starts_with("import"))
        });
    });

    // Benchmark worst case (no matches - full scan)
    let content_no_match = generate_file(10_000, "code");
    group.bench_function("10K_lines_no_matches", |b| {
        b.iter(|| {
            find_last_matching_line(black_box(&content_no_match), |line| {
                line.starts_with("import")
            })
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: insert_line_at
// ============================================================================

fn bench_insert_line_at(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_line_at");

    // Benchmark at different positions
    for size in [100, 1_000, 10_000].iter() {
        let content = generate_file(*size, "code");

        // Insert at beginning
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("beginning", size), size, |b, _| {
            b.iter(|| insert_line_at(black_box(&content), 0, "NEW_IMPORT"));
        });

        // Insert at middle
        group.bench_with_input(BenchmarkId::new("middle", size), size, |b, _| {
            b.iter(|| insert_line_at(black_box(&content), size / 2, "NEW_IMPORT"));
        });

        // Insert at end
        group.bench_with_input(BenchmarkId::new("end", size), size, |b, _| {
            b.iter(|| insert_line_at(black_box(&content), *size, "NEW_IMPORT"));
        });
    }

    // Benchmark with CRLF line endings
    let content_crlf = generate_file(1_000, "code").replace("\n", "\r\n");
    group.bench_function("1K_lines_CRLF", |b| {
        b.iter(|| insert_line_at(black_box(&content_crlf), 500, "NEW_IMPORT"));
    });

    group.finish();
}

// ============================================================================
// Benchmark: remove_lines_matching
// ============================================================================

fn bench_remove_lines_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove_lines_matching");

    // Benchmark removing different percentages
    for (ratio, label) in [(0.1, "10%"), (0.25, "25%"), (0.5, "50%"), (0.75, "75%")].iter() {
        let content = generate_import_file(1_000, *ratio);

        group.throughput(Throughput::Elements(1_000));
        group.bench_function(format!("1K_lines_remove_{}", label), |b| {
            b.iter(|| {
                remove_lines_matching(black_box(&content), |line| line.starts_with("import"))
            });
        });
    }

    // Benchmark removing all lines (worst case)
    let content = generate_file(1_000, "import");
    group.bench_function("1K_lines_remove_all", |b| {
        b.iter(|| remove_lines_matching(black_box(&content), |_| true));
    });

    // Benchmark removing none (best case)
    group.bench_function("1K_lines_remove_none", |b| {
        b.iter(|| remove_lines_matching(black_box(&content), |_| false));
    });

    // Benchmark with realistic pattern
    let content_realistic = generate_realistic_file(1_000);
    group.bench_function("1K_lines_realistic_pattern", |b| {
        b.iter(|| {
            remove_lines_matching(black_box(&content_realistic), |line| {
                line.trim().starts_with("import")
            })
        });
    });

    // Benchmark different file sizes
    for size in [100, 1_000, 10_000].iter() {
        let content = generate_import_file(*size, 0.2);
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_lines", size)),
            size,
            |b, _| {
                b.iter(|| {
                    remove_lines_matching(black_box(&content), |line| line.starts_with("import"))
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark: replace_in_lines
// ============================================================================

fn bench_replace_in_lines(c: &mut Criterion) {
    let mut group = c.benchmark_group("replace_in_lines");

    // Benchmark different replacement densities
    for size in [100, 1_000, 10_000].iter() {
        let content = generate_file(*size, "old_module");

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_lines", size)),
            size,
            |b, _| {
                b.iter(|| replace_in_lines(black_box(&content), "old_module", "new_module"));
            },
        );
    }

    // Benchmark no replacements (early exit optimization test)
    let content_no_match = generate_file(1_000, "code");
    group.bench_function("1K_lines_no_matches", |b| {
        b.iter(|| replace_in_lines(black_box(&content_no_match), "import", "use"));
    });

    // Benchmark multiple replacements per line
    let content_multi = (0..1_000)
        .map(|i| format!("old old old line_{}", i))
        .collect::<Vec<_>>()
        .join("\n");
    group.bench_function("1K_lines_3x_per_line", |b| {
        b.iter(|| replace_in_lines(black_box(&content_multi), "old", "new"));
    });

    // Benchmark realistic import renaming
    let content_imports = (0..1_000)
        .map(|i| {
            if i % 10 == 0 {
                format!("import {{ Foo }} from 'old-package';")
            } else {
                format!("class Class_{} {{}}", i)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    group.bench_function("1K_lines_realistic_import_rename", |b| {
        b.iter(|| replace_in_lines(black_box(&content_imports), "old-package", "new-package"));
    });

    // Benchmark short vs long replacement strings
    let content = generate_file(1_000, "x");
    group.bench_function("1K_lines_short_to_short", |b| {
        b.iter(|| replace_in_lines(black_box(&content), "x", "y"));
    });

    let content = generate_file(1_000, "x");
    group.bench_function("1K_lines_short_to_long", |b| {
        b.iter(|| replace_in_lines(black_box(&content), "x", "very_long_replacement_string"));
    });

    group.finish();
}

// ============================================================================
// Benchmark: Combined Operations (Realistic Workflows)
// ============================================================================

fn bench_combined_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_operations");

    let content = generate_realistic_file(1_000);

    // Workflow: Find last import, insert new import
    group.bench_function("find_and_insert_import", |b| {
        b.iter(|| {
            let idx = find_last_matching_line(black_box(&content), |line| {
                line.trim().starts_with("import")
            });
            if let Some(idx) = idx {
                insert_line_at(black_box(&content), idx + 1, "import new_module;")
            } else {
                content.clone()
            }
        });
    });

    // Workflow: Remove old imports, replace module name
    group.bench_function("remove_and_replace_imports", |b| {
        b.iter(|| {
            let (result, _) =
                remove_lines_matching(black_box(&content), |line| line.contains("deprecated_"));
            replace_in_lines(&result, "old_module", "new_module")
        });
    });

    // Workflow: Full refactoring pipeline
    group.bench_function("full_refactoring_pipeline", |b| {
        b.iter(|| {
            // 1. Remove deprecated imports
            let (step1, _) =
                remove_lines_matching(black_box(&content), |line| line.contains("deprecated"));

            // 2. Replace old module name
            let (step2, _) = replace_in_lines(&step1, "module_0", "core_module");

            // 3. Find last import and add new one
            let idx = find_last_matching_line(&step2, |line| line.trim().starts_with("import"));

            if let Some(idx) = idx {
                insert_line_at(&step2, idx + 1, "import feature_module;")
            } else {
                step2
            }
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: Edge Cases and Stress Tests
// ============================================================================

fn bench_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_cases");

    // Very long single line
    let very_long_line = "import ".to_string() + &"a".repeat(100_000);
    group.bench_function("single_very_long_line_100K_chars", |b| {
        b.iter(|| {
            find_last_matching_line(black_box(&very_long_line), |line| {
                line.starts_with("import")
            })
        });
    });

    // Many short lines
    let many_short_lines = (0..100_000)
        .map(|i| format!("i{}", i))
        .collect::<Vec<_>>()
        .join("\n");
    group.bench_function("100K_very_short_lines", |b| {
        b.iter(|| {
            find_last_matching_line(black_box(&many_short_lines), |line| {
                line.starts_with("import")
            })
        });
    });

    // Empty lines
    let empty_lines = "\n".repeat(10_000);
    group.bench_function("10K_empty_lines", |b| {
        b.iter(|| find_last_matching_line(black_box(&empty_lines), |line| !line.is_empty()));
    });

    // Unicode content
    let unicode_content = (0..1_000)
        .map(|i| format!("�e !W_{}", i))
        .collect::<Vec<_>>()
        .join("\n");
    group.bench_function("1K_lines_unicode", |b| {
        b.iter(|| find_last_matching_line(black_box(&unicode_content), |line| line.contains("�e")));
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_find_last_matching_line,
    bench_insert_line_at,
    bench_remove_lines_matching,
    bench_replace_in_lines,
    bench_combined_operations,
    bench_edge_cases,
);

criterion_main!(benches);
