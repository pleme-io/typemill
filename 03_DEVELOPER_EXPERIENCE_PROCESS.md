# Process Improvement Proposal: Learning from C# Plugin Development

**Date**: 2025-10-07
**Source**: Analysis of C# branch development dialogue
**Goal**: Improve scaffolding, documentation, and developer experience

---

## Executive Summary

The C# plugin development process revealed several **workflow friction points** and **architectural insights** that can improve our system. The developer successfully completed the task but encountered:

1. ✅ **Good**: Scaffolding script worked well
2. ⚠️ **Issue**: Build system integration challenges
3. ⚠️ **Issue**: Mid-development architectural pivot required
4. ✅ **Success**: RefactoringSupport trait emerged as valuable pattern
5. ⚠️ **Issue**: Dependency management gaps

**Key Insight**: The developer discovered `extract_module_to_package` was Rust-only and had to refactor the entire system mid-development. This should have been documented upfront.

---

## Timeline Analysis

### Phase 1: Scaffolding (Oct 6 16:51 - 17:02)
**Duration**: 11 minutes
**Status**: ✅ Successful

```bash
cd crates/languages && ./new-lang.sh csharp --manifest "*.csproj" --extensions cs,csx
```

**What worked**:
- Script generated boilerplate successfully
- Directory structure created correctly
- Language registered in `languages.toml`

**Issue encountered**:
- Build failed due to missing Java parser artifact
- Non-exhaustive match in `package_extractor.rs`

**Action taken**:
```bash
cd crates/languages/cb-lang-java/resources/java-parser && mvn package
```

**Lesson**: Build dependencies should be pre-checked or documented.

---

### Phase 2: Manifest Parsing (Oct 6 17:13 - 17:23)
**Duration**: 10 minutes
**Status**: ✅ Successful

**Implementation**:
- Used `quick-xml` for `.csproj` parsing
- Platform-agnostic path normalization
- Proper XML deserialization

**What worked**:
- Existing patterns in other plugins provided clear guidance
- `cb-lang-common` utilities helpful

---

### Phase 3: Parser Setup (Oct 6 17:23 - 17:52)
**Duration**: 29 minutes
**Status**: ⚠️ Partial (sandbox issues)

**Approach**:
- Created .NET console app for Roslyn-based parsing
- Installed .NET SDK on system
- Built self-contained executable

**Issues encountered**:
- Sandbox environment corruption (`getcwd` errors)
- Multiple retry attempts needed

**Lesson**: External language runtimes (Java, .NET) should have setup guides.

---

### Phase 4: Architectural Discovery (Oct 6 17:52 - 22:18)
**Duration**: 4 hours 26 minutes
**Status**: 🔴 Major pivot required

**Critical Discovery**:
> "The extract_module_to_package functionality is very tightly coupled to the Rust language plugin. It won't work for C# without significant refactoring."

**Developer's question**:
> "Would you agree that we should proceed with the current plan and address the extract_module_to_package refactoring as a separate, future task?"

**User's response**:
> "no, address the refactoring now, so c# works as the other languages should already"

**Lesson**: **This is the critical failure point** - developer wasn't informed upfront that advanced refactoring support would require system-level changes.

---

### Phase 5: Refactoring System (Oct 6 22:18 - 23:11)
**Duration**: 53 minutes
**Status**: ✅ Successful (major architectural improvement)

**What was done**:
1. Created `RefactoringSupport` trait
2. Moved Rust-specific code to plugin
3. Made `extract_module_to_package` language-agnostic
4. Reduced `package_extractor.rs` by 1,008 lines

**Result**: **High-quality architectural improvement**, but unplanned work.

---

### Phase 6: C# Implementation Resume (Oct 6 23:11 - 23:22)
**Duration**: 11 minutes
**Status**: ✅ Completed

**Deliverables**:
- C# parser (Roslyn-based)
- Manifest parser
- RefactoringSupport implementation (partial)
- Full integration

---

## Problems Identified

