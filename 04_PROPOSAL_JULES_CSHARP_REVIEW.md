# Jules's C# Plugin Development - Implementation Review & Improvements

**Reviewer**: Claude Code Assistant
**Date**: 2025-10-07
**Subject**: Analysis of C# plugin development process and recommendations
**Developer**: Jules (AI assistant)
**Branch**: `csharp-support-with-refactor`

---

## Executive Summary

Jules successfully delivered a **high-quality C# language plugin** with an **architectural improvement** (RefactoringSupport trait) that benefits the entire codebase. However, the process revealed **documentation gaps** and **workflow friction** that can be addressed with targeted improvements.

**Overall Assessment**: ✅ **Excellent work** with valuable lessons learned

**Key Achievement**: Created RefactoringSupport trait, removing 1,008 lines from core while enabling language-agnostic refactoring.

---

## Top 5 Improvements Recommended

### 1. Feature Complexity Matrix (HIGH IMPACT, 15 minutes) 🎯

**Problem**: Jules didn't know upfront that `extract_module_to_package` would require system-level refactoring.

**Solution**: Add complexity matrix to `crates/languages/README.md`

**Location**: `crates/languages/README.md` → Add section after "Quick Start"

**Content**:
```markdown
## Feature Implementation Complexity

Understanding what you're getting into before you start:

| Feature | Time Estimate | System Changes? | Prerequisites | Status |
|---------|---------------|-----------------|---------------|--------|
| **Basic Parsing** | 1-2 hours | ❌ No | Language runtime (for parser) | Required |
| **Manifest Parsing** | 1-2 hours | ❌ No | - | Required |
| **ImportSupport** | 2-4 hours | ❌ No | Trait exists | Optional |
| **WorkspaceSupport** | 2-4 hours | ❌ No | Trait exists | Optional |
| **RefactoringSupport** | 8-16 hours | ⚠️ **Maybe** | May need trait creation | Optional |

### What "System Changes" Means

- **No**: You can implement entirely within your plugin
- **Maybe**: Check if trait exists first; may need core team coordination

### Implementation Levels

Choose your scope based on project needs:

#### Level 1: Minimal Plugin (2-4 hours)
- ✅ Basic parsing
- ✅ Manifest parsing
- ✅ Symbol extraction
- **Use case**: Basic code navigation, LSP integration

#### Level 2: Standard Plugin (4-8 hours)
- ✅ Level 1 features
- ✅ ImportSupport trait
- **Use case**: Import analysis, code refactoring

#### Level 3: Complete Plugin (8-12 hours)
- ✅ Level 2 features
- ✅ WorkspaceSupport trait
- **Use case**: Workspace-wide operations, dependency management

#### Level 4: Advanced Plugin (12-24 hours)
- ✅ Level 3 features
- ✅ RefactoringSupport trait
- ⚠️ **Requires coordination**: May need trait design if not exists
- **Use case**: Advanced refactoring (extract module to package)

### Before Starting Level 4

If implementing RefactoringSupport:

1. **Check if trait exists**:
   ```bash
   grep -r "trait RefactoringSupport" crates/cb-plugin-api/
   ```

2. **If trait doesn't exist**:
   - Create GitHub issue describing the refactoring operations needed
   - Coordinate with core maintainers on trait design
   - Budget 4-8 hours for trait creation + core refactoring
   - **Do not proceed without approval** - this affects all languages

3. **If trait exists**:
   - Review existing implementations (e.g., `cb-lang-rust`)
   - Implement trait for your language
   - Test thoroughly
```

**Impact**: Prevents 4+ hours of unexpected architectural work

**Implementation**:
```bash
# Open crates/languages/README.md
# Add section after line 100 (after "Quick Start")
# Paste content above
```

---

### 2. External Dependencies Guide (HIGH IMPACT, 30 minutes) 🎯

**Problem**: Jules had to figure out .NET SDK installation on the fly, causing delays.

**Solution**: Create centralized prerequisites documentation

**Location**: `docs/development/LANGUAGE_PLUGIN_PREREQUISITES.md` (new file)

**Content**:
```markdown
# Language Plugin Development Prerequisites

Last updated: 2025-10-07

## Overview

Some language plugins require **external language runtimes** for their parsers. This document lists all requirements and installation instructions.

---

## Quick Reference

| Language | Parser Type | Runtime Required | Build Command |
|----------|-------------|------------------|---------------|
| Rust | Native (syn) | ❌ None | N/A |
| TypeScript | Embedded Node | ⚠️ Node.js 18+ (build only) | `npm install` |
| Python | Native (RustPython) | ❌ None | N/A |
| Go | Subprocess | ⚠️ Go 1.19+ (if using subprocess) | N/A |
| Java | Subprocess (Maven) | ✅ Maven 3.6+, JDK 11+ | `mvn package` |
| C# | Subprocess (.NET) | ✅ .NET 8.0 SDK | `dotnet publish` |
| Swift | Subprocess | ✅ SourceKitten, Swift CLI | N/A |

**Legend**:
- ❌ None: No external dependencies
- ⚠️ Build-time only: Required during build, not runtime
- ✅ Required: Must be installed for plugin to work

---

## Installation Guides

### Java Plugin

**Requirements**:
- Maven 3.6 or later
- JDK 11 or later

**Installation**:
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y maven default-jdk

# macOS
brew install maven openjdk@11

# Verify
mvn --version
java --version
```

