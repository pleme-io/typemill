# Docker Shared Volumes for CodeBuddy

**Status**: Proposal
**Date**: 2025-09-30

---

## TL;DR

**You don't need FUSE.** Docker volumes provide instant two-way file sync between containers.

One CodeBuddy hub → Multiple project containers → Same files, different paths.

---

## The Problem

You want:
- Single CodeBuddy MCP server (in a container)
- Multiple project containers (TypeScript, Python, Rust, etc.)
- CodeBuddy can edit files in all projects
- Projects stay isolated from each other
- Two-way file sync

**You thought you needed FUSE. You don't.**

---

## The Solution

### Architecture Diagram

```
HOST: ~/projects/typescript-app/
              ↓
         Docker Volume
         (bind mount)
              ↓
         ┌────┴────┐
         ↓         ↓
CodeBuddy Hub    Project Container
/workspaces/     /workspace/
  project-a/       (only sees own files)

SAME FILES. INSTANT SYNC. NO FUSE.
```

### How It Works

1. Docker mounts the **same filesystem path** into multiple containers
2. CodeBuddy writes to `/workspaces/project-a/src/index.ts`
3. Project container sees change at `/workspace/src/index.ts` **instantly**
4. Same inode, same file, zero latency

**No synchronization protocol. No FUSE. Just Docker volumes.**

---

## Implementation

### Complete docker-compose.yml

```yaml
version: '3.8'

services:
  # THE HUB - CodeBuddy sees ALL projects
  codebuddy:
    image: codebuddy:latest
    container_name: codebuddy-hub
    ports:
      - "3000:3000"
    networks:
      - codebuddy-net
    volumes:
      # Mounts ALL projects
      - ~/projects/typescript-app:/workspaces/project-a:rw
      - ~/projects/python-service:/workspaces/project-b:rw
      - ~/projects/rust-cli:/workspaces/project-c:rw
    command: serve --port 3000

  # PROJECT A - Only sees its own files
  typescript-app:
    image: node:20
    container_name: project-a
    working_dir: /workspace
    networks:
      - codebuddy-net
    volumes:
      - ~/projects/typescript-app:/workspace:rw  # ONLY project-a
    environment:
      - CODEBUDDY_URL=ws://codebuddy:3000
    command: bash -c "
      npm install &&
      npm install -g typescript-language-server typescript &&
      typescript-language-server --stdio &
      tail -f /dev/null
    "

  # PROJECT B - Only sees its own files
  python-service:
    image: python:3.11
    container_name: project-b
    working_dir: /workspace
    networks:
      - codebuddy-net
    volumes:
      - ~/projects/python-service:/workspace:rw  # ONLY project-b
    environment:
      - CODEBUDDY_URL=ws://codebuddy:3000
    command: bash -c "
      pip install -r requirements.txt &&
      pip install python-lsp-server[all] &&
      pylsp &
      tail -f /dev/null
    "

  # PROJECT C - Only sees its own files
  rust-cli:
    image: rust:1.75
    container_name: project-c
    working_dir: /workspace
    networks:
      - codebuddy-net
    volumes:
      - ~/projects/rust-cli:/workspace:rw  # ONLY project-c
    environment:
      - CODEBUDDY_URL=ws://codebuddy:3000
    command: bash -c "
      rustup component add rust-analyzer &&
      rust-analyzer &
      tail -f /dev/null
    "

networks:
  codebuddy-net:
    driver: bridge
```

### Deploy

```bash
# 1. Build CodeBuddy image
docker build -t codebuddy:latest .

# 2. Start everything
docker-compose up -d

# 3. Verify
curl http://localhost:3000/health

# DONE.
```

---

## What Each Container Sees

### CodeBuddy Hub
```
/workspaces/
├── project-a/  ← TypeScript app (CAN EDIT)
├── project-b/  ← Python service (CAN EDIT)
└── project-c/  ← Rust CLI (CAN EDIT)
```

### Project A Container
```
/workspace/     ← ONLY sees project-a
├── src/
├── package.json
└── tsconfig.json

# Cannot see project-b or project-c
```

### Project B Container
```
/workspace/     ← ONLY sees project-b
├── app/
└── requirements.txt

# Cannot see project-a or project-c
```

**Complete isolation. CodeBuddy hub sees all.**

---

## File Sync Flow

```
Claude: "Edit project-a/src/index.ts"
    ↓
CodeBuddy hub writes to:
  /workspaces/project-a/src/index.ts
    ↓
Project-A container immediately sees change at:
  /workspace/src/index.ts
    ↓
typescript-language-server updates diagnostics
    ↓
DONE. No sync daemon. No polling. Instant.
```

---

## Why Not FUSE?

FUSE is for **interception and control**:
- Auto-format files on save
- Access control / audit trails
- Generate virtual files on-the-fly
- Transform data (encryption, compression)

**You don't need any of that.** You just need file sharing → Docker volumes.

### Comparison

