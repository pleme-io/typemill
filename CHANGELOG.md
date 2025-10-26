# Changelog

All notable changes to TypeMill will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## Rust Implementation (Current)

The project underwent a complete architectural transformation from TypeScript/Node.js to pure Rust in 2025, bringing native performance, memory safety, and compile-time type guarantees.

### [Unreleased]

### [0.7.0] - 2025-10-24

🚀 **Version 0.7.0** - TypeMill rebranding, API standardization, and comprehensive rename architecture

#### Breaking Changes

- **TypeMill Branding Migration** - Complete rename from mill to TypeMill
  - Configuration directory: `.mill/` → `.typemill/`
  - Binary name: `mill` → `mill` (apps/mill → apps/mill)
  - Environment variables: `TYPEMILL__*` → `TYPEMILL__*`
  - All internal crates renamed: `cb-*` → `mill-*` (27 crate renames)
  - Package name: `mill` → `typemill`

- **JSON API Standardization** - All JSON parameters converted to camelCase
  - 487 field replacements across codebase for consistency
  - Documentation updated with 153 camelCase conversions
  - Affects all MCP tool parameters and responses
  - CLI maintains both snake_case flags and camelCase JSON compatibility

- **Scope Architecture Changes**
  - Scope preset renamed: `"project"` → `"standard"`
  - Deprecated presets (`"code-only"`, `"all"`) still work with warnings

#### Added

- **New Scope Architecture** - Comprehensive rename scope control system
  - 5 scope presets: `code`, `standard`, `comments`, `everything`, `custom`
  - CLI help text with detailed examples and usage patterns
  - E2E tests validating all scope modes and edge cases
  - Fine-grained control over what files get updated during renames

- **workspace.find_replace Tool** - Find and replace text across workspace
  - 100% test coverage with edge case handling
  - UTF-8 safe character boundary handling
  - MCP protocol-compliant response format
  - Health check polling for reliable test infrastructure

- **Batch Rename Support** - Enhanced `targets` parameter
  - Rename multiple files/directories in single operation
  - Comprehensive documentation with examples
  - Conflict detection and validation

- **Tool Access Control** - Internal tools blocked from CLI/MCP
  - Clean separation of public API (36 tools) vs internal tools (20 tools)
  - Prevents accidental usage of internal plumbing tools
  - Better security and API surface management

#### Changed

- **Test Infrastructure Modernization** - Comprehensive test consolidation
  - Closure-based test helpers (Solution A pattern)
  - Migration of 32 test files to new architecture
  - Removed 5 superfluous test files (-302 lines)
  - LSP integration tests gated behind `lsp-tests` feature
  - Fixed 5s sleep → health check polling for reliability

- **Documentation Overhaul** - Category-based organization
  - Split tool reference into 5 category files
  - Added table of contents to all tool docs
  - Fixed critical accuracy issues in AI assistant docs
  - Documentation consolidation with proper README structure
  - Fixed broken internal links and case sensitivity issues

- **Rust Crate Organization** - Complete mill-* naming convention
  - Analysis crates: cb-analysis-* → mill-analysis-* (5 crates)
  - Language plugins: cb-lang-* → mill-lang-* (5 crates)
  - Core crates: mill-* → mill-* (17 crates)
  - Foundation crates: cb-plugin-api → mill-plugin-api, etc.

#### Fixed

- **Rust Import Rewrite System** - Complete overhaul for reliability
  - Fixed incomplete Cargo.toml updates in directory renames
  - Support for `extern crate` and `pub use` patterns
  - Handle bare crate names in feature arrays (`crate/feature` syntax)
  - Guard format string escaping in import rewriter
  - Fix workspace.dependencies key updates during crate rename
  - Batch rename now collects `documentChanges` correctly

- **Test Suite Fixes** - 100% pass rate maintained
  - Fixed 13 tests for camelCase JSON field expectations
  - Fixed 3 error handling tests for protocol errors
  - Fixed 3 deep dead code tests
  - Fixed 2 workspace.extract_deps tests
  - Updated test expectations for camelCase field names

- **CLI & Configuration** - Environment variable handling
  - Config loader uses `TYPEMILL__` prefix
  - Removed legacy CLI flags for clean minimal API
  - Standardized JSON parameters to snake_case, then to camelCase

#### Removed

- **Legacy Naming** - All references to old TypeMill branding
  - Removed `.mill/` configuration directory support
  - Removed `mill` binary name (use `mill` instead)
  - Removed `TYPEMILL__*` environment variable support
  - Cleaned up stale nested crates in mill-lang-markdown

#### Migration Notes

- **Configuration**: Move `.mill/` to `.typemill/` in your projects
- **Binary**: Update scripts and commands from `mill` to `mill`
- **Environment**: Rename `TYPEMILL__*` to `TYPEMILL__*` in your environment
- **JSON API**: Update client code to use camelCase for all JSON parameters
- **Scope**: Update `"project"` to `"standard"` (old name deprecated with warning)

---

### [0.6.1] - 2025-10-22

🚀 **Version 0.6.1** - Rename tool enhancements, quick refactoring operations, and comprehensive comment/prose updates

#### Added

- **CLI convert-naming Command** - Bulk naming convention conversion tool
  - Convert between kebab-case, snake_case, camelCase, PascalCase
  - Supports batch file renaming with convention transformations
  - Standalone utility for naming convention migration

- **Batch Rename Support** - Enhanced rename tool with batch operations (unified dryRun API)
  - Multiple file/directory renames in single operation
  - Coordinated reference updates across batch
  - Optimized for large-scale refactoring projects