**Build Parser**:
```bash
cd crates/languages/cb-lang-java/resources/java-parser
mvn package

# Verify artifact
ls target/java-parser-1.0-SNAPSHOT.jar
```

**Troubleshooting**:
- **Error: JAVA_HOME not set**
  ```bash
  export JAVA_HOME=/usr/lib/jvm/default-java
  ```

---

### C# Plugin

**Requirements**:
- .NET 8.0 SDK

**Installation**:
```bash
# Ubuntu 22.04
wget https://packages.microsoft.com/config/ubuntu/22.04/packages-microsoft-prod.deb -O packages-microsoft-prod.deb
sudo dpkg -i packages-microsoft-prod.deb
rm packages-microsoft-prod.deb
sudo apt-get update
sudo apt-get install -y dotnet-sdk-8.0

# macOS
brew install --cask dotnet-sdk

# Windows
# Download from: https://dotnet.microsoft.com/download/dotnet/8.0

# Verify
dotnet --version  # Should be 8.0.x
```

**Build Parser**:
```bash
cd crates/languages/cb-lang-csharp/resources/csharp-parser
dotnet publish -c Release -r linux-x64 --self-contained

# Verify executable
ls bin/Release/net8.0/linux-x64/publish/csharp-parser
```

**Troubleshooting**:
- **Error: No .NET SDKs were found**
  - Reinstall .NET SDK
  - Check `dotnet --list-sdks`

---

### Swift Plugin

**Requirements**:
- SourceKitten (Swift AST parser)
- Swift CLI (for Package.swift parsing)

**Installation**:
```bash
# macOS
brew install sourcekitten

# Swift CLI comes with Xcode or Swift toolchain
xcode-select --install

# Linux (build from source)
git clone https://github.com/jpsim/SourceKitten.git
cd SourceKitten
swift build -c release
sudo cp .build/release/sourcekitten /usr/local/bin/

# Verify
sourcekitten version
swift --version
```

**No Build Step**: SourceKitten is executed at runtime

**Troubleshooting**:
- **Error: sourcekitten: command not found**
  - Ensure `/usr/local/bin` is in PATH
  - Or use Homebrew installation

---

### TypeScript Plugin

**Requirements**:
- Node.js 18+ (build-time only)

**Installation**:
```bash
# Ubuntu/Debian
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# macOS
brew install node@18

# Verify
node --version  # Should be v18.x or later
```

**Build Parser**:
```bash
cd crates/languages/cb-lang-typescript/resources
npm install

# Verify
node ast_tool.js --version
```

---

### Go Plugin

**Requirements**:
- Go 1.19+ (only if using subprocess parser approach)

**Installation**:
```bash
# Ubuntu/Debian
wget https://go.dev/dl/go1.21.0.linux-amd64.tar.gz
sudo tar -C /usr/local -xzf go1.21.0.linux-amd64.tar.gz
export PATH=$PATH:/usr/local/go/bin

# macOS
brew install go

# Verify
go version
```

**Current Implementation**: Uses regex fallback, Go optional

---

## Build All Parsers

**Makefile target** (recommended):
```bash
make build-parsers
```

**Manual approach**:
```bash
# Java
cd crates/languages/cb-lang-java/resources/java-parser && mvn package && cd -

# C#
cd crates/languages/cb-lang-csharp/resources/csharp-parser && \
  dotnet publish -c Release -r linux-x64 --self-contained && cd -

# TypeScript
cd crates/languages/cb-lang-typescript/resources && npm install && cd -
```

---

## Verification Script

Check if all dependencies are installed:

```bash
#!/bin/bash
# File: scripts/check-language-deps.sh

echo "Checking language plugin dependencies..."

# Java
if command -v mvn &> /dev/null && command -v java &> /dev/null; then
    echo "✅ Java: Maven $(mvn -v | head -1 | awk '{print $3}'), JDK $(java -version 2>&1 | head -1 | awk '{print $3}')"
else
    echo "❌ Java: Maven or JDK not found"
fi

# C#
if command -v dotnet &> /dev/null; then
    echo "✅ C#: .NET $(dotnet --version)"
else
    echo "❌ C#: .NET SDK not found"
fi

# Swift
if command -v sourcekitten &> /dev/null && command -v swift &> /dev/null; then
    echo "✅ Swift: SourceKitten $(sourcekitten version), Swift $(swift --version | head -1)"
else
    echo "❌ Swift: SourceKitten or Swift CLI not found"
fi

# TypeScript
if command -v node &> /dev/null; then
    echo "✅ TypeScript: Node.js $(node --version)"
else
    echo "❌ TypeScript: Node.js not found"
fi

# Go
if command -v go &> /dev/null; then
    echo "✅ Go: $(go version)"
else
    echo "⚠️  Go: Not found (optional for current implementation)"
fi
```

**Usage**:
```bash
chmod +x scripts/check-language-deps.sh
./scripts/check-language-deps.sh
```

---

## CI/CD Considerations

For automated builds, ensure all dependencies are installed:

