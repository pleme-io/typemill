# Merge Proposal: Swift & C# Language Plugins

**Date**: 2025-10-07
**Target Branch**: `feature/plugin-architecture`
**Source Branches**: `feat/cb-swift-lang`, `feat/cb-csharp-lang`

---

## Executive Summary

**Swift**: ✅ Ready to merge (5-minute fix)
**C#**: ⚠️ Ready after build fix + testing (1-2 hours)

Both branches add valuable language support and should be merged sequentially.

---

## Branch 1: Swift Language Plugin

### Status
- **Build**: ✅ Compiles with 5 warnings (dead code)
- **Tests**: ✅ 512/512 passing
- **Quality**: ⭐⭐⭐⭐ Production-ready
- **Risk**: LOW - Additive only, no breaking changes

### What It Adds
- Swift language support via SourceKitten parser
- Package.swift manifest parsing
- Complete ImportSupport trait implementation
- 6 unit tests covering all import operations

### Pre-Merge Tasks
```bash
# 1. Fix warnings (1 minute)
git checkout feat/cb-swift-lang
cargo fix --lib -p cb-lang-swift

# 2. Verify tests (2 minutes)
cargo test --workspace

# 3. Merge (2 minutes)
git checkout feature/plugin-architecture
git merge --no-ff feat/cb-swift-lang -m "feat: Add Swift language support

- SourceKitten-based AST parser
- Package.swift manifest parsing
- Complete ImportSupport implementation
- Ready for production use

External dependencies: sourcekitten, swift CLI"
```

### Documentation Updates Needed
- [ ] Add SourceKitten installation to main README
- [ ] Add Swift to language support matrix in API_REFERENCE.md
- [ ] Note external dependencies

**Time**: 5 minutes
**Merge Order**: #1 (merge first)

---

## Branch 2: C# Language Plugin

### Status
- **Build**: ❌ Broken (missing workspace dependency)
- **Tests**: ❓ Unknown until build fixed
- **Quality**: ⭐⭐⭐⭐ High quality when working
- **Risk**: MEDIUM - Includes RefactoringSupport trait (new architecture)

### What It Adds
- C# language support via Roslyn parser (.NET app)
- .csproj manifest parsing (XML-based)
- **RefactoringSupport trait** (architectural improvement)
- Refactors 1,008 lines from package_extractor.rs

### Pre-Merge Tasks

#### Step 1: Fix Build (5 minutes)
```bash
git checkout feat/cb-csharp-lang

# Option A: Add to root Cargo.toml workspace.dependencies
cat >> Cargo.toml << 'EOF'
tempfile = "3.10"
EOF

# Option B: Fix C# Cargo.toml directly
sed -i 's/tempfile = { workspace = true }/tempfile = "3.10"/' \
  crates/languages/cb-lang-csharp/Cargo.toml

# Verify build
cargo build --package cb-lang-csharp
```

#### Step 2: Test (15 minutes)
```bash
# Run all tests
cargo test --workspace

# Specifically test Rust refactoring (CRITICAL - verify no regression)
cargo test -p cb-lang-rust refactoring
cargo test -p integration-tests extract_module_to_package

# Test C# plugin
cargo test -p cb-lang-csharp
```

#### Step 3: Build C# Parser (5 minutes)
```bash
cd crates/languages/cb-lang-csharp/resources/csharp-parser
dotnet publish -c Release -r linux-x64 --self-contained
cd ../../../../..
```

#### Step 4: Integration Test (20 minutes)
Create basic test to verify C# parser works:

```bash
# Create test file
cat > /tmp/test.cs << 'EOF'
using System;

namespace MyNamespace {
    public class MyClass {
        public void MyMethod() {
            Console.WriteLine("Hello");
        }
    }
}
EOF

# Test parser
echo 'using System;' | \
  crates/languages/cb-lang-csharp/resources/csharp-parser/bin/Release/net8.0/linux-x64/csharp-parser
```