- **Quick Refactoring Operations** - One-step plan+execute tools
  - Generic `QuickRefactoringHandler` eliminates code duplication
  - 7 quick tools: `rename`, `extract`, `inline`, `move`, `reorder`, `transform`, `delete`
  - Same parameters as `*.plan` versions but auto-apply changes
  - CLI flag support for convenient command-line usage

- **Comment and Prose Updates** - Comprehensive rename coverage for documentation
  - Rust plugin: Phase 3 comment scanning with smart boundary matching
  - TOML plugin: Comment updates in configuration files
  - Markdown plugin: Prose identifier updates in documentation
  - Smart boundary regex for hyphenated identifiers: `(?<![a-zA-Z0-9])identifier(?![a-zA-Z0-9])`
  - Opt-in via `update_comments` and `update_markdown_prose` flags

- **Comprehensive Scope Mode** - 100% file coverage for `--update-all`
  - Auto-upgrade to custom scope when update flags are present
  - Scans ALL files matching scope filters (no reference detection gaps)
  - Plugins receive merged rename_info with all scope flags
  - Ensures no files missed due to detection heuristics

- **stdin and File Input Support** - Handle large JSON payloads for tool command
  - `--input-file <path>` reads arguments from file
  - Args value `"-"` reads from stdin
  - Originally for two-step workflow (renamed to unified API in Phase 5)
  - Solves shell argument limit issues (600KB+ plans)

#### Fixed

- **Multi-line Grouped Import Rewriting** - Rust plugin correctly handles multi-line import groups
- **Cargo Crate Rename Edge Cases** - Fixed two critical bugs:
  - Feature flag references (`crate-name/feature` and `dep:crate-name` syntax)
  - Self-referencing imports within renamed crate (hyphen→underscore conversion)
- **File Path Handling** - Edits to files inside renamed directories now target correct new paths
- **Mod Declaration Detection** - Works for projects without Cargo.toml (uses directory name fallback)
- **CLI Flag Parser** - `--update-all` flag now works without requiring `--scope custom`
- **Quick Tool Flags** - All 7 quick refactoring tools now support CLI flags

#### Changed

- **Internal Crate Renames** - Dogfooding rename tool with mill-* naming convention:
  - `cb-server` → `mill-server` (core MCP server)
  - `cb-lsp` → `mill-lsp` (LSP integration layer)
  - `cb-handlers` → `mill-handlers` (tool handlers)
  - `cb-services` → `mill-services` (business logic)
  - `cb-transport` → `mill-transport` (WebSocket/stdio)
  - `cb-client` → `mill-client` (client library)

#### Documentation

- **Version Numbering Clarification** - Separated package version (0.6.x) from API version (1.0.0-rcX)
- **Removed operations.md References** - Cleaned up references to non-existent documentation
- **Quick Operations Documentation** - Added complete docs for all 7 quick refactoring tools
- **Tool Count Updates** - Updated from 23 → 35 public MCP tools across all docs
- **Architecture Documentation** - Synced AGENTS.md and GEMINI.md with latest architecture

---

### [0.6.0] - 2025-10-21

🚀 **Version 0.6.0** - Plugin architecture modernization and comprehensive refactoring/analysis APIs

#### Added

- **Capability Trait Pattern** - Modern plugin architecture with zero compile-time feature flags
  - `ManifestUpdater`, `ModuleLocator`, `RefactoringProvider` capability traits
  - File-extension-based automatic routing to language plugins
  - Eliminated all `cfg` guards and downcasting from shared code
  - True plug-and-play plugin system with compile-time type safety

- **Dependency Injection for Plugins** - Complete architectural decoupling
  - Plugin registry now injected throughout service layer
  - Eliminated global plugin state and compile-time coupling
  - Language plugins fully decoupled from core system

- **Comprehensive Rename Coverage** - 100% coverage of affected references across multiple file types
  - String literal path updates in Rust files
  - Markdown link updates in documentation
  - TOML/YAML config file updates for build configs and CI/CD
  - Smart path detection (requires `/` or file extension)
  - All edits surface in rename dry-run (options.dryRun: true) for review

- **Unified Refactoring API** - Integrated dryRun option (migrated to unified API in Phase 5)
  - Originally: Two-step pattern with `.plan` tools and `workspace.apply_edit`
  - Now: Single tools with `options.dryRun: true/false`
  - Safe dry-run previews without filesystem modifications
  - Atomic execution with rollback on errors

- **Unified Analysis API** - 6 analysis tools with 26 detection kinds
  - `analyze.quality` - complexity, smells, maintainability, readability
  - `analyze.dead_code` - unused imports/symbols/parameters/variables/types
  - `analyze.dependencies` - imports, circular deps, coupling, cohesion
  - `analyze.structure` - symbols, hierarchy, interfaces, inheritance
  - `analyze.documentation` - coverage, quality, style, examples
  - `analyze.tests` - coverage, quality, assertions, organization
  - `analyze.batch` - optimized multi-file analysis with AST caching
  - Configuration via `.typemill/analysis.toml` with 3 presets

- **Additional Language Plugins** - Markdown, TOML, YAML plugins for comprehensive rename support
- **Build Automation** - xtask pattern with cross-platform Rust tasks (`cargo xtask install`, `check-all`, etc.)
- **Dependency Auditing** - cargo-deny integration for security and license checks

#### Changed

- **Plugin Architecture** - Runtime plugin discovery replaces compile-time coupling
- **Single-language Builds** - Optional feature flags for focused development (`lang-rust`, `lang-typescript`)
- **Crate Consolidation** - Merged multiple crates into `mill-foundation` and `mill-plugin-system`