| Feature | Docker Volumes | FUSE |
|---------|----------------|------|
| Setup | ✅ 1 line per project | ❌ Custom filesystem code |
| Performance | ✅ Native (no overhead) | ⚠️ Userspace overhead |
| Sync | ✅ Instant (same inode) | ❌ Need sync protocol |
| Complexity | ✅ Docker handles it | ❌ Debug kernel issues |
| Dev Time | ✅ 1-2 weeks | ❌ 4-6 weeks |

**For your use case: Docker volumes win.**

---

## Use Cases

### Example 1: Edit File
```
1. Claude → MCP request → CodeBuddy hub
2. CodeBuddy writes to /workspaces/project-a/src/utils.ts
3. Project-A sees change at /workspace/src/utils.ts (SAME FILE)
4. LSP server updates diagnostics
```

### Example 2: Multi-Project Refactoring
```
1. Claude: "Rename function across all projects"
2. CodeBuddy hub:
   - Edits /workspaces/project-a/...
   - Edits /workspaces/project-b/...
   - Edits /workspaces/project-c/...
3. Each project container sees changes instantly
4. Each LSP server updates independently
```

### Example 3: Run Tests in Isolation
```
1. Claude: "Run tests in all projects"
2. CodeBuddy sends commands:
   - project-a: npm test
   - project-b: pytest
   - project-c: cargo test
3. Each container has own dependencies/environment
4. Results aggregated by CodeBuddy
```

---

## Implementation Checklist

### Week 1: Basic Setup
- [ ] Build CodeBuddy Docker image
- [ ] Create docker-compose.yml
- [ ] Test with 1-2 projects
- [ ] Verify file sync works

### Week 2: MCP Integration
- [ ] Add WebSocket MCP server to CodeBuddy
- [ ] Implement file operation handlers
- [ ] Test MCP commands across containers

### Week 3: LSP Integration
- [ ] Configure LSP servers in project containers
- [ ] Test code intelligence features
- [ ] Add workspace-wide refactoring

### Week 4: Production
- [ ] Add health checks
- [ ] Logging and monitoring
- [ ] Performance testing (5+ projects)
- [ ] Documentation

---

## When to Consider FUSE

If you need these in the future:

**Auto-format on save:**
```rust
fn write(&mut self, path: &Path, data: &[u8]) -> Result<()> {
    let formatted = format_code(data);
    self.real_fs.write(path, formatted)
}
```

**Access control:**
```rust
fn open(&mut self, path: &Path) -> Result<FileHandle> {
    if !user_has_permission(path) {
        return Err(EACCES);
    }
    self.real_fs.open(path)
}
```

**Virtual files:**
```rust
fn read(&mut self, path: &Path) -> Result<Vec<u8>> {
    if path == "/.metadata/ast.json" {
        return Ok(generate_ast());  // Generate on-the-fly
    }
    self.real_fs.read(path)
}
```

**But not for simple file sharing. Docker volumes got you.**

---

## Performance

### Why This Is Fast

1. **Native filesystem** - No FUSE overhead
2. **Same inode** - No copying, no sync protocol
3. **Zero latency** - Changes are instant
4. **LSP servers in project containers** - Close to the files

### Scalability

- **5-10 projects**: Easy on a single host
- **10-50 projects**: Add more resources, same pattern
- **50+ projects**: Consider Kubernetes

---

## Questions & Answers

**Q: Can CodeBuddy edit files in project-a while project-a is using them?**
A: Yes. Same file, multiple file descriptors. OS handles it.

**Q: What if a project container crashes?**
A: Docker restart policy brings it back. Files unchanged.

**Q: Can projects talk to each other?**
A: Only via CodeBuddy hub (MCP). Direct access blocked by Docker networking.

**Q: Can I add more projects?**
A: Yes. Add another service block in docker-compose.yml.

**Q: Does this work on Windows/Mac?**
A: Yes. Docker Desktop handles volume mounts.

---

## Next Steps

1. **Build CodeBuddy Docker image**
   ```bash
   docker build -t codebuddy:latest .
   ```

2. **Create docker-compose.yml** (see above)

3. **Test with 2 projects first**

4. **Add MCP file operations to CodeBuddy**

5. **Deploy and iterate**

---

## References

- [Docker Volumes Docs](https://docs.docker.com/storage/volumes/)
- [MCP Specification](https://github.com/anthropics/mcp)
- [CodeBuddy Architecture](./CLAUDE.md)

---

**Document Version**: 2.0 (Braindead Edition)
**Last Updated**: 2025-09-30
**Status**: Ready to implement

---

## Appendix: The Insight

> "So you're saying that we can have a directory in 1 remote Docker that's shared with the CodeBuddy server, and then a directory in another Docker that's shared with the CodeBuddy MCP server, and keep it separate that way so it's two-way, the multiple two-way?"

**YES. Exactly.**

Docker volumes mount the **same underlying path** into multiple containers with different mount points. It's not "syncing" - it's literally the same file. When any container modifies it, all containers see the change instantly because **they're all looking at the same inode**.

No FUSE. No complexity. Just Docker volumes doing what they were designed to do.

**That's the whole proposal.**