### 1. Documentation Gap: Capability Matrix
**Problem**: Developer didn't know which features require system-level work vs plugin-level work.

**What was missing**:
```
Feature Complexity Matrix:
┌────────────────────────┬──────────────┬────────────────────┐
│ Feature                │ Complexity   │ System Changes?    │
├────────────────────────┼──────────────┼────────────────────┤
│ Basic parsing          │ Simple       │ No                 │
│ Manifest parsing       │ Simple       │ No                 │
│ ImportSupport          │ Medium       │ No                 │
│ WorkspaceSupport       │ Medium       │ No                 │
│ RefactoringSupport     │ High         │ YES (create trait) │
│ extract_module_to_pkg  │ Very High    │ YES (refactor core)│
└────────────────────────┴──────────────┴────────────────────┘
```

**Recommendation**: Add this to `crates/languages/README.md`

---

### 2. Scaffolding Script Incomplete
**Problem**: `new-lang.sh` doesn't add dependency to workspace `Cargo.toml`

**What happened**:
- C# plugin uses `tempfile = { workspace = true }`
- But `tempfile` not in root workspace dependencies
- Build fails

**Current workflow**:
```bash
./new-lang.sh csharp ...
# ❌ Developer must manually add to root Cargo.toml
```

**Should be**:
```bash
./new-lang.sh csharp ...
# ✅ Script checks workspace deps and prompts or auto-adds
```

**Recommendation**: Enhance `new-lang.sh` to:
1. Check if all dev-dependencies exist in workspace
2. Prompt to add missing ones
3. Update root `Cargo.toml` automatically (with confirmation)

---

### 3. External Runtime Dependencies Unclear
**Problem**: Developer had to figure out .NET SDK installation on the fly

**What was needed but not documented**:
- Java: Maven + JDK for Java parser
- C#: .NET 8.0 SDK for C# parser
- Swift: SourceKitten + Swift CLI

**Current state**: Scattered in READMEs, not centralized

**Recommendation**: Create `docs/development/LANGUAGE_PLUGIN_PREREQUISITES.md`:

```markdown
# Language Plugin Development Prerequisites

## External Language Runtimes

### Java Plugin
- Maven 3.6+
- JDK 11+
- Build: `cd resources/java-parser && mvn package`

### C# Plugin
- .NET 8.0 SDK
- Build: `cd resources/csharp-parser && dotnet publish -c Release`

### Swift Plugin
- SourceKitten: `brew install sourcekitten`
- Swift CLI: Included with Xcode

### Go Plugin
- Go 1.19+ (for parser if using subprocess approach)

### TypeScript Plugin
- Node.js 18+ (for ast_tool.js)
```

---

### 4. Build System Integration Issues
**Problem**: Multiple build failures unrelated to C# work

**Failures encountered**:
1. Missing `java-parser.jar` (different plugin!)
2. Non-exhaustive match in `package_extractor.rs` (core code!)

**Why this happened**:
- Adding C# to `ProjectLanguage` enum broke existing match statement
- Java parser artifact not checked in / not built automatically

**Recommendation**:
1. **Pre-commit hook** to build all parsers
2. **CI check** for external artifacts
3. **Exhaustive match linting** for language enums

---

### 5. Architectural Guidance Missing
**Problem**: No guidance on when to create new traits vs when to implement existing ones

**Developer's discovery process**:
1. Started implementing C# plugin
2. Realized `extract_module_to_package` is Rust-only
3. Asked if should defer refactoring
4. User said no, do it now
5. Had to design RefactoringSupport trait from scratch

**Should have been**:
1. Documentation shows: "RefactoringSupport trait needed for advanced refactoring"
2. Developer knows upfront this is system-level work
3. Can plan accordingly

**Recommendation**: Add to `crates/languages/README.md`:

