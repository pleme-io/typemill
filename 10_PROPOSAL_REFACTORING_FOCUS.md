# Proposal: Refactoring-First Vision for TypeMill

**Status**: Draft
**Author**: Project Team
**Date**: 2025-10-10
**Current Positioning**: General-purpose AI code assistance tool
**Proposed Positioning**: Precision refactoring tool powered by LSP and AST intelligence

---

## Executive Summary

This proposal redefines TypeMill's vision from a general-purpose "AI buddy" to a **refactoring-first tool** with two core pillars:

1. **Refactoring Primitives** - Atomic code transformation operations
2. **Analysis Primitives** - Code understanding and insight operations

By focusing on these primitives and their intelligent composition, TypeMill becomes the most powerful refactoring tool for AI-assisted development.

---

## Motivation

### Why Refocus on Refactoring?

1. **Clear, Differentiated Value Proposition**
   - "AI code buddy" is saturated (Copilot, Cursor, Cody, etc.)
   - "Refactoring powerhouse" is underserved - most tools offer basic rename/extract
   - TypeMill has unique LSP + AST hybrid architecture perfect for precision refactoring

2. **Leverages Core Strengths**
   - Multi-language LSP integration (7 languages: TypeScript, Python, Go, Rust, Java, Swift, C#)
   - Native Rust AST parsing for deeper analysis
   - Atomic multi-file edit system with rollback
   - Import tracking and automatic dependency updates

3. **AI Agent Perfect Fit**
   - AI agents excel at orchestrating primitive operations into complex workflows
   - Refactoring primitives compose naturally (rename → extract → move → inline)
   - Analysis primitives guide AI decision-making (complexity → suggest → refactor)

4. **Measurable Quality Improvements**
   - Reduces cognitive complexity (measurable metric)
   - Eliminates dead code (quantifiable results)
   - Improves test coverage (trackable progress)
   - Unlike "help me code," refactoring has clear success criteria

---

## The Two-Pillar Architecture

### Pillar 1: Refactoring Primitives (Code Transformation)

**Core Concept**: Atomic, composable operations that restructure code without changing behavior.

#### 1.1 Rename Operations

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `rename_symbol` | Rename variables, functions, classes across workspace | ✅ Implemented | Core |
| `rename_symbol_strict` | Position-specific rename (disambiguates) | ✅ Implemented | Core |
| `rename_file` | Rename file + auto-update imports | ✅ Implemented | Core |
| `rename_directory` | Rename directory + update all imports | ✅ Implemented | Core |
| `rename_parameter` | Rename function parameter (dedicated operation) | ❌ Missing | High |
| `rename_type` | Rename type/interface with propagation | ❌ Missing | Medium |

**Gap Analysis**: Basic rename operations exist, but need parameter-specific and type-specific variants for precision.

#### 1.2 Extract Operations

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `extract_function` | Extract code block into new function | ✅ Implemented (LSP/AST) | Core |
| `extract_variable` | Extract expression into named variable | ✅ Implemented (LSP/AST) | Core |
| `extract_module` | Extract symbols to new module/file | ⚠️ Partial (Rust only) | High |
| `extract_interface` | Extract interface from implementation | ❌ Missing | High |
| `extract_class` | Extract methods into new class | ❌ Missing | Medium |
| `extract_constant` | Extract magic numbers/strings to constants | ❌ Missing | High |
| `extract_type` | Extract type definition from usage | ❌ Missing | Medium |

**Gap Analysis**: Basic extraction exists, but advanced OOP extractions missing. Magic number detection exists (analysis), but extraction not wired up.

#### 1.3 Inline Operations

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `inline_variable` | Replace variable with its value | ✅ Implemented (LSP/AST) | Core |
| `inline_function` | Replace function call with body | ❌ Missing | High |
| `inline_constant` | Replace constant with literal value | ❌ Missing | Low |
| `inline_type` | Expand type alias to concrete type | ❌ Missing | Low |

**Gap Analysis**: Only variable inlining exists. Function inlining critical for simplification workflows.

#### 1.4 Move Operations

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `move_symbol` | Move function/class to different file | ❌ Missing | High |
| `move_to_module` | Move symbols to existing module | ❌ Missing | High |
| `move_to_namespace` | Move to different namespace/package | ❌ Missing | Medium |
| `consolidate_module` | Merge module into parent (Rust-specific) | ⚠️ Partial (rename_directory consolidate mode) | Medium |

**Gap Analysis**: No granular symbol moving. Directory consolidation exists for Rust, but not symbol-level moves.

#### 1.5 Reorder Operations

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `reorder_parameters` | Change parameter order (updates all calls) | ❌ Missing | Medium |
| `reorder_imports` | Sort imports by convention | ⚠️ Partial (organize_imports) | Low |
| `reorder_members` | Sort class members (fields, methods) | ❌ Missing | Low |
| `reorder_statements` | Reorder independent statements for clarity | ❌ Missing | Low |

**Gap Analysis**: Only basic import sorting. Parameter reordering valuable for refactoring APIs.

#### 1.6 Transform Operations (New Category)

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `convert_to_arrow_function` | Function declaration → arrow function | ❌ Missing | Low |
| `convert_to_async` | Convert sync function to async | ❌ Missing | Medium |
| `convert_loop_to_iterator` | for-loop → map/filter/reduce | ❌ Missing | Low |
| `convert_callback_to_promise` | Callback pattern → Promise/async | ❌ Missing | Low |
| `add_null_check` | Wrap code with null safety guards | ❌ Missing | Medium |
| `remove_dead_branch` | Remove unreachable if/else branch | ❌ Missing | High |

**Gap Analysis**: Entire category missing. These are LSP "code actions" - should expose as primitives.

#### 1.7 Deletion/Cleanup Operations

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `delete_unused_imports` | Remove unused imports | ⚠️ Partial (optimize_imports) | Core |
| `delete_dead_code` | Remove unreachable/unused code | ❌ Missing (analysis only) | High |
| `delete_redundant_code` | Remove duplicate logic | ❌ Missing | Medium |
| `delete_file` | Delete file with safety checks | ✅ Implemented | Core |

**Gap Analysis**: Dead code detection exists, but no automated removal. Should have separate `delete_dead_code` primitive.

---

### Pillar 2: Analysis Primitives (Code Understanding)

**Core Concept**: Operations that reveal code structure, quality, and optimization opportunities without modifying code.

#### 2.1 Complexity Analysis

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `analyze_complexity` | Cyclomatic + cognitive complexity per function | ✅ Implemented | Core |
| `analyze_project_complexity` | Project-wide complexity scanning | ✅ Implemented | Core |
| `find_complexity_hotspots` | Top N most complex functions | ✅ Implemented | Core |
| `analyze_nesting_depth` | Maximum nesting levels per function | ⚠️ Partial (in analyze_complexity) | Medium |
| `analyze_parameter_count` | Functions with too many parameters | ⚠️ Partial (in analyze_complexity) | Medium |
| `analyze_function_length` | Functions exceeding SLOC thresholds | ⚠️ Partial (in analyze_complexity) | Medium |

**Gap Analysis**: Core metrics exist. Should extract individual analyzers as standalone primitives for targeted queries.

#### 2.2 Code Smell Detection

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `suggest_refactoring` | Pattern-based refactoring suggestions | ✅ Implemented | Core |
| `find_magic_numbers` | Detect hard-coded numeric literals | ⚠️ Partial (in suggest_refactoring) | High |
| `find_long_methods` | Methods exceeding length thresholds | ⚠️ Partial (in suggest_refactoring) | Medium |
| `find_god_classes` | Classes with too many responsibilities | ❌ Missing | High |
| `find_duplicated_code` | Detect copy-paste duplication | ❌ Missing | High |
| `find_primitive_obsession` | Overuse of primitives vs domain types | ❌ Missing | Low |
| `find_feature_envy` | Methods using external data heavily | ❌ Missing | Low |

**Gap Analysis**: Basic suggestions exist. Need dedicated detectors for classic code smells (God Class, Duplicate Code).

#### 2.3 Dead Code Analysis

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `find_dead_code` | LSP-based unused symbol detection | ✅ Implemented | Core |
| `find_unused_imports` | AST-based import analysis | ✅ Implemented | Core |
| `find_unused_parameters` | Parameters never referenced | ❌ Missing | High |
| `find_unreachable_code` | Code after return/throw | ❌ Missing | High |
| `find_unused_variables` | Local variables never read | ❌ Missing | Medium |
| `find_unused_types` | Type definitions never referenced | ❌ Missing | Medium |

**Gap Analysis**: Core dead code detection exists. Missing granular unused entity detection.

#### 2.4 Dependency Analysis

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `analyze_imports` | Parse and categorize imports | ✅ Implemented | Core |
| `analyze_dependencies` | Dependency graph (file/module level) | ❌ Missing | High |
| `find_circular_dependencies` | Detect circular import cycles | ❌ Missing | High |
| `find_coupling` | Measure module coupling strength | ❌ Missing | Medium |
| `find_cohesion` | Measure module cohesion | ❌ Missing | Low |
| `analyze_dependency_depth` | Transitive dependency depth | ❌ Missing | Low |

**Gap Analysis**: Basic import parsing exists. No graph analysis or cycle detection (existing proposal: `51_PROPOSAL_CIRCULAR_DEPENDENCY_DETECTION.md`).

#### 2.5 Structural Analysis

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `get_document_symbols` | Hierarchical symbol tree (LSP) | ✅ Implemented | Core |
| `search_workspace_symbols` | Project-wide symbol search (LSP) | ✅ Implemented | Core |
| `find_definition` | Symbol definition location | ✅ Implemented | Core |
| `find_references` | All symbol usage locations | ✅ Implemented | Core |
| `find_implementations` | Interface implementations | ✅ Implemented | Core |
| `analyze_inheritance` | Class hierarchy analysis | ❌ Missing | Medium |
| `analyze_interface_usage` | Interface adoption patterns | ❌ Missing | Low |

**Gap Analysis**: LSP navigation primitives are comprehensive. Missing OOP hierarchy analysis.

#### 2.6 Documentation & Comments Analysis

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `analyze_comment_ratio` | Comment density metrics | ⚠️ Partial (in analyze_complexity) | Medium |
| `find_undocumented_exports` | Public APIs without docs | ❌ Missing | High |
| `find_outdated_comments` | Comments contradicting code | ❌ Missing | Low |
| `find_todo_comments` | Extract TODO/FIXME markers | ❌ Missing | Medium |

**Gap Analysis**: Basic comment ratio exists. Missing documentation quality checks.

#### 2.7 Test Coverage Analysis

| Primitive | Description | Current Status | Priority |
|-----------|-------------|----------------|----------|
| `analyze_test_coverage` | Coverage percentage per file/function | ❌ Missing | High |
| `find_untested_code` | Functions without test coverage | ❌ Missing | High |
| `analyze_test_quality` | Assertion count, mock usage | ❌ Missing | Medium |
| `find_test_smells` | Slow tests, fragile tests | ❌ Missing | Low |

**Gap Analysis**: Entire category missing. Critical for refactoring confidence (requires integration with coverage tools).

---

## Proposed Command Structure

### Organization by Pillar

All commands should be organized and documented by their pillar to reinforce the refactoring-first identity.

#### Commands: Refactoring Primitives (Transformation)

**Rename**
- `rename_symbol` ✅
- `rename_symbol_strict` ✅
- `rename_file` ✅
- `rename_directory` ✅
- `rename_parameter` ⬜
- `rename_type` ⬜

**Extract**
- `extract_function` ✅
- `extract_variable` ✅
- `extract_module` ⬜
- `extract_interface` ⬜
- `extract_constant` ⬜

**Inline**
- `inline_variable` ✅
- `inline_function` ⬜
- `inline_constant` ⬜

**Move**
- `move_symbol` ⬜
- `move_to_module` ⬜
- `move_to_namespace` ⬜

**Reorder**
- `reorder_parameters` ⬜
- `reorder_imports` ⚠️ (organize_imports)

**Transform**
- `convert_to_async` ⬜
- `add_null_check` ⬜
- `remove_dead_branch` ⬜

**Delete**
- `delete_unused_imports` ⚠️ (optimize_imports)
- `delete_dead_code` ⬜
- `delete_file` ✅

#### Commands: Analysis Primitives (Understanding)

**Complexity**
- `analyze_complexity` ✅
- `analyze_project_complexity` ✅
- `find_complexity_hotspots` ✅

**Code Smells**
- `suggest_refactoring` ✅
- `find_magic_numbers` ⬜
- `find_duplicated_code` ⬜
- `find_god_classes` ⬜

**Dead Code**
- `find_dead_code` ✅
- `find_unused_imports` ✅
- `find_unused_parameters` ⬜
- `find_unreachable_code` ⬜

**Dependencies**
- `analyze_imports` ✅
- `find_circular_dependencies` ⬜
- `analyze_dependencies` ⬜

**Structure**
- `find_definition` ✅
- `find_references` ✅
- `get_document_symbols` ✅
- `search_workspace_symbols` ✅

**Documentation**
- `find_undocumented_exports` ⬜
- `find_todo_comments` ⬜

**Testing**
- `analyze_test_coverage` ⬜
- `find_untested_code` ⬜

---

## Documentation & Messaging Changes

### README.md Repositioning

**Before:**
> Codebuddy is a pure Rust MCP server bridging Language Server Protocol (LSP) functionality to AI coding assistants.

**After:**
> **TypeMill** is a refactoring powerhouse for AI-assisted development. Built on LSP and AST intelligence, it provides **refactoring primitives** (rename, extract, move, inline) and **analysis primitives** (complexity, smells, dead code) that compose into sophisticated code transformations.

### Tagline Options

1. "Precision refactoring for AI agents"
2. "The refactoring engine behind AI code assistants"
3. "Composable refactoring primitives for intelligent code transformation"
4. "Turn complexity into clarity - automated refactoring at scale"

### API_REFERENCE.md Reorganization

Current organization:
- Navigation & Intelligence
- Editing & Refactoring
- Code Analysis
- File Operations
- Workspace Operations
- Advanced Operations

**Proposed organization:**

```markdown
# TypeMill API Reference

## Refactoring Primitives (Transform Code)

### Rename Operations
- rename_symbol
- rename_symbol_strict
- rename_file
- rename_directory
- rename_parameter (🚧 coming soon)

### Extract Operations
- extract_function
- extract_variable
- extract_module (🚧 coming soon)
- extract_interface (🚧 coming soon)

### Inline Operations
- inline_variable
- inline_function (🚧 coming soon)

### Move Operations
- move_symbol (🚧 coming soon)
- move_to_module (🚧 coming soon)

### Delete Operations
- delete_unused_imports
- delete_dead_code (🚧 coming soon)
- delete_file

## Analysis Primitives (Understand Code)

### Complexity Analysis
- analyze_complexity
- analyze_project_complexity
- find_complexity_hotspots

### Code Smell Detection
- suggest_refactoring
- find_magic_numbers (🚧 coming soon)
- find_duplicated_code (🚧 coming soon)
- find_god_classes (🚧 coming soon)

### Dead Code Detection
- find_dead_code
- find_unused_imports
- find_unused_parameters (🚧 coming soon)

### Dependency Analysis
- analyze_imports
- find_circular_dependencies (🚧 coming soon)

### Structural Navigation
- find_definition
- find_references
- find_implementations
- get_document_symbols
- search_workspace_symbols

## Foundation Tools (Enable Primitives)

### LSP Integration
- get_hover
- get_completions
- get_diagnostics
- format_document

### File Operations
- create_file
- read_file
- write_file
- list_files

### Advanced Orchestration
- apply_edits (atomic multi-file)
- batch_execute
- workflow engine (see docs/features/WORKFLOWS.md)
```

---

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-2) - **Documentation & Messaging**

