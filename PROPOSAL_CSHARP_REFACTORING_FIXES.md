# Proposal: Fix C# Refactoring Implementation Bugs

**Status**: Draft
**Created**: 2025-10-09
**Author**: Development Team
**Related Commit**: `efd9754b` - feat: Add C# AST-based refactoring support (partial)

## Summary

The C# refactoring implementation was cherry-picked from `feat/add-refactoring-for-more-languages` with 2 out of 3 operations working. This proposal outlines the fixes needed for the two failing refactoring operations.

## Current Status

### Working ✅
- **Extract Function** (`test_extract_csharp_method`) - Passes successfully
  - Correctly extracts code into new C# method
  - Properly handles AST traversal for method insertion

### Failing ⚠️
1. **Extract Variable** (`test_extract_csharp_variable`)
   - Error: `"Could not find statement to insert before."`
   - Location: `crates/cb-lang-csharp/src/refactoring.rs:344`

2. **Inline Variable** (`test_inline_csharp_variable`)
   - Error: `"Not a local variable declaration"`
   - Location: `crates/cb-lang-csharp/src/refactoring.rs:382`

## Problem Analysis

### Issue 1: Extract Variable - Statement Insertion Point

**Root Cause:**
The `plan_extract_variable` function cannot find an appropriate AST node to insert the variable declaration before.

**Code Location:**
```rust
// crates/cb-lang-csharp/src/refactoring.rs:130-136
let insertion_node = find_ancestor_of_kind(selected_node, "local_declaration_statement")
    .or_else(|| find_ancestor_of_kind(selected_node, "expression_statement"))
    .ok_or_else(|| RefactoringError::Analysis("Could not find statement to insert before.".to_string()))?;
```

**Likely Causes:**
1. C# tree-sitter node kinds don't match the expected names
2. Missing C#-specific AST node types (e.g., `assignment_expression`, `return_statement`)
3. Selected expression might be in a context without traditional statements (e.g., property initializer, lambda)

**Test Case:**
```csharp
void MyMethod() {
    int x = 10 + 20;  // Extract "10 + 20" into variable
    Console.WriteLine(x);
}
```

### Issue 2: Inline Variable - Declaration Recognition

**Root Cause:**
The `extract_csharp_var_info` helper cannot identify C# variable declarations.

**Code Location:**
```rust
// crates/cb-lang-csharp/src/refactoring.rs:304-306
let declaration_node = find_ancestor_of_kind(node, "local_declaration_statement")
    .ok_or_else(|| RefactoringError::Analysis("Not a local variable declaration".to_string()))?;
```

**Likely Causes:**
1. C# variable declarations have different AST structure than expected
2. Need to handle both `local_declaration_statement` and `variable_declaration`
3. C# grammar might use different node kinds (e.g., `variable_declarator`)

**Test Case:**
```csharp
void MyMethod() {
    string greeting = "Hello";  // Inline this variable
    Console.WriteLine(greeting);
}
```

## Proposed Solution

### Phase 1: AST Investigation (2-4 hours)

**Goal:** Understand C# tree-sitter AST structure

**Tasks:**
1. Write diagnostic tool to print C# AST for test cases:
   ```rust
   // Debug helper to visualize tree-sitter AST
   fn print_ast(source: &str) {
       let mut parser = Parser::new();
       parser.set_language(tree_sitter_c_sharp::language()).unwrap();
       let tree = parser.parse(source, None).unwrap();
       println!("{}", tree.root_node().to_sexp());
   }
   ```

2. Identify actual node kinds for:
   - Variable declarations (`int x = 10;`)
   - Expression statements (`x + y`)
   - Assignment statements
   - Return statements

3. Compare with tree-sitter-c-sharp grammar documentation:
   - https://github.com/tree-sitter/tree-sitter-c-sharp

**Deliverable:** Document mapping of expected node kinds to actual C# AST node kinds

### Phase 2: Fix Extract Variable (2-3 hours)

**Changes to `plan_extract_variable()`:**

1. **Expand insertion point search** to include C#-specific nodes:
   ```rust
   let insertion_node = find_ancestor_of_kind(selected_node, "local_declaration_statement")
       .or_else(|| find_ancestor_of_kind(selected_node, "variable_declaration"))
       .or_else(|| find_ancestor_of_kind(selected_node, "expression_statement"))
       .or_else(|| find_ancestor_of_kind(selected_node, "return_statement"))
       .or_else(|| find_ancestor_of_kind(selected_node, "assignment_expression"))
       .or_else(|| find_ancestor_of_kind(selected_node, "argument"))
       .ok_or_else(|| RefactoringError::Analysis(
           "Could not find statement to insert before.".to_string()
       ))?;
   ```

2. **Handle edge cases:**
   - Property initializers: `public int Value { get; set; } = 10;`
   - Lambda expressions: `var func = () => x + y;`
   - LINQ expressions: `var result = items.Where(x => x > 0);`

