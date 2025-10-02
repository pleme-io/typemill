# Proposal: Finalize Backend Roadmap

**Status**: Proposal
**Date**: 2025-10-01

## 1. Overview

The core feature set of the CodeBuddy server is now complete. The system can manage remote workspace containers, execute commands within them, and perform complex, AST-aware refactorings on their files.

This document outlines the final three phases of backend development required to transition the service from a feature-complete engine to a robust, scalable, and secure production-ready application.

## 2. The Roadmap

The remaining work is divided into three distinct, sequential phases:

1.  **LSP Architecture Refactoring:** Decouple Language Servers from the main application.
2.  **Multi-Tenancy & Scoped Access:** Introduce user-scoping for all resources.
3.  **State Persistence:** Make the server resilient to restarts.

---

### **Phase 1: LSP Architecture Refactoring**

**Problem:** Currently, all Language Server Protocol (LSP) servers (e.g., `pylsp`, `typescript-language-server`) are installed and run inside the main CodeBuddy Docker container. This monolithic approach is inflexible, bloats the main server image, and prevents workspaces from using different or project-specific LSP versions.

**Solution:**
- Refactor the system to align with the original architectural vision of running LSPs inside their respective workspace containers.
- The `python-workspace` will be updated to run its own `pylsp` process, exposing it over a TCP socket.
- The CodeBuddy server's `LspClient` and `PluginDispatcher` will be modified to manage connections to these remote LSP servers instead of spawning local processes.
- The bundled LSP servers will be removed from the main `codebuddy` Docker image.

**Benefits:**
- **Isolation:** Language server crashes are isolated to their specific workspace and cannot affect the main CodeBuddy server.
- **Flexibility:** Each workspace can use its own version of an LSP server, managed within its own container.
- **Efficiency:** The main CodeBuddy server image becomes smaller and more focused.

---

### **Phase 2: Multi-Tenancy & Scoped Access**

**Problem:** The server currently operates with a global state. All connected clients see the same set of workspaces. There is no concept of a "user," which is a critical security and usability gap for any shared environment.

**Solution:**
- Introduce the concept of a `user_id` or `tenant_id` throughout the application.
- The JWT issued upon authentication will be updated to include a `user_id` claim.
- The `WorkspaceManager` will be refactored to partition all workspaces by `user_id`.
- All API endpoints and tool calls that reference a `workspaceId` will be updated to ensure the request is scoped to the workspaces owned by the authenticated user.

**Benefits:**
- **Security:** Users can only see and interact with their own workspace containers, preventing unauthorized access.
- **Scalability:** Enables the server to correctly and safely handle multiple simultaneous users.

---

### **Phase 3: State Persistence**

**Problem:** All server state, most importantly the list of registered workspaces and their agent URLs, is stored in-memory. If the CodeBuddy server restarts, this information is lost, and all agents must re-register.

**Solution:**
- Integrate a simple, file-based persistent database, such as **SQLite**.
- Refactor the `WorkspaceManager` to read from and write to this database instead of only an in-memory `DashMap`.
- On startup, the server will load the list of known workspaces from the database, allowing it to potentially reconnect to agents without requiring them to re-register.

**Benefits:**
- **Robustness:** The server becomes resilient to restarts and crashes, preserving its state.
- **Improved UX:** Workspaces remain "known" across server restarts, providing a more stable experience.

## 3. Conclusion

Completing these three phases will address the remaining architectural, security, and reliability concerns. The result will be a mature backend service that is not only powerful in its features but also secure, scalable, and robust enough for production deployment.
