## 2024-05-22 - SearchHandler Allocations
**Learning:** `SearchHandler` was cloning all search results from plugins and allocating strings for every kind check. `serde_json::Value` arrays should be consumed via pattern matching (`Value::Array(vec)`) rather than cloning (`as_array().unwrap().clone()`) to avoid O(N) deep clones.
**Action:** Inspect other handlers for similar pattern (`.as_array()` followed by `.clone()`) and replace with destructuring.