#### Fixed

- **LSP Zombie Processes** - Comprehensive prevention with proper cleanup and shutdown
- **Clippy Warnings** - Resolved all 16 warnings across workspace
- **Import Updates** - Fixed cross-workspace and cross-crate import detection
- **Rename Scope** - `find_project_files` now respects RenameScope for complete coverage
- **Plugin Discovery** - Fixed discovery in isolated test packages

#### Removed

- **Legacy Refactoring Tools** - Replaced by unified API (evolved to dryRun option in Phase 5)
- **Dead-Weight Analysis Tools** - Removed tools fully covered by unified analysis API
- **Internal Tool Count** - Reduced from 25 → 20 tools

---

### [0.5.0] - 2025-10-10

🚀 **Version 0.5.0** - Temporary language reduction for unified API refactoring

#### Breaking Changes

- **Temporary Language Reduction** - Language support temporarily reduced to TypeScript + Rust
  - **BREAKING**: Python, Go, Java, Swift, and C# language plugins temporarily removed from codebase
  - Removed to enable focused refactoring on unified API architecture
  - All language plugin code preserved in git tag `pre-language-reduction` for future restoration
  - **Impact**: Only TypeScript and Rust projects supported in this release
  - **Timeline**: Multi-language support to be restored after unified API implementation complete

#### Changed

- **Language Support Matrix** - Updated all documentation to reflect TypeScript + Rust focus
- **Test Infrastructure** - Simplified test harness for two-language focus
- **Build Configuration** - Updated Makefile and build scripts for TS + Rust

#### Documentation

- **Comprehensive Documentation Updates** - All docs updated for language reduction with disclaimers and git tag references
- **API Contracts and Proposals** - Refined unified API implementation plans
- **Plugin Development Guide** - Updated to use `mill_plugin!` macro instead of `languages.toml`

#### Added

- **Plugin Self-Registration System** - Self-registering language plugins with link-time discovery
  - New `cb-plugin-registry` crate with `PluginDescriptor` and `mill_plugin!` macro
  - Plugins self-register using `inventory` crate for automatic discovery at link time
  - Core crates (`cb-core`, `mill-services`, `cb-ast`) completely decoupled from specific languages
  - No more `languages.toml` or build-time code generation required
  - Adding/removing languages requires no core crate changes - just link the crate
  - Contract tests/e2e automatically validate all discovered plugins

#### Changed

- **Plugin Architecture** - Complete decoupling of language plugins from core system
  - Replaced build-time code generation with runtime discovery via `iter_plugins()`
  - Language detection now iterates over registered plugins dynamically
  - Registry builder discovers all plugins at startup automatically
  - Enhanced registry builder with validation for duplicate names and extension conflicts
  - Removed unused `lazy_static` dependency from cb-plugin-registry

#### Removed

- **Language Plugin Source Code** - Temporarily removed 5 language plugins: Python, Go, Java, Swift, and C# (preserved in git tag `pre-language-reduction`)
- **Language-Specific Tests** - Removed tests for deleted languages
- **Build Scripts** - Removed `languages.toml` and all language-related `build.rs` files from core crates

#### Migration Notes

- **For users needing Python/Go/Java/Swift/C# support**: Use git tag `pre-language-reduction` or version `0.4.0`
- **For contributors**: Multi-language support will be restored in future release after unified API implementation
- **Git tag preservation**: `git checkout pre-language-reduction` to access full multi-language implementation
- **Plugin developers**: Use `mill_plugin!` macro in your plugin's `lib.rs` to enable self-registration

---

### [0.4.0] - 2025-10-09

🚀 **Version 0.4.0** - Analysis subsystem architecture, Go refactoring parity, and major setup simplification

#### Added

- **Analysis Subsystem Architecture** - Complete implementation of advanced analysis subsystem
  - New `analysis/mill-analysis-common` crate with shared traits (`AnalysisEngine`, `LspProvider`)
  - New `analysis/mill-analysis-deep-dead-code` crate with dependency graph analysis
  - Cross-file dead code detection with import/export tracking
  - Configurable analysis via feature flags (`analysis-dead-code`)
  - Trait-based architecture for dependency inversion and extensibility

- **Go Language Refactoring Parity** - Go now has full AST-based refactoring support (4 of 7 languages complete: TypeScript, Python, Rust, Go)

- **Dev Container Support** - VS Code Dev Container and GitHub Codespaces configuration with pre-configured Rust toolchain
- **Version Flag** - Added `--version` flag to CLI

#### Changed

- **Radically Simplified Setup** - Setup flow reduced from 6 paths to 2 paths (eliminated 674 lines of complexity)
- **Documentation Migration** - Completed migration to cargo-nextest across all docs

#### Fixed

- **Rust/Python AST Refactoring** - Fixed routing layer to properly dispatch to language-specific implementations
- **Import Update Assertions** - Skip import update assertions in tests without LSP support

#### Removed

- **Jules Duplicate Crates** - Removed duplicate Jules crates from workspace
- **Temporal References** - Proposals now use priority numbers instead of dates

---

### [0.3.0] - 2025-10-09

🚀 **Version 0.3.0** - Swift language support, comprehensive documentation overhaul, build/test optimizations, and critical import handling fixes

#### Added

- **Advanced MCP Analysis Tools** - Added 4 new tools: `find_unused_imports`, `optimize_imports`, `analyze_complexity`, `suggest_refactoring`
- **Cognitive Complexity Metrics** - Enhanced code metrics with cognitive complexity scoring

