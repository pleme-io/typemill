# Backend Multi-Tenancy Implementation

## Overview

The CodeBuddy server can manage remote workspace containers, execute commands within them, and perform complex AST-aware refactorings on their files.

**Critical Security Gap**: The current implementation has no user isolation - all clients can see and access all workspaces.

This proposal implements multi-tenancy with scoped access to enable secure multi-user operation.

**Files to Modify**: 15 files (11 existing + 1 new + 3 docs)

## Problem Analysis

### Multi-Tenancy & Scoped Access (CRITICAL)

**Problem:** Global state - all clients see all workspaces. Security gap prevents production deployment.

**Solution:**
- Add `user_id` claim to JWT authentication
- Partition `WorkspaceManager` by user
- Scope all workspace operations to authenticated user

**Assessment:**
- Essential for production deployment
- Security critical - prevents unauthorized access
- Clean implementation - JWT already supports claims

### LSP Architecture Refactoring (DEFERRED)

**Original Problem:** LSP servers in main container "bloat" the image and prevent workspace-specific versions.

**Analysis:**
- ❌ Network latency: LSP over TCP adds overhead (completions/hovers on every keystroke)
- ❌ Reconnection complexity: Container restarts break LSP connections
- ❌ Resource multiplication: Each workspace needs own LSP instance
- ✅ Image bloat: Weak argument (~10-50MB total, solvable with multi-stage builds)
- ✅ Local stdio: Current approach is simple, fast, proven

**Recommendation:** Keep LSP servers local. Complexity outweighs benefits.

### State Persistence (NEEDS REDESIGN)

**Original Problem:** Server restart loses workspace registrations (in-memory `DashMap`).

**Proposed Solution:** SQLite database for workspace persistence.

**Analysis:**
- Partial solution: Saves workspace *registration* but not container *state*
- Container lifecycle gap: Docker containers are ephemeral by default
  - Code persists (volumes) ✅
  - Container restarts → agent process dies → needs re-registration anyway ❌
- Limited value: DB helps with *server* restarts, not *container* restarts

**Alternative Approach:**
- Rethink container lifecycle: long-lived vs on-demand recreation
- Consider container orchestration (health checks, auto-restart)
- If containers are ephemeral, embrace it - make registration fast/automatic
- If containers are long-lived, need full state management

## Implementation Plan

### Core Changes (2 files)

| File | Changes | Lines | Complexity |
|------|---------|-------|------------|
| `crates/cb-core/src/workspaces.rs` | Change `DashMap<String, Workspace>` to `DashMap<(String, String), Workspace>`. Update 3 methods: `register()`, `list()`, `get()` | 48 | 🟡 Medium |
| `crates/cb-core/src/auth/jwt.rs` | Add `user_id: Option<String>` field to `Claims` struct. Update `generate_token()` to accept user_id | 167 | 🟢 Easy |

**Implementation:**
```rust
// workspaces.rs - BEFORE
pub struct WorkspaceManager {
    workspaces: Arc<DashMap<String, Workspace>>,
}

// workspaces.rs - AFTER
pub struct WorkspaceManager {
    workspaces: Arc<DashMap<(String, String), Workspace>>,  // (user_id, workspace_id)
}

impl WorkspaceManager {
    pub fn register(&self, user_id: &str, workspace: Workspace) {
        self.workspaces.insert((user_id.to_string(), workspace.id.clone()), workspace);
    }

    pub fn list(&self, user_id: &str) -> Vec<Workspace> {
        self.workspaces
            .iter()
            .filter(|entry| entry.key().0 == user_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get(&self, user_id: &str, id: &str) -> Option<Workspace> {
        self.workspaces
            .get(&(user_id.to_string(), id.to_string()))
            .map(|entry| entry.value().clone())
    }
}
```

```rust
// jwt.rs - ADD THIS FIELD
pub struct Claims {
    pub sub: Option<String>,
    pub exp: Option<usize>,
    pub iat: Option<usize>,
    pub iss: Option<String>,
    pub aud: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,  // NEW - REQUIRED for multi-tenancy
}
```

### Endpoint Changes (1 file)

| File | Changes | Lines | Complexity |
|------|---------|-------|------------|
| `crates/cb-transport/src/admin.rs` | Add JWT extraction helper. Update 3 endpoints: `register_workspace()`, `list_workspaces()`, `execute_command()` to extract user_id and scope operations | 368 | 🟡 Medium |

