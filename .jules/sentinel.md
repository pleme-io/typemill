## 2025-05-23 - Path Traversal in Background Workers
**Vulnerability:** The `OperationQueue` worker in `mill-server` executed file operations (create, write, delete, rename) using raw paths from the operation object without validating they were within the project root.
**Learning:** Background workers that process serialized operations are a common bypass for security checks enforced at the API layer. The API layer might validate the request, but if the worker is "dumb" and blindly executes the queued operation, an internal attacker or a buggy component can exploit it.
**Prevention:** Validation must happen at the *execution point* (in the worker), not just at the ingestion point. We introduced `validate_path` in the worker loop to enforce project root containment using `canonicalize` (handling non-existent files correctly).

## 2025-05-23 - Unauthenticated Token Generation Endpoint
**Vulnerability:** The admin server exposed an unauthenticated `/auth/generate-token` endpoint on localhost, allowing anyone with network access to the admin port (e.g., via SSRF) to generate valid JWTs with arbitrary claims.
**Learning:** Convenience endpoints for development can become critical vulnerabilities in production. Exposing sensitive operations like credential issuance over HTTP, even on localhost, bypasses the "physical access" requirement that CLI tools implicitly enforce.
**Prevention:** Move sensitive administrative operations (like token generation, user creation) to CLI commands that require shell access and file system permissions (to read secrets), rather than exposing them via HTTP APIs.