- **Enhanced `rename_directory` Workspace Operations** - Auto-update Cargo.toml path dependencies and manifest updates
- **mill-lang-common Utility Library** - Shared utility modules for language plugins with ImportGraph builder

- **Swift language support** - Complete implementation with AST-based parsing, import manipulation, and Swift Package Manager support
- **Build and test performance optimizations** - Added test feature flags (`fast`/`lsp`/`e2e`/`heavy`), cargo-nextest support, and build configuration improvements
- **Import support refactoring** - Refactored import support across all 6 language plugins (260 lines saved, 15% reduction, zero regressions)

#### Changed

- **Language Plugin Integration** - All language plugins now integrate with mill-lang-common utilities
- **Crate Organization** - Moved all language plugins to flat `crates/` layout for consistency

#### Fixed

- **Import Handling** - Fixed duplicate imports, column position errors, and malformed spacing in `rename_directory` operations
- **Testing Infrastructure** - Fixed 4 failing tests to achieve 100% test pass rate (550/550 tests passing)

---

### [0.2.0] - 2025-10-05

🚀 **Version 0.2.0** - Plugin architecture modernization, workspace operations, and 5-language support

#### Added

- **Java language support** - Complete implementation with AST-based parsing
  - JavaParser subprocess integration for accurate symbol extraction
  - Import manipulation (add, remove, rewrite, parse package declarations)
  - Maven workspace support (pom.xml multi-module projects)
  - Full ImportSupport and WorkspaceSupport trait implementations

- **Workspace operations for all plugin languages**
  - **Python**: Poetry (`pyproject.toml`), PDM, Hatch workspace support
  - **TypeScript/JavaScript**: npm, yarn, pnpm workspace support
  - **Go**: `go.work` workspace file management
  - **Rust**: Cargo workspace support (existing, enhanced)
  - **Java**: Maven multi-module project support

- **Language plugin development tooling**
  - `new-lang.sh` generator script with auto-integration
  - `check-features.sh` validation script
  - Comprehensive plugin development documentation
  - Reference implementations (Rust, Go, TypeScript, Python, Java)

- **Cross-language testing framework**
  - Parameterized test harness for multi-language refactoring
  - Comprehensive test scenarios for all 5 languages
  - Behavior expectations (Success/NotSupported/PartialSuccess)
  - Language-agnostic test infrastructure

#### Changed

- **Plugin architecture refactored to capability-based traits** - Replaced monolithic plugin with composable traits (29-42% LOC reduction per plugin)
- **Refactoring operations switched to AST-first approach** - AST tried before LSP for faster, more reliable operations (4/5 languages support multiline extract)
- **Simplified language plugin generator** - Reduced `new-lang.sh` from 817 to 607 lines with TOML-based generation
- **LSP infrastructure improvements** - Replaced arbitrary sleeps with smart LSP polling and added hybrid fallback for `find_dead_code`

#### Fixed

- XML event handling in Java workspace module rewriting
- Git operations tests failing with BrokenPipe errors
- EditPlan structure in refactoring implementations
- Python plugin delegation and manifest support

---

### [0.1.0] - 2025-10-03

🎉 **Version 0.1.0** - Production-ready Rust MCP server with comprehensive tooling

#### Added

- **Cross-platform installation script** - Enterprise-grade `install.sh` with support for macOS, Ubuntu/Debian, Fedora/RHEL, and Arch
- **Plugin architecture completion** - Full language adapter migration with composable plugin system
- **Java AST support** - Tree-sitter based parser integration
- **Refactoring tools** - Full AST-based implementation for extract_function, inline_variable, and extract_variable (evolved through unified API phases, now using dryRun option)
- **SWC-based AST parsing** - TypeScript/JavaScript AST parsing with native Rust performance
- **VFS (Virtual Filesystem)** - Optional experimental feature (Unix only, feature-gated)
- **44 MCP Tools** - Complete implementation across all categories

#### Changed

- **Structured logging** - All production code now uses tracing framework with structured key-value logging
- **Error handling** - Replaced all `.unwrap()` calls with `.expect()` containing descriptive messages
- **Dependencies** - Unified thiserror to v2.0 and jsonwebtoken to v10.0 across workspace

#### Fixed

- install.sh now installs git before attempting to clone repository
- Java tests compilation errors and unused imports
- LSP tools plugin delegation in analyze_imports
- Text edits correctly apply to target files in EditPlan

---

## TypeScript/Node.js Implementation History (2024-2025)

The following entries document the original TypeScript/Node.js implementation (v0.1.0 - v1.3.0).
Many features have been reimplemented in Rust with enhanced performance and safety guarantees.

## [1.3.0] - 2025-09-24

### Added
- **🚀 LSP Server Pooling**: Intelligent server resource management with automatic scaling and lifecycle management
  - Multi-language support with separate pools (TypeScript, Python, Go, etc.)
  - Project isolation preventing cross-project interference
  - Automatic scaling up to configured limits per language
  - Idle cleanup with configurable timeout periods
  - Crash recovery with automatic restart and request queuing
  - **Performance Impact**: Eliminates LSP server startup time (200-1000ms savings per request)

- **⚡ Predictive Loading System**: Proactive file loading for reduced LSP operation latency
  - TypeScript AST parsing to extract import statements (ES6, CommonJS, dynamic imports)
  - Multi-strategy import resolution (exact match, extensions, index files)
  - Background file preloading to warm LSP servers
  - Intelligent caching with deduplication to prevent redundant work
  - **Performance Impact**: 99.9% faster LSP operations on imported files (validated with crates/cb-bench)

