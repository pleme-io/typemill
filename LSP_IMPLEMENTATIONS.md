# Comprehensive Language Server Protocol (LSP) Implementations

A complete reference of production-ready, actively maintained LSP servers across all major programming languages as of October 2025.

## Complete LSP Server Comparison Table

| Language | Project Name | Completions | Diagnostics | Go-to-Def | Hover | References | Rename | Formatting | Code Actions | Last Update | Repository | Maturity | Notable Strengths |
|----------|-------------|-------------|-------------|-----------|-------|------------|--------|------------|--------------|-------------|------------|----------|-------------------|
| **Bash/Shell** | bash-language-server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Apr 2025 (v5.6.0) | [github.com/bash-lsp/bash-language-server](https://github.com/bash-lsp/bash-language-server) | Stable | Tree Sitter parser, shellcheck integration, shfmt formatting, explainshell docs |
| **C/C++** | clangd | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Sep 2024 (weekly) | [github.com/clangd/clangd](https://github.com/clangd/clangd) | Stable | Official LLVM project, highest parsing accuracy, excellent C++20/23 support, fast indexing |
| **C/C++** | ccls | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Nov 2024 (0.20241108) | [github.com/MaskRay/ccls](https://github.com/MaskRay/ccls) | Stable | Superior cross-reference, rainbow semantic highlighting, advanced hierarchy queries, global indexing |
| **C#** | OmniSharp-Roslyn | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Dec 2024 (v1.39.13) | [github.com/OmniSharp/omnisharp-roslyn](https://github.com/OmniSharp/omnisharp-roslyn) | Stable | Roslyn-based, mature ecosystem, broad editor support, MSBuild integration |
| **C#** | C# Dev Kit (MS LSP) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Active 2024-2025 | [github.com/dotnet/vscode-csharp](https://github.com/dotnet/vscode-csharp) | Stable | Official Microsoft implementation, VS integration, best performance, IntelliCode AI |
| **C#** | csharp-ls | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Apr 2025 (v0.17.0) | [github.com/razzmatazz/csharp-language-server](https://github.com/razzmatazz/csharp-language-server) | Beta/Stable | Lightweight alternative, MIT licensed, ILSpy decompilation, smaller footprint |
| **Clojure** | clojure-lsp | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Aug 2025 (2025.08.25) | [github.com/clojure-lsp/clojure-lsp](https://github.com/clojure-lsp/clojure-lsp) | Stable | clj-kondo integration, GraalVM native binaries, 20+ refactorings, custom linters |
| **Dart** | Dart Analysis Server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Feb 2025 (SDK 3.7) | [github.com/dart-lang/sdk](https://github.com/dart-lang/sdk) | Stable | Built into Dart SDK, official Google support, Flutter integration, quarterly releases |
| **Elixir** | ElixirLS | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Dec 2024 (v0.26+) | [github.com/elixir-lsp/elixir-ls](https://github.com/elixir-lsp/elixir-ls) | Stable | Debugger support, automatic Dialyzer, @spec suggestions, most feature-complete |
| **Elixir** | Lexical | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 2024 (v0.7.0) | [github.com/lexical-lsp/lexical](https://github.com/lexical-lsp/lexical) | Beta/Stable | Isolated VM environment, context-aware completions, as-you-type compilation |
| **Elm** | elm-language-server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Dec 2024 (v2.8.0) | [github.com/elm-tooling/elm-language-server](https://github.com/elm-tooling/elm-language-server) | Stable | Built-in type inference, automatic namespace cleaning, elm-format integration |
| **Go** | gopls | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 2025 (v0.20.0) | [github.com/golang/tools/tree/master/gopls](https://github.com/golang/tools/tree/master/gopls) | Stable | Official Go team server, quarterly releases, persistent index, web-based refactoring tools |
| **Haskell** | Haskell Language Server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Dec 2024 (v2.11.0) | [github.com/haskell/haskell-language-server](https://github.com/haskell/haskell-language-server) | Stable | Official Haskell IDE, plugin architecture, supports GHC 9.4-9.12, semantic highlighting |
| **HTML/CSS/JSON** | vscode-langservers-extracted | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 2023 (v4.10.0) | [github.com/hrsh7th/vscode-langservers-extracted](https://github.com/hrsh7th/vscode-langservers-extracted) | Stable | Extracted from VSCode, multiple servers in one package, HTML/CSS/JSON/ESLint support |
| **Java** | Eclipse JDT LS | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Active 2024-2025 | [github.com/eclipse-jdtls/eclipse.jdt.ls](https://github.com/eclipse-jdtls/eclipse.jdt.ls) | Stable | Most mature Java LSP, Eclipse JDT compiler, Maven/Gradle integration, Java 1.8-24 support |
| **JavaScript/TypeScript** | typescript-language-server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Jan 2025 (v5.0.0) | [github.com/typescript-language-server/typescript-language-server](https://github.com/typescript-language-server/typescript-language-server) | Stable | Thin wrapper around tsserver, VSCode feature parity, inlay hints, wide adoption |
| **JavaScript/TypeScript** | vtsls | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Sep 2024 (v0.2.9) | [github.com/yioneko/vtsls](https://github.com/yioneko/vtsls) | Beta | Wraps VSCode TS extension, significantly faster completions, default in Zed editor |
| **JavaScript/TypeScript** | Deno LSP | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Active (Deno 1.43+) | [github.com/denoland/deno](https://github.com/denoland/deno) | Stable | Built into Deno runtime, Rust-based, optimized for Deno modules, 6-8s to <1s completions |
| **Kotlin** | kotlin-lsp (Official) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 2024-2025 (v0.253) | [github.com/Kotlin/kotlin-lsp](https://github.com/Kotlin/kotlin-lsp) | Pre-alpha | Official JetBrains implementation, IntelliJ IDEA analysis, pull-based diagnostics |
| **Kotlin** | kotlin-language-server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Jan 2024 (v1.3.11) | [github.com/fwcd/kotlin-language-server](https://github.com/fwcd/kotlin-language-server) | Beta (Deprecated) | Community-driven, fully functional, **now deprecated** in favor of official kotlin-lsp |
| **LaTeX** | texlab | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Oct 2024 (v5.23.1) | [github.com/latex-lsp/texlab](https://github.com/latex-lsp/texlab) | Stable | Rust-based, automatic project detection, SyncTeX support, unicode/image preview, build integration |
| **LaTeX** | digestif | ✓ | Limited | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ | Sep 2024 (v0.6) | [github.com/astoff/digestif](https://github.com/astoff/digestif) | Stable | Lua-based, minimal dependencies, ConTeXt/plain TeX/Texinfo support, fuzzy matching |
| **Lua** | lua-language-server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Oct 2024 (v3.11.1) | [github.com/LuaLS/lua-language-server](https://github.com/LuaLS/lua-language-server) | Stable | Written in Lua, 1M+ installs, 20+ annotations, dynamic type checking, plugin system |
| **OCaml** | ocaml-lsp-server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 2024 (v1.24.0) | [github.com/ocaml/ocaml-lsp](https://github.com/ocaml/ocaml-lsp) | Stable | Official OCaml LSP, destruct/construct features, Dune RPC integration, typed holes, Merlin-powered |
| **PHP** | Intelephense | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ (Premium) | ✓ | ✓ (Premium) | Active 2024 | Proprietary ([docs](https://github.com/bmewburn/intelephense-docs)) | Stable | **Freemium proprietary**, high-performance, HTML/JS/CSS embedded support, most popular |
| **PHP** | Phpactor | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Active 2024 | [github.com/phpactor/phpactor](https://github.com/phpactor/phpactor) | Stable | **Fully open source (MIT)**, comprehensive refactoring, class generation, native VIM plugin |
| **Python** | Pyright | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Oct 2025 (v1.1.406) | [github.com/microsoft/pyright](https://github.com/microsoft/pyright) | Stable | High-performance TypeScript-based, large codebase optimization, powers Pylance, parallel processing |
| **Python** | python-lsp-server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Mar 2024 (v1.11.0) | [github.com/python-lsp/python-lsp-server](https://github.com/python-lsp/python-lsp-server) | Stable | Spyder-maintained, extensive plugin ecosystem (mypy/black/ruff), multiple linter support |
| **Python** | jedi-language-server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ | ✓ | 2024 (v0.45.1) | [github.com/pappasam/jedi-language-server](https://github.com/pappasam/jedi-language-server) | Beta | Jedi-focused, notebook support, simple implementation, no external dependencies |
| **R** | languageserver | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Aug 2023 (v0.3.16) | [github.com/REditorSupport/languageserver](https://github.com/REditorSupport/languageserver) | Stable | Official CRAN package, most comprehensive features, styler/lintr integration, semantic tokens |
| **Ruby** | Solargraph | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Jul 2025 (v0.56.2) | [github.com/castwide/solargraph](https://github.com/castwide/solargraph) | Stable | Veteran server (2017), YARD documentation, RBS type support, type checker, plugin system |
| **Ruby** | ruby-lsp | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Jan 2025 (v0.26.1) | [github.com/Shopify/ruby-lsp](https://github.com/Shopify/ruby-lsp) | Stable | Shopify-developed, modern opinionated design, Rails add-ons, AI agent support, fast |
| **Rust** | rust-analyzer | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Aug 2024 (v0.3.2577) | [github.com/rust-lang/rust-analyzer](https://github.com/rust-lang/rust-analyzer) | Stable | Official Rust project, extensive refactoring, macro expansion, works without compilation, inlay hints |
| **Scala** | Metals | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Aug 2025 (v1.6.2) | [github.com/scalameta/metals](https://github.com/scalameta/metals) | Stable | Official Scala Center server, Bloop integration, sbt/Gradle/Maven/Mill/Bazel support, MCP server, DAP debugging |
| **SQL** | sql-language-server | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ | Active 2024 | [github.com/joe-re/sql-language-server](https://github.com/joe-re/sql-language-server) | Beta | Node.js-based, sqlint integration, MySQL/PostgreSQL/SQLite3, SSH tunnel support |
| **SQL** | sqls | ✓ | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ | ✓ | 2024 (v0.2.28) | [github.com/sqls-server/sqls](https://github.com/sqls-server/sqls) | Alpha/Beta | Go-based, 6 database systems, intelligent JOIN completion, execute/explain SQL |
| **Swift** | SourceKit-LSP | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Swift 6.1.1 (2025) | [github.com/swiftlang/sourcekit-lsp](https://github.com/swiftlang/sourcekit-lsp) | Stable | Official Apple implementation, bundled with Xcode, sourcekitd/clangd-based, IndexStoreDB indexing |
| **Zig** | zls | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | 2025 (v0.15.0) | [github.com/zigtools/zls](https://github.com/zigtools/zls) | Beta | Build-on-save feature, cImport support, inlay hints, comptime interpreter, follows Zig releases |

## Key Findings

### Most Mature Ecosystems
**Java** (Eclipse JDT LS), **Go** (gopls), **Rust** (rust-analyzer), and **C/C++** (clangd) represent the most mature and feature-complete LSP implementations with years of production usage and official backing from language teams.

### Newest Official Implementations
Several languages have recently consolidated around official LSP servers: **Kotlin** (official kotlin-lsp in pre-alpha), **Elixir** (three servers merging into unified implementation), and **C#** (Microsoft's new LSP host replacing OmniSharp in VS Code).

### Performance Leaders
**vtsls** for TypeScript delivers significantly faster completions in large codebases. **Pyright** excels with parallel processing for Python. **clangd** and **ccls** both handle multi-million line C++ projects efficiently with different trade-offs (clangd for accuracy, ccls for cross-reference).

### Languages with Multiple Production Options

**Python**: Three stable choices - Pyright (performance), python-lsp-server (extensibility), jedi-language-server (simplicity)

**Ruby**: Two modern options - Solargraph (veteran with plugins) and ruby-lsp (Shopify's modern approach)

**PHP**: Intelephense (proprietary freemium, most popular) vs Phpactor (fully open source, MIT)

**C/C++**: clangd (LLVM official, best parsing) vs ccls (superior navigation/hierarchy)

**Elixir**: ElixirLS (most features, debugger) and Lexical (isolated builds, context-aware) both excellent; merging with Next LS into official implementation

### Unique Capabilities

**ccls** (C/C++) - Rainbow semantic highlighting and advanced hierarchy queries ($ccls/call, $ccls/inheritance, $ccls/member) unmatched by other servers

**Metals** (Scala) - Only LSP with Model Context Protocol (MCP) server for AI agent integration

**OCaml-LSP** - "Destruct" and "construct" features for type-driven development

**rust-analyzer** - Macro expansion viewing and comprehensive assists/refactoring

**SourceKit-LSP** - "Indexing while building" technique for efficient Swift project indexing

### Cross-Language Servers

**SourceKit-LSP** supports Swift, C, C++, Objective-C, and Objective-C++

**vscode-langservers-extracted** provides HTML, CSS, JSON, and ESLint in one package

**clangd** handles C, C++, Objective-C, and Objective-C++

### Installation Considerations

Most servers require specific runtimes:
- **Node.js**: TypeScript servers, bash-language-server, sql-language-server
- **.NET SDK**: C# servers (9.0+ for csharp-ls)
- **JVM**: JVM language servers
- **Native binaries**: clangd, rust-analyzer, gopls, texlab, clojure-lsp (GraalVM)

### Licensing Notes

**Proprietary**: Intelephense (PHP) uses freemium model with premium features requiring license

**Commercial restrictions**: C# Dev Kit free for individuals/academia/OSS, requires Visual Studio subscription for commercial teams of 6+

**Fully open source**: All others are MIT, Apache 2.0, GPL, or similar permissive licenses

## Selection Recommendations

**For official backing**: Choose language-team maintained servers (gopls, rust-analyzer, SourceKit-LSP, Eclipse JDT, Dart Analysis Server, HLS)

**For maximum features**: rust-analyzer (Rust), Metals (Scala), ElixirLS (Elixir), languageserver (R), Eclipse JDT (Java)

**For performance**: Pyright (Python), vtsls (TypeScript), clangd (C++), gopls (Go)

**For exploration/navigation**: ccls (C++), rust-analyzer (Rust), Metals (Scala)

**For lightweight setups**: digestif (LaTeX), csharp-ls (C#), jedi-language-server (Python)

**For open source commitment**: Phpactor over Intelephense (PHP), OmniSharp/csharp-ls over C# Dev Kit (if avoiding restrictions)

All servers listed are actively maintained, production-ready (except pre-alpha Kotlin LSP and deprecated kotlin-language-server), and have real-world usage as of October 2025.