3. **Add fallback strategy:**
   ```rust
   // If no statement found, try to find the containing block
   let block_node = find_ancestor_of_kind(selected_node, "block")
       .ok_or_else(|| RefactoringError::Analysis(
           "Selection is not inside a code block".to_string()
       ))?;

   // Insert at beginning of block
   let insertion_point = block_node.start_position();
   ```

### Phase 3: Fix Inline Variable (2-3 hours)

**Changes to `extract_csharp_var_info()`:**

1. **Expand declaration recognition:**
   ```rust
   fn extract_csharp_var_info<'a>(
       node: Node<'a>,
       source: &str
   ) -> RefactoringResult<(String, String, Node<'a>)> {
       // Try multiple patterns for C# variable declarations
       let declaration_node = find_ancestor_of_kind(node, "local_declaration_statement")
           .or_else(|| find_ancestor_of_kind(node, "variable_declaration"))
           .or_else(|| find_ancestor_of_kind(node, "variable_declarator"))
           .ok_or_else(|| RefactoringError::Analysis(
               "Not a local variable declaration".to_string()
           ))?;

       // Extract variable name and value based on C# AST structure
       // ...
   }
   ```

2. **Handle C# variable declaration patterns:**
   - Explicit type: `int x = 5;`
   - Implicit type: `var x = 5;`
   - Multiple declarators: `int x = 5, y = 10;`
   - Nullable types: `int? x = null;`

3. **Improve error messages:**
   ```rust
   .ok_or_else(|| RefactoringError::Analysis(format!(
       "Not a local variable declaration. Found node kind: '{}'. Expected one of: \
        local_declaration_statement, variable_declaration, variable_declarator",
       node.kind()
   )))?;
   ```

### Phase 4: Testing & Validation (1-2 hours)

**Test Coverage:**

1. **Unit Tests** - Verify existing tests pass:
   ```bash
   cargo nextest run --package cb-lang-csharp refactoring
   ```
   - Target: 3/3 tests passing

2. **Additional Test Cases:**
   ```rust
   #[test]
   fn test_extract_variable_from_property_initializer() {
       let source = r#"
   class Foo {
       public int Value { get; set; } = 10 + 20;
   }
   "#;
       // Should extract "10 + 20" into variable
   }

   #[test]
   fn test_inline_variable_with_var_keyword() {
       let source = r#"
   void MyMethod() {
       var greeting = "Hello";
       Console.WriteLine(greeting);
   }
   "#;
       // Should inline greeting with "Hello"
   }

   #[test]
   fn test_extract_variable_in_linq() {
       let source = r#"
   var filtered = items.Where(x => x.Value > 10);
   "#;
       // Should extract "10" into variable
   }
   ```

3. **Integration Tests:**
   - Run full cross-language refactoring test suite
   - Verify C# doesn't break other languages

## Success Criteria