```markdown
## Trait Implementation Levels

### Level 1: Basic Plugin (1-2 hours)
- `LanguagePlugin` trait (required)
- Parsing + symbol extraction
- **No system changes needed**

### Level 2: Import Support (2-4 hours)
- `ImportSupport` trait (optional)
- Import parsing and rewriting
- **No system changes needed** (trait exists)

### Level 3: Workspace Support (4-8 hours)
- `WorkspaceSupport` trait (optional)
- Manifest operations
- **No system changes needed** (trait exists)

### Level 4: Advanced Refactoring (8-16 hours)
- `RefactoringSupport` trait (optional)
- Enables `extract_module_to_package`
- ⚠️ **May require system changes if trait doesn't exist**
- ⚠️ **Coordinate with maintainers first**
```

---

## What Went Well

### 1. Scaffolding Script ✅
The `new-lang.sh` script worked perfectly for initial setup:
- Created directory structure
- Generated boilerplate files
- Registered in `languages.toml`
- Updated language enum

**Evidence**: Developer used it successfully without issues.

---

### 2. Common Utilities ✅
The `cb-lang-common` crate provided helpful abstractions:
- Error handling utilities
- Subprocess execution patterns
- Manifest reading helpers

**Evidence**: Developer specifically mentioned reviewing common directory.

---

### 3. Existing Plugin Patterns ✅
Having multiple reference implementations (Rust, Go, TypeScript, Python, Java) was helpful:
- Developer could compare approaches
- Patterns emerged clearly
- Copy-paste-adapt workflow

**Evidence**: Developer compared implementations before choosing approach.

---

### 4. Architecture Flexibility ✅
The system was flexible enough to accommodate major refactoring:
- RefactoringSupport trait emerged naturally
- Backward compatible (default `None` return)
- Clean separation of concerns

**Evidence**: 1,008 lines removed from core, moved to plugin.

---

### 5. Quality Result ✅
Despite challenges, final implementation is high quality:
- Roslyn-based parser (production-grade)
- Proper XML manifest parsing
- RefactoringSupport trait (architectural improvement)
- Full integration

---

## Recommendations

### Priority 1: Documentation (HIGH IMPACT, LOW EFFORT)

#### A. Create Feature Complexity Matrix
**File**: `crates/languages/README.md`

```markdown
## Feature Implementation Complexity

| Feature | Time | System Changes? | Prerequisites |
|---------|------|-----------------|---------------|
| Basic parsing | 1-2h | No | Language runtime for parser |
| Manifest parsing | 1-2h | No | - |
| ImportSupport | 2-4h | No | Trait exists |
| WorkspaceSupport | 2-4h | No | Trait exists |
| RefactoringSupport | 8-16h | **Maybe** | May need to create trait |

**Note**: If implementing RefactoringSupport and the trait doesn't exist,
coordinate with maintainers to design the trait first.
```

#### B. Create Prerequisites Guide
**File**: `docs/development/LANGUAGE_PLUGIN_PREREQUISITES.md`

(See content in section 3 above)

#### C. Update Scaffolding README
**File**: `crates/languages/README.md`

Add section:
```markdown
## Common Pitfalls

1. **Workspace dependency errors**
   - Problem: `tempfile = { workspace = true }` but tempfile not in root
   - Solution: Check root `Cargo.toml` workspace.dependencies

2. **External parser build errors**
   - Problem: Java parser missing, .NET SDK not installed
   - Solution: See LANGUAGE_PLUGIN_PREREQUISITES.md

3. **Non-exhaustive match errors**
   - Problem: Adding language breaks existing match statements
   - Solution: Search codebase for `ProjectLanguage::` matches and update
```

---

### Priority 2: Tooling Improvements (MEDIUM IMPACT, MEDIUM EFFORT)

#### A. Enhance `new-lang.sh` Script

**Add dependency checking**:
```bash
#!/bin/bash
# In new-lang.sh

# After creating plugin files...

# Check if dev-dependencies exist in workspace
echo "Checking workspace dependencies..."
DEV_DEPS=$(grep -A 100 "\[dev-dependencies\]" "$PLUGIN_DIR/Cargo.toml" | \
           grep "workspace = true" | \
           awk -F'=' '{print $1}' | \
           tr -d ' ')

for dep in $DEV_DEPS; do
    if ! grep -q "^$dep = " ../../Cargo.toml; then
        echo "⚠️  Warning: $dep not found in workspace dependencies"
        echo "   Add to root Cargo.toml: $dep = \"<version>\""
    fi
done
```

