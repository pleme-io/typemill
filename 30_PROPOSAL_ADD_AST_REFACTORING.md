# Add AST-Based Refactoring to Go, Java, Swift, and C#

**Date**: 2025-10-09
**Status**: Draft

---

## Executive Summary

We successfully wired up Python and Rust AST-based refactoring support that was already implemented but not connected to the routing layer. Now we should extend AST-based refactoring to the remaining 4 languages that only have LSP fallback support: **Go, Java, Swift, and C#**.

**Current State:**
- ✅ Python, Rust, TypeScript/JavaScript: Full AST support
- ❌ Go, Java, Swift, C#: LSP-only (no AST fallback)

**Benefits:**
- Faster refactoring (no LSP roundtrip)
- More reliable (no dependency on external LSP servers)
- Consistent behavior across all languages
- Better test coverage (works in mock environments)

---

## Background

### What We Just Accomplished

We successfully wired up Python and Rust refactoring by:

1. **Adding routing cases** in `crates/cb-ast/src/refactoring/*.rs`:
   ```rust
   match detect_language(file_path) {
       "python" => ast_extract_function_python(...),
       "rust" => ast_extract_function_rust(...),
       // ...
   }
   ```

2. **Creating wrapper functions** that convert between types:
   ```rust
   fn ast_extract_function_python(...) -> AstResult<EditPlan> {
       let python_range = cb_lang_python::refactoring::CodeRange { /* convert */ };
       cb_lang_python::refactoring::plan_extract_function(...)
           .map_err(|e| AstError::analysis(format!("Python error: {}", e)))
   }
   ```

3. **Test Results**: All Python and Rust refactoring tests now pass! 🎉

### Why LSP-Only is Insufficient

**Problems with LSP-only approach:**
1. **Requires external processes** - LSP servers must be installed and running
2. **Slower** - Network/process communication overhead
3. **Unreliable in tests** - Mock tests can't use LSP
4. **Inconsistent** - Different LSP servers behave differently
5. **Limited control** - Can't customize refactoring logic

---

## Proposed Solution

Add AST-based refactoring support to the 4 remaining languages following the proven pattern from Python, Rust, and TypeScript.

### Language Priority Order

1. **Go** - Simplest, proves pattern works for new language
2. **Java** - High demand, moderate complexity
3. **Swift** - Growing demand, unique features
4. **C#** - Most complex, but completes the set

---

## Implementation Details

### Go Implementation

**Rationale**: Go has excellent parsing libraries and simpler syntax than Java/C#/Swift.

**Tasks:**

1. **Create `crates/cb-lang-go/src/refactoring.rs`**
   - Use existing Go parser (likely `tree-sitter-go` or `go-parser` crate)
   - Implement 3 core operations:
     - `plan_extract_function()`
     - `plan_extract_variable()`
     - `plan_inline_variable()`

2. **Add Go-specific types**:
   ```rust
   pub struct CodeRange {
       pub start_line: u32,
       pub start_col: u32,
       pub end_line: u32,
       pub end_col: u32,
   }

   pub fn plan_extract_function(
       source: &str,
       range: &CodeRange,
       function_name: &str,
       file_path: &str,
   ) -> RefactoringResult<EditPlan> {
       // Parse Go AST
       // Analyze selection
       // Generate edit plan
   }
   ```

3. **Wire up routing in `crates/cb-ast/src/refactoring/extract_function.rs`**:
   ```rust
   match detect_language(file_path) {
       // existing cases...
       "go" => ast_extract_function_go(source, range, new_function_name, file_path),
       _ => Err(...)
   }
   ```

4. **Add wrapper functions**:
   ```rust
   fn ast_extract_function_go(...) -> AstResult<EditPlan> {
       let go_range = cb_lang_go::refactoring::CodeRange {
           start_line: range.start_line,
           start_col: range.start_col,
           end_line: range.end_line,
           end_col: range.end_col,
       };
       cb_lang_go::refactoring::plan_extract_function(source, &go_range, ...)
           .map_err(|e| AstError::analysis(format!("Go refactoring error: {}", e)))
   }
   ```

**Estimated Effort**: 3-4 days
- Research Go parsing libraries, design AST approach
- Implement extract_variable (simplest operation)
- Implement extract_function and inline_variable
- Testing, edge cases, documentation

**Parsing Library Options**:
- `tree-sitter-go` - Fast, battle-tested
- `go-parser` (if Rust binding exists)
- Custom parser using `nom` or `pest` (last resort)

---

### Java Implementation

**Rationale**: Java is widely used in enterprise environments, high demand.

