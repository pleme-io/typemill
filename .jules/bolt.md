## 2024-05-22 - SearchHandler Allocations
**Learning:** `SearchHandler` was cloning all search results from plugins and allocating strings for every kind check. `serde_json::Value` arrays should be consumed via pattern matching (`Value::Array(vec)`) rather than cloning (`as_array().unwrap().clone()`) to avoid O(N) deep clones.
**Action:** Inspect other handlers for similar pattern (`.as_array()` followed by `.clone()`) and replace with destructuring.

## 2024-05-27 - EditingToolsHandler Allocations
**Learning:** `EditingToolsHandler` was cloning entire symbol arrays from plugins during workspace search, similar to the `SearchHandler` issue.
**Action:** Always check for `as_array().clone()` patterns when handling `serde_json::Value` responses and replace them with pattern matching to consume the value.

## 2024-05-30 - SearchHandler File Scanning Allocations
**Learning:** `SearchHandler::find_representative_files` and `find_files_recursive` were allocating `PathBuf` for every file scanned using `entry.path()`, even if the file was excluded or didn't match the extension. `DirEntry::file_name()` is much cheaper (allocates only the filename) and avoids constructing the full path.
**Action:** Use `entry.file_name()` for filtering files by name or extension during directory traversal, and only call `entry.path()` when a match is found or full path is needed.
