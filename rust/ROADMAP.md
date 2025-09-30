# CodeBuddy Roadmap

## Current Status: Pre-1.0 Development (v0.1.0)

CodeBuddy is in active development with core functionality working but no API stability guarantees.

---

## 🎯 Path to 1.0 Release

**Target:** Q2 2025

### Requirements for 1.0
- [ ] API stability commitment
- [ ] Complete documentation coverage
- [ ] Production deployments validated
- [ ] Performance benchmarks met
- [ ] Security audit completed
- [ ] All HIGH priority technical debt addressed

---

## 🚀 Planned Features & Technical Debt

### 🔥 HIGH Priority (Before 1.0)

#### 1. Structured Logging - ✅ COMPLETE
- [x] **Foundation**: Tracing framework integrated in cb-server ✅
- [x] **Production Code**: All production libraries use tracing ✅
  - Status: 100% done - Fixed remaining eprintln! calls
  - Fixed: cb-ast/parser.rs (2 eprintln! → tracing::debug!)
  - Acceptable: 1 eprintln! in server main.rs during logger init (can't log before logger exists)
  - Acceptable: 434 println! calls in CLI tools (user-facing output) and tests
  - Breakdown: cb-client has ~200 println for interactive prompts, table output, help text
  - **Decision**: ✅ Complete - Production code uses structured logging, CLI uses println appropriately
  - Priority: **DONE**

#### 2. Error Handling - Remove .unwrap() from production code
- [ ] **Phase 1**: Production hot paths (services/, handlers/) - 8-10 hours (TODO)
- [x] **Phase 2**: CLI and startup code - ✅ COMPLETE (10 minutes actual)
  - Fixed 5 production unwraps in cb-client (formatting, connect, status, mcp, call)
  - Remaining 38 unwraps: 37 in tests (acceptable) + 4 ProgressStyle templates (safe hardcoded)
- [ ] **Phase 3**: Keep .unwrap() in tests (tests are allowed to panic)
  - Status: Phase 2 complete, Phase 1 pending
  - Phase 1 effort: 82 unwraps in services/handlers remain
  - **Decision**: 📋 Phase 1 next priority
  - Priority: **HIGH** for Phase 1

#### 3. Dependency Cleanup - ✅ COMPLETE
- [x] Run `cargo tree --duplicates` to identify all duplicates
- [x] Align versions in Cargo.toml across workspace
- [x] Verify build and tests pass
  - Status: Done - Consolidated thiserror 2.0 and jsonwebtoken 10.0
  - Unified across cb-plugins, cb-mcp-proxy, cb-server, cb-transport, tests
  - Remaining duplicates are from external transitive dependencies (acceptable)
  - **Decision**: ✅ Complete - Core dependencies unified
  - Priority: **DONE**

### ⚠️ MEDIUM Priority (Consider for 1.0)

#### 4. VFS Feature - ✅ COMPLETE (Feature-gated)
- [x] Add `#[cfg(feature = "vfs")]` guards to usage sites ✅
- [x] Update Cargo.toml to make vfs an optional feature ✅
- [x] Remove cb-vfs from default workspace build ✅
- [x] Document as experimental ✅
  - Status: Complete - cb-vfs excluded from workspace.members
  - Usage guarded with #[cfg(all(unix, feature = "vfs"))]
  - Build with VFS: `cargo build --features vfs` (Unix only)
  - Default build: VFS not compiled (faster builds, smaller binary)
  - **Decision**: ✅ Complete - Properly feature-gated, Docker volumes proposal eliminates immediate need
  - Priority: **DONE** - Not included in default 1.0 release

### 📦 LOW Priority (Post-1.0)

#### 5. SWC Integration - Faster TypeScript/JavaScript parsing
- [ ] Integrate SWC for AST parsing
- [ ] Benchmark performance improvements
- [ ] Update existing TS/JS tools to use SWC
  - Status: Not implemented, blocked by network restrictions during initial dev
  - Estimated effort: 20-40 hours
  - **Decision**: ⏸️ Defer - Current AST parsing works, this is optimization
  - Priority: **LOW** - Post-1.0 optimization if performance becomes issue

#### 6. Benchmark Suite - ✅ COMPLETE
- [x] Delete `benchmark-harness/benches/config_benchmark.rs.disabled`
- [x] Document that benchmarks can be recreated later if needed
  - Status: Done - Removed 238 lines of stale code
  - API changed (ClientConfig::load_with_path doesn't exist), unmaintainable
  - **Decision**: ✅ Complete - Clutter removed, can recreate if needed
  - Priority: **DONE**

---

## 📅 Release Timeline

### Q4 2024 (Current)
- ✅ Core LSP integration
- ✅ MCP protocol support
- ✅ Plugin architecture
- 🔄 Technical debt reduction (in progress)

### Q1 2025
- Performance optimization
- Documentation improvements
- Security hardening
- Beta testing program

### Q2 2025
- API stabilization
- 1.0 Release candidate
- Production readiness validation
- **1.0 RELEASE**

### Post-1.0
- Follow semantic versioning (semver 2.0)
- Breaking changes only in major versions
- Regular security updates
- Community-driven feature development

---

## 🔧 Technical Debt Summary

See section above for detailed breakdown. Quick reference:

**✅ COMPLETED:**
1. ✅ Structured Logging - DONE (production code uses tracing, CLI println appropriate)
2. ✅ Dependency Cleanup - DONE (thiserror 2.0, jsonwebtoken 10.0 unified)
3. ✅ Benchmark Suite - DONE (stale code removed)

**HIGH Priority (Before 1.0):**
4. 📋 Error Handling Phase 1 - 8-10 hours (production hot paths)

**MEDIUM Priority (Consider for 1.0):**
5. ⚠️ VFS Feature-gating - 1-2 hours (defer decision, keep code)

**LOW Priority (Post-1.0):**
6. ⏸️ SWC Integration - 20-40 hours (optimization, not required)

---

## 📊 Version Strategy

### Pre-1.0 (Current: 0.1.0)
- Breaking changes allowed without notice
- No API stability guarantees
- Rapid iteration and experimentation
- Internal use and testing only

### Post-1.0
- **Major version** (X.0.0): Breaking changes
- **Minor version** (0.X.0): New features, backwards compatible
- **Patch version** (0.0.X): Bug fixes only

---

## 🤝 Contributing

Want to help shape CodeBuddy's future?

- Review open issues tagged with `roadmap`
- Discuss features in GitHub Discussions
- Submit PRs for planned features
- Help with documentation and testing

---

**Last Updated:** 2025-09-30