- **📦 Client Package**: Complete WebSocket client SDK with CLI and library interfaces
  - WebSocket client with automatic reconnection and session management
  - MCP proxy for transparent protocol bridging
  - CLI tools for direct command-line interaction
  - HTTP proxy for REST-style access to MCP tools
  - Configuration management with persistent settings
  - Comprehensive test suite ensuring reliability

- **🧪 Performance Testing Suite**: Real benchmarking infrastructure with statistical validation
  - Dynamic temporary test environments preventing caching artifacts
  - Statistical analysis with multiple iterations and confidence intervals
  - Authentic LSP operations using real language servers (no mocks)
  - Automated performance regression detection
  - **Results**: Validated 99.9% improvement from predictive loading (1004ms → 0.9ms average)

- **📚 Architecture Documentation**: Professional-grade documentation with interactive diagrams
  - LSP Server Pooling documentation with sequence diagrams
  - Predictive Loading documentation with detailed flowcharts
  - Configuration guides with default values and optimization tips
  - Performance characteristics and resource usage analysis
  - Integration points and monitoring capabilities

### Enhanced
- **Type Safety**: Comprehensive TypeScript improvements with stricter type checking
- **Code Quality**: Major lint cleanup reducing warnings by 66%
- **Configuration System**: Formalized server options with structured configuration interfaces
- **Error Handling**: Graceful degradation ensuring predictive loading never blocks main operations
- **Testing Infrastructure**: Enhanced test runner with system capability detection and performance optimization

### Performance
- **LSP Operations**: Up to 99.9% faster operations on preloaded files
- **Resource Usage**: Optimized memory consumption with intelligent server pooling
- **Startup Time**: Eliminated repeated LSP server initialization overhead
- **Network Efficiency**: Reduced redundant file loading through predictive caching
- **Throughput**: Improved concurrent request handling through server reuse

### Configuration
- **LSP Server Pooling**:
  - `maxServersPerLanguage`: 2 (default) - Maximum concurrent servers per language
  - `idleTimeoutMs`: 60000 (default) - Idle server timeout in milliseconds
  - `crashRestartDelayMs`: 2000 (default) - Delay before restarting crashed servers

- **Predictive Loading**:
  - `enablePredictiveLoading`: true (default) - Master enable/disable switch
  - `predictiveLoadingDepth`: 0 (default) - Import recursion depth (0 = direct imports only)
  - `predictiveLoadingExtensions`: ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'] - File extensions to process

### Developer Experience
- **package.json**: New `test:performance` script for running performance crates/cb-bench
- **Architecture Docs**: Comprehensive system documentation with Mermaid diagrams
- **Performance Validation**: Tools to verify and measure system improvements
- **Configuration Examples**: Clear configuration patterns and best practices

## [1.2.0] - 2025-09-24

### 🚀 Enterprise Architecture & LSP Server Pooling

This release represents a complete architectural transformation, implementing advanced resource management, enterprise deployment capabilities, and intelligent pooling systems for optimal performance.

### Added
- **🏊 LSP Server Pooling**: Enhanced resource management with intelligent server pooling and lifecycle management
  - **Resource Efficiency**: Max 2 servers per language instead of unlimited
  - **Reduced Latency**: Server reuse eliminates cold start delays
  - **Workspace Isolation**: Servers can be reassigned between workspaces
  - **Intelligent Queuing**: Automatic waiting when pools are at capacity

- **⚡ Performance Enhancements**: Multiple optimization systems for superior performance
  - **Advanced Caching System**: Event-driven cache invalidation with hit rate tracking and persistent file caching
  - **Delta Updates**: Efficient file synchronization using diff-match-patch with automatic compression analysis
  - **Analysis Cache**: Prevents re-computation for workspace symbols

- **🏗️ Architecture Transformation**: Complete restructure for enterprise deployment
  - **Monorepo Structure**: Clean packages/client and packages/server separation
  - **Transaction Manager**: Atomic operations with rollback capabilities
  - **Workflow Orchestrator**: Automated tool chain execution with dependencies
  - **Service Architecture**: Modular service-based design patterns

- **🔧 Enterprise Features**: Production-ready deployment capabilities
  - **WebSocket Server**: Production-ready multi-client enterprise support
  - **JWT Authentication**: Token-based access control with configurable permissions
  - **Health Monitoring**: `/healthz` and `/metrics` endpoints for monitoring
  - **Session Management**: Connection recovery with 60-second grace periods

- **Foundation Features Implementation**: All 6 core features from PROPOSAL_FOUNDATION_FEATURES.md
  - Self-modification detection with auto-restart capabilities
  - Enhanced error context with actionable debugging information
  - Position index consistency with unified LSP positioning
  - Interactive tool debugging with comprehensive validation
  - Tool dependency management with workflow orchestration
  - Rollback & undo system with transaction management and checkpoints

### Enhanced
- **Dead Code Detection**: Advanced analysis using MCP tool orchestration
- **Streaming File Access**: Real-time file change notification with intelligent caching

### Changed
- **Architecture**: Migrated to monorepo structure with packages/client and packages/server
- **Configuration**: Smart setup with auto-detection and gitignore support
- **Performance**: Advanced caching replacing TTL-based expiration with event-driven invalidation

## [1.1.0] - 2025-09-22 - ARM64 Native FUSE Support

### 🏗️ Native FUSE Implementation
- **FUSE Implementation**: Replaced mock FUSE implementation with native `@cocalc/fuse-native` for ARM64 compatibility
- Replaced `fuse-native` with `@cocalc/fuse-native` for ARM64 compatibility
- Updated FUSE mount operations to use callback-style API for compatibility with native library
- Removed all mock FUSE fallback code paths for cleaner architecture
- Now using 100% native FUSE implementation