#### B. Add Pre-commit Hook

**File**: `.git/hooks/pre-commit` (or use husky/pre-commit framework)

```bash
#!/bin/bash
# Check for external parser artifacts

missing_artifacts=()

# Check Java parser
if [ ! -f "crates/languages/cb-lang-java/resources/java-parser/target/java-parser-1.0-SNAPSHOT.jar" ]; then
    missing_artifacts+=("Java parser JAR")
fi

# Check C# parser (if lang-csharp feature enabled)
if grep -q 'lang-csharp' Cargo.toml; then
    if [ ! -f "crates/languages/cb-lang-csharp/resources/csharp-parser/bin/Release/net8.0/csharp-parser" ]; then
        missing_artifacts+=("C# parser executable")
    fi
fi

if [ ${#missing_artifacts[@]} -gt 0 ]; then
    echo "⚠️  Missing external parser artifacts:"
    for artifact in "${missing_artifacts[@]}"; do
        echo "   - $artifact"
    done
    echo ""
    echo "Run: make build-parsers"
    exit 1
fi
```

#### C. Add Makefile Target

**File**: `Makefile`

```makefile
.PHONY: build-parsers
build-parsers:
	@echo "Building external language parsers..."
	cd crates/languages/cb-lang-java/resources/java-parser && mvn package
	cd crates/languages/cb-lang-csharp/resources/csharp-parser && \
		dotnet publish -c Release -r linux-x64 --self-contained
	@echo "✅ All parsers built"

.PHONY: check-parser-deps
check-parser-deps:
	@echo "Checking parser dependencies..."
	@which mvn > /dev/null || echo "❌ Maven not found (required for Java parser)"
	@which dotnet > /dev/null || echo "❌ .NET SDK not found (required for C# parser)"
	@which sourcekitten > /dev/null || echo "❌ SourceKitten not found (required for Swift parser)"
	@echo "✅ Dependency check complete"
```

---

### Priority 3: Process Improvements (MEDIUM IMPACT, HIGH EFFORT)

#### A. Create "Trait Design Checklist"

**File**: `docs/development/TRAIT_DESIGN_CHECKLIST.md`

```markdown
# Adding a New Trait to the Plugin System

## Before You Start

Ask yourself:
1. Does this trait apply to **multiple languages**?
   - If yes: Create trait in `cb-plugin-api`
   - If no: Keep as language-specific helper

2. Is this trait **optional** functionality?
   - If yes: Return `Option<&dyn Trait>` from `LanguagePlugin`
   - If no: Add as required method to `LanguagePlugin`

## Design Checklist

- [ ] Trait defined in `cb-plugin-api/src/`
- [ ] Default implementation returns `None` (for backward compat)
- [ ] All methods use `&self` (trait object safe)
- [ ] Error type uses `PluginResult<T>`
- [ ] Documentation includes:
  - [ ] Purpose of trait
  - [ ] When to implement it
  - [ ] Example implementation
- [ ] At least one reference implementation (e.g., Rust plugin)
- [ ] Tests for trait implementation

## Update Checklist

- [ ] Export trait from `cb-plugin-api/src/lib.rs`
- [ ] Add method to `LanguagePlugin` trait
- [ ] Update existing plugins (or add default impl)
- [ ] Update `crates/languages/README.md` with new trait
- [ ] Add to complexity matrix
```

#### B. Add "Language Plugin Onboarding" Doc

**File**: `docs/development/LANGUAGE_PLUGIN_ONBOARDING.md`