**Goals:**
- Establish refactoring-first identity in all documentation
- Reorganize API reference by pillars
- Create clear "coming soon" roadmap

**Tasks:**
1. Update README.md with new positioning
2. Reorganize API_REFERENCE.md by pillars
3. Update CLAUDE.md/AGENTS.md with refactoring focus
4. Create visual primitive hierarchy diagram
5. Add "🚧 Coming Soon" badges to planned primitives
6. Write blog post: "Why TypeMill Focuses on Refactoring"

**Success Criteria:**
- [ ] All docs reflect refactoring-first vision
- [ ] API organized by Refactoring/Analysis pillars
- [ ] Clear roadmap visible to users

### Phase 2: Quick Wins (Weeks 3-4) - **Low-Hanging Fruit Primitives**

**Goals:**
- Implement 3-5 missing primitives that leverage existing infrastructure
- Demonstrate progress toward completeness

**Candidates:**

1. **`delete_dead_code`** (High Priority)
   - Leverage existing `find_dead_code` analysis
   - Implement removal with safety checks
   - Atomic multi-file deletion with rollback

2. **`extract_constant`** (High Priority)
   - Leverage existing magic number detection
   - Extract to module-level constant
   - Update all occurrences

3. **`find_unused_parameters`** (High Priority)
   - AST-based parameter analysis
   - Check for parameter usage in function body
   - Report location + suggestion

