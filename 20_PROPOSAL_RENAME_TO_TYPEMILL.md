# Proposal: Rename Project to TypeMill

**Status**: Draft
**Author**: Project Team
**Date**: 2025-10-10
**Current Name**: `codebuddy` / `codebuddy` CLI
**Proposed Name**: `typemill` / `mill` CLI

---

## Executive Summary

This proposal outlines the rationale, scope, and implementation plan for renaming the project from `codebuddy` to `typemill`, with the CLI command changing from `codebuddy` to `mill`.

## Motivation

### Why "TypeMill"?

1. **Better Reflects Core Functionality**
   - The project is fundamentally a "mill" that processes and refines code through LSP servers
   - "Type" emphasizes the strong type-safety focus (LSP intelligence, static analysis, refactoring)
   - Metaphor: A mill processes raw materials into refined products; TypeMill processes code into better code

2. **Stronger Brand Identity**
   - "codebuddy" is generic and conflicts with existing tools/services
   - "typemill" is distinctive and memorable
   - Conveys professionalism and precision

3. **CLI Ergonomics**
   - `mill` is short, fast to type (4 characters vs 9)
   - Follows Unix tradition of concise commands (`git`, `grep`, `sed`, `make`)
   - Natural verb-like quality: "mill the code", "run the mill"

4. **Technical Alignment**
   - Emphasizes the "grinding/processing" nature of the tool
   - "Type" connects to type systems, TypeScript support, and static analysis
   - Better SEO and searchability in developer tools space

5. **Domain Assets**
   - Project owns both `typemill.org` and `typemill.com`
   - `.org` hosts the open-source mill CLI and documentation
   - `.com` reserved for future Design Evolve product (visual/commercial layer)
   - Complete brand protection and clear product positioning

## Scope of Changes

### 1. Crate and Package Names

**Rust Workspace:**
- `codebuddy` → `typemill`
- `cb-protocol` → `mill-protocol`
- `cb-server` → `mill-server`
- `cb-client` → `mill-client`
- `cb-lsp` → `mill-lsp`
- `cb-services` → `mill-services`
- `cb-ast` → `mill-ast`
- `cb-vfs` → `mill-vfs`
- `cb-plugins` → `mill-plugins`
- `cb-language-plugin` → `mill-language-plugin`
- `cb-*` → `mill-*` (all crates)

**Naming Convention:**
- Old: `cb-{component}` (e.g., `cb-services`)
- New: `mill-{component}` (e.g., `mill-services`)

### 2. CLI Commands

**Primary Command:**
```bash
# Old
codebuddy setup
codebuddy status
codebuddy start
codebuddy serve

# New
mill setup
mill status
mill start
mill serve
```

**All Subcommands:**
- `mill setup` - Smart setup with auto-detection
- `mill status` - Show current status
- `mill start` - Start MCP server
- `mill stop` - Stop MCP server
- `mill serve` - Start WebSocket server
- `mill link` - Link to AI assistants
- `mill unlink` - Remove AI from config
- `mill --version` - Show version

### 3. Configuration and Paths

**Configuration Directory:**
- `.codebuddy/` → `.typemill/`
- `.codebuddy/config.json` → `.typemill/config.json`

**Binary Path:**
- `target/release/codebuddy` → `target/release/mill`
- `/usr/local/bin/codebuddy` → `/usr/local/bin/mill`

### 4. Environment Variables

**Prefix Migration:**
- `CODEBUDDY__*` (multilevel config) → `TYPEMILL__*`
- `CODEBUDDY_*` (CLI/runtime helpers) → `TYPEMILL_*`

**Migration Strategy:**
- Maintain dual-read support for legacy variables for at least two release cycles
- Emit structured `warn!` logs when legacy variables are detected
- Provide a one-time migration helper (`mill env migrate`) to rewrite `.env`/shell exports
- Update docs and examples to prefer the new prefix while noting backward compatibility
- Coordinate updates across `cb-core` config loaders and `cb-client` CLI parsing (see `crates/cb-core/src/config.rs` and `crates/cb-client/src/client_config.rs`)