### 🐳 Multi-Tenant Docker Support
- Production-ready Docker Compose configuration for multi-tenant deployments
- Multi-tenant FUSE folder mounting capabilities
- Session-based workspace isolation
- Automatic cleanup on client disconnect
- Tenant client example implementation
- Quick-start script for multi-tenant FUSE service

### 🛠️ Stability Improvements
- Fixed FUSE native library compatibility issues on ARM64 systems
- Resolved TypeScript type errors in FUSE mount operations
- Fixed test isolation issues in WebSocket FUSE integration tests
- Fixed duplicate `handleSessionDisconnect` method in WebSocket server
- Fixed incorrect `disconnectSession` method call in session cleanup
- Better error handling in session cleanup

### Added
- Full ARM64 architecture support for FUSE operations

### Platform Support
- ✅ x86_64 Linux
- ✅ ARM64 Linux
- ✅ macOS (Intel)
- ✅ macOS (Apple Silicon)
- ✅ Windows (via WSL2)

## [1.0.1] - 2025-09-21

### Changed
- Project renamed from `codeflow-buddy` to `mill` for better clarity
- Updated repository URLs and package references
- Streamlined test suite by removing redundant tests

### Fixed
- Removed superfluous test files to eliminate redundancy
- Fixed PostToolUse hook configuration issues
- Updated documentation to reflect current codebase state

## [1.0.0] - 2025-09-20

### Added
- Major version release with stable API
- Complete MCP tool suite (28 tools)
- Comprehensive test coverage
- Enhanced error handling and recovery

## [0.5.13] - 2025-08-30

### Added
- **JAR File Language Support**: Added language ID mapping for JAR and Java class files
  - `.jar` files now properly mapped to Java language ID
  - `.class` files now properly mapped to Java language ID
  - Enables LSP features for JAR files when Java LSP server is configured

## [0.5.12] - 2025-08-25

### Added

