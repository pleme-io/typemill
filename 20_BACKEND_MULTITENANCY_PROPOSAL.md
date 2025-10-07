# Proposal: Finalize Backend Roadmap

**Status**: Proposal - Under Review
**Date**: 2025-10-01
**Last Updated**: 2025-10-02

## 1. Overview

The core feature set of the CodeBuddy server is now complete. The system can manage remote workspace containers, execute commands within them, and perform complex, AST-aware refactorings on their files.

This document outlines remaining backend development to transition the service to a production-ready application.

## 2. Priority Assessment

**Critical (Must-Have):**
- ✅ **Phase 2: Multi-Tenancy & Scoped Access** - Essential security requirement

**Questionable (Reevaluate):**
- ⚠️ **Phase 1: LSP Architecture Refactoring** - Adds complexity for unclear benefit
- 🟡 **Phase 3: State Persistence** - Solves partial problem, needs broader container lifecycle strategy

## Implementation Checklist

### Phase 1: LSP Architecture Refactoring ⚠️ DEFERRED
- [ ] ~~Move LSP servers to workspace containers~~ (Deferred - keep LSP local)

### Phase 2: Multi-Tenancy & Scoped Access ✅ CRITICAL
- [ ] Add `user_id` claim to JWT authentication
- [ ] Partition `WorkspaceManager` by user
- [ ] Scope all workspace operations to authenticated user
- [ ] Add user isolation tests
- [ ] Update documentation

### Phase 3: State Persistence 🟡 NEEDS REDESIGN
- [ ] ~~Add SQLite database for workspace persistence~~ (Needs broader container lifecycle strategy)
- [ ] Design container lifecycle strategy (long-lived vs ephemeral)
- [ ] Implement based on lifecycle decision

## 3. Recommended Approach

1. **Implement Phase 2 immediately** - blocking security issue
2. **Defer Phase 1** - keep LSP local (simpler, faster, proven)
3. **Redesign Phase 3** - address full container lifecycle, not just registration persistence

---

### **Phase 1: LSP Architecture Refactoring** ⚠️

**Original Problem:** LSP servers in main container "bloat" the image and prevent workspace-specific versions.

**Critical Analysis:**
- ❌ **Network latency**: LSP over TCP adds overhead. LSP is chatty (completions/hovers on every keystroke)
- ❌ **Reconnection complexity**: Workspace container restarts break LSP connections, need recovery logic
- ❌ **Resource multiplication**: Each workspace needs own LSP instance (memory × N users)
- ✅ **Image bloat**: Weak argument - LSP binaries ~10-50MB total, solvable with multi-stage builds
- ✅ **Local stdio**: Current approach is simple, fast, proven

**Recommendation:** **DEFER** - Keep LSP servers local. Complexity outweighs benefits.

---

### **Phase 2: Multi-Tenancy & Scoped Access** ✅

**Problem:** Global state - all clients see all workspaces. Critical security gap.

**Solution:**
- Add `user_id` claim to JWT authentication
- Partition `WorkspaceManager` by user
- Scope all workspace operations to authenticated user

**Assessment:**
- ✅ **Essential for production**: Cannot deploy without user isolation
- ✅ **Security critical**: Prevents unauthorized access
- ✅ **Clean implementation**: JWT already supports claims, straightforward scoping

**Recommendation:** **IMPLEMENT IMMEDIATELY** - Blocking security requirement.

---

### **Phase 3: State Persistence** 🟡

**Original Problem:** Server restart loses workspace registrations (in-memory `DashMap`).

**Proposed Solution:** SQLite database for workspace persistence.

**Critical Analysis:**
- 🟡 **Partial solution**: Saves workspace *registration* but not container *state*
- ⚠️ **Container lifecycle gap**: Docker containers are ephemeral by default
  - Code persists (volumes) ✅
  - Container restarts → agent process dies → needs re-registration anyway ❌
- 🟡 **Limited value**: DB helps with *server* restarts, not *container* restarts

**Better Approach:**
- Rethink container lifecycle: long-lived vs on-demand recreation
- Consider container orchestration (health checks, auto-restart)
- If containers are ephemeral, embrace it - make registration fast/automatic
- If containers are long-lived, need full state management (not just workspace list)

**Recommendation:** **REDESIGN** - Address full container lifecycle strategy, not just registration persistence.

## 4. Conclusion

**Immediate Priority:** Phase 2 (Multi-Tenancy) - critical security requirement.

**Deferred:** Phase 1 (LSP in Workspaces) - current approach is simpler and faster.

**Needs Redesign:** Phase 3 (Persistence) - requires broader container lifecycle thinking.
