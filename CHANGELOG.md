# Changelog

All notable changes to CodeBuddy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## Rust Implementation (Current)

The project underwent a complete architectural transformation from TypeScript/Node.js to pure Rust in 2025, bringing native performance, memory safety, and compile-time type guarantees.

### [1.0.0-rc2] - 2025-10-06

🚀 **Release Candidate 2** - Plugin architecture modernization, workspace operations, and 5-language support

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

- **Build-time code generation infrastructure**
  - Single source of truth: `languages.toml` configuration file
  - Automatic generation of `ProjectLanguage` enum from TOML
  - Automatic generation of `LanguageMetadata` constants
  - Automatic generation of plugin registration code
  - Zero manual synchronization across crates

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

- **Plugin architecture refactored to capability-based traits** (Phase 1A-1F, Phase 2)
  - Replaced monolithic `LanguageIntelligencePlugin` (22 methods) with composable `LanguagePlugin` trait
  - Optional `ImportSupport` and `WorkspaceSupport` capability traits
  - Metadata access via `plugin.metadata()` instead of individual trait methods
  - Downcasting pattern for plugin-specific methods
  - **Benefit**: 29-42% LOC reduction per plugin, opt-in capabilities

- **Refactoring operations switched to AST-first approach**
  - `extract_function`, `inline_variable`, `extract_variable` now try AST before LSP
  - **Rationale**: LSP servers have inconsistent multiline extraction behavior
  - **Benefit**: Faster, more reliable, under our control
  - 4/5 languages now support multiline extract via AST (Python, TypeScript, Rust, Go)

- **Simplified language plugin generator** (`new-lang.sh`)
  - Reduced from 817 to 607 lines (-210 lines, 25.7% reduction)
  - Replaced 5-file patching with single TOML append
  - Build scripts auto-generate all integration code
  - Portable across macOS/Linux (no BSD sed issues)

