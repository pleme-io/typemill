# Auto-Download LSP Servers

## Problem

Users must manually install LSP servers (typescript-language-server, rust-analyzer, pylsp) before TypeMill can function. This creates friction:
- Requires knowledge of which LSP servers are needed
- Multiple package managers (npm, rustup, pip)
- Installation steps vary by platform
- Breaks "just works" experience

## Solution

Implement on-demand LSP auto-download with zero-configuration setup:

1. **One-line install** - Single command installs TypeMill binary only (5MB)
2. **Auto-detection** - Scan project on first run, detect languages from file extensions and manifest files
3. **Smart download** - Download only needed LSP servers to `~/.mill/lsp/`
4. **Graceful fallback** - Use system-installed LSPs if available, download if missing
5. **Caching** - Reuse downloaded LSPs across projects

## Checklists

### Phase 1: Detection & Download Infrastructure

- [ ] Add language detection logic to CLI
  - [ ] Scan for `.ts`, `.tsx`, `.js`, `.jsx` files (TypeScript)
  - [ ] Scan for `Cargo.toml`, `.rs` files (Rust)
  - [ ] Scan for `.py`, `pyproject.toml` files (Python)
- [ ] Create LSP downloader module
  - [ ] Download rust-analyzer from GitHub releases
  - [ ] Download typescript-language-server from npm registry
  - [ ] Download pylsp from PyPI
  - [ ] Verify checksums/signatures
- [ ] Implement `~/.mill/lsp/` cache directory
  - [ ] Store downloaded LSP binaries
  - [ ] Version management
  - [ ] Cleanup old versions

### Phase 2: CLI Commands

- [ ] Add `mill install-lsp <language>` command
  - [ ] Manual LSP installation
  - [ ] Force re-download with `--force` flag
- [ ] Update `mill setup` command
  - [ ] Auto-detect languages on first run
  - [ ] Download missing LSPs automatically
  - [ ] Generate `.typemill/config.json` with LSP paths
- [ ] Add progress indicators
  - [ ] Download progress bars
  - [ ] Clear success/error messages
  - [ ] Minimize output verbosity

### Phase 3: Install Script Enhancement

- [ ] Update `install.sh` script
  - [ ] Single command: `curl -fsSL https://typemill.org | bash`
  - [ ] Auto-detect OS and architecture
  - [ ] Download correct mill binary
  - [ ] Add to PATH automatically (`.bashrc`, `.zshrc`, etc.)
- [ ] Add `mill update` command
  - [ ] Check for newer mill version
  - [ ] Self-update binary
  - [ ] Update LSP servers

### Phase 4: GitHub Actions for LSP Hosting

- [ ] Create LSP mirror workflow
  - [ ] Download LSP binaries from upstream
  - [ ] Host on GitHub Releases or separate repo
  - [ ] Provide fast CDN delivery
- [ ] Add fallback sources
  - [ ] Primary: typemill.org CDN
  - [ ] Fallback: Upstream GitHub releases

### Phase 5: Configuration & Validation

- [ ] Update config generation
  - [ ] Auto-populate LSP paths in `.typemill/config.json`
  - [ ] Detect system-installed LSPs first
  - [ ] Prefer `~/.mill/lsp/` if system version missing
- [ ] Add validation checks
  - [ ] Verify LSP binary is executable
  - [ ] Test LSP server initialization
  - [ ] Graceful error messages if LSP fails

### Phase 6: Documentation

- [ ] Update README.md
  - [ ] One-line install command
  - [ ] Remove manual LSP installation steps
- [ ] Update CLAUDE.md
  - [ ] Document auto-download behavior
  - [ ] Manual override instructions
- [ ] Add troubleshooting guide
  - [ ] LSP download failures
  - [ ] Network/firewall issues
  - [ ] Manual installation fallback

## Success Criteria

- [ ] Fresh Ubuntu box: `curl | bash && mill` works without additional commands
- [ ] TypeScript project: `mill setup` auto-downloads typescript-language-server
- [ ] Rust project: `mill setup` auto-downloads rust-analyzer
- [ ] Multi-language project: Downloads only detected LSPs
- [ ] Second project: Reuses cached LSPs (no re-download)
- [ ] Offline after first download: Works without internet
- [ ] Install time: <30 seconds for binary + 1 LSP

## Benefits

- **Zero manual setup** - Install and use in one command
- **Bandwidth efficient** - Download only what's needed (5MB binary + ~15-50MB per LSP vs 80MB bundle)
- **Disk efficient** - Shared LSP cache across projects
- **Faster onboarding** - New developers productive in <1 minute
- **Platform agnostic** - Works on Linux, macOS, Windows without package managers
- **Self-contained** - All dependencies in `~/.mill/` directory
- **Clean uninstall** - `rm -rf ~/.mill ~/.local/bin/mill` removes everything