### 5. Documentation Updates

**Files to Update:**
- `README.md` - Project name, CLI examples, installation
- `CLAUDE.md` / `AGENTS.md` - All references to project name and CLI
- `API_REFERENCE.md` - Package names and examples
- `CONTRIBUTING.md` - Development workflow references
- All `docs/**/*.md` files
- `Cargo.toml` - Package metadata
- `package.json` (if exists) - NPM package name

**Examples in Documentation:**
```bash
# Old examples
cargo run --bin codebuddy
./target/release/codebuddy setup

# New examples
cargo run --bin mill
./target/release/mill setup
```

### 6. Code References

**Rust Code:**
- Module imports: `use codebuddy::*` → `use typemill::*`
- Binary targets in `Cargo.toml`
- Error messages and help text
- Log messages mentioning project name

**Configuration Examples:**
- JSON schema references
- Sample configurations
- Docker compose files

### 7. Infrastructure

**Docker:**
- Image names: `codebuddy:latest` → `typemill:latest`
- Container names in docker-compose
- Volume mount paths

**GitHub/CI:**
- Repository name (if applicable)
- GitHub Actions workflow references
- Release artifact names

**Homebrew/Package Managers:**
- Formula/package names
- Installation paths

## Implementation Plan

### Phase 1: Preparation (Week 1)

1. **Backup and Branch**
   ```bash
   git checkout -b rename-to-typemill
   git tag pre-typemill-rename
   ```

2. **Impact Analysis**
   - Run global search for all instances of `codebuddy`, `cb-*`, `.codebuddy`
   - Document external dependencies (CI, deployment scripts, user guides)
   - Identify breaking changes for users
   - Inventory all `CODEBUDDY_*` and `CODEBUDDY__*` environment variables in code and docs

3. **Communication Plan**
   - Draft migration guide for existing users
   - Prepare changelog entry
   - Update README with migration notice

### Phase 2: Core Rename (Week 2)

**Priority 1: Cargo Workspace**
1. Rename all `crates/cb-*` directories to `crates/mill-*`
2. Update `Cargo.toml` in each crate:
   ```toml
   [package]
   name = "mill-server"  # was cb-server
   ```
3. Update workspace root `Cargo.toml`
4. Update all internal imports across crates

**Priority 2: Binary and CLI**
1. Rename binary target in root `Cargo.toml`:
   ```toml
   [[bin]]
   name = "mill"  # was codebuddy
   ```
2. Update CLI help text and error messages
3. Update clap command definitions

**Priority 3: Configuration**
1. Update config path logic to use `.typemill/`
2. Add migration code to auto-detect and migrate `.codebuddy/` → `.typemill/`
3. Update all config examples and schemas

**Priority 4: Environment Variables**
1. Extend config loaders (`cb-core`) and CLI parsing (`cb-client`) to read both `CODEBUDDY*` and `TYPEMILL*`
2. Emit structured warnings when legacy prefixes are used to encourage migration
3. Implement the `mill env migrate` helper to rewrite `.env`/shell export files
4. Update acceptance tests to cover dual-prefix support and warning output

### Phase 3: Documentation (Week 2-3)

1. **Core Documentation**
   - Update `README.md`
   - Update `CLAUDE.md` / `AGENTS.md`
   - Update `API_REFERENCE.md`
   - Update `CONTRIBUTING.md`

2. **Technical Documentation**
   - Update all `docs/**/*.md` files
   - Update architecture diagrams
   - Update workflow examples

3. **Code Examples**
   - Update all code snippets in documentation
   - Update integration test examples
   - Update plugin development guides
   - Document new `TYPEMILL__*` / `TYPEMILL_*` environment variables and migration guidance

### Phase 4: Infrastructure (Week 3)

1. **Docker**
   - Update Dockerfiles
   - Update docker-compose.yml
   - Update image tags and names

2. **CI/CD**
   - Update GitHub Actions workflows
   - Update release scripts
   - Update artifact names

3. **Package Management**
   - Update Homebrew formula (if exists)
   - Update installation scripts
   - Update package metadata

### Phase 5: Testing and Validation (Week 4)