4. **`inline_function`** (High Priority)
   - LSP-first approach (code action)
   - AST fallback for simple cases
   - Multi-language support

5. **`find_undocumented_exports`** (Medium Priority)
   - AST-based public API detection
   - Check for doc comments
   - Language-specific conventions (JSDoc, docstrings, etc.)

**Success Criteria:**
- [ ] 5 new primitives implemented
- [ ] Tests pass for all languages
- [ ] Documentation complete with examples

### Phase 3: Core Gaps (Weeks 5-8) - **Critical Missing Primitives**

**Goals:**
- Fill critical gaps in refactoring capabilities
- Enable common refactoring workflows

**Priorities:**

1. **Move Operations** (Weeks 5-6)
   - `move_symbol` - Move function/class to different file
   - `move_to_module` - Move to existing module
   - Leverage existing import update infrastructure
   - Multi-language support

2. **Advanced Extraction** (Week 7)
   - `extract_module` - Extract symbols to new file
   - `extract_interface` - Extract interface from class (OOP languages)
   - Complex import generation

3. **Circular Dependency Detection** (Week 8)
   - `find_circular_dependencies` - Detect import cycles
   - Graph-based analysis (see proposal #51)
   - Suggest breaking strategies

**Success Criteria:**
- [ ] Move operations work across 7 languages
- [ ] Advanced extraction for TypeScript/Python/Java/C#
- [ ] Circular dependency detection functional

### Phase 4: Advanced Analysis (Weeks 9-12) - **Code Smell Detection**

**Goals:**
- Implement sophisticated pattern detection
- Enable AI-guided refactoring suggestions

**Priorities:**

1. **Duplication Detection** (Weeks 9-10)
   - `find_duplicated_code` - AST-based clone detection
   - Token-based similarity analysis
   - Report location pairs + similarity score

2. **God Class Detection** (Week 11)
   - `find_god_classes` - Detect classes with too many responsibilities
   - Metrics: method count, field count, complexity, coupling
   - Suggest extraction candidates

3. **Dependency Graph Analysis** (Week 12)
   - `analyze_dependencies` - Full dependency graph
   - Coupling/cohesion metrics
   - Visualization-ready output (GraphViz, Mermaid)

**Success Criteria:**
- [ ] Duplication detection with configurable thresholds
- [ ] God class detection with actionable suggestions
- [ ] Dependency graph export to standard formats

### Phase 5: Test Integration (Weeks 13-16) - **Coverage Analysis**

**Goals:**
- Integrate with test coverage tools
- Enable "refactor with confidence" workflows

**Priorities:**

1. **Coverage Integration** (Weeks 13-14)
   - `analyze_test_coverage` - Parse coverage reports (lcov, cobertura)
   - Per-function coverage mapping
   - Integration with language-specific tools (pytest-cov, nyc, tarpaulin)

2. **Untested Code Detection** (Week 15)
   - `find_untested_code` - Functions with 0% coverage
   - Prioritize by complexity (high complexity + low coverage = refactor risk)

3. **Refactoring Safety Checks** (Week 16)
   - Pre-refactor coverage snapshot
   - Post-refactor coverage comparison
   - Warn if refactor reduces coverage

**Success Criteria:**
- [ ] Coverage parsing for 3+ coverage formats
- [ ] Untested code detection working
- [ ] Safety checks prevent coverage regression

---

## Success Metrics

### User-Facing Metrics

1. **Refactoring Operations Per Session**
   - Track: rename, extract, move, inline operations
   - Goal: 10+ primitives per refactoring session

2. **Code Quality Improvements**
   - Measure: Average complexity reduction
   - Measure: Dead code removal percentage
   - Goal: 20% complexity reduction in typical refactor

3. **Time Savings**
   - Baseline: Manual refactoring time
   - Measure: Time with TypeMill assistance
   - Goal: 5x faster refactoring

### Technical Metrics

1. **Primitive Coverage**
   - Current: ~40% of proposed primitives
   - Phase 2: 60% coverage
   - Phase 3: 80% coverage
   - Phase 5: 95% coverage

2. **Language Parity**
   - All primitives work across 7 supported languages
   - LSP-first ensures consistency
   - AST fallback for LSP gaps

3. **Workflow Composition**
   - Number of built-in refactoring workflows
   - Current: 3-5 workflows
   - Goal: 20+ workflows (see docs/features/WORKFLOWS.md)

---

## Positioning vs. Competitors

### TypeMill vs. IDE Refactoring Tools

| Feature | IntelliJ | VS Code | TypeMill |
|---------|----------|---------|----------|
| **Rename** | ✅ | ✅ | ✅ Multi-language |
| **Extract** | ✅ | ⚠️ Limited | ✅ Multi-language |
| **Move** | ✅ Java-focused | ❌ | 🚧 Coming soon |
| **Complexity Analysis** | ⚠️ Plugins | ❌ | ✅ Built-in |
| **Dead Code Detection** | ⚠️ Plugins | ⚠️ Limited | ✅ LSP-based |
| **Circular Dependencies** | ⚠️ Plugins | ❌ | 🚧 Coming soon |
| **AI Integration** | ❌ | ⚠️ Copilot | ✅ MCP protocol |
| **Multi-Language** | ❌ Per-IDE | ⚠️ Per-extension | ✅ Unified |
| **Atomic Multi-File** | ⚠️ Limited | ❌ | ✅ Built-in |

**TypeMill Advantage**: Unified, AI-native, multi-language refactoring across all codebases.

### TypeMill vs. AI Coding Assistants

| Feature | GitHub Copilot | Cursor | Claude Code | TypeMill |
|---------|----------------|--------|-------------|----------|
| **Code Generation** | ✅ | ✅ | ✅ | ❌ (not focus) |
| **Refactoring** | ⚠️ Manual | ⚠️ Manual | ⚠️ Manual | ✅ Automated |
| **Analysis** | ❌ | ❌ | ⚠️ Limited | ✅ Deep |
| **LSP Integration** | ❌ | ⚠️ Basic | ⚠️ Basic | ✅ Full |
| **Multi-Language** | ✅ | ✅ | ✅ | ✅ |
| **Atomic Edits** | ❌ | ❌ | ❌ | ✅ |
| **Dry-Run Mode** | ❌ | ❌ | ❌ | ✅ |

**TypeMill Advantage**: AI assistants use TypeMill as the refactoring engine, composing primitives intelligently.

---

## Communication Plan

### Internal Alignment

1. **Team Discussion** (Week 1)
   - Review this proposal
   - Prioritize primitives for Phases 2-5
   - Assign ownership for implementation

2. **Documentation Sprint** (Week 2)
   - Update all docs with new positioning
   - Create visual diagrams of primitive hierarchy
   - Write migration guide for existing users

### External Messaging

1. **Blog Post Series**
   - Week 2: "Introducing TypeMill: Refactoring-First AI Tool"
   - Week 4: "Refactoring Primitives: The Building Blocks of Clean Code"
   - Week 8: "Analysis Primitives: Understanding Before Transforming"

2. **Social Media**
   - Thread: "Why we're focusing on refactoring, not code generation"
   - Demo videos: Before/after refactoring examples
   - Metrics: Show complexity reduction, dead code elimination

3. **Integration Examples**
   - Claude Code integration guide
   - Cursor integration example
   - MCP protocol showcase

---

## Risks & Mitigations

### Risk 1: Scope Creep
**Impact**: High - Could delay core primitives
**Mitigation**:
- Strict prioritization by phase
- "No" to features outside two pillars
- Focus on primitive completeness over fancy workflows

### Risk 2: LSP Server Gaps
**Impact**: Medium - Some primitives may be LSP-unsupported
**Mitigation**:
- AST fallback for all critical primitives
- Contribute upstream to LSP servers (rust-analyzer, typescript-language-server)
- Document LSP support matrix clearly

### Risk 3: User Confusion (Positioning Change)
**Impact**: Low - Existing users may expect broader tool
**Mitigation**:
- Clear communication in CHANGELOG
- Emphasize "better at refactoring, still supports basics"
- Showcase improved primitives as value-add

### Risk 4: AI Integration Complexity
**Impact**: Medium - AI agents must orchestrate primitives correctly
**Mitigation**:
- Provide built-in workflows for common patterns
- Excellent documentation with examples
- Integration guides for major AI assistants

---

## Alternatives Considered

### Alternative 1: Stay General-Purpose
**Pros**: Broader appeal, more use cases
**Cons**: Diluted value prop, saturated market, no differentiation

**Why Rejected**: "Jack of all trades, master of none" - better to be best-in-class at refactoring.

### Alternative 2: Focus on Single Language
**Pros**: Easier to implement, deeper features
**Cons**: Limited market, doesn't leverage multi-language LSP advantage

**Why Rejected**: Multi-language support is a core strength - don't abandon it.

### Alternative 3: Analysis-Only Tool
**Pros**: Simpler scope, lower risk
**Cons**: No transformation capability = limited value

**Why Rejected**: Analysis without action is frustrating. The magic is in **Analyze → Refactor** loop.

---

## Open Questions

1. **Naming Convention for Primitives**
   - Should we use `refactor_*` prefix (e.g., `refactor_extract_function`)?
   - Or keep short names (`extract_function`) with pillar organization?
   - **Recommendation**: Keep short names, organize by pillar in docs

2. **Dry-Run Default Behavior**
   - Should destructive operations default to dry-run=true?
   - Or keep dry-run=false with clear warnings?
   - **Recommendation**: Keep default false, but improve warning messages

3. **Workflow Engine Exposure**
   - Should workflows be first-class primitives or internal orchestration?
   - See docs/features/WORKFLOWS.md
   - **Recommendation**: Keep workflows internal for now, expose if users demand it

4. **Test Coverage Integration**
   - Which coverage formats to prioritize (lcov, cobertura, jacoco)?
   - Should we run tests ourselves or parse existing reports?
   - **Recommendation**: Parse existing reports (non-invasive), support top 3 formats

---

## Next Steps

1. **Approve Proposal** (Week 1)
   - Gather team feedback
   - Finalize primitive priorities
   - Commit to refactoring-first positioning

2. **Begin Phase 1** (Week 1-2)
   - Documentation updates
   - API reorganization
   - Visual diagrams

3. **Launch Phase 2** (Week 3)
   - Implement quick-win primitives
   - Publish progress updates
   - Gather user feedback

4. **Track Metrics** (Ongoing)
   - Primitive coverage percentage
   - User refactoring session metrics
   - Code quality improvements

---

## Conclusion

Repositioning TypeMill as a **refactoring-first tool** with **Refactoring Primitives** and **Analysis Primitives** provides:

✅ **Clear differentiation** in crowded AI coding assistant market
✅ **Leverage core strengths** (LSP, AST, multi-language, atomic edits)
✅ **Measurable value** (complexity reduction, dead code removal, test coverage)
✅ **Natural AI composition** (primitives compose into workflows)
✅ **Focused roadmap** (fill gaps in two pillars, not sprawl)

**Recommendation**: **Approve and proceed** with Phase 1 (Documentation & Messaging).

---

## Appendix A: Full Primitive Inventory

### Refactoring Primitives (24 total)

**Implemented**: 8
**Partial**: 3
**Missing**: 13

| Status | Count | Percentage |
|--------|-------|------------|
| ✅ Implemented | 8 | 33% |
| ⚠️ Partial | 3 | 13% |
| ❌ Missing | 13 | 54% |

### Analysis Primitives (35 total)

**Implemented**: 12
**Partial**: 5
**Missing**: 18

| Status | Count | Percentage |
|--------|-------|------------|
| ✅ Implemented | 12 | 34% |
| ⚠️ Partial | 5 | 14% |
| ❌ Missing | 18 | 51% |

### Overall Primitive Coverage

The table above reflects analysis primitives only. When combined with the refactoring inventory (24 primitives), the total footprint is 59 primitives overall:

| Status | Count | Percentage |
|--------|-------|------------|
| ✅ Implemented | 20 | 34% |
| ⚠️ Partial | 8 | 14% |
| ❌ Missing | 31 | 53% |

**Total Primitives**: 59
**Implemented**: 20 (34%)
**Partial**: 8 (14%)
**Missing**: 31 (53%)

**Phase Goals**:
- Phase 2: 60% coverage (35 primitives)
- Phase 3: 80% coverage (47 primitives)
- Phase 5: 95% coverage (56 primitives)

---

## Appendix B: Example Refactoring Workflows

### Workflow 1: Reduce Complexity

**Input**: Function with complexity > 20

**Steps**:
1. `analyze_complexity` → Identify complex function
2. `suggest_refactoring` → Get actionable suggestions
3. `extract_function` → Extract nested blocks
4. `inline_variable` → Remove temporary variables
5. `analyze_complexity` → Verify reduction

**Expected Outcome**: Complexity reduced to < 10

### Workflow 2: Clean Dead Code

**Input**: Project with 15% unused code

**Steps**:
1. `find_dead_code` → Identify unused symbols
2. `find_unused_imports` → Identify unused imports
3. `delete_dead_code` → Remove unused symbols (batch)
4. `delete_unused_imports` → Remove unused imports
5. `analyze_project_complexity` → Measure improvement

**Expected Outcome**: 95% of dead code removed

### Workflow 3: Extract Module

**Input**: God class with 2000+ lines

**Steps**:
1. `analyze_complexity` → Identify god class
2. `find_god_classes` → Confirm anti-pattern
3. `analyze_dependencies` → Find logical groupings
4. `extract_module` → Extract related methods
5. `move_symbol` → Move symbols to new module
6. `rename_symbol` → Rename for clarity
7. `analyze_dependencies` → Verify reduced coupling

**Expected Outcome**: God class split into 3-5 focused modules

---

## Appendix C: Where to Document This

### Primary Location

**`/workspace/PROPOSAL_REFACTORING_FOCUS.md`** (this document)
- Source of truth for vision and roadmap
- Updated quarterly with progress
- Referenced in all planning discussions

### Integration into Existing Docs

1. **`README.md`** (Introduction)
   ```markdown
   # TypeMill

   **Precision refactoring for AI-assisted development**

   TypeMill provides refactoring primitives (rename, extract, move, inline) and
   analysis primitives (complexity, smells, dead code) that compose into
   sophisticated code transformations.

   See [PROPOSAL_REFACTORING_FOCUS.md](PROPOSAL_REFACTORING_FOCUS.md) for vision.
   ```

2. **`API_REFERENCE.md`** (Header)
   ```markdown
   # TypeMill API Reference

   **Refactoring-First Design**: All tools organized by Refactoring Primitives
   (transform code) and Analysis Primitives (understand code). See
   [PROPOSAL_REFACTORING_FOCUS.md](PROPOSAL_REFACTORING_FOCUS.md) for details.
   ```

3. **`CLAUDE.md` / `AGENTS.md`** (Project Information)
   ```markdown
   ## Vision

   TypeMill is a **refactoring-first tool** built on two pillars:
   1. **Refactoring Primitives** - Atomic code transformations
   2. **Analysis Primitives** - Code understanding operations

   See [PROPOSAL_REFACTORING_FOCUS.md](PROPOSAL_REFACTORING_FOCUS.md) for
   complete vision and roadmap.
   ```

4. **`CONTRIBUTING.md`** (Adding New Tools)
   ```markdown
   ### Tool Categories

   When adding new tools, categorize by pillar:
   - **Refactoring Primitives**: rename_*, extract_*, inline_*, move_*, delete_*
   - **Analysis Primitives**: analyze_*, find_*, suggest_*

   See [PROPOSAL_REFACTORING_FOCUS.md](PROPOSAL_REFACTORING_FOCUS.md) for
   priority primitives.
   ```

5. **New File: `docs/vision/REFACTORING_FIRST.md`** (User-Facing)
   - Simplified version of this proposal
   - Focus on "why refactoring matters"
   - Examples of before/after transformations
   - Link to full proposal for contributors