**GitHub Actions example**:
```yaml
- name: Install language dependencies
  run: |
    sudo apt-get update
    sudo apt-get install -y maven default-jdk
    wget https://packages.microsoft.com/config/ubuntu/22.04/packages-microsoft-prod.deb
    sudo dpkg -i packages-microsoft-prod.deb
    sudo apt-get update
    sudo apt-get install -y dotnet-sdk-8.0

- name: Build external parsers
  run: make build-parsers
```

---

## Adding a New Language with External Parser

**Checklist**:
1. [ ] Choose parser approach (native vs subprocess)
2. [ ] If subprocess: Document runtime requirement here
3. [ ] Add build instructions
4. [ ] Add to verification script
5. [ ] Update CI/CD workflows
6. [ ] Test on clean environment

**Example PR description**:
```markdown
## New Language: Ruby

**Parser**: Subprocess using Ripper (Ruby's built-in parser)

**Prerequisites**:
- Ruby 3.0+

**Build**:
```bash
cd crates/languages/cb-lang-ruby/resources/ruby-parser
bundle install
```

**Documentation updated**:
- [x] LANGUAGE_PLUGIN_PREREQUISITES.md
- [x] Verification script
- [x] CI workflow
```
```

**Also Update**:
- `crates/languages/README.md` → Add link to this doc
- Root `README.md` → Mention external dependencies in setup section

**Impact**: Saves 30-60 minutes of trial-and-error per new developer

**Implementation**:
```bash
# Create new file
touch docs/development/LANGUAGE_PLUGIN_PREREQUISITES.md
# Paste content above
# Update links in other docs
```

---

### 3. Enhanced Scaffolding Script (MEDIUM IMPACT, 1 hour) 🔧

**Problem**: `new-lang.sh` created C# plugin with `tempfile = { workspace = true }` but tempfile wasn't in root workspace deps → build failure

**Solution**: Add dependency validation to scaffolding script

**Location**: `crates/languages/new-lang.sh`

**Changes**:

```bash
#!/bin/bash
# File: crates/languages/new-lang.sh
# ... existing code ...

# AFTER plugin generation, ADD this section:

echo ""
echo "🔍 Checking workspace dependencies..."

# Extract dev-dependencies that reference workspace
WORKSPACE_DEPS=$(grep -A 100 "\[dev-dependencies\]" "$PLUGIN_DIR/Cargo.toml" | \
                 grep "{ workspace = true }" | \
                 awk -F'=' '{print $1}' | \
                 tr -d ' ' | \
                 grep -v "^#" | \
                 grep -v "^\[")

# Check if each dependency exists in root workspace
MISSING_DEPS=()
for dep in $WORKSPACE_DEPS; do
    if ! grep -q "^$dep = " "../../Cargo.toml"; then
        MISSING_DEPS+=("$dep")
    fi
done

if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
    echo "⚠️  Warning: Workspace dependencies not found in root Cargo.toml:"
    echo ""
    for dep in "${MISSING_DEPS[@]}"; do
        echo "   ❌ $dep"
    done
    echo ""
    echo "📝 Action required:"
    echo "   Add these to root Cargo.toml under [workspace.dependencies]:"
    echo ""
    for dep in "${MISSING_DEPS[@]}"; do
        echo "   $dep = \"<version>\""
    done
    echo ""
    echo "   Or update $PLUGIN_DIR/Cargo.toml to specify versions directly:"
    echo "   (Remove '{ workspace = true }' and add version number)"
    echo ""
    read -p "Press Enter to continue or Ctrl+C to fix now..."
else
    echo "✅ All workspace dependencies found"
fi

echo ""
echo "📋 Next steps:"
echo "   1. cargo build --features lang-$LANG_NAME"
echo "   2. Implement parsing in src/parser.rs"
echo "   3. Implement manifest in src/manifest.rs"
echo "   4. cargo test -p cb-lang-$LANG_NAME"
echo ""
echo "📖 Documentation:"
echo "   - crates/languages/README.md"
echo "   - docs/development/LANGUAGE_PLUGIN_PREREQUISITES.md"
echo ""
```

**Example output**:
```
🔍 Checking workspace dependencies...
⚠️  Warning: Workspace dependencies not found in root Cargo.toml:

   ❌ tempfile

📝 Action required:
   Add these to root Cargo.toml under [workspace.dependencies]:

   tempfile = "3.10"

   Or update crates/languages/cb-lang-csharp/Cargo.toml to specify versions directly:
   (Remove '{ workspace = true }' and add version number)

Press Enter to continue or Ctrl+C to fix now...
```

**Impact**: Catches build errors **before** first compile attempt

**Implementation**:
```bash
# Edit crates/languages/new-lang.sh
# Add dependency checking section after plugin generation
# Test with: ./new-lang.sh test-lang --manifest "*.test" --extensions tst
```

---

### 4. Onboarding Documentation (HIGH IMPACT, 2 hours) 📚

**Problem**: Jules had to piece together information from multiple sources without a clear roadmap

**Solution**: Create step-by-step onboarding guide

**Location**: `docs/development/LANGUAGE_PLUGIN_ONBOARDING.md` (new file)

**Content**:
```markdown
# Language Plugin Development - Onboarding Guide

Welcome to language plugin development! This guide will walk you through adding a new language to Codebuddy, step by step.