1. **Functionality Testing**
   ```bash
   cargo build --release
   ./target/release/mill setup
   ./target/release/mill status
   # Test all CLI commands
   ```

2. **Integration Testing**
   ```bash
   cargo nextest run --workspace --all-features
   ```

3. **Documentation Review**
   - Verify all links work
   - Check all code examples compile
   - Validate configuration examples

4. **Migration Testing**
   - Test upgrade path from `.codebuddy/` to `.typemill/`
   - Verify backward compatibility where needed
   - Document breaking changes

### Phase 6: Release (Week 5)

1. **Pre-Release**
   - Create detailed CHANGELOG entry
   - Write migration guide
   - Update version number (consider major version bump: 1.0.0 → 2.0.0)

2. **Release**
   - Merge rename branch to main
   - Tag release: `v2.0.0-typemill`
   - Publish to crates.io (if applicable)
   - Update package registries

3. **Post-Release**
   - Announce rename on relevant channels
   - Monitor for issues
   - Support users with migration questions

## Migration Path for Users

### Automatic Migration

The tool will automatically detect and migrate:
```bash
# On first run of new version
mill setup
# → Detects .codebuddy/ directory
# → Offers to migrate to .typemill/
# → Preserves all configuration
```

### Manual Migration

Users can manually migrate:
```bash
# Backup old config
cp -r .codebuddy .codebuddy.backup

# Rename directory
mv .codebuddy .typemill

# Update any custom scripts
sed -i 's/codebuddy/mill/g' scripts/*.sh
```

### Backward Compatibility

**Deprecation Period (Optional):**
- Keep `codebuddy` as symlink to `mill` for 2-3 releases
- Show deprecation warning when `codebuddy` command is used
- Remove symlink in major version bump (v3.0.0)

## Breaking Changes

### For End Users

1. **CLI Command Change**
   - All scripts using `codebuddy` must change to `mill`
   - Shell aliases and shortcuts need updating

2. **Configuration Directory**
   - `.codebuddy/` → `.typemill/`
   - Automatic migration provided

3. **Binary Name**
   - Installation paths change
   - System PATH may need adjustment

### For Developers/Contributors

1. **Import Paths**
   - All `use codebuddy::*` → `use typemill::*`
   - Crate dependencies updated

2. **Crate Names**
   - All `cb-*` → `mill-*`
   - Affects plugin development

3. **Repository Structure**
   - Directory names changed
   - Update local development setups

## Risks and Mitigations

### Risk 1: User Confusion
**Impact**: Medium
**Mitigation**:
- Clear migration guide
- Deprecation warnings
- Comprehensive changelog
- Consider keeping old binary name as alias temporarily

### Risk 2: SEO and Discoverability
**Impact**: Low
**Mitigation**:
- Redirect old documentation URLs
- Update all external references
- Maintain old repository name redirects

### Risk 3: Broken External Integrations
**Impact**: High
**Mitigation**:
- Survey known integrations before rename
- Provide migration timeline (not immediate)
- Maintain backward compatibility symlinks
- Update integration examples in documentation

### Risk 4: Build System Disruption
**Impact**: Medium
**Mitigation**:
- Comprehensive testing before merge
- Staged rollout (internal testing first)
- Clear rollback plan (git tag before rename)

## Success Criteria

1. ✅ All tests pass with new names
2. ✅ All documentation updated and accurate
3. ✅ CLI commands work with `mill` prefix
4. ✅ Migration path tested and documented
5. ✅ No regression in functionality
6. ✅ Docker builds succeed with new names
7. ✅ Package registries updated (if applicable)
8. ✅ Users can successfully migrate from old version

## Timeline Summary

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| Preparation | Week 1 | Impact analysis, backup, communication plan |
| Core Rename | Week 2 | Cargo workspace, binary, config updated |
| Documentation | Week 2-3 | All docs updated, examples verified |
| Infrastructure | Week 3 | Docker, CI/CD, packages updated |
| Testing | Week 4 | Full test suite, migration testing |
| Release | Week 5 | Release, announce, support users |

**Total Duration**: 5 weeks