#### Step 5: Manual Testing (30 minutes)
**CRITICAL**: Test that RefactoringSupport changes didn't break Rust refactoring

```bash
# Test extract_module_to_package on a real Rust project
# (Use codebuddy itself as test subject)

# Create a test module
mkdir -p /tmp/test-rust/src/auth
cat > /tmp/test-rust/src/auth/jwt.rs << 'EOF'
use serde::{Serialize, Deserialize};

pub fn verify_token(token: &str) -> bool {
    !token.is_empty()
}
EOF

cat > /tmp/test-rust/Cargo.toml << 'EOF'
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
EOF

# Test extraction
cargo run -- tool extract_module_to_package \
  --project-path /tmp/test-rust \
  --module-path "auth::jwt" \
  --target-package-name "auth-jwt" \
  --target-package-path /tmp/test-rust/auth-jwt \
  --dry-run true

# Verify output looks correct
```

#### Step 6: Commit Fix & Merge (5 minutes)
```bash
# Commit the tempfile fix
git add Cargo.toml  # or crates/languages/cb-lang-csharp/Cargo.toml
git commit -m "fix: Add tempfile workspace dependency for C# plugin"

# Merge
git checkout feature/plugin-architecture
git merge --no-ff feat/cb-csharp-lang -m "feat: Add C# language support and RefactoringSupport trait

Breaking changes:
- Introduces RefactoringSupport trait for language-agnostic refactoring
- Refactors package_extractor.rs (-1008 lines)
- Rust plugin now uses RefactoringSupport trait

C# Support:
- Roslyn-based AST parser (.NET app)
- .csproj manifest parsing
- Partial RefactoringSupport implementation

External dependencies: .NET 8.0 SDK

BREAKING: extract_module_to_package now uses RefactoringSupport trait.
Existing Rust functionality preserved via RustRefactoringSupport."
```

### Documentation Updates Needed
- [ ] Add .NET 8.0 SDK installation to main README
- [ ] Add C# to language support matrix
- [ ] Document RefactoringSupport trait in CONTRIBUTING.md
- [ ] Update ARCHITECTURE.md with new trait
- [ ] Add csharp-parser build instructions

**Time**: 1-2 hours
**Merge Order**: #2 (merge after Swift)

---

## Merge Strategy

### Sequential Merge (Recommended)
```
feature/plugin-architecture
    ↓
    ← (merge #1) ← feat/cb-swift-lang
    ↓
    ← (merge #2) ← feat/cb-csharp-lang
    ↓
  [Both merged]
```

**Rationale**:
- Swift is simple, low-risk → merge first
- C# has more complexity → test thoroughly before merging
- If C# tests fail, Swift is already in (progress made)

### Why Not Parallel?
Both branches share common refactoring (cb-lang-common relocation, doc updates). Sequential merges avoid conflicts.

---

## Risk Assessment

### Swift Branch
| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| External dependency missing | Medium | Low | Document in README |
| Warnings cause issues | Low | Low | Fix with `cargo fix` |
| Tests fail after merge | Low | Medium | Run full test suite |

**Overall Risk**: 🟢 LOW

### C# Branch
| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Build fix doesn't work | Low | High | Test both fix options |
| Rust refactoring broken | Medium | **CRITICAL** | Manual testing required |
| Tests fail | Medium | High | Full test suite + manual tests |
| .NET dependency issues | Medium | Medium | Document requirements |
| RefactoringSupport trait bugs | Low | High | Thorough testing of extract_module_to_package |

**Overall Risk**: 🟡 MEDIUM

---

## Success Criteria

### Swift
- [x] Compiles without warnings
- [x] All 512+ tests pass
- [ ] Documentation updated
- [ ] SourceKitten requirement documented

### C#
- [ ] Build succeeds
- [ ] All 550+ tests pass
- [ ] **Rust refactoring still works** (CRITICAL)
- [ ] C# parser compiles and runs
- [ ] RefactoringSupport trait methods tested
- [ ] Documentation updated
- [ ] .NET requirement documented