- **LSP infrastructure improvements**
  - Replaced arbitrary sleeps with smart LSP polling (Bug #2 fix)
  - Hybrid fallback for `find_dead_code` to support rust-analyzer
  - Multi-plugin workspace symbol search (Bug #1 fix)

- **Documentation overhaul**
  - Updated all language support matrices to include Java (5 languages total)
  - Added build-time code generation documentation
  - Removed legacy manual integration steps
  - Consolidated 4 obsolete planning documents

#### Fixed

- XML event handling in Java workspace module rewriting
- Git operations tests failing with BrokenPipe errors
- EditPlan structure in refactoring implementations
- Python plugin delegation and manifest support
- Language plugin configuration and validation
- Test expectations for multiline extract operations

#### Removed

- Manual file patching in `new-lang.sh` (replaced with TOML + codegen)
- Arbitrary sleep statements in LSP operations
- Obsolete planning documents (4 files)
  - `JAVA_AST_CAPABILITY_PLAN.md`
  - `PHASE4_RESOLUTION.md`
  - `TYPESCRIPT_TEST_FIX_PROPOSAL.md`
  - `WORKSPACE_IMPLEMENTATION_PLANS.md`
- `.claude/` directory from version control (kept locally)

#### Technical Debt Improvements

- Migrated Python tests to cross-language framework
- Removed duplicate TypeScript refactoring tests
- Updated test expectations to reflect reality (NotSupported → Success for AST-based operations)
- Improved structured logging across refactoring operations
- Enhanced error messages with actionable context

#### Performance

- Build-time code generation eliminates runtime overhead
- AST-first refactoring avoids LSP round-trips
- Smart LSP polling reduces unnecessary waiting
- Multi-plugin symbol search parallelizes queries

---

### [1.0.0-rc1] - 2025-10-04

🎉 **Release Candidate 1** - Production-ready Rust MCP server with comprehensive tooling

#### Added

- **Cross-platform installation script** (`install.sh`)
  - Enterprise-grade security (package verification, timeout protection)
  - Support for macOS (Homebrew), Ubuntu/Debian (APT), Fedora/RHEL (DNF), Arch (Pacman)
  - Automatic Rust and git installation
  - Delegates to Makefile for consistent build process
  - Comprehensive error handling with actionable fixes

- **Complete documentation overhaul**
  - Fixed tool count accuracy (44 tools verified)
  - Updated all binary references (cb-server → codebuddy)
  - Consolidated contracts.md into ARCHITECTURE.md (-501 lines)
  - Added workflow cross-references
  - Removed redundant documentation files

- **Plugin architecture completion**
  - Full language adapter migration completed
  - Composable plugin system with multi-tiered priority
  - Zero deprecated adapters remaining

- **Java AST support**
  - Tree-sitter based parser integration
  - Full test coverage for Java language operations

- **Refactoring tools**: Full AST-based implementation for extract_function, inline_variable, and extract_variable
  - Python: Full AST analysis with parameter detection and scope handling
  - TypeScript/JavaScript: Functional implementation with SWC parser
  - Atomic operations with automatic rollback on failure
  - Dry-run mode for previewing changes
- **SWC-based AST parsing** for TypeScript/JavaScript (production-ready since parser v0.3.0)
  - Full TypeScript/JavaScript AST parsing with regex fallback for robustness
  - Native Rust performance via swc_ecma_parser
- **VFS (Virtual Filesystem)** support as optional experimental feature (Unix only)
  - Feature-gated with `#[cfg(all(unix, feature = "vfs"))]`
  - Build with: `cargo build --features vfs`
  - Not included in default build
- **44 MCP Tools** - Complete implementation across all categories
  - Navigation & Intelligence (14 tools)
  - Editing & Refactoring (10 tools)
  - File Operations (6 tools)
  - Workspace Operations (7 tools)
  - Advanced Operations (2 tools)
  - LSP Lifecycle (3 tools)
  - System & Health (2 tools)

#### Changed

- **CLI port default**: WebSocket server now defaults to port 3040 (consistent with config)
- **Installation methods**: install.sh now recommended over manual installation
- **Build integration**: install.sh delegates to Makefile for single source of truth

- **Structured logging**: All production code now uses tracing framework
  - Production code uses structured key-value logging
  - CLI uses println for user-facing output
- **Error handling**: Replaced all `.unwrap()` calls in production code with `.expect()` containing descriptive messages
  - LSP client: 4 unwraps → expect() (command parsing, JSON serialization, directory access)
  - AST parser: ~10 unwraps → expect() (regex compilation and capture groups)
  - Python parser: ~10 unwraps → expect() (import/function/variable parsing)
  - CLI: 5 production unwraps → expect()
- **Dependencies**: Unified thiserror to v2.0 and jsonwebtoken to v10.0 across workspace
- **VFS excluded** from default workspace build (faster builds, smaller binary)

#### Fixed

- **Critical**: install.sh now installs git before attempting to clone repository
- **Documentation accuracy**: All tool counts corrected to 44
- **Port consistency**: CLI default port matches config default (3040)
- **Java tests**: Resolved compilation errors and unused imports
- **LSP tools**: Fixed plugin delegation in analyze_imports
- **Text edits**: Correctly apply edits to target files in EditPlan
- Production code error handling improved with descriptive expect() messages
- Structured logging in cb-ast/parser.rs (2 eprintln! → tracing::debug!)

#### Removed

- **Deprecated code**: All adapter methods with zero callers removed
- **Completed proposals**: Java AST proposal (implementation complete)
- **Redundant documentation**: contracts.md (merged), USAGE.md (redundant)
- Stale benchmark suite (238 lines) - API changed, unmaintainable
- cb-vfs from default workspace members (feature-gated, optional)

#### Performance

- **Build optimization**: sccache and mold integration via Makefile
- **Native compilation**: Zero-cost abstractions throughout
- **Memory safety**: Compile-time guarantees, no garbage collection overhead

#### Documentation

All documentation now 100% accurate and synchronized with codebase:
- README.md, API.md, CLAUDE.md, TOOLS.md
- CONTRIBUTING.md, ONBOARDING.md, OPERATIONS.md
- ARCHITECTURE.md (includes contracts)
- WORKFLOWS.md (intent-based workflow engine)

---

### [Unreleased]

#### Planned
- Package publishing to crates.io
- Homebrew formula
- Additional language server integrations

### [0.1.0] - 2024-Q4

#### Added
- Core LSP integration with multiple language servers
- MCP protocol support (44 tools)
- Plugin architecture with LSP adapters
- WebSocket transport with JWT authentication
- Production-ready error handling
- Structured logging with tracing framework
- Smart setup with auto-detection via `codebuddy setup`
- Complete CLI: setup, start, stop, serve, status, link, unlink

#### Technical Debt Resolved
- ✅ Structured Logging - Complete
- ✅ Error Handling (.unwrap() removal) - Complete
- ✅ Dependency Cleanup - Complete
- ✅ VFS Feature-gating - Complete
- ✅ Benchmark Suite cleanup - Complete
- ✅ SWC Integration - Complete

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
  - **Performance Impact**: 99.9% faster LSP operations on imported files (validated with benchmarks)

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
- **package.json**: New `test:performance` script for running performance benchmarks
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
- Project renamed from `codeflow-buddy` to `codebuddy` for better clarity
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
  - Corrected to: `claude mcp add codebuddy <command> [args...] --env <env>`
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
  - Resolves "Config file specified in CODEBUDDY_CONFIG_PATH does not exist" error on Unix systems

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
  - Integration tests for MCP command structure
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
  - `dry_run` parameter for safe preview mode on both `rename_symbol` and `rename_symbol_strict`

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

Special thanks to @secondcircle for the major enhancement that transforms codebuddy from a read-only query tool into a functional refactoring tool with actual file editing capabilities (#13). This change significantly improves the user experience from preview-only to actually applying changes.

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

Your contributions help make codebuddy better for everyone! 🙏

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
- `rename_symbol` now accepts `symbol_name` and `symbol_kind` instead of `line` and `character`
- Enhanced LSP stderr forwarding directly to MCP stderr for better debugging
- Improved position accuracy for `SymbolInformation` with file content analysis

### Added

- `textDocument/documentSymbol` LSP functionality for comprehensive symbol discovery
- Automatic symbol matching by name and kind for improved LLM accuracy
- `rename_symbol_strict` tool for precise position-based renaming when multiple matches exist
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

- Fixed setup command to use `npx @goobits/codebuddy@latest` instead of `npx @goobits/codebuddy` for MCP configuration
- Updated all documentation to consistently use `npx @goobits/codebuddy@latest` for better version control

## [0.3.3] - 2025-06-28

### Changed

- MCP tools now use 1-based indexing by default for both line and character positions
- Tool parameter `character` now defaults to 1-indexed (human-readable) instead of 0-indexed
- Added `use_zero_index` parameter to all tools for backward compatibility with 0-based indexing
- Updated tool descriptions to clearly indicate indexing behavior

### Added

- Comprehensive test coverage for 1-based and 0-based indexing behavior
- Character position conversion tests for all MCP tools
- Edge case testing for character indexing boundaries

## [0.3.2] - 2025-06-27

### Fixed

- Improved CI/CD version detection for npm publishing
- Replaced git-based version change detection with npm registry comparison
- Enhanced logging for version comparison process in CI workflow

## [0.3.1] - 2025-06-27

### Fixed

- `npx @goobits/codebuddy@latest setup` command now executes properly without hanging
- Setup subcommand execution flow and error handling
- Eliminated duplicate execution when running setup via `node dist/index.js setup`
- Streamlined build process by removing separate setup.js compilation

## [0.3.0]

### Added

- Interactive configuration generator with `codebuddy setup` command
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

- `rename_symbol` MCP tool for refactoring symbols across codebases
- Enhanced error handling for LSP server failures

### Changed

- Improved documentation clarity for tool outputs
- Better type safety in tool interfaces

## [0.2.0]

### Added

- npm publishing configuration
- Executable binary support (`codebuddy` command)
- Proper package.json metadata
- Installation instructions in README

### Changed

- Project renamed from `lsmcp` to `codebuddy` for better clarity
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

[0.2.1]: https://github.com/ktnyt/codebuddy/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/ktnyt/codebuddy/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ktnyt/codebuddy/releases/tag/v0.1.0
