# Performance Audit Report

This report details performance issues identified across the `mill` codebase, specifically within `crates/` and `languages/`. The analysis focuses on blocking I/O in async contexts, sequential processing bottlenecks, algorithmic inefficiencies, and excessive memory usage.

## Executive Summary

The most critical widespread issue is the use of **blocking I/O (`std::fs`) inside asynchronous functions**. This blocks the Tokio runtime threads, severely degrading throughput for concurrent requests. Additionally, several core services (Reference Updater, File Discovery) use **sequential processing** for tasks that are embarrassingly parallel, and some algorithms scale linearly (O(N)) or quadratically (O(N²)) with project size, which will cause timeouts on large repositories.

## Critical Issues (High Severity)

| File | Issue Type | Description | Recommendation |
|------|------------|-------------|----------------|
| `crates/mill-handlers/src/handlers/search_handler.rs` | Blocking I/O | `find_representative_file` and `find_file_recursive` use `std::fs::read_dir` inside async methods. | Switch to `tokio::fs::read_dir` and `tokio::fs::metadata`. |
| `languages/mill-lang-rust/src/reference_detector.rs` | Blocking I/O | `find_affected_files` calls `std::fs::canonicalize` and `std::fs::read_to_string` inside loops. | Use `tokio::fs` equivalents. Canonicalization should be async or cached. |
| `languages/mill-lang-typescript/src/tsconfig.rs` | Blocking I/O | `TsConfig::from_file` and `load_and_merge_recursive` use `std::fs` calls. This blocks the runtime during plugin initialization or request handling. | Make these methods `async` and use `tokio::fs`. |
| `crates/mill-services/src/services/filesystem/file_service/basic_ops.rs` | Blocking I/O | `to_absolute_path_checked` and `list_files_recursive` use synchronous file operations. | Use `tokio::fs`. |
| `crates/mill-foundation/src/core/utils/system.rs` | Blocking Process | `test_command_with_version` uses `std::process::Command` which blocks the thread. | Switch to `tokio::process::Command`. |
| `crates/mill-handlers/src/handlers/inspect_handler.rs` | Sequential Async | `aggregate_intelligence` awaits tasks in a loop (`for task in tasks { task.await }`), executing them serially instead of concurrently. | Use `futures::future::join_all(tasks).await` to run LSP queries in parallel. |

## Major Issues (Medium Severity)

| File | Issue Type | Description | Recommendation |
|------|------------|-------------|----------------|
| `crates/mill-services/src/services/reference_updater/mod.rs` | Sequential Processing | `update_references` iterates affected files and awaits `plugin.rewrite_file_references` sequentially. | Use `futures::stream::iter(..).for_each_concurrent` to process files in parallel. |
| `crates/mill-services/src/services/reference_updater/mod.rs` | O(N) Algorithm | `find_project_files` and `find_affected_files` iterate through all files sequentially reading content. | Parallelize file reading. Use search index or `ripgrep`-like strategy if possible. |
| `languages/mill-lang-typescript/src/imports.rs` | Excessive Parsing | `remove_named_import_from_line` initializes a full SWC parser/compiler *per line* of code processed. | Reuse `SourceMap` and parser instances. Use regex for simple line removals if AST is overkill. |
| `languages/mill-lang-typescript/src/imports.rs` | Data Loss / Perf | `update_import_reference_ast` parses and re-emits the entire file to change one string, potentially losing comments (emitter `comments: None`). | Use a more targeted replacement (AST-guided span replacement) or ensure comments are preserved. |
| `crates/mill-handlers/src/handlers/prune_handler.rs` | O(N²) Complexity | `extract_files_from_doc_changes` uses `Vec::contains` inside a loop for deduplication. | Use `HashSet` for O(1) lookups during collection, then convert to Vec. |

## Minor Issues (Low Severity)

| File | Issue Type | Description | Recommendation |
|------|------------|-------------|----------------|
| `crates/mill-handlers/src/handlers/inspect_handler.rs` | Memory / Cloning | `apply_pagination` clones the entire result set (`Vec<Value>`) before slicing. | Slice the reference or iterator before collecting/cloning. |
| `crates/mill-handlers/src/handlers/rename_all_handler.rs` | Memory | `ToolCall.arguments.clone()` copies potentially large JSON payloads. | Pass references where possible. |
| `crates/mill-handlers/src/handlers/search_handler.rs` | Memory | `all_symbols.extend(symbols.clone())` copies large symbol arrays. | Use `append` or extend with owned iterator if possible to avoid cloning. |

## Detailed File Analysis

### `crates/mill-handlers/src/handlers/`

*   **`search_handler.rs`**:
    *   **Blocking I/O**: `find_file_recursive` uses `std::fs`. This is the most dangerous function here as it traverses directories on the async thread.
    *   **Sequential Plugin Query**: `search_workspace_symbols` loops `for plugin_name in plugin_names` and awaits. This makes search latency sum(plugins) instead of max(plugins).

*   **`inspect_handler.rs`**:
    *   **Sequential Await**: The loop `for task in tasks { task.await }` defeats the purpose of spawning futures if they are not joined concurrently.
    *   **Inefficient Symbol Resolution**: `resolve_symbol_position` iterates linearly over all document symbols.

*   **`prune_handler.rs`**:
    *   **Inefficient Deduplication**: `!files.contains(&uri)` on a `Vec` is O(N). Inside a loop of M items, this is O(M*N). For large refactors, this adds up.

### `crates/mill-services/src/services/`

*   **`reference_updater/mod.rs`**:
    *   **Scalability**: The `ReferenceUpdater` relies on `tokio::fs::read_to_string` for *every* candidate file in `find_affected_files`. This is O(N) disk I/O. For a repo with 10k files, this will be very slow.
    *   **Sequential Writes**: `update_references` writes files one by one.

*   **`filesystem/file_service/basic_ops.rs`**:
    *   **Blocking Recursion**: `list_files_recursive` is a recursive async function but it processes directories sequentially.
    *   **Synchronous Checks**: `path.exists()` (std) is used in some places instead of `tokio::fs::try_exists`.

### `languages/`

*   **`mill-lang-rust/src/reference_detector.rs`**:
    *   **Blocking Canonicalization**: `project_root.canonicalize()` is a syscall that blocks.
    *   **Heavy I/O Loop**: The loop over `project_files` reads every file to check for imports. This is the same scalability issue as `ReferenceUpdater` but specific to Rust crate detection.

*   **`mill-lang-typescript/src/imports.rs`**:
    *   **Parser Overhead**: `remove_named_import_from_line` is extremely heavyweight. It spins up a full TypeScript parser infrastructure for a string manipulation task.

*   **`mill-lang-typescript/src/tsconfig.rs`**:
    *   **Blocking Config Load**: `TsConfig::from_file` uses `std::fs::read_to_string`. This is likely called during plugin initialization or request handling, causing hiccups.
