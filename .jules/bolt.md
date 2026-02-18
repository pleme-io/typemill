## 2024-05-22 - SearchHandler Allocations
**Learning:** `SearchHandler` was cloning all search results from plugins and allocating strings for every kind check. `serde_json::Value` arrays should be consumed via pattern matching (`Value::Array(vec)`) rather than cloning (`as_array().unwrap().clone()`) to avoid O(N) deep clones.
**Action:** Inspect other handlers for similar pattern (`.as_array()` followed by `.clone()`) and replace with destructuring.

## 2024-05-27 - EditingToolsHandler Allocations
**Learning:** `EditingToolsHandler` was cloning entire symbol arrays from plugins during workspace search, similar to the `SearchHandler` issue.
**Action:** Always check for `as_array().clone()` patterns when handling `serde_json::Value` responses and replace with pattern matching to consume the value.

## 2024-05-30 - SearchHandler File Scanning Allocations
**Learning:** `SearchHandler::find_representative_files` and `find_files_recursive` were allocating `PathBuf` for every file scanned using `entry.path()`, even if the file was excluded or didn't match the extension. `DirEntry::file_name()` is much cheaper (allocates only the filename) and avoids constructing the full path.
**Action:** Use `entry.file_name()` for filtering files by name or extension during directory traversal, and only call `entry.path()` when a match is found or full path is needed.

## 2024-05-31 - HashSet Insert vs Contains
**Learning:** In `convert_find_replace_response`, attempting to optimize `HashSet<String>::insert` by checking `contains(&str)` first (to avoid `to_string()` allocation) resulted in a 2x slowdown in debug builds (1.2s -> 2.3s). This suggests that for `HashSet<String>`, the cost of hashing and probing twice (once for `contains`, once for `insert`) outweighs the cost of allocating a short string and hashing once, at least in some environments or for small strings.
**Action:** Be cautious when optimizing `HashSet::insert` with `contains`. Benchmark first. `insert` already handles existence checks efficiently.

## 2025-05-19 - File Discovery Allocations
**Learning:** In `discover_importing_files`, `WalkBuilder` results were being converted to `PathBuf` via `.map(|e| e.into_path())` *before* filtering. This caused allocations for every single file in the workspace (including excluded files and directories).
**Action:** Filter `ignore::DirEntry` directly using `entry.file_type()` and `entry.path()` before mapping to `PathBuf`. This avoids allocations for non-matching files.

## 2025-05-19 - Vec::with_capacity and usize::MAX
**Learning:** When using `Vec::with_capacity(limit)` where `limit` comes from external input (like a request), always sanitize the limit. If `limit` is `usize::MAX`, it will panic with "capacity overflow".
**Action:** Use `std::cmp::min(limit, known_upper_bound)` or a safe default cap when pre-allocating vectors based on user input. In pagination, `total.saturating_sub(offset)` provides a safe upper bound.