**Challenges**:
- Complex grammar (generics, annotations, lambdas)
- Need to handle imports properly
- Package structure considerations

**Implementation Strategy**:
1. Use `tree-sitter-java` for parsing
2. Follow Python refactoring pattern for analysis
3. Generate Java-style code (camelCase, proper indentation)

**Estimated Effort**: 4-5 days
- More complex grammar than Go
- Need to handle Java-specific features (generics, annotations)
- Import management is critical

**Example Operations**:
```java
// Extract variable
int x = 10 + 20;  // Before
final int sum = 10 + 20;  // After (with suggested type)
int x = sum;

// Extract function
private int calculateSum() {
    return 10 + 20;
}
```

---

### Swift Implementation

**Rationale**: Growing iOS/macOS development community, unique syntax features.

**Challenges**:
- Optional chaining and unwrapping
- Protocol-oriented programming patterns
- Complex closure syntax

**Implementation Strategy**:
1. Use `tree-sitter-swift` for parsing
2. Handle Swift-specific features (optionals, protocols)
3. Generate idiomatic Swift code

**Estimated Effort**: 4-5 days
- Modern language with clean syntax
- Need to handle optionals and guards properly

**Example Operations**:
```swift
// Extract variable
let x = userList?.first?.name ?? "Unknown"  // Before
let userName = userList?.first?.name ?? "Unknown"  // After
let x = userName

// Extract function
func extractedFunction() -> String {
    return userList?.first?.name ?? "Unknown"
}
```

---

### C# Implementation

**Rationale**: .NET ecosystem is large, especially in enterprise.

**Challenges**:
- LINQ expressions
- Properties vs fields
- Async/await patterns
- Multiple syntax styles (expression bodied members, etc.)

**Implementation Strategy**:
1. Use `tree-sitter-c-sharp` for parsing
2. Handle C#-specific features (properties, LINQ, async)
3. Generate modern C# code (C# 10+ features)

**Estimated Effort**: 5-6 days
- Most complex language of the four
- Many language features to handle
- Need to respect C# coding conventions

**Example Operations**:
```csharp
// Extract variable
var x = users.Where(u => u.Age > 18).ToList();  // Before
var adults = users.Where(u => u.Age > 18).ToList();  // After
var x = adults;

// Extract method
private List<User> ExtractedMethod()
{
    return users.Where(u => u.Age > 18).ToList();
}
```

---

## Shared Infrastructure

All languages will reuse:
- ✅ `cb-protocol::EditPlan` - Already exists
- ✅ Routing layer in `cb-ast/src/refactoring/*.rs` - Pattern proven
- ✅ Type conversion pattern - Established with Python/Rust
- ✅ Test harness - `RefactoringScenarios` supports all languages

---

## Reference Implementation

### Files to Reference

**Python implementation (simplest, best documented):**
```
crates/cb-lang-python/src/refactoring.rs (633 lines)
  - plan_extract_function() - Line 221
  - plan_extract_variable() - Line 340
  - plan_inline_variable() - Line 270
```

**Rust implementation (native language, good example):**
```
crates/cb-lang-rust/src/refactoring.rs (315 lines)
  - plan_extract_function() - Line 32
  - plan_extract_variable() - Line 118
  - plan_inline_variable() - Line 218
```

**TypeScript implementation (most complex, handles JS/TS):**
```
crates/cb-lang-typescript/src/refactoring.rs (579 lines)
  - Uses SWC parser (very fast)
  - Shows how to handle multiple file extensions (.ts, .tsx, .js, .jsx)
```

### Routing Pattern (Copy this for each language)

**Extract Function** (`cb-ast/src/refactoring/extract_function.rs`):
```rust
match detect_language(file_path) {
    "typescript" | "javascript" => ast_extract_function_ts_js(...),
    "python" => ast_extract_function_python(...),
    "rust" => ast_extract_function_rust(...),
    "go" => ast_extract_function_go(...),        // ADD THIS
    "java" => ast_extract_function_java(...),    // ADD THIS
    "swift" => ast_extract_function_swift(...),  // ADD THIS
    "csharp" => ast_extract_function_csharp(...), // ADD THIS
    _ => Err(AstError::analysis(...))
}
```

**Extract Variable** (`cb-ast/src/refactoring/extract_variable.rs`):
```rust
match detect_language(file_path) {
    "typescript" | "javascript" => ast_extract_variable_ts_js(...),
    "python" => ast_extract_variable_python(...),
    "rust" => ast_extract_variable_rust(...),
    "go" => ast_extract_variable_go(...),        // ADD THIS
    "java" => ast_extract_variable_java(...),    // ADD THIS
    "swift" => ast_extract_variable_swift(...),  // ADD THIS
    "csharp" => ast_extract_variable_csharp(...), // ADD THIS
    _ => Err(AstError::analysis(...))
}
```

