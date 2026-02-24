## 2025-05-23 - Path Traversal in Background Workers
**Vulnerability:** The `OperationQueue` worker in `mill-server` executed file operations (create, write, delete, rename) using raw paths from the operation object without validating they were within the project root.
**Learning:** Background workers that process serialized operations are a common bypass for security checks enforced at the API layer. The API layer might validate the request, but if the worker is "dumb" and blindly executes the queued operation, an internal attacker or a buggy component can exploit it.
**Prevention:** Validation must happen at the *execution point* (in the worker), not just at the ingestion point. We introduced `validate_path` in the worker loop to enforce project root containment using `canonicalize` (handling non-existent files correctly).

## 2025-05-24 - Unbounded Decompression in LSP Installation
**Vulnerability:** The `decompress_gzip` utility in `mill-lang-common` used `std::io::copy` to stream compressed data to disk without checking the total output size. This allowed a small malicious GZIP file ("zip bomb") to expand indefinitely, potentially filling the disk and causing a Denial of Service.
**Learning:** Streaming data (e.g., `std::io::copy`) is memory-efficient but can be dangerous if the output size is not bounded. Standard library utilities often lack built-in limits for these operations. Security-critical file operations must enforce explicit resource limits.
**Prevention:** Replaced `std::io::copy` with a manual read/write loop that tracks the total bytes written and aborts if a predefined limit (5GB) is exceeded.
