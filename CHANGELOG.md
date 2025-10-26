# Changelog

All notable changes to TypeMill will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## Rust Implementation (Current)

The project underwent a complete architectural transformation from TypeScript/Node.js to pure Rust in 2025, bringing native performance, memory safety, and compile-time type guarantees.

### [Unreleased]

### [0.8.0] - 2025-10-26

🚀 **Version 0.8.0** - Python restoration, unified API completion, and code quality improvements

#### Added

- **Python Language Plugin Restoration** - Complete restoration with 100% feature parity
  - Full ImportSupport and WorkspaceSupport trait implementations
  - AST-based refactoring operations (extract, inline, move)
  - Poetry/PDM/Hatch workspace support
  - Comprehensive test coverage matching TypeScript and Rust plugins

- **Gitignore Plugin Enhancement** - Automatic `.gitignore` pattern updates during rename operations

- **Project Factory Pattern** - TypeScript and Python now support `workspace.create_package`
  - Language-specific project templates and scaffolding
  - 100% parity across TypeScript, Rust, and Python

- **Security Improvements** - Enhanced secrets management and environment variable documentation

#### Changed

- **Unified dryRun API Migration (Phase 5)** - Complete migration from two-step workflow
  - All refactoring tools now use single `options.dryRun` parameter
  - Eliminated separate `.plan` suffix tools (7 tools reduced to unified API)
  - Safe default (`dryRun: true`) prevents accidental modifications
  - Updated all 550+ tests to use unified API
  - Removed legacy `workspace.apply_edit` tool

- **Code Quality** - Achieved 100% clippy-clean workspace
  - Resolved all clippy warnings across all crates
  - Applied suggestions to core, handlers, and plugins

- **Documentation** - Unified API migration cleanup
  - Removed all references to deprecated `.plan` workflow
  - Updated API examples to unified dryRun pattern
  - Consolidated development guides and fixed broken links

#### Fixed

- **Test Infrastructure** - Improved reliability and robustness
  - Fixed path validation and Default trait implementation
  - Corrected dryRun parameter mapping in test harness
  - Resolved compilation errors and warnings

- **Plugin System** - Fixed project factory templates and pnpm support

#### Removed

- **Legacy Workflow Infrastructure**
  - Deleted `workspace.apply_edit` internal tool
  - Removed all `.plan` suffix tool references and validation tests
  - Cleaned up deprecated two-step workflow documentation

#### Migration Notes

- **Unified API**: All refactoring tools now use `options.dryRun: true/false` directly instead of separate `.plan` tools
- **Python Support**: Projects can now use Python alongside TypeScript and Rust with full feature parity
- **Security**: Review new environment variable documentation for secrets management best practices

---

### [0.7.0] - 2025-10-24

🚀 **Version 0.7.0** - TypeMill rebranding, API standardization, and rename architecture

#### Breaking Changes

- **TypeMill Branding Migration** - Complete rename from CodeBuddy to TypeMill
  - Configuration directory: `.codebuddy/` → `.typemill/`
  - Binary name: `codebuddy` → `mill`
  - Environment variables: `CODEBUDDY__*` → `TYPEMILL__*`
  - All internal crates renamed: `cb-*` → `mill-*` (27 crate renames)
  - Package name: `codebuddy` → `typemill`

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

- **Test Suite Fixes** - Updated tests for camelCase JSON fields and new expectations

- **CLI & Configuration** - Environment variable handling
  - Config loader uses `TYPEMILL__` prefix
  - Removed legacy CLI flags for clean minimal API
  - Standardized JSON parameters to snake_case, then to camelCase

#### Removed

- **Legacy Naming** - All references to CodeBuddy branding
  - Removed `.codebuddy/` configuration directory support
  - Removed `codebuddy` binary and command
  - Removed `CODEBUDDY__*` environment variable support
  - Cleaned up stale nested crates in mill-lang-markdown

#### Migration Notes

- **Configuration**: Move `.codebuddy/` to `.typemill/` in your projects
- **Binary**: Update scripts and commands from `codebuddy` to `mill`
- **Environment**: Rename `CODEBUDDY__*` to `TYPEMILL__*` in your environment
- **JSON API**: Update client code to use camelCase for all JSON parameters
- **Scope**: Update `"project"` to `"standard"` (old name deprecated with warning)