**Implementation:**
```rust
// NEW helper function
fn extract_user_id_from_jwt(
    headers: &HeaderMap,
    config: &AppConfig,
) -> Result<String, (StatusCode, String)> {
    // 1. Extract Authorization header
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?
        .to_str()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid Authorization header".to_string()))?;

    // 2. Extract token (Bearer <token>)
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid Authorization format".to_string()))?;

    // 3. Validate and decode JWT
    let auth_config = config.server.auth.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth not configured".to_string()))?;

    let key = DecodingKey::from_secret(auth_config.jwt_secret.as_ref());
    let mut validation = Validation::default();
    validation.validate_aud = false;

    let token_data = decode::<Claims>(token, &key, &validation)
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e)))?;

    // 4. Extract user_id claim
    token_data.claims.user_id
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Token missing user_id claim".to_string()))
}

// UPDATE endpoints:
async fn register_workspace(
    State(state): State<Arc<AdminState>>,
    headers: HeaderMap,  // NEW
    Json(workspace): Json<Workspace>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let user_id = extract_user_id_from_jwt(&headers, &state.config)?;  // NEW
    info!(workspace_id = %workspace.id, user_id = %user_id, "Registering workspace");
    state.workspace_manager.register(&user_id, workspace);  // MODIFIED
    Ok(Json(json!({ "status": "registered" })))
}

async fn list_workspaces(
    State(state): State<Arc<AdminState>>,
    headers: HeaderMap,  // NEW
) -> Result<Json<Vec<Workspace>>, (StatusCode, String)> {
    let user_id = extract_user_id_from_jwt(&headers, &state.config)?;  // NEW
    Ok(Json(state.workspace_manager.list(&user_id)))  // MODIFIED
}

async fn execute_command(
    State(state): State<Arc<AdminState>>,
    headers: HeaderMap,  // NEW
    Path(workspace_id): Path<String>,
    Json(request): Json<ExecuteCommandRequest>,
) -> Result<Json<ExecuteCommandResponse>, (StatusCode, String)> {
    let user_id = extract_user_id_from_jwt(&headers, &state.config)?;  // NEW
    let workspace = state.workspace_manager.get(&user_id, &workspace_id)  // MODIFIED
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Workspace '{}' not found", workspace_id)))?;
    // ... rest unchanged
}
```

### Handler Changes (3 files)

| File | Changes | Lines | Complexity |
|------|---------|-------|------------|
| `crates/cb-handlers/src/utils/remote_exec.rs` | Add `user_id` parameter, pass to `workspace_manager.get()` | 25 | 🟢 Easy |
| `crates/cb-handlers/src/handlers/file_operation_handler.rs` | Extract user_id from context, pass to `remote_exec::execute_in_workspace()` | 439 | 🟢 Easy |
| `crates/cb-handlers/src/handlers/refactoring_handler.rs` | Extract user_id from context, pass to `remote_exec::execute_in_workspace()` | ~400 | 🟢 Easy |

**Implementation:**
```rust
// remote_exec.rs
pub async fn execute_in_workspace(
    workspace_manager: &WorkspaceManager,
    user_id: &str,  // NEW parameter
    workspace_id: &str,
    command: &str,
) -> Result<String, HandlerError> {
    let workspace = workspace_manager.get(user_id, workspace_id)  // MODIFIED
        .ok_or_else(|| HandlerError::InvalidRequest(format!("Workspace '{}' not found", workspace_id)))?;
    // ... rest unchanged
}
```

### Test Changes (5 files)

| File | Changes | Lines | Complexity |
|------|---------|-------|------------|
| `crates/cb-server/src/test_helpers.rs` | Update helper to create JWTs with user_id | ~100 | 🟢 Easy |
| `crates/cb-server/tests/common/mod.rs` | Update test setup with user_id | ~50 | 🟢 Easy |
| `apps/codebuddy/tests/integration_services.rs` | Update test cases with user_id | ~200 | 🟡 Medium |
| `apps/codebuddy/tests/mcp_handler_runners.rs` | Update test helpers with user_id | ~150 | 🟢 Easy |
| **`crates/cb-core/tests/multitenancy_tests.rs`** | **NEW FILE** - User isolation tests | ~200 | 🟡 Medium |

**New test file:**
```rust
// crates/cb-core/tests/multitenancy_tests.rs
#[test]
fn test_user_isolation() {
    let manager = WorkspaceManager::new();

    let workspace_a = Workspace { id: "ws1".into(), /* ... */ };
    let workspace_b = Workspace { id: "ws2".into(), /* ... */ };

    manager.register("user_a", workspace_a);
    manager.register("user_b", workspace_b);

    // User A can only see their workspace
    assert_eq!(manager.list("user_a").len(), 1);
    assert!(manager.get("user_a", "ws1").is_some());
    assert!(manager.get("user_a", "ws2").is_none());

    // User B can only see their workspace
    assert_eq!(manager.list("user_b").len(), 1);
    assert!(manager.get("user_b", "ws2").is_some());
    assert!(manager.get("user_b", "ws1").is_none());
}

#[test]
fn test_same_workspace_id_different_users() {
    let manager = WorkspaceManager::new();

    let workspace_a = Workspace { id: "project".into(), language: "rust".into(), /* ... */ };
    let workspace_b = Workspace { id: "project".into(), language: "python".into(), /* ... */ };

    manager.register("user_a", workspace_a);
    manager.register("user_b", workspace_b);

    // Both users can have workspaces with the same ID
    let ws_a = manager.get("user_a", "project").unwrap();
    let ws_b = manager.get("user_b", "project").unwrap();

    assert_eq!(ws_a.language, "rust");
    assert_eq!(ws_b.language, "python");
}
```