**Estimated time**: 2-24 hours depending on scope (see [complexity matrix](#complexity-by-scope))

---

## Table of Contents

1. [Before You Start](#before-you-start)
2. [Step-by-Step Guide](#step-by-step-guide)
3. [Complexity by Scope](#complexity-by-scope)
4. [Common Patterns](#common-patterns)
5. [Troubleshooting](#troubleshooting)
6. [Success Checklist](#success-checklist)

---

## Before You Start

### Prerequisites (15-30 minutes)

**1. Read the architecture documentation**:
- [ ] `crates/languages/README.md` - Plugin system overview
- [ ] `docs/development/LANGUAGE_PLUGIN_PREREQUISITES.md` - External dependencies
- [ ] `API_REFERENCE.md` - Language support matrix

**2. Decide on your scope**:
See [Complexity by Scope](#complexity-by-scope) below. Choose:
- **Minimal** (2-4h): Just parsing
- **Standard** (4-8h): + ImportSupport
- **Complete** (8-12h): + WorkspaceSupport
- **Advanced** (12-24h): + RefactoringSupport

**3. Check external dependencies**:
- [ ] Does your language need an external parser runtime? (Java, .NET, Swift, etc.)
- [ ] If yes: Install prerequisites (see LANGUAGE_PLUGIN_PREREQUISITES.md)

**4. Review reference implementations**:

Find a similar language:
- **Java → C#**: Both use external parsers, XML manifests
- **TypeScript → JavaScript**: Similar syntax, shared patterns
- **Go → Rust**: Both compiled, similar module systems

Study 2-3 existing plugins:
- [ ] How do they parse source code? (native vs subprocess)
- [ ] How do they handle manifests?
- [ ] Which traits do they implement?

---

## Step-by-Step Guide

### Step 0: Environment Setup (5 minutes)

```bash
# Ensure you're on a clean branch
git checkout -b feat/add-<language>-support

# Verify build works
cargo build

# Install language prerequisites if needed
# (See LANGUAGE_PLUGIN_PREREQUISITES.md)
```

---

### Step 1: Scaffold Plugin (5 minutes)

```bash
cd crates/languages

# Run scaffolding script
./new-lang.sh <language> \
  --manifest "<manifest_pattern>" \
  --extensions <ext1,ext2>

# Examples:
# ./new-lang.sh csharp --manifest "*.csproj" --extensions cs,csx
# ./new-lang.sh ruby --manifest "Gemfile" --extensions rb
# ./new-lang.sh kotlin --manifest "build.gradle.kts" --extensions kt,kts
```

**What this creates**:
- `crates/languages/cb-lang-<language>/` directory
- Boilerplate files: `src/lib.rs`, `src/parser.rs`, `src/manifest.rs`, `Cargo.toml`, `README.md`
- Registration in `languages.toml`
- Updates to language enum

**Verify**:
```bash
# Should compile (with warnings)
cargo build --features lang-<language>
```

**Checkpoint ✅**: Plugin scaffolded, builds successfully

---

### Step 2: Implement Manifest Parsing (1-2 hours)

**File**: `crates/languages/cb-lang-<language>/src/manifest.rs`

**Goal**: Parse your language's manifest file (Cargo.toml, package.json, *.csproj, etc.)

**Implementation pattern**:

```rust
// Example for JSON manifest (package.json-like)
use serde::Deserialize;
use std::path::Path;
use cb_plugin_api::{ManifestData, Dependency, PluginResult};

#[derive(Deserialize)]
struct MyManifest {
    name: String,
    version: String,
    dependencies: Option<HashMap<String, String>>,
}

pub async fn analyze_manifest(path: &Path) -> PluginResult<ManifestData> {
    let content = tokio::fs::read_to_string(path).await?;
    let manifest: MyManifest = serde_json::from_str(&content)?;

    let dependencies = manifest.dependencies
        .unwrap_or_default()
        .into_iter()
        .map(|(name, version)| Dependency {
            name,
            version: Some(version),
            source: DependencySource::Registry,
            optional: false,
        })
        .collect();

    Ok(ManifestData {
        project_name: Some(manifest.name),
        version: Some(manifest.version),
        dependencies,
    })
}
```

**Patterns for different manifest types**:
- **TOML** (Rust): Use `toml::from_str()` or `toml_edit::Document`
- **JSON** (TypeScript): Use `serde_json::from_str()`
- **XML** (Java, C#): Use `quick-xml` with `serde`
- **Custom** (Go): Regex or custom parser

**Test**:
```bash
cargo test -p cb-lang-<language> manifest
```

**Checkpoint ✅**: Manifest parsing works

---

### Step 3: Implement Source Parsing (2-4 hours)

**File**: `crates/languages/cb-lang-<language>/src/parser.rs`

**Goal**: Parse source code and extract symbols (functions, classes, etc.)

**Choose your approach**:

#### Option A: Native Rust Parser
**When**: Good Rust parser library exists (syn for Rust, tree-sitter bindings)
**Pros**: Fast, no external dependencies
**Cons**: Limited languages

```rust
pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    // Use parser library
    let syntax_tree = some_parser::parse(source)?;

    let symbols = extract_symbols(&syntax_tree);

    Ok(ParsedSource {
        symbols,
        errors: vec![],
    })
}
```

#### Option B: Subprocess Parser
**When**: Official language parser available (Roslyn, SourceKitten)
**Pros**: Accurate, comprehensive
**Cons**: Requires external runtime

```rust
use cb_lang_common::SubprocessAstTool;

pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    let tool = SubprocessAstTool::new("csharp-parser");
    let output = tool.parse(source)?;
    let symbols = serde_json::from_str(&output)?;

    Ok(ParsedSource {
        symbols,
        errors: vec![],
    })
}
```

**For subprocess approach**:
1. Create parser tool in `resources/<language>-parser/`
2. See existing examples: `cb-lang-java`, `cb-lang-csharp`
3. Document build steps in plugin README

#### Option C: Regex Fallback
**When**: No good parser, or as backup
**Pros**: Always works
**Cons**: Inaccurate, fragile

```rust
use regex::Regex;

pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    let func_regex = Regex::new(r"function\s+(\w+)")?;

    let symbols = func_regex.captures_iter(source)
        .map(|cap| Symbol {
            name: cap[1].to_string(),
            kind: SymbolKind::Function,
            location: /* ... */,
        })
        .collect();

    Ok(ParsedSource { symbols, errors: vec![] })
}
```

**Test**:
```bash
cargo test -p cb-lang-<language> parser
```

**Checkpoint ✅**: Source parsing works

---

### Step 4: Wire Up Plugin (30 minutes)

**File**: `crates/languages/cb-lang-<language>/src/lib.rs`

**Goal**: Implement `LanguagePlugin` trait

```rust
use cb_plugin_api::{LanguagePlugin, LanguageMetadata, LanguageCapabilities};

pub struct MyLanguagePlugin {
    metadata: LanguageMetadata,
}

impl MyLanguagePlugin {
    pub fn new() -> Self {
        Self {
            metadata: LanguageMetadata::MY_LANGUAGE, // Auto-generated
        }
    }
}

#[async_trait]
impl LanguagePlugin for MyLanguagePlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            imports: false,  // Update when ImportSupport implemented
            workspace: false,
        }
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        parser::parse_source(source)
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        manifest::analyze_manifest(path).await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

**Test**:
```bash
cargo test -p cb-lang-<language>
cargo build --features lang-<language>
```

**Checkpoint ✅**: Basic plugin works end-to-end

---

### Step 5: Optional Traits (Choose based on scope)

#### 5A: ImportSupport (2-4 hours)

**File**: `crates/languages/cb-lang-<language>/src/import_support.rs`

```rust
use cb_plugin_api::ImportSupport;

pub struct MyLanguageImportSupport;

impl ImportSupport for MyLanguageImportSupport {
    fn parse_imports(&self, content: &str) -> Vec<String> {
        // Extract import statements
        // Example: "import foo" → ["foo"]
    }

    fn rewrite_imports_for_rename(&self, content: &str, old: &str, new: &str) -> (String, usize) {
        // Replace "import old" with "import new"
    }

    // ... other methods
}
```

**Update lib.rs**:
```rust
impl LanguagePlugin for MyLanguagePlugin {
    fn import_support(&self) -> Option<&dyn ImportSupport> {
        Some(&self.import_support)
    }
}
```

#### 5B: WorkspaceSupport (4-8 hours)

**File**: `crates/languages/cb-lang-<language>/src/workspace_support.rs`

See `cb-lang-rust` for reference implementation.

#### 5C: RefactoringSupport (8-16 hours)

⚠️ **STOP**: Before implementing, check:
```bash
grep -r "trait RefactoringSupport" crates/cb-plugin-api/
```

**If trait doesn't exist**:
1. Create GitHub issue describing needed operations
2. Coordinate with maintainers on trait design
3. **Do not proceed without approval**

**If trait exists**:
Implement following existing patterns (see `cb-lang-rust/src/refactoring_support.rs`)

---

### Step 6: Integration Testing (1-2 hours)

**File**: `integration-tests/tests/e2e_<language>_features.rs` (new file)

```rust
use integration_tests::harness::{TestClient, TestWorkspace};

#[tokio::test]
async fn test_parse_<language>_source() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let file = workspace.path().join("test.<ext>");
    std::fs::write(&file, "<sample code>").unwrap();

    let response = client.call_tool("parse_file", json!({
        "file_path": file.to_str().unwrap()
    })).await.unwrap();

    assert!(response["result"]["symbols"].as_array().unwrap().len() > 0);
}
```

---

### Step 7: Documentation (30 minutes)

**Update these files**:

- [ ] `crates/languages/cb-lang-<language>/README.md`
  - Usage examples
  - External dependencies
  - Build instructions

- [ ] `API_REFERENCE.md`
  - Add language to support matrix

- [ ] `README.md` (root)
  - Add to supported languages list
  - Document external dependencies if any

- [ ] `CONTRIBUTING.md`
  - Add to language plugin examples

---

### Step 8: Pre-Commit Checks (15 minutes)

```bash
# Format code
cargo fmt

# Lint
cargo clippy --features lang-<language>

# Full test suite
cargo test --workspace

# Build release
cargo build --release --features lang-<language>
```

---

### Step 9: Create Pull Request

**PR Template**:
```markdown
## New Language: <Language Name>

**Scope**: [Minimal / Standard / Complete / Advanced]

**Features Implemented**:
- [x] Basic parsing
- [x] Manifest parsing
- [ ] ImportSupport
- [ ] WorkspaceSupport
- [ ] RefactoringSupport

**External Dependencies**:
- <Runtime>: <version>
- Build: `<build command>`

**Testing**:
- Unit tests: <X> passing
- Integration tests: <Y> passing
- Manual testing: [describe what you tested]

**Documentation Updated**:
- [x] Plugin README
- [x] LANGUAGE_PLUGIN_PREREQUISITES.md (if external deps)
- [x] API_REFERENCE.md
- [x] Root README.md

**References**:
- Related issue: #<number>
- Similar implementations: cb-lang-<similar>
```

---

## Complexity by Scope

### Minimal Plugin (2-4 hours)
**What you get**:
- Source code parsing
- Manifest parsing
- Symbol extraction
- Basic MCP integration

**Good for**:
- Quick prototyping
- LSP integration only
- No refactoring needed

**Time breakdown**:
- Scaffolding: 5 min
- Manifest parsing: 1h
- Source parsing: 1-2h
- Integration: 30 min
- Documentation: 30 min

---

### Standard Plugin (4-8 hours)
**What you get**:
- Minimal plugin features
- **+ ImportSupport trait**
- Import analysis
- Import rewriting

**Good for**:
- Code navigation with imports
- Basic refactoring (rename with import updates)

**Time breakdown**:
- Minimal plugin: 2-4h
- ImportSupport implementation: 2-3h
- Additional testing: 1h

---

### Complete Plugin (8-12 hours)
**What you get**:
- Standard plugin features
- **+ WorkspaceSupport trait**
- Manifest manipulation
- Workspace-wide operations

**Good for**:
- Full IDE-like experience
- Dependency management
- Multi-package workspaces

**Time breakdown**:
- Standard plugin: 4-8h
- WorkspaceSupport implementation: 3-4h
- Additional testing: 1-2h

---

### Advanced Plugin (12-24 hours)
**What you get**:
- Complete plugin features
- **+ RefactoringSupport trait**
- Extract module to package
- Advanced refactoring operations

**Good for**:
- Full refactoring support
- Large-scale code reorganization

⚠️ **Warning**: May require trait design if RefactoringSupport doesn't exist

**Time breakdown**:
- Complete plugin: 8-12h
- Trait design (if needed): 2-4h
- RefactoringSupport implementation: 4-8h
- Core refactoring: 2-4h (if trait new)
- Extensive testing: 2-4h

---

## Common Patterns

### Pattern 1: Subprocess Parser

**When to use**: Official language parser available

**Structure**:
```
crates/languages/cb-lang-<language>/
├── resources/
│   └── <language>-parser/
│       ├── src/ or Program.cs or main.go
│       ├── Cargo.toml or .csproj or go.mod
│       └── README.md (build instructions)
├── src/
│   ├── parser.rs (calls subprocess)
│   └── ...
```

**Implementation**:
```rust
use cb_lang_common::subprocess::run_ast_tool;

pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    let binary_path = "resources/<language>-parser/bin/parser";
    let output = run_ast_tool(binary_path, source)?;
    let data: ParsedSource = serde_json::from_str(&output)?;
    Ok(data)
}
```

**Examples**: `cb-lang-java`, `cb-lang-csharp`

---

### Pattern 2: Dual-Mode Parser

**When to use**: Want accuracy + fallback

**Implementation**:
```rust
pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    // Try subprocess first
    match parse_with_subprocess(source) {
        Ok(result) => Ok(result),
        Err(e) => {
            warn!("Subprocess parser failed, using regex fallback: {}", e);
            parse_with_regex(source)
        }
    }
}
```

**Examples**: `cb-lang-go`

---

### Pattern 3: Native Rust Parser

**When to use**: Excellent Rust library exists

**Implementation**:
```rust
use syn::{parse_file, Item};

pub fn parse_source(source: &str) -> PluginResult<ParsedSource> {
    let ast = parse_file(source)?;
    let symbols = ast.items.iter()
        .filter_map(|item| match item {
            Item::Fn(f) => Some(Symbol::from(f)),
            _ => None,
        })
        .collect();
    Ok(ParsedSource { symbols, errors: vec![] })
}
```

**Examples**: `cb-lang-rust` (uses `syn`)

---

## Troubleshooting

### Problem: Build fails with "non-exhaustive match"

**Cause**: Adding language to `ProjectLanguage` enum broke existing match statements

**Solution**:
```bash
# Find all matches on ProjectLanguage
rg "ProjectLanguage::" --type rust

# Update each match to include your language
# OR add wildcard: _ => { /* default */ }
```

**Files commonly affected**:
- `crates/cb-ast/src/package_extractor.rs`
- `crates/cb-handlers/src/handlers/plugin_dispatcher.rs`

---

### Problem: Workspace dependency not found

**Cause**: Plugin Cargo.toml uses `{ workspace = true }` but dep not in root

**Solution**:
```bash
# Add to root Cargo.toml
[workspace.dependencies]
your-dep = "version"

# OR remove workspace reference
# In plugin Cargo.toml:
your-dep = "version"  # Remove { workspace = true }
```

---

### Problem: External parser not found

**Cause**: Parser not built or not in expected location

**Solution**:
```bash
# Build parser
cd crates/languages/cb-lang-<language>/resources/<language>-parser
<build command>  # See LANGUAGE_PLUGIN_PREREQUISITES.md

# Verify artifact exists
ls bin/ or ls target/
```

---

### Problem: Tests fail with "plugin not registered"

**Cause**: Plugin not added to registry

**Solution**:
```bash
# Check registration in:
# crates/cb-services/src/services/registry_builder.rs

# Should have:
#[cfg(feature = "lang-<language>")]
registry.register(Box::new(cb_lang_<language>::<Language>Plugin::new()));
```

---

## Success Checklist

**Before submitting PR**:

### Code
- [ ] Plugin compiles: `cargo build --features lang-<language>`
- [ ] Tests pass: `cargo test -p cb-lang-<language>`
- [ ] Full test suite passes: `cargo test --workspace`
- [ ] Clippy clean: `cargo clippy --features lang-<language>`
- [ ] Formatted: `cargo fmt`

### Documentation
- [ ] Plugin README complete with examples
- [ ] External dependencies documented (if any)
- [ ] API_REFERENCE.md updated
- [ ] Root README.md updated
- [ ] LANGUAGE_PLUGIN_PREREQUISITES.md updated (if external deps)

### Integration
- [ ] Language registered in `languages.toml`
- [ ] Language enum updated
- [ ] Feature flag works: `cargo build --features lang-<language>`
- [ ] Plugin appears in MCP tools list

### Testing
- [ ] Unit tests for parser
- [ ] Unit tests for manifest
- [ ] Unit tests for traits (if implemented)
- [ ] Integration test (optional but recommended)
- [ ] Manual testing with real code samples

---

## Next Steps

After your PR is merged:

1. **Monitor issues**: Watch for bug reports related to your language
2. **Improve coverage**: Add more tests as edge cases are discovered
3. **Enhance features**: Implement additional traits over time
4. **Help others**: Review PRs for similar languages

---

## Questions?

- **Stuck?** Check similar implementations in `crates/languages/`
- **Trait design?** Create GitHub issue for discussion
- **External parser issues?** See LANGUAGE_PLUGIN_PREREQUISITES.md
- **Still stuck?** Ask in GitHub Discussions

**Good luck!** 🚀
```

**Impact**: Reduces onboarding time by 50%, provides clear roadmap

**Implementation**:
```bash
# Create file
touch docs/development/LANGUAGE_PLUGIN_ONBOARDING.md
# Paste content above
# Add link from crates/languages/README.md
```

---

### 5. Pre-commit Hook for Parser Artifacts (LOW IMPACT, 30 minutes) 🔧

**Problem**: Jules encountered missing Java parser artifact, blocking builds

**Solution**: Add pre-commit hook to verify external parsers are built

**Location**: `.git/hooks/pre-commit` (or use `husky`/`pre-commit` framework)

**Content**:
```bash
#!/bin/bash
# File: .git/hooks/pre-commit

echo "🔍 Checking external parser artifacts..."

MISSING=()

# Check Java parser
JAVA_JAR="crates/languages/cb-lang-java/resources/java-parser/target/java-parser-1.0-SNAPSHOT.jar"
if [ -d "crates/languages/cb-lang-java" ] && [ ! -f "$JAVA_JAR" ]; then
    MISSING+=("Java parser JAR")
fi

# Check C# parser
CSHARP_EXE="crates/languages/cb-lang-csharp/resources/csharp-parser/bin/Release/net8.0/linux-x64/csharp-parser"
if [ -d "crates/languages/cb-lang-csharp" ] && [ ! -f "$CSHARP_EXE" ]; then
    MISSING+=("C# parser executable")
fi

# Add more checks as languages are added...

if [ ${#MISSING[@]} -gt 0 ]; then
    echo "❌ Missing external parser artifacts:"
    for artifact in "${MISSING[@]}"; do
        echo "   - $artifact"
    done
    echo ""
    echo "To fix:"
    echo "   make build-parsers"
    echo ""
    echo "Or bypass this check:"
    echo "   git commit --no-verify"
    echo ""
    exit 1
fi

echo "✅ All parser artifacts present"
```

**Also add Makefile target**:
```makefile
# File: Makefile

.PHONY: build-parsers
build-parsers:
	@echo "🔨 Building external language parsers..."
	@echo "Building Java parser..."
	@cd crates/languages/cb-lang-java/resources/java-parser && mvn package
	@echo "Building C# parser..."
	@cd crates/languages/cb-lang-csharp/resources/csharp-parser && \
		dotnet publish -c Release -r linux-x64 --self-contained
	@echo "✅ All parsers built"

.PHONY: check-parser-deps
check-parser-deps:
	@echo "Checking parser build dependencies..."
	@command -v mvn >/dev/null 2>&1 || echo "❌ Maven not found"
	@command -v dotnet >/dev/null 2>&1 || echo "❌ .NET SDK not found"
	@command -v sourcekitten >/dev/null 2>&1 || echo "⚠️  SourceKitten not found"
	@echo "✅ Dependency check complete"
```

**Impact**: Prevents "works on my machine" build failures

**Implementation**:
```bash
# Make hook executable
chmod +x .git/hooks/pre-commit

# Test
git add .
git commit -m "test" --dry-run
```

---

## Biggest Win: RefactoringSupport Trait 🏆

### What Jules Created

**Before Jules**:
```rust
// crates/cb-ast/src/package_extractor.rs (1500+ lines)

// Hardcoded Rust-only logic
use cb_lang_rust::RustPlugin;

let rust_plugin = plugin
    .as_any()
    .downcast_ref::<RustPlugin>()?;  // ❌ Type-unsafe

// Only works for Rust
rust_plugin.locate_module_files(...);
```

**After Jules**:
```rust
// crates/cb-ast/src/package_extractor.rs (500 lines - generic)

// Language-agnostic
let refactor_support = plugin.refactoring_support()?;  // ✅ Type-safe

// Works for ANY language implementing trait
refactor_support.locate_module_files(...);
```

**New trait** (`crates/cb-plugin-api/src/refactoring_support.rs`):
```rust
pub trait RefactoringSupport: Send + Sync {
    async fn locate_module_files(&self, ...) -> PluginResult<Vec<PathBuf>>;
    async fn generate_manifest(&self, ...) -> PluginResult<String>;
    async fn add_manifest_path_dependency(&self, ...) -> PluginResult<String>;
    async fn remove_module_declaration(&self, ...) -> PluginResult<String>;
    fn rewrite_import(&self, ...) -> String;
    async fn parse_imports(&self, ...) -> PluginResult<Vec<String>>;
    async fn find_source_files(&self, ...) -> PluginResult<Vec<PathBuf>>;
}
```

### Impact Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Lines in package_extractor.rs | 1,508 | 500 | -1,008 lines (-67%) |
| Languages supported | 1 (Rust) | Any | ∞% increase 😄 |
| Type safety | Downcast (unsafe) | Trait (safe) | ✅ |
| Extensibility | Hardcoded | Pluggable | ✅ |
| Maintainability | Poor | Excellent | ✅ |

### Why This Matters

**For C#**:
- Can now support `extract_module_to_package`
- Partial implementation already in place

**For future languages**:
- Clear contract for advanced refactoring
- No core code changes needed
- Just implement 7 trait methods

**For codebase health**:
- Removed 1,000+ lines of language-specific code from core
- Better separation of concerns
- Easier to maintain and test

### Celebrate This! 🎉

This is **not a problem** - it's an **architectural improvement** that emerged from real needs:

1. ✅ Trait design is excellent
2. ✅ Implementation is clean
3. ✅ Backward compatible (default `None`)
4. ✅ Benefits entire codebase

**Recommendation**: Document this in architecture docs as a **success pattern** for others to follow.

---

## Summary of Impact

| Improvement | Impact | Effort | ROI |
|------------|--------|--------|-----|
| 1. Complexity Matrix | HIGH | 15 min | ⭐⭐⭐⭐⭐ |
| 2. External Deps Guide | HIGH | 30 min | ⭐⭐⭐⭐⭐ |
| 3. Enhanced Scaffolding | MEDIUM | 1 hour | ⭐⭐⭐⭐ |
| 4. Onboarding Doc | HIGH | 2 hours | ⭐⭐⭐⭐⭐ |
| 5. Pre-commit Hook | LOW | 30 min | ⭐⭐⭐ |

**Total implementation time**: ~4.5 hours
**Estimated time saved per future developer**: 4-8 hours

**ROI after 2 new languages**: Break-even
**ROI after 5 new languages**: 20-40 hours saved

---

## Implementation Priority

### Phase 1: Quick Wins (1 hour total)
1. Feature Complexity Matrix (15 min)
2. External Dependencies Guide (30 min)
3. Enhanced Scaffolding Script (15 min) - basic version

**Impact**: Addresses 80% of Jules's pain points

### Phase 2: Comprehensive (3.5 hours total)
1. Onboarding Documentation (2 hours)
2. Enhanced Scaffolding Script (45 min) - full version
3. Pre-commit Hook (30 min)

**Impact**: Creates best-in-class developer experience

---

## Action Items

**For maintainers**:
- [ ] Review this document
- [ ] Approve improvements to implement
- [ ] Assign implementation tasks
- [ ] Create tracking issue

**For documentation**:
- [ ] Implement Phase 1 improvements (1 hour)
- [ ] Test with hypothetical "Ruby plugin" scenario
- [ ] Gather feedback from next language implementation
- [ ] Iterate based on feedback

**For Jules (feedback)**:
- ✅ Excellent work on RefactoringSupport trait
- ✅ C# plugin implementation is high quality
- ✅ Process improvement suggestions documented
- 📝 This analysis will help future developers avoid the same friction

---

## Conclusion

Jules successfully delivered **high-quality work** under challenging circumstances:

**Achievements**:
- ✅ Complete C# language plugin
- ✅ Roslyn-based parser (production-grade)
- ✅ RefactoringSupport trait (architectural improvement)
- ✅ Removed 1,008 lines from core

**Challenges overcome**:
- ⚠️ Undocumented external dependencies
- ⚠️ Mid-development architectural pivot
- ⚠️ Build system integration issues
- ⚠️ Sandbox environment problems

**Lessons learned**:
- Document feature complexity upfront
- Provide clear onboarding path
- Check workspace dependencies early
- Celebrate architectural improvements

**Recommendation**: Implement **Phase 1 improvements immediately** (1 hour) to prevent future friction. The ROI is clear - Jules spent 4.5 hours on unplanned work that could have been avoided with 1 hour of documentation.

**Final grade**: ⭐⭐⭐⭐⭐ Excellent work, valuable lessons learned

Thank you, Jules! 🙏
