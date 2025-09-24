# Release Notes - v1.2.0

## 🚀 Enterprise Architecture & LSP Server Pooling

This release represents a complete architectural transformation, implementing advanced resource management, enterprise deployment capabilities, and intelligent pooling systems for optimal performance.

### 🎯 Major Features in v1.2.0

#### 🏊 LSP Server Pooling
- **Resource Efficiency**: Max 2 servers per language instead of unlimited
- **Reduced Latency**: Server reuse eliminates cold start delays
- **Workspace Isolation**: Servers can be reassigned between workspaces
- **Intelligent Queuing**: Automatic waiting when pools are at capacity

#### ⚡ Performance Enhancements
- **Delta Updates**: Efficient file synchronization using diff-match-patch
- **Advanced Caching**: Event-driven invalidation with hit rate tracking
- **Analysis Cache**: Prevents re-computation for workspace symbols

#### 🏗️ Architecture Transformation
- **Monorepo Structure**: Clean packages/client and packages/server separation
- **Transaction Manager**: Atomic operations with rollback capabilities
- **Workflow Orchestrator**: Automated tool chain execution with dependencies
- **Service Architecture**: Modular service-based design patterns

#### 🔧 Enterprise Features
- **WebSocket Server**: Production-ready multi-client support
- **JWT Authentication**: Token-based access control
- **Health Monitoring**: `/healthz` and `/metrics` endpoints
- **Session Management**: Connection recovery with 60-second grace periods

### Installation

```bash
# Install latest version globally
npm install -g @goobits/codeflow-buddy

# Smart setup with auto-detection
codeflow-buddy setup

# Start WebSocket server with authentication
node dist/index.js serve --require-auth --jwt-secret "your-secret-key"
```

## Previous Release - v1.1.0 - ARM64 Native FUSE Support

### Key Features

#### 🏗️ Native FUSE Implementation
- Replaced `fuse-native` with `@cocalc/fuse-native` for ARM64 compatibility
- Removed all mock implementations - now using 100% native FUSE
- Full callback-style API implementation for better compatibility

#### 🐳 Multi-Tenant Docker Support
- Production-ready Docker Compose configuration
- Multi-tenant FUSE folder mounting capabilities
- Session-based workspace isolation
- Automatic cleanup on client disconnect

#### 🛠️ Stability Improvements
- Fixed duplicate method definitions in WebSocket server
- Resolved TypeScript type errors in FUSE operations
- Improved test isolation for FUSE integration tests
- Better error handling in session cleanup

### Breaking Changes
None - this release maintains full backward compatibility.

### Installation

```bash
# Install globally
npm install -g @goobits/codeflow-buddy@1.1.0

# Or use with npx
npx @goobits/codeflow-buddy@1.1.0 setup
```

### Docker Deployment

```bash
# Quick start multi-tenant service
docker-compose up --build

# Or use the production configuration
docker-compose -f docker-compose.production.yml up -d
```

### Requirements
- Node.js 18+
- FUSE support in kernel (for FUSE features)
- Docker with privileged mode (for containerized FUSE)

### Platform Support
- ✅ x86_64 Linux
- ✅ ARM64 Linux
- ✅ macOS (Intel)
- ✅ macOS (Apple Silicon)
- ✅ Windows (via WSL2)

### Contributors
Special thanks to everyone who helped test and validate ARM64 support!

---

For more information, see the [CHANGELOG](CHANGELOG.md) and [README](README.md).