**Inline Variable** (`cb-ast/src/refactoring/inline_variable.rs`):
```rust
match detect_language(file_path) {
    "python" => ast_inline_variable_python(...),
    "rust" => ast_inline_variable_rust(...),
    "go" => ast_inline_variable_go(...),        // ADD THIS
    "java" => ast_inline_variable_java(...),    // ADD THIS
    "swift" => ast_inline_variable_swift(...),  // ADD THIS
    "csharp" => ast_inline_variable_csharp(...), // ADD THIS
    _ => Err(AstError::analysis(...))
}
```

---

## Testing Strategy

### Unit Tests (Required for each language)

Each `crates/cb-lang-{language}/src/refactoring.rs` must include:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_variable_simple() {
        let source = "/* language-specific code */";
        let plan = plan_extract_variable(source, 1, 8, 1, 15, Some("var_name"), "test.ext").unwrap();
        assert_eq!(plan.edits.len(), 2);
        assert_eq!(plan.metadata.intent_name, "extract_variable");
    }

    #[test]
    fn test_inline_variable_basic() {
        // Test inline variable
    }

    #[test]
    fn test_extract_function_multiline() {
        // Test extract function
    }
}
```

### Integration Tests (Automatic via RefactoringScenarios)

The test harness in `crates/cb-test-support/src/harness/refactoring_harness.rs` already supports all languages. Once you add AST support, update the expectations:

```rust
pub fn extract_simple_expression() -> RefactoringScenario {
    RefactoringScenario {
        scenario_name: "extract_simple_expression",
        fixtures: vec![
            // ... existing fixtures ...
            LanguageFixture {
                language: Language::Go,
                source_code: GO_SOURCE,  // Define language-specific source
                operation: RefactoringOperation::ExtractVariable { /* ... */ },
            },
        ],
        expected: hashmap! {
            Language::Go => ExpectedBehavior::Success,  // Change from NotSupported
            // ...
        },
    }
}
```

### E2E Tests (Already exist, will auto-pass once wired up)

```bash
# These tests will automatically include new languages:
cargo nextest run --workspace --test e2e_refactoring_cross_language
```

---

## Success Criteria

### Per-Language Checklist

For each language (Go, Java, Swift, C#), the following must work:

- [ ] **Extract Variable**: Single-line expression → variable declaration
- [ ] **Extract Function**: Multi-line code block → function definition
- [ ] **Inline Variable**: Variable references → inline value
- [ ] **Unit tests**: At least 5 tests per operation (15 total)
- [ ] **Integration tests**: Pass in `e2e_refactoring_cross_language.rs`
- [ ] **Documentation**: Add language to `API_REFERENCE.md` Language Support Matrix
- [ ] **Code review**: Follows patterns from Python/Rust/TypeScript

### Overall Project Success

- [ ] All 7 languages support AST-based refactoring
- [ ] 100% test pass rate for refactoring tests
- [ ] Performance benchmarks show <100ms for typical operations
- [ ] Documentation updated with language support matrix
- [ ] No LSP dependency for basic refactoring operations

---

## Risk Assessment

### Low Risk

- **Go**: Simple syntax, good tooling, lots of Rust parser options
- **Pattern proven**: We successfully did this for Python and Rust

### Medium Risk

- **Java generics**: Type erasure and generics can be tricky
- **Swift optionals**: Need careful handling of optional chaining
- **C# LINQ**: Complex expression trees

### High Risk

- **Parsing library availability**: Some languages might lack good Rust parsers
  - **Mitigation**: Use tree-sitter (available for all 4 languages)
- **Edge cases**: Each language has unique corner cases
  - **Mitigation**: Start with simple operations, iterate

### Dependencies

**All languages need tree-sitter bindings:**
```toml
# In respective Cargo.toml files
tree-sitter = "0.20"
tree-sitter-go = "0.19"      # For Go
tree-sitter-java = "0.20"    # For Java
tree-sitter-swift = "0.20"   # For Swift
tree-sitter-c-sharp = "0.20" # For C#
```

---

## Resource Requirements

### Developer Time

- **Go**: 3-4 days (1 developer)
- **Java**: 4-5 days (1 developer)
- **Swift**: 4-5 days (1 developer)
- **C#**: 5-6 days (1 developer)

**Total**: 16-20 developer days (~4 weeks for 1 developer, ~2 weeks for 2 developers)

### Code Size Estimates

Based on existing implementations:
- Python: 633 lines
- Rust: 315 lines
- TypeScript: 579 lines

**Expected per language**: 300-600 lines

**Total new code**: ~1,200-2,400 lines across 4 languages

---

## Alternatives Considered

### Alternative 1: Keep LSP-Only for These Languages

**Pros:**
- No implementation effort
- Leverages existing LSP servers

**Cons:**
- Slower performance
- External dependency (LSP servers must be installed)
- Unreliable in tests
- Inconsistent behavior across languages

**Decision**: Rejected - AST support provides significant benefits

### Alternative 2: Implement Only Most Popular Languages (Go + Java)

**Pros:**
- Less effort (8-9 days vs 16-20 days)
- Covers majority use cases

**Cons:**
- Swift and C# users get inferior experience
- Incomplete language matrix

**Decision**: Rejected - Better to complete all languages for consistency

### Alternative 3: Use Language-Specific External Tools

**Pros:**
- Leverage existing tools (gofmt, JavaParser, etc.)

**Cons:**
- External process overhead (similar to LSP)
- Complex integration
- Platform dependencies

**Decision**: Rejected - In-process AST parsing is faster and more reliable

---

## Appendix A: Code Templates

### Template: refactoring.rs Structure

```rust
//! {Language}-specific refactoring operations
//!
//! Provides AST-based refactoring capabilities including:
//! - Extract function
//! - Extract variable
//! - Inline variable