```markdown
# Language Plugin Development Onboarding

Welcome! This guide walks you through adding a new language plugin.

## Step 0: Prerequisites (15 minutes)

1. **Read the architecture docs**:
   - [ ] `crates/languages/README.md`
   - [ ] `docs/development/LANGUAGE_PLUGIN_PREREQUISITES.md`
   - [ ] `API_REFERENCE.md` (language support matrix)

2. **Check external dependencies**:
   - [ ] Do you need a language runtime? (Java, .NET, Swift, etc.)
   - [ ] Install prerequisites (see LANGUAGE_PLUGIN_PREREQUISITES.md)

3. **Review reference implementations**:
   - [ ] Similar language? (Java → C#, TypeScript → JavaScript)
   - [ ] Check 2-3 existing plugins to understand patterns

## Step 1: Scaffold (5 minutes)

```bash
cd crates/languages
./new-lang.sh <language> --manifest "<pattern>" --extensions <ext1,ext2>

# Example for C#:
./new-lang.sh csharp --manifest "*.csproj" --extensions cs,csx
```

**Checklist**:
- [ ] Plugin directory created
- [ ] Boilerplate files generated
- [ ] Language registered in `languages.toml`
- [ ] Builds successfully: `cargo build --features lang-<language>`

## Step 2: Basic Implementation (2-4 hours)

Implement in order:

1. **Manifest parsing** (`src/manifest.rs`)
   - [ ] Parse manifest file format
   - [ ] Extract dependencies
   - [ ] Return `ManifestData`
   - [ ] Test: `cargo test -p cb-lang-<language> manifest`

2. **Source parsing** (`src/parser.rs`)
   - [ ] AST parsing (subprocess or native)
   - [ ] Symbol extraction (functions, classes, etc.)
   - [ ] Return `ParsedSource`
   - [ ] Test: `cargo test -p cb-lang-<language> parser`

3. **Plugin integration** (`src/lib.rs`)
   - [ ] Wire up manifest + parser
   - [ ] Implement `LanguagePlugin` trait
   - [ ] Test: `cargo test -p cb-lang-<language>`

**Checkpoint**: Basic plugin compiles and tests pass ✅

## Step 3: Advanced Features (Optional, 4-16 hours)

Choose based on language capabilities:

### Option A: ImportSupport (2-4 hours)
- [ ] Implement `ImportSupport` trait
- [ ] Parse import statements
- [ ] Rewrite imports for refactoring
- [ ] Test import operations

### Option B: WorkspaceSupport (4-8 hours)
- [ ] Implement `WorkspaceSupport` trait
- [ ] Manifest manipulation (add deps, etc.)
- [ ] Workspace member management
- [ ] Test workspace operations

### Option C: RefactoringSupport (8-16 hours)
⚠️ **Check first**: Does `RefactoringSupport` trait exist?
- If NO: Coordinate with maintainers to design trait
- If YES: Implement existing trait

- [ ] Implement `RefactoringSupport` trait
- [ ] Locate module files
- [ ] Generate manifests
- [ ] Rewrite imports/declarations
- [ ] Test refactoring operations

## Step 4: Integration Testing (1-2 hours)

- [ ] Create integration test in `integration-tests/`
- [ ] Test parsing with real code samples
- [ ] Test manifest operations
- [ ] Test end-to-end workflows (if applicable)

## Step 5: Documentation (30 minutes)

- [ ] Update plugin README.md with examples
- [ ] Add to language support matrix in API_REFERENCE.md
- [ ] Document external dependencies in main README
- [ ] Add to CONTRIBUTING.md language list

## Step 6: Submit

- [ ] All tests pass: `cargo test --workspace`
- [ ] Build succeeds: `cargo build --release`
- [ ] Documentation complete
- [ ] Create PR with comprehensive description

---

## Time Estimates by Scope

| Scope | Time | Features |
|-------|------|----------|
| Minimal | 2-4h | Parsing only |
| Standard | 4-8h | + ImportSupport |
| Complete | 8-16h | + WorkspaceSupport |
| Advanced | 16-24h | + RefactoringSupport |

## Common Issues

See `crates/languages/README.md` → "Common Pitfalls"
```

---

### Priority 4: Architectural Improvements (LOW IMPACT, HIGH EFFORT)

These emerged from the C# work but are long-term improvements:

