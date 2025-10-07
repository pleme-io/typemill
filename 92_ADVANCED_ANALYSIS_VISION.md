# Proposal: Advanced Code Analysis Capabilities

**Status**: Concept - Open for Discussion
**Date**: 2025-10-02

## 1. Vision

Extend CodeBuddy beyond real-time code intelligence (LSP) into **batch whole-program analysis** - the kind of deep insights that require cross-file understanding and are too expensive for interactive tools.

## 2. Why This Matters

**Current limitations of LSP/linters:**
- Single-file context only
- No call graph analysis
- Can't trace data flow across modules
- Miss architectural issues

**What we can add:**
- Whole-program reasoning
- Cross-file dependency tracking
- AST + semantic analysis combined
- Architecture-level insights

## 3. Proposed Analysis Capabilities

### High-Value, Proven Demand

**1. Circular Dependency Detection**
- Map import cycles across codebase
- Suggest refactoring to break cycles
- Critical for large codebases, prevents build issues

**2. Dead Code Analysis (Beyond Functions)**
- Unused types, interfaces, constants
- Trace through generics/templates
- Production codebases have 20-40% dead code

**3. API Breaking Change Impact**
- "What breaks if I change this function signature?"
- Full call graph analysis
- Essential for library maintainers, microservices

**4. Security Vulnerability Patterns**
- SQL injection (string concat in queries)
- XSS (unescaped user input)
- Path traversal (user input in file operations)
- Context-aware, not just dependency scanning

### Medium-Value, Nice-to-Have

**5. Code Duplication/Clone Detection**
- Semantic similarity (not text matching)
- Suggest extract-common-function

**6. Architectural Boundary Violations**
- Enforce layered architecture rules
- "Frontend importing from database layer" ❌

**7. Error Handling Coverage**
- Unhandled exceptions/promise rejections
- Crash risk analysis

### Exploratory

**8. Performance Hotspot Prediction**
- O(n²) loops, excessive allocations
- Static analysis before profiling

**9. Data Flow Tracing**
- "Where does this user input end up?" (security)
- "What touches this database field?" (migrations)

**10. Code Complexity Heatmap**
- McCabe complexity, nesting depth
- Guide refactoring priorities

## 4. Implementation Checklist

### Phase 1: Foundation (Existing)
- [x] AST parsing (cb-ast)
- [x] Multi-file traversal
- [x] LSP integration for type info

### Phase 2: Graph Construction
- [ ] Build dependency graph (imports, calls, data flow)
- [ ] Persist graph for incremental updates
- [ ] Cache expensive computations

### Phase 3: Analysis Algorithms - High Value
- [ ] Implement circular dependency detection
- [ ] Implement dead code analysis (beyond functions)
- [ ] Implement API breaking change impact analysis
- [ ] Implement security vulnerability pattern detection

### Phase 3b: Analysis Algorithms - Medium Value
- [ ] Implement code duplication/clone detection
- [ ] Implement architectural boundary violations
- [ ] Implement error handling coverage

### Phase 3c: Analysis Algorithms - Exploratory
- [ ] Implement performance hotspot prediction
- [ ] Implement data flow tracing
- [ ] Implement code complexity heatmap

### Phase 4: Delivery
- [ ] Create MCP tools (run from AI assistants)
- [ ] Create CLI commands (`codebuddy analyze --dead-code`)
- [ ] Add CI/CD integration (GitHub Actions, pre-commit hooks)

### Phase 5: Business Model
- [ ] Validate demand via user interviews
- [ ] Prototype circular deps + dead code analysis
- [ ] Alpha test with early adopters
- [ ] Finalize pricing model

## 5. Business Model Fit

**Why pay-per-request API works here:**
- Analyses are **expensive** (whole-program, cross-file)
- Run **infrequently** (CI, releases, not every keystroke)
- High **value-per-run** (prevent bugs, guide architecture)

**Pricing sketch:**
```
- Dead code scan (1000 files): $0.10
- Security scan: $0.50
- Breaking change impact: $0.25
- Full suite: $2.00/scan

OR: $99/month unlimited for <100k LOC
```

**Freemium tier:**
- 10 scans/month free
- Single-file analysis free (via MCP)
- Upsell to unlimited for teams

## 6. Differentiation

**vs. SonarQube:** Deeper semantic analysis, AI-assisted fixes
**vs. GitHub CodeQL:** Lighter weight, faster, no custom query language
**vs. Semgrep:** AST-aware (not pattern matching), language-agnostic via LSP

**Unique advantage:** Already integrated with AI assistants via MCP - analyses can suggest fixes directly in conversation.

## 7. Open Questions

- Which analyses have most demand? (Start with user interviews)
- Incremental analysis strategy? (Graph invalidation on file changes)
- Language coverage priority? (TypeScript/Python first, or multi-language from start?)
- Self-hosted vs. API-only? (Both, like current architecture)

## 8. Next Steps

1. **Validate demand** - Survey potential users (library maintainers, platform teams)
2. **Prototype #1 & #2** - Circular deps + dead code (proven value, achievable)
3. **Build graph infrastructure** - Foundation for all other analyses
4. **Alpha test with early adopters** - Gather feedback before expanding

---

**Core Philosophy:** Build capabilities that are **impossible in single-file tools**, focus on **whole-program reasoning**, deliver via **AI-native interfaces** (MCP).