- **InitializationOptions Support**: Added support for passing LSP server initialization options (#15 by @colinmollenhour)
  - New `initializationOptions` field in server configuration for LSP-specific settings
  - Enables passing settings like `pylsp.plugins.pycodestyle.enabled` for Python Language Server
  - Improves LSP server compatibility with servers requiring specific initialization configuration

### Fixed

- **MCP Command Execution**: Fixed argument order and escaping for Claude CLI integration
  - Corrected command argument ordering for proper MCP server registration
  - Fixed path escaping issues with spaces in configuration paths
  - Improved cross-platform compatibility for Windows, macOS, and Linux

## [0.5.10] - 2025-08-22

### Fixed

- **MCP Command Argument Order**: Fixed `claude mcp add` command argument order
  - Corrected to: `claude mcp add mill <command> [args...] --env <env>`
  - Server name and command are now properly positioned as positional arguments
  - Options are placed after the command as required by the CLI
  - Resolves "missing required argument 'commandOrUrl'" error

- **Path Escaping on Non-Windows Platforms**: Fixed path handling for spaces
  - Windows: Continues to use quotes for paths with spaces
  - macOS/Linux: Now escapes spaces with backslashes instead of quotes
  - Ensures proper path handling across all platforms

## [0.5.7] - 2025-08-22

### Fixed

- **Claude CLI Fallback**: Setup script now falls back to `npx @anthropic-ai/claude-code@latest` when Claude CLI is not installed

  - Automatically detects if `claude` command is available
  - Uses npx to run Claude commands without requiring global installation
  - Improves setup experience for users without Claude CLI installed

- **MCP Command Syntax**: Fixed incorrect argument order in MCP add command
  - Options (`--env`, `--scope`) now correctly placed before server name
  - Resolves "unknown option '--env'" error
  - Commands now follow proper Claude MCP CLI syntax

- **Platform-specific Path Quoting**: Fixed config path quoting based on platform (#14)
  - Windows: Paths with spaces are quoted in environment variables
  - macOS/Linux: Paths are not quoted to avoid literal quotes in values
  - Resolves "Config file specified in TYPEMILL_CONFIG_PATH does not exist" error on Unix systems

### Enhanced

- **Setup Robustness**: Improved error handling and fallback mechanisms
  - Better detection of Claude CLI availability
  - Clear messaging when falling back to npx
  - Consistent behavior across all MCP operations (list, remove, add)

## [0.5.6] - 2025-08-20

### Enhanced

- **Path Quoting**: Always quote configuration paths for improved safety
  - Paths are now always quoted regardless of spaces
  - Better handling of special characters in file paths
  - Improved cross-platform compatibility

### Added

- **Execution Tests**: Added comprehensive command execution tests for CI
  - Real command execution simulation with `echo`
  - Verification that quoted paths work correctly in actual execution
  - Integration tests/e2e for MCP command structure
  - New test deployment/scripts: `test:execution` and `test:all`

### Fixed

- **Path Resolution**: Fixed absolute path detection for Windows drive letters
  - Correctly handles paths like `C:\Program Files\...`
  - Prevents unnecessary path resolution for already absolute paths

## [0.5.5] - 2025-08-20

### Fixed

- **Windows Support**: Fixed setup script to properly handle Windows environments
  - Added `cmd /c` prefix for npx commands on Windows platform
  - Ensures correct MCP configuration command generation across all platforms
  - Added comprehensive test coverage for Windows-specific behavior

## [0.5.4] - 2025-08-18

### Added

- **File Editing Capability**: Complete transformation of rename operations from preview-only to actual file modification (PR #13 by @secondcircle)
  - Atomic file operations with automatic backup and rollback support
  - Symlink handling - correctly resolves and edits target files
  - Multi-file workspace edits for complex rename operations across multiple files
  - Comprehensive validation for file existence, permissions, and types
  - `dry_run` parameter for safe preview mode on both `rename_symbol` and `rename_symbol_strict` (legacy tools, evolved through unified API phases, now using options.dryRun)

### Enhanced

- **LSP Server Synchronization**: Improved file synchronization after edits
  - All modified files are properly synced with LSP servers after edits
  - Version tracking for proper LSP protocol compliance
  - Auto-open files that weren't previously opened get opened and synced automatically

### Fixed

- **Multi-file Rename Operations**: Now actually applies rename changes across all affected files instead of just returning preview
- **LSP Document Synchronization**: Fixed sync issues with files modified by rename operations

### Testing

- Added comprehensive test suite for file editing functionality (100+ test cases)
- Implemented CI workarounds for environment-specific test issues

### Acknowledgements

Special thanks to @secondcircle for the major enhancement that transforms mill from a read-only query tool into a functional refactoring tool with actual file editing capabilities (#13). This change significantly improves the user experience from preview-only to actually applying changes.

## [0.5.3] - 2025-08-16

### Fixed

- **Rename Operations**: Fixed rename operations with modern LSP servers like gopls that use DocumentChanges format (PR #11 by @secondcircle)
  - Now properly handles both WorkspaceEdit and DocumentChanges response formats
  - Improved compatibility with language servers using the newer LSP specification

### Documentation

- Updated MCP tools documentation to match current implementation
- Added MseeP.ai badge to README (PR #4 by @lwsinclair)

### Acknowledgements

Special thanks to the contributors of recent enhancements and fixes.

- @secondcircle for fixing the critical rename operation issue with modern LSP servers (#11)
- @lwsinclair for adding the MseeP.ai badge to improve project visibility (#4)
- @maschwenk for the rootDir preloading fix in the previous release (#5)

Your contributions help make mill better for everyone! 🙏

## [0.5.2] - 2025-08-04

### Added

- **Manual Server Restart**: Added `restart_server` MCP tool for manually restarting LSP servers
  - Restart specific servers by file extension (e.g., `["ts", "tsx"]`)
  - Restart all running servers when no extensions specified
  - Detailed success/failure reporting for each server

### Enhanced

- **Server Management**: Improved LSP server lifecycle management with proper cleanup of restart timers

### Fixed

- **Server Preloading**: Fixed server preloading to respect `rootDir` configuration (PR #5 by @maschwenk)
  - Now correctly scans each server's configured directory instead of using project root

## [0.5.1] - 2025-07-14

### Enhanced

- **Improved Diagnostic Idle Detection**: Added intelligent idle detection for publishDiagnostics notifications
  - Tracks diagnostic versions and update timestamps to determine when LSP servers are idle
  - Ensures all diagnostics are received before returning results
- **Optimized MCP Timeouts**: Adjusted wait times for better reliability in MCP usage
  - Initial diagnostics: 5 seconds (previously 2 seconds)
  - After changes: 3 seconds (previously 1.5 seconds)
  - Idle detection: 300ms (previously 200ms)

### Fixed

- Fixed Windows path handling in diagnostics tests by using `path.resolve()` consistently

## [0.5.0] - 2025-07-14

### Added

- **PublishDiagnostics Support**: Added support for push-based diagnostics (textDocument/publishDiagnostics) in addition to pull-based diagnostics
- **Diagnostic Caching**: Implemented caching for diagnostics received via publishDiagnostics notifications
- **Fallback Mechanism**: Added automatic fallback to trigger diagnostics generation for servers that don't support pull-based diagnostics

### Enhanced

- Improved compatibility with language servers like gopls that primarily use publishDiagnostics
- Better diagnostic retrieval with multiple strategies: cached diagnostics, pull request, and triggered generation

## [0.4.4] - 2025-07-10

### Fixed

- **LSP Server Initialization**: Improved initialization handling to properly wait for server's initialized notification
- **Setup Script Improvements**: Fixed Claude command detection to use local installation when global command is not available
- **Type Safety**: Replaced `any` types with proper type annotations (NodeJS.ErrnoException)

### Enhanced

- Better error handling in setup script with more descriptive error messages
- More robust process spawning with proper error event handling

## [0.4.3] - 2025-06-30

### Added

- **Vue.js Language Server Support**: Added official Vue.js language server (Volar) configuration
- **Svelte Language Server Support**: Added Svelte language server configuration
- Support for `.vue` and `.svelte` file extensions in setup wizard
- Installation guides and auto-install commands for Vue.js and Svelte language servers

### Maintenance

- Cleaned up temporary test files (`test-example.ts`, `test-mcp.mjs`, `test-rename.ts`)

## [0.4.2] - 2025-06-29

### Added

- **LSP Server Auto-Restart**: Added `restartInterval` option to server configuration for automatic LSP server restarts to prevent long-running server degradation
- Configurable restart intervals in minutes with minimum 0.1 minute (6 seconds) for testing
- Comprehensive test coverage for restart functionality including timer setup, configuration validation, and cleanup

### Enhanced

- Improved LSP server stability for long-running sessions, particularly beneficial for Python Language Server (pylsp)
- Updated documentation with configuration examples and restart interval guidelines
- **Setup Wizard Improvements**: Enhanced file extension detection with comprehensive .gitignore support
- Improved project structure scanning to exclude common build artifacts, dependencies, and temporary files
- Better accuracy in detecting project's primary programming languages for LSP server configuration

## [0.4.1] - 2025-06-28

### Added

- **Intelligent symbol kind fallback**: When a specific `symbol_kind` is specified but no matches are found, automatically search all symbol types and return results with descriptive warning messages
- Enhanced user experience for LLM-based tools that may specify incorrect symbol kinds
- Comprehensive test coverage for all fallback scenarios

### Fixed

- Improved robustness of symbol searches when exact kind matches are not available

## [0.4.0] - 2025-06-28

### Changed

- **BREAKING**: Complete redesign of MCP tool API from position-based to symbol name/kind-based lookup
- `find_definition` now accepts `symbol_name` and `symbol_kind` instead of `line` and `character`
- `find_references` now accepts `symbol_name` and `symbol_kind` instead of `line` and `character`
- `rename_symbol` now accepts `symbol_name` and `symbol_kind` instead of `line` and `character` (legacy tool, evolved through unified API phases, now using unified rename tool)
- Enhanced LSP stderr forwarding directly to MCP stderr for better debugging
- Improved position accuracy for `SymbolInformation` with file content analysis

### Added

- `textDocument/documentSymbol` LSP functionality for comprehensive symbol discovery
- Automatic symbol matching by name and kind for improved LLM accuracy
- `rename_symbol_strict` tool for precise position-based renaming when multiple matches exist (legacy tool, evolved through unified API phases, now using dryRun option)
- Symbol kind validation with helpful error messages listing valid options
- Comprehensive debug logging throughout the symbol resolution pipeline
- File content analysis for precise symbol position detection in `SymbolInformation`
- Enhanced pylsp configuration with jedi plugin settings for Python support
- Invalid symbol kind warnings embedded in response text instead of breaking execution

### Fixed

- Position accuracy issues with Python Language Server (pylsp) symbol detection
- Character position estimation for better symbol name targeting

## [0.3.5] - 2025-06-28

### Changed

- **BREAKING**: Removed `use_zero_index` option from all MCP tools
- Tools now automatically try multiple position combinations (line±1, character±1) to handle different indexing conventions
- Enhanced error messages with better debugging information
- Results show which position combination was successful

### Added

- Multi-position symbol resolution for better compatibility with different editors and LSP implementations
- Comprehensive test suite for multi-position functionality

## [0.3.4] - 2025-06-28

### Fixed

- Fixed setup command to use `npx @goobits/mill@latest` instead of `npx @goobits/mill` for MCP configuration
- Updated all documentation to consistently use `npx @goobits/mill@latest` for better version control

## [0.3.3] - 2025-06-28

### Changed

- MCP tools now use 1-based indexing by default for both line and character positions
- Tool parameter `character` now defaults to 1-indexed (human-readable) instead of 0-indexed
- Added `use_zero_index` parameter to all tools for backward compatibility with 0-based indexing
- Updated tool descriptions to clearly indicate indexing behavior

### Added

- Comprehensive test coverage for 1-based and 0-based indexing behavior
- Character position conversion tests/e2e for all MCP tools
- Edge case testing for character indexing boundaries

## [0.3.2] - 2025-06-27

### Fixed

- Improved CI/CD version detection for npm publishing
- Replaced git-based version change detection with npm registry comparison
- Enhanced logging for version comparison process in CI workflow

## [0.3.1] - 2025-06-27

### Fixed

- `npx @goobits/mill@latest setup` command now executes properly without hanging
- Setup subcommand execution flow and error handling
- Eliminated duplicate execution when running setup via `node dist/index.js setup`
- Streamlined build process by removing separate setup.js compilation

## [0.3.0]

### Added

- Interactive configuration generator with `mill setup` command
- Support for 15 language servers (TypeScript, Python, Go, Rust, C/C++, Java, Ruby, PHP, C#, Swift, Kotlin, Dart, Elixir, Haskell, Lua)
- Emacs-style keyboard navigation (Ctrl+P/Ctrl+N) for setup interface
- Automatic installation instructions display for selected language servers
- Configuration file preview and validation
- Comprehensive test suite for setup functionality
- GitHub issue templates for bug reports, feature requests, language support, and questions
- `CONTRIBUTING.md` with detailed contribution guidelines
- `CODE_OF_CONDUCT.md` following Contributor Covenant
- `SECURITY.md` with security policy and reporting guidelines
- `ROADMAP.md` outlining project vision and planned features
- GitHub Actions CI/CD pipeline for automated testing and npm publishing
- Additional badges in README (CI status, npm downloads, PRs welcome)
- Comprehensive troubleshooting section in README
- Real-world usage examples in README

### Changed

- Enhanced README with better structure and more detailed documentation
- Improved project metadata for better npm discoverability

## [0.2.1]

### Added

- `rename_symbol` MCP tool for refactoring symbols across codebases (legacy tool, evolved through unified API phases, now using unified rename tool)
- Enhanced error handling for LSP server failures

### Changed

- Improved documentation clarity for tool outputs
- Better type safety in tool interfaces

## [0.2.0]

### Added

- npm publishing configuration
- Executable binary support (`mill` command)
- Proper package.json metadata
- Installation instructions in README

### Changed

- Project renamed from `lsmcp` to `mill` for better clarity
- Updated all references and documentation

## [0.1.0]

### Added

- Initial implementation of MCP server for LSP functionality
- `find_definition` tool for locating symbol definitions
- `find_references` tool for finding all symbol references
- Support for multiple language servers via configuration
- TypeScript language server as default
- Basic error handling and logging
- Test suite with Bun
- Documentation for setup and usage

[0.2.1]: https://github.com/ktnyt/mill/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/ktnyt/mill/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ktnyt/mill/releases/tag/v0.1.0