---

### [0.6.1] - 2025-10-22

🚀 **Version 0.6.1** - Rename tool enhancements, quick refactoring operations, and comment/prose updates

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
  - Same parameters as dry-run mode but automatically execute changes
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
- **Quick Operations Documentation** - Added complete docs for all 7 quick refactoring tools
- **Architecture Documentation** - Synced AGENTS.md and GEMINI.md with latest architecture

---

### [0.6.0] - 2025-10-21

🚀 **Version 0.6.0** - Plugin architecture modernization and refactoring/analysis APIs

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

- **Unified Refactoring API** - Single-step refactoring with integrated dryRun option
  - **Current API**: All refactoring tools (`rename`, `extract`, `inline`, `move`, `delete`, `transform`, `reorder`) accept `options.dryRun: true/false` directly
  - **Previous workflow (before Phase 5, legacy only)**: required generating a `.plan` file and manually calling a now-removed `workspace.apply_edit` tool
  - **Now**: Single tool call with preview (`dryRun: true`) or execution (`dryRun: false`)
  - Safe dry-run previews without filesystem modifications
  - Atomic execution with rollback on errors
  - **See**: [docs/tools/refactoring.md](docs/tools/refactoring.md) for complete unified API documentation

- **Unified Analysis API** - 6 analysis tools with 26 detection kinds
  - `analyze.quality` - complexity, smells, maintainability, readability
  - `analyze.dead_code` - unused imports/symbols/parameters/variables/types
  - `analyze.dependencies` - imports, circular deps, coupling, cohesion
  - `analyze.structure` - symbols, hierarchy, interfaces, inheritance
  - `analyze.documentation` - coverage, quality, style, examples
  - `analyze.tests` - coverage, quality, assertions, organization
  - `analyze.batch` - optimized multi-file analysis with AST caching
  - Configuration via `.typemill/analysis.toml` with 3 presets

- **Additional Language Plugins** - Markdown, TOML, YAML plugins for rename support
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

- **Language Support Matrix** - Updated all documentation to reflect TypeScript + Rust focus
- **Test Infrastructure** - Simplified test harness for two-language focus
- **Build Configuration** - Updated Makefile and build scripts for TS + Rust
- **Documentation** - Updated for language reduction with disclaimers and git tag references
- **Plugin Development Guide** - Updated to use `mill_plugin!` macro instead of `languages.toml`

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

- **Duplicate Crates** - Removed duplicate crates from workspace
- **Temporal References** - Proposals now use priority numbers instead of dates

---

### [0.3.0] - 2025-10-09

🚀 **Version 0.3.0** - Swift language support, documentation updates, build/test optimizations, and import handling fixes

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

🎉 **Version 0.1.0** - Initial Rust MCP server release

#### Added

- **Cross-platform installation script** - `install.sh` with support for macOS, Ubuntu/Debian, Fedora/RHEL, and Arch
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

The project was originally implemented in TypeScript/Node.js from September 2024 to October 2025 (versions 0.1.0 through 1.3.0) before undergoing a complete rewrite in Rust. The TypeScript implementation featured LSP server pooling, predictive loading, WebSocket server support, JWT authentication, and comprehensive MCP tooling. Many of these features have been reimplemented in Rust with enhanced performance and memory safety guarantees.

**Historical versions:**
- v1.3.0 (Sept 2024) - LSP server pooling, predictive loading, client SDK
- v1.2.0 (Sept 2024) - Enterprise architecture, transaction manager, WebSocket server
- v1.1.0 (Sept 2024) - ARM64 FUSE support, multi-tenant Docker
- v1.0.0 (Sept 2024) - Stable API release
- v0.5.x - v0.3.x (Jul-Aug 2024) - Core LSP features, diagnostics, setup wizard
- v0.2.x - v0.1.0 (Jun 2024) - Initial MCP server implementation

For complete TypeScript implementation history, see git tags v0.1.0 through v1.3.0.