use cb_protocol::{EditPlan, EditPlanMetadata, EditType, TextEdit, ValidationRule, ValidationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Code range for refactoring operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Error type for refactoring operations
#[derive(Debug, thiserror::Error)]
pub enum RefactoringError {
    #[error("Analysis error: {0}")]
    Analysis(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type RefactoringResult<T> = Result<T, RefactoringError>;

/// Generate edit plan for extract function refactoring
pub fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    function_name: &str,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    // Parse AST
    // Analyze selection
    // Generate edits
    todo!("Implement for {language}")
}

/// Generate edit plan for extract variable refactoring
pub fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    todo!("Implement for {language}")
}

/// Generate edit plan for inline variable refactoring
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> RefactoringResult<EditPlan> {
    todo!("Implement for {language}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_variable_simple() {
        // Add test
    }
}
```

### Template: Wrapper Function

```rust
// In cb-ast/src/refactoring/extract_function.rs
fn ast_extract_function_{language}(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> AstResult<EditPlan> {
    let {language}_range = cb_lang_{language}::refactoring::CodeRange {
        start_line: range.start_line,
        start_col: range.start_col,
        end_line: range.end_line,
        end_col: range.end_col,
    };

    cb_lang_{language}::refactoring::plan_extract_function(
        source,
        &{language}_range,
        new_function_name,
        file_path,
    )
    .map_err(|e| AstError::analysis(format!("{Language} refactoring error: {}", e)))
}
```

---

## Appendix B: Parsing Library Research

### Go

**Recommended**: `tree-sitter-go`
- **Crate**: `tree-sitter-go = "0.19"`
- **Pros**: Fast, battle-tested, official tree-sitter binding
- **Cons**: None
- **Documentation**: https://github.com/tree-sitter/tree-sitter-go

### Java

**Recommended**: `tree-sitter-java`
- **Crate**: `tree-sitter-java = "0.20"`
- **Pros**: Handles Java 17+ features, well-maintained
- **Cons**: Complex grammar
- **Documentation**: https://github.com/tree-sitter/tree-sitter-java

### Swift

**Recommended**: `tree-sitter-swift`
- **Crate**: `tree-sitter-swift = "0.3"`
- **Pros**: Supports Swift 5.x
- **Cons**: Less mature than other tree-sitter parsers
- **Documentation**: https://github.com/alex-pinkus/tree-sitter-swift

### C#

**Recommended**: `tree-sitter-c-sharp`
- **Crate**: `tree-sitter-c-sharp = "0.20"`
- **Pros**: Supports C# 10+
- **Cons**: Complex language features
- **Documentation**: https://github.com/tree-sitter/tree-sitter-c-sharp

---

## Conclusion

Adding AST-based refactoring to Go, Java, Swift, and C# will:
1. **Complete language parity** across all 7 supported languages
2. **Improve performance** by eliminating LSP dependency
3. **Increase reliability** especially for testing
4. **Simplify maintenance** with consistent patterns

The pattern is proven (Python and Rust work perfectly), the effort is manageable (16-20 days), and the benefits are significant.
