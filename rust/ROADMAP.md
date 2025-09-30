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

## 🚀 Planned Features

### Language Support
- [ ] **SWC Integration** - Faster TypeScript/JavaScript parsing
  - Status: Planned but not yet implemented
  - Blocker: Network restrictions during initial development
  - Priority: Medium
  - Estimated effort: 20-40 hours

### Architecture Improvements
- [ ] **Structured Logging** - Replace println!/eprintln! with proper logging framework
  - Priority: High
  - Target: Before 1.0
  - Estimated effort: 6-8 hours

- [ ] **VFS Feature Completion** - Complete or remove partial FUSE implementation
  - Priority: High
  - Needs decision: Complete or remove?
  - Estimated effort: 16-24 hours (complete) or 4 hours (remove)

### Developer Experience
- [ ] **Benchmark Suite Restoration** - Re-enable disabled performance benchmarks
  - File: `benchmark-harness/benches/config_benchmark.rs.disabled` (238 lines)
  - Status: Disabled, requires API update investigation
  - Blocker: `ClientConfig::load_with_path` method no longer exists in current API
  - Decision needed: Update to current API or remove permanently
  - Priority: Low (informational value only)
  - Estimated effort: 4-8 hours (investigation + update) or 5 minutes (delete)

- [ ] **Error Handling** - Systematic removal of `.unwrap()` from production code
  - Priority: High
  - Target: Before 1.0
  - Estimated effort: 40+ hours

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

## 🔧 Technical Debt

See [Legacy Code Analysis Report](docs/legacy-analysis.md) for detailed technical debt tracking.

**Priority Items:**
1. Remove `.unwrap()` from production code (HIGH RISK)
2. Complete or remove VFS driver (HIGH RISK)
3. Fix test infrastructure error handling (HIGH RISK)
4. Add structured logging (MEDIUM RISK)
5. Resolve dependency duplicates (MEDIUM RISK)

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