### Documentation Changes (4 files)

| File | Changes | Complexity |
|------|---------|------------|
| `API_REFERENCE.md` | Document JWT user_id requirement | 🟢 Easy |
| `docs/architecture/ARCHITECTURE.md` | Document multi-tenancy design | 🟢 Easy |
| `CHANGELOG.md` | Add breaking change notice | 🟢 Easy |
| `20_BACKEND_MULTITENANCY_PROPOSAL.md` | Mark as implemented | 🟢 Easy |

## Testing Strategy

**Unit Tests:**
- User isolation in WorkspaceManager
- JWT user_id extraction and validation
- Same workspace ID for different users

**Integration Tests:**
- Register workspace with JWT user_id
- List workspaces scoped to user
- Execute command in user's workspace
- Reject access to other users' workspaces
- Reject JWTs without user_id claim

**Manual Testing:**
```bash
# 1. Generate tokens for two users
curl -X POST http://localhost:3001/auth/generate-token \
  -H "Content-Type: application/json" \
  -d '{"user_id": "alice"}'

curl -X POST http://localhost:3001/auth/generate-token \
  -H "Content-Type: application/json" \
  -d '{"user_id": "bob"}'

# 2. Register workspaces for each user
curl -X POST http://localhost:3001/workspaces/register \
  -H "Authorization: Bearer $ALICE_TOKEN" \
  -d '{"id": "alice-workspace", ...}'

curl -X POST http://localhost:3001/workspaces/register \
  -H "Authorization: Bearer $BOB_TOKEN" \
  -d '{"id": "bob-workspace", ...}'

# 3. Verify isolation
curl http://localhost:3001/workspaces \
  -H "Authorization: Bearer $ALICE_TOKEN"
# Should only return alice-workspace

curl http://localhost:3001/workspaces \
  -H "Authorization: Bearer $BOB_TOKEN"
# Should only return bob-workspace
```

## Migration Path

**Breaking Change:** All workspace operations now require JWT with user_id

**Migration Steps:**
1. Update JWT generation to include user_id claim
2. Update all clients to include user_id in tokens
3. Deploy new server version
4. Verify user isolation with manual tests
5. Run full integration test suite

**Backward Compatibility:** None - this is a security-critical breaking change

## Implementation Checklist

**Core Implementation:**
- [ ] Update `WorkspaceManager` to use `(user_id, workspace_id)` composite key
- [ ] Add `user_id` field to JWT `Claims` struct
- [ ] Update `generate_token()` to accept user_id parameter

**Endpoint Updates:**
- [ ] Add `extract_user_id_from_jwt()` helper function
- [ ] Update `register_workspace()` endpoint to extract and use user_id
- [ ] Update `list_workspaces()` endpoint to scope by user_id
- [ ] Update `execute_command()` endpoint to scope by user_id

**Handler Updates:**
- [ ] Update `remote_exec::execute_in_workspace()` with user_id parameter
- [ ] Update `file_operation_handler.rs` to pass user_id
- [ ] Update `refactoring_handler.rs` to pass user_id

**Testing:**
- [ ] Create `crates/cb-core/tests/multitenancy_tests.rs`
- [ ] Add user isolation tests
- [ ] Add same-workspace-id-different-users test
- [ ] Update existing tests with user_id
- [ ] Add integration tests for JWT validation

**Documentation:**
- [ ] Update API_REFERENCE.md with user_id requirement
- [ ] Update docs/architecture/ARCHITECTURE.md with multi-tenancy design
- [ ] Add breaking change notice to CHANGELOG.md
- [ ] Mark this proposal as implemented

**Verification:**
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] Manual testing confirms user isolation
- [ ] Integration tests verify JWT validation

## Success Criteria

- [ ] All 15 files modified as specified
- [ ] All existing tests pass
- [ ] All new multi-tenancy tests pass
- [ ] User A cannot access User B's workspaces
- [ ] JWT without user_id is rejected
- [ ] Documentation updated
- [ ] Zero clippy warnings
- [ ] Manual testing confirms isolation

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| JWT extraction breaks existing clients | High | High | Comprehensive testing, staged rollout |
| Composite key degrades performance | Low | Medium | DashMap is designed for multi-key scenarios |
| Tests miss edge cases | Medium | High | Thorough code review, manual testing |
| Documentation drift | Medium | Low | Update docs as part of implementation |

## Future Enhancements

- Admin API to list all users' workspaces (for debugging)
- Workspace quotas per user
- Workspace sharing between users
- Audit logging for workspace access
