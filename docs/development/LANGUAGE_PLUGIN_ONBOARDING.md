# Language Plugin Development - Onboarding Guide

Welcome to language plugin development! This guide will walk you through adding a new language to Codebuddy, step by step.

**Estimated time**: 2-24 hours depending on scope (see the [Feature Complexity Matrix](../crates/languages/README.md#feature-implementation-complexity)).

---

## Table of Contents

1. [Before You Start](#before-you-start)
2. [Step-by-Step Guide](#step-by-step-guide)
3. [Common Patterns](#common-patterns)
4. [Troubleshooting](#troubleshooting)
5. [Success Checklist](#success-checklist)

---

## Before You Start

### Prerequisites (15-30 minutes)

**1. Read the architecture documentation**:
- [ ] [`crates/languages/README.md`](../crates/languages/README.md) - Plugin system overview and complexity matrix.
- [ ] [`docs/development/LANGUAGE_PLUGIN_PREREQUISITES.md`](./LANGUAGE_PLUGIN_PREREQUISITES.md) - External dependencies for parsers.

**2. Decide on your scope**:
Review the [Feature Implementation Complexity](../crates/languages/README.md#feature-implementation-complexity) matrix to choose your target:
- **Minimal** (2-4h): Just parsing
- **Standard** (4-8h): + ImportSupport
- **Complete** (8-12h): + WorkspaceSupport
- **Advanced** (12-24h): + RefactoringSupport

**3. Check external dependencies**:
- [ ] Does your language need an external parser runtime (e.g., Java, .NET, Swift)?
- [ ] If yes, install all prerequisites by following [`LANGUAGE_PLUGIN_PREREQUISITES.md`](./LANGUAGE_PLUGIN_PREREQUISITES.md).
- [ ] Run `make check-parser-deps` to verify your setup.

**4. Review reference implementations**:
Find a similar language to use as a template:
- **Java → C#**: Both use external parsers and XML manifests.
- **TypeScript → JavaScript**: Similar syntax and module systems.
- **Go → Rust**: Both are compiled languages with distinct module systems.

---

## Step-by-Step Guide

### Step 0: Environment Setup (5 minutes)

```bash
# Ensure you're on a clean branch
git checkout -b feat/add-<language>-support

# Verify the project builds
cargo build

# Build all existing external parsers to avoid unrelated errors
make build-parsers
```

---

### Step 1: Scaffold Plugin (5 minutes)

```bash
cd crates/languages

# Run the scaffolding script
./new-lang.sh <language> \
  --manifest "<manifest_pattern>" \
  --extensions <ext1,ext2>

# Examples:
# ./new-lang.sh csharp --manifest "*.csproj" --extensions cs,csx
# ./new-lang.sh ruby --manifest "Gemfile" --extensions rb
```

The script will create the plugin directory, boilerplate files, and register the language. Pay attention to its output, especially the workspace dependency check.

**Verify**:
```bash
# Should compile with warnings
cargo build --features lang-<language>
```
**Checkpoint ✅**: Plugin scaffolded and builds successfully.

---

### Step 2: Implement Manifest Parsing (1-2 hours)

**File**: `crates/languages/cb-lang-<language>/src/manifest.rs`

**Goal**: Parse your language's manifest file (e.g., `package.json`, `pom.xml`).

**Common patterns**:
- **TOML** (Rust): Use the `toml` crate.
- **JSON** (TypeScript): Use `serde_json`.
- **XML** (Java, C#): Use the `quick-xml` crate with serde.
- **Custom** (Go): Use regex or a custom parser.

**Test**:
```bash
cargo test -p cb-lang-<language>
```
**Checkpoint ✅**: Manifest parsing works.

---

### Step 3: Implement Source Parsing (2-4 hours)

**File**: `crates/languages/cb-lang-<language>/src/parser.rs`

**Goal**: Parse source code and extract symbols (functions, classes, etc.).

**Choose your approach**:

- **Option A: Native Rust Parser**: Best when a good Rust parsing library (e.g., `syn`, `tree-sitter`) exists. It's fast and has no external dependencies.
- **Option B: Subprocess Parser**: Best when an official, external parser exists (e.g., Roslyn for C#, SourceKitten for Swift). It's accurate but requires an external runtime. See `cb-lang-java` or `cb-lang-csharp` for examples.
- **Option C: Regex Fallback**: Use as a last resort or for simple languages. It's easy to implement but can be fragile and inaccurate.

**Test**:
```bash
cargo test -p cb-lang-<language>
```
**Checkpoint ✅**: Source parsing works.

---

### Step 4: Wire Up Plugin (30 minutes)

**File**: `crates/languages/cb-lang-<language>/src/lib.rs`

**Goal**: Implement the `LanguagePlugin` trait and connect your parsing logic. Update the `capabilities()` method to reflect the features you've implemented.

**Test**:
```bash
cargo test -p cb-lang-<language>
cargo build --features lang-<language>
```
**Checkpoint ✅**: Basic plugin works end-to-end.

---

### Step 5: Implement Optional Traits (Varies)

Based on your chosen scope, implement the optional traits (`ImportSupport`, `WorkspaceSupport`, `RefactoringSupport`).
- Create new modules for the implementation (e.g., `src/import_support.rs`).
- Update `lib.rs` to return your implementation from the trait method (e.g., `fn import_support(&self) -> Option<&dyn ImportSupport>`).
- Remember to consult the **Feature Complexity Matrix** before starting `RefactoringSupport`.

---

### Step 6: Integration Testing (1-2 hours)

**File**: `apps/codebuddy/tests/e2e_<language>_features.rs` (new file)

Add end-to-end tests that use the `TestClient` to call your plugin's features on real code samples. This is crucial for ensuring your plugin works correctly within the larger system.

---

### Step 7: Documentation (30 minutes)

Update these files:
- [ ] `crates/languages/cb-lang-<language>/README.md`: Add usage examples, build instructions, and any specific details about your implementation.
- [ ] `API_REFERENCE.md`: Add your language to the support matrix.
- [ ] `README.md` (root): Add your language to the list of supported languages.

---

### Step 8: Pre-Commit Checks & Submission

1. **Format and Lint**:
   ```bash
   cargo fmt
   cargo clippy --features lang-<language> -- -D warnings
   ```
2. **Run All Tests**:
   ```bash
   cargo test --workspace
   ```
3. **Create Pull Request**:
   Use a descriptive title and fill out the PR template, detailing the features you've implemented, any external dependencies, and how you tested your work.

---

## Troubleshooting

See the [Common Pitfalls](../crates/languages/README.md#common-pitfalls) section in the language plugins README for solutions to common build and dependency issues.

## Success Checklist

Before submitting your PR, ensure you've checked all the boxes for code quality, documentation, integration, and testing.

- [ ] All code compiles without warnings.
- [ ] All tests pass.
- [ ] All relevant documentation has been updated.
- [ ] The plugin is correctly registered and enabled via its feature flag.
- [ ] You have manually tested the plugin's functionality with real-world code.

---

## Questions?

- **Stuck?** Check similar implementations in `crates/languages/`.
- **Trait design?** Create a GitHub issue for discussion.
- **External parser issues?** See [`LANGUAGE_PLUGIN_PREREQUISITES.md`](./LANGUAGE_PLUGIN_PREREQUISITES.md).
- **Still stuck?** Ask in the project's communication channels.

**Good luck!** 🚀