- [ ] `test_extract_csharp_variable` passes
- [ ] `test_inline_csharp_variable` passes
- [ ] `test_extract_csharp_method` continues to pass
- [ ] All cb-lang-csharp tests pass (14/14)
- [ ] Cross-language refactoring tests pass (21/21 with C# included)
- [ ] No regressions in Java, Swift, Python, TypeScript, Rust, Go

## Timeline Estimate

| Phase | Duration | Description |
|-------|----------|-------------|
| Phase 1 | 2-4 hours | AST investigation and documentation |
| Phase 2 | 2-3 hours | Fix extract variable |
| Phase 3 | 2-3 hours | Fix inline variable |
| Phase 4 | 1-2 hours | Testing and validation |
| **Total** | **7-12 hours** | End-to-end implementation |

## Risks & Mitigations

### Risk 1: C# AST Complexity
**Description:** C# has complex syntax (generics, LINQ, async/await) that may not map cleanly to refactoring operations.

**Mitigation:**
- Start with simple cases (local variables in methods)
- Add complexity incrementally
- Document unsupported patterns clearly

### Risk 2: tree-sitter-c-sharp Grammar Gaps
**Description:** The tree-sitter grammar might not expose all needed AST details.

**Mitigation:**
- Contribute fixes upstream if grammar issues found
- Use LSP fallback for complex cases
- Document known limitations

### Risk 3: Build Dependency on .NET SDK
**Description:** .NET SDK requirement adds complexity to CI/CD.

**Mitigation:**
- Already addressed: .NET SDK 8.0 installed successfully
- tree-sitter-c-sharp builds correctly
- Document .NET requirement in setup instructions

## Implementation Notes

### Reference Implementations

**Working Examples:**
- Java refactoring: `crates/cb-lang-java/src/refactoring.rs`
  - Good patterns for handling typed languages
- Swift refactoring: `crates/cb-lang-swift/src/refactoring.rs`
  - Similar AST traversal patterns
- Python refactoring: `crates/cb-lang-python/src/refactoring.rs`
  - Simpler but demonstrates core concepts

**Key Lessons from Java/Swift:**
1. Always check multiple AST node patterns
2. Provide detailed error messages with node kinds
3. Handle edge cases progressively
4. Use helper functions for AST traversal

### Debugging Tools

**Recommended approach:**
```rust
// Add temporary debug helper in refactoring.rs
#[cfg(test)]
fn debug_print_ast(source: &str, highlight_line: u32) {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_c_sharp::language()).unwrap();
    let tree = parser.parse(source, None).unwrap();

    println!("=== Full AST ===");
    println!("{}", tree.root_node().to_sexp());

    println!("\n=== Nodes at line {} ===", highlight_line);
    let point = Point::new(highlight_line as usize - 1, 0);
    let mut cursor = tree.walk();
    // ... print relevant nodes
}
```

## Related Work

- **Original Feature Branch**: `feat/add-refactoring-for-more-languages` (commit `ca44780d`)
- **Java Implementation**: Fully working (commit `d65d39d3`)
- **Swift Implementation**: Fully working (commit `b6276909`)
- **C# Partial Implementation**: This proposal addresses remaining issues (commit `efd9754b`)

## Future Enhancements

After fixing the core issues, consider:

1. **Advanced C# Features:**
   - Extract method for async/await patterns
   - Handle LINQ query expressions
   - Support for properties and indexers

2. **IDE Integration:**
   - Quick fixes for variable extraction
   - Inline variable with live preview
   - Extract to property vs. field

3. **Performance:**
   - Cache parsed AST for repeated operations
   - Optimize tree traversal for large files

## Questions & Discussion

### Q1: Should we support extracting from property initializers?
**A:** Start with method bodies only. Add property support in follow-up PR.

### Q2: How to handle `var` keyword vs explicit types?
**A:** Preserve user's original choice. Don't change `var` to explicit type or vice versa.

### Q3: What about extracting across multiple statements?
**A:** Out of scope for this fix. Focus on single expression extraction first.

## Approval & Next Steps

**Proposed by:** Development Team
**Requires review from:** Maintainers

**After approval:**
1. Create GitHub issue: "Fix C# extract_variable and inline_variable refactoring"
2. Assign to developer
3. Implement in feature branch: `fix/csharp-refactoring-bugs`
4. Submit PR with fixes and additional tests
5. Merge after CI passes and code review

---

## Appendix A: Test Failure Details

### Test 1: `test_extract_csharp_variable`

**Test Code:**
```rust
#[test]
fn test_extract_csharp_variable() {
    let source = r#"void MyMethod() {
    int x = 10 + 20;
    Console.WriteLine(x);
}"#;
    let plan = plan_extract_variable(
        source,
        2, 12, 2, 19,  // Select "10 + 20"
        Some("sum".to_string()),
        "test.cs"
    ).unwrap();
    // ... assertions
}
```

**Error:**
```
thread 'refactoring::tests::test_extract_csharp_variable' panicked at
crates/cb-lang-csharp/src/refactoring.rs:344:100:
called `Result::unwrap()` on an `Err` value:
Analysis("Could not find statement to insert before.")
```

**Expected Behavior:**
```csharp
void MyMethod() {
    int sum = 10 + 20;  // Inserted declaration
    int x = sum;         // Replaced expression
    Console.WriteLine(x);
}
```

### Test 2: `test_inline_csharp_variable`

**Test Code:**
```rust
#[test]
fn test_inline_csharp_variable() {
    let source = r#"void MyMethod() {
    string greeting = "Hello";
    Console.WriteLine(greeting);
}"#;
    let plan = plan_inline_variable(
        source,
        2, 11,  // Position on "greeting"
        "test.cs"
    ).unwrap();
    // ... assertions
}
```

**Error:**
```
thread 'refactoring::tests::test_inline_csharp_variable' panicked at
crates/cb-lang-csharp/src/refactoring.rs:382:67:
called `Result::unwrap()` on an `Err` value:
Analysis("Not a local variable declaration")
```

**Expected Behavior:**
```csharp
void MyMethod() {
    // Declaration removed
    Console.WriteLine("Hello");  // Variable inlined
}
```

## Appendix B: tree-sitter-c-sharp Node Reference

**Common C# AST Node Kinds** (to be filled during Phase 1):

```
TBD: Actual node kinds from tree-sitter-c-sharp grammar:
- local_declaration_statement: ?
- variable_declaration: ?
- variable_declarator: ?
- expression_statement: ?
- binary_expression: ?
- assignment_expression: ?
- identifier: ?
```

**Investigation Script:**
```rust
// Run this to discover actual node kinds
fn investigate_csharp_ast() {
    let test_cases = vec![
        ("int x = 10;", "Simple variable declaration"),
        ("var x = 10;", "Implicit type"),
        ("int x = 10 + 20;", "Expression initializer"),
        ("string s = \"hello\";", "String literal"),
    ];

    for (code, description) in test_cases {
        println!("\n=== {} ===", description);
        print_ast(code);
    }
}
```