#### A. Standardize Parser Approach

**Current state**: Mixed approaches
- Java: External subprocess (Maven app)
- C#: External subprocess (.NET app)
- Swift: External subprocess (SourceKitten)
- TypeScript: Embedded Node script
- Rust: Native syn crate
- Go: Subprocess + regex fallback

**Recommendation**: Document "when to use which approach" in README

```markdown
## Parser Implementation Strategies

### Strategy 1: Native Rust Crate
**When**: Mature Rust parser library exists
**Examples**: Rust (syn), Python (ruff-python-parser)
**Pros**: Fast, no external dependencies
**Cons**: Limited to languages with good Rust parsers

### Strategy 2: Language Runtime Subprocess
**When**: Official language parser available
**Examples**: Java (Roslyn), C# (Microsoft.CodeAnalysis), Swift (SourceKitten)
**Pros**: Accurate, feature-complete
**Cons**: Requires external runtime, slower

### Strategy 3: Embedded Script
**When**: Language has good self-parsing (JavaScript/TypeScript)
**Examples**: TypeScript (using TypeScript compiler API)
**Pros**: Accurate, bundled with project
**Cons**: Requires runtime at build time

### Strategy 4: Regex Fallback
**When**: No parser available or as backup
**Examples**: Go (partial fallback)
**Pros**: Always works
**Cons**: Inaccurate, fragile
```

#### B. Create Parser Template Repository

Create `crates/languages/resources/parser-templates/`:
```
parser-templates/
├── java-maven-template/
├── dotnet-console-template/
├── node-script-template/
└── README.md
```

Developers can copy-paste and customize.

---

## Success Metrics

To measure if these improvements work, track:

1. **Time to first successful build** (new plugin)
   - Current: Unknown (C# had multiple build failures)
   - Target: < 30 minutes from scaffolding to first build

2. **Architectural surprises encountered**
   - Current: 1 (RefactoringSupport didn't exist)
   - Target: 0 (all requirements documented upfront)

3. **Documentation lookups needed**
   - Current: Multiple (external deps, trait design, etc.)
   - Target: 1-2 (onboarding doc + reference implementation)

4. **System-level changes required**
   - Current: 1 major (RefactoringSupport trait creation)
   - Target: Documented in advance (decision, not surprise)

---

## Implementation Plan

### Phase 1: Documentation (1-2 hours)
- [ ] Create `LANGUAGE_PLUGIN_PREREQUISITES.md`
- [ ] Update `crates/languages/README.md` with complexity matrix
- [ ] Create `LANGUAGE_PLUGIN_ONBOARDING.md`
- [ ] Add "Common Pitfalls" section

### Phase 2: Tooling (2-3 hours)
- [ ] Enhance `new-lang.sh` with dependency checking
- [ ] Add `Makefile` targets for parser builds
- [ ] Create pre-commit hook for parser artifacts

### Phase 3: Templates (1-2 hours)
- [ ] Create parser template directory
- [ ] Document parser strategy decision tree
- [ ] Add examples for each approach

### Phase 4: Validation (1 hour)
- [ ] Test with hypothetical "Ruby plugin" using new docs
- [ ] Verify onboarding doc completeness
- [ ] Check that all tools work as expected

**Total time**: 5-8 hours

---

## Conclusion

The C# plugin development revealed that our **scaffolding and initial setup are excellent**, but **mid-development architectural guidance is weak**.

**Key improvements needed**:
1. ✅ Feature complexity matrix (so developers know what they're getting into)
2. ✅ External dependency documentation (so setup is smooth)
3. ✅ Enhanced scaffolding script (catches dependency issues early)
4. ✅ Trait design guidance (so system-level work is coordinated)

**Biggest win from this process**:
The RefactoringSupport trait is a **high-quality architectural improvement** that emerged from this work. We should celebrate this as a success story and ensure future developers can build on it.

**Recommendation**: Implement Phase 1 (documentation) immediately. It's high-impact, low-effort, and addresses the root cause of most friction points.