## Open Questions

1. **Version Bump Strategy**: Should this be v2.0.0 (major) or v1.x.0 (minor)?
   - Recommendation: **v2.0.0** (breaking change for CLI command)

2. **Deprecation Period**: How long should we maintain `codebuddy` symlink?
   - Recommendation: **2-3 releases** or **6 months**, whichever is longer

3. **Repository Name**: Should GitHub repository also be renamed?
   - Recommendation: **Yes**, with automatic redirect from old name

4. **NPM Package** (if applicable): Claim `typemill` package name?
   - Recommendation: **Reserve name early** to prevent squatting

5. **Domain Strategy**: How to launch the .org and .com sites?
   - Recommendation: **Launch typemill.org immediately** with CLI docs; point typemill.com to "coming soon"

## Alternatives Considered

### Alternative 1: Keep `codebuddy` name
**Pros**: No migration effort, no user disruption
**Cons**: Misses opportunity for better branding, name conflicts persist

### Alternative 2: Rename to something else
**Other names considered**:
- `codemason` - Building metaphor, but less precise
- `forgemill` - Emphasizes crafting, but verbose
- `lspmill` - Too technical, less approachable
- `refinemill` - Clear purpose, but redundant with "mill"

**Why TypeMill wins**: Best balance of brevity, meaning, and technical accuracy

### Alternative 3: Gradual rename (keep both names)
**Pros**: Easier migration path
**Cons**: Confusing documentation, technical debt, diluted brand

## Conclusion

Renaming to **TypeMill** with CLI command **mill** provides:
- ✅ Better brand identity and memorability
- ✅ Improved CLI ergonomics (shorter command)
- ✅ Clearer technical positioning
- ✅ Professional and distinctive naming
- ✅ Better SEO and discoverability

**Recommendation**: **Approve and proceed** with phased implementation plan.

---

## Next Steps

1. **Gather feedback** on this proposal from team and stakeholders
2. **Finalize timeline** based on project priorities
3. **Begin Phase 1** (Preparation) once approved
4. **Track progress** using project management tools
5. **Update this document** with decisions on open questions

## Appendix A: Search Patterns for Rename

```bash
# Find all references to codebuddy
rg "codebuddy" --type rust
rg "codebuddy" --type md
rg "cb-[a-z]+" --type rust
rg "\.codebuddy"

# Find cargo.toml files
fd Cargo.toml

# Find all documentation
fd -e md

# Find configuration examples
rg "\.codebuddy" -g "*.md" -g "*.json"
```

## Appendix B: Critical Files Checklist

- [ ] `Cargo.toml` (root workspace)
- [ ] `Cargo.toml` (all crate manifests)
- [ ] `README.md`
- [ ] `CLAUDE.md` / `AGENTS.md`
- [ ] `API_REFERENCE.md`
- [ ] `CONTRIBUTING.md`
- [ ] All `docs/**/*.md`
- [ ] `Dockerfile`
- [ ] `docker-compose.yml`
- [ ] `.github/workflows/*.yml`
- [ ] All Rust source files (imports)
- [ ] CLI help text and error messages
- [ ] Test files and examples
- [ ] Configuration schema files

## Appendix C: Communication Templates

### Migration Guide Template
```markdown
# Migrating from codebuddy to mill

Version 2.0.0 introduces a new name: **TypeMill** (CLI: `mill`)

## Quick Migration

1. Update CLI commands: `codebuddy` → `mill`
2. Configuration automatically migrates on first run
3. Update scripts and aliases

See full guide: [MIGRATION.md](MIGRATION.md)
```

### Changelog Entry Template
```markdown
## [2.0.0] - 2025-XX-XX

### BREAKING CHANGES
- **Project renamed to TypeMill** - CLI command is now `mill` instead of `codebuddy`
- Configuration directory changed from `.codebuddy/` to `.typemill/`
- All crate names updated from `cb-*` to `mill-*`

### Migration
- Run `mill setup` to automatically migrate configuration
- Update scripts to use `mill` command
- See [MIGRATION.md](MIGRATION.md) for detailed guide
```