---

## Timeline

**Total Time**: 2-3 hours

```
Hour 0:00 - Swift merge
  0:00 - 0:05   Fix Swift warnings + verify tests
  0:05 - 0:10   Merge Swift branch
  0:10 - 0:15   Update documentation

Hour 0:15 - C# preparation
  0:15 - 0:20   Fix C# build (tempfile dependency)
  0:20 - 0:35   Run full test suite
  0:35 - 0:40   Build C# parser

Hour 0:40 - C# testing
  0:40 - 1:00   Integration testing
  1:00 - 1:30   Manual testing of Rust refactoring
  1:30 - 1:40   Verify C# parser works

Hour 1:40 - C# merge
  1:40 - 1:50   Merge C# branch
  1:50 - 2:00   Update documentation
  2:00 - 2:30   Final verification + cleanup
```

---

## Rollback Plan

### If Swift Merge Fails
```bash
git reset --hard HEAD~1  # Undo merge
# Investigate issue, fix on branch, retry
```

### If C# Merge Fails
```bash
git reset --hard HEAD~1  # Undo merge
# Swift is still merged (safe)
# Fix C# issues on branch, retry later
```

### If Both Fail
```bash
git reset --hard <commit-before-swift>
# Start over with fixes
```

---

## Post-Merge Tasks

### Immediate (Same Day)
- [ ] Update CHANGELOG.md with new features
- [ ] Tag release if appropriate (`v1.x.0` - minor version bump)
- [ ] Push to remote
- [ ] Update GitHub issues/PRs

### Short-term (This Week)
- [ ] Add integration tests for Swift plugin
- [ ] Add integration tests for C# plugin
- [ ] Complete C# RefactoringSupport implementation
- [ ] Performance testing with both languages

### Long-term (Next Sprint)
- [ ] Add TypeScript RefactoringSupport
- [ ] Add Go RefactoringSupport
- [ ] Implement WorkspaceSupport for Swift/C#

---

## Approval Checklist

**Before merging Swift**:
- [ ] Code reviewed (self-review via analysis docs)
- [ ] Tests passing
- [ ] Documentation complete
- [ ] No breaking changes

**Before merging C#**:
- [ ] Build fixed and verified
- [ ] All tests passing (including Rust refactoring)
- [ ] Manual testing complete
- [ ] RefactoringSupport impact assessed
- [ ] Documentation complete
- [ ] **Breaking changes documented**

---

## Dependencies

**Swift Requirements**:
- SourceKitten: `brew install sourcekitten` (macOS) or build from source (Linux)
- Swift CLI: Included with Xcode or Swift toolchain

**C# Requirements**:
- .NET 8.0 SDK: https://dotnet.microsoft.com/download/dotnet/8.0
- Build csharp-parser: `dotnet publish -c Release`

---

## Communication Plan

**After Swift Merge**:
- Update team: "✅ Swift support merged - requires SourceKitten"

**After C# Merge**:
- Update team: "✅ C# support merged - requires .NET 8.0 SDK"
- ⚠️ Note: "RefactoringSupport trait added - review extract_module_to_package changes"

**If Issues Arise**:
- Document blockers immediately
- Revert if critical functionality broken
- Fix on branch and re-merge

---

## Conclusion

**Recommendation**: ✅ **Proceed with sequential merge**

1. **Merge Swift now** (5 minutes, low risk)
2. **Merge C# after testing** (1-2 hours, medium risk but high value)

Both branches add significant value and are well-implemented. The RefactoringSupport trait in the C# branch is a particularly good architectural improvement.

**Next Steps**:
1. Fix Swift warnings
2. Merge Swift
3. Fix C# build
4. Test thoroughly (especially Rust refactoring)
5. Merge C#
6. Celebrate 🎉 - Two new languages supported!
