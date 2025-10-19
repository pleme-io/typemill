# Remaining Proposals (10 Total)

## Phase 0: Actionable Suggestions (1 proposal)

### 00_actionable_suggestions_integration.proposal.md
- **Goal:** Integrate actionable suggestion generation into analysis commands
- **Status:** Partial (some infrastructure exists, needs completion)
- **Priority:** Medium (enhances UX for analysis features)

---

## Phase 1: Build & Dev Infrastructure (3 proposals)

### 01_xtask_pattern_adoption.proposal.md
- **Goal:** Replace Makefile with cargo xtask pattern
- **Benefits:** Standardize build/dev workflows, cargo-native task runner
- **Status:** Draft
- **Priority:** Medium (improves developer experience)

### 01a_rust_directory_structure.proposal.md
- **Goal:** Adopt Rust-optimized directory structure
- **Status:** Draft
- **Priority:** Low (organizational improvement)

### 01b_cargo_deny_integration.proposal.md
- **Goal:** Add cargo-deny for dependency auditing
- **Benefits:** Security and license compliance checking
- **Status:** Draft
- **Priority:** Medium (security enhancement)

---

## Phase 3: Single Language Builds (1 proposal)

### 03_single_language_builds.proposal.md
- **Goal:** Support building with single language (TypeScript OR Rust only)
- **Benefits:** Remove hard-wired cross-language dependencies, modular plugin system
- **Status:** Proposal
- **Priority:** High (architectural flexibility)

---

## Phase 4: Language Expansion (1 proposal)

### 04_language_expansion.proposal.md
- **Goal:** Re-enable Python, Go, Java, Swift, C# language support
- **Benefits:** Restore multi-language capabilities from pre-reduction
- **Status:** Draft
- **Dependencies:** Requires Phase 3 (single language builds) first
- **Priority:** High (feature completeness)

---

## Phase 5: Project Rename (1 proposal)

### 05_rename_to_typemill.proposal.md
- **Goal:** Rename project from "Codebuddy" to "TypeMill"
- **Benefits:** Branding and identity update
- **Status:** Draft
- **Priority:** Low-Medium (branding decision)

---

## Workspace & Architecture (3 proposals)

### 06_workspace_consolidation.proposal.md
- **Goal:** Consolidate workspace structure and harden architecture
- **Status:** Draft
- **Priority:** Medium (organizational improvement)

### 07_expose_consolidate_in_rename_command.md
- **Goal:** Expose consolidate feature in rename command UI
- **Benefits:** Make consolidation mode more discoverable for users
- **Status:** Draft
- **Priority:** Low (UX enhancement for existing feature)

### 07_plugin_architecture_decoupling.proposal.md
- **Goal:** Decouple plugin architecture, remove direct language plugin dependencies
- **Benefits:** Cleaner separation of concerns
- **Status:** Draft
- **Priority:** Medium (architectural improvement)

---

## Summary by Priority

**High Priority (2):**
- 03_single_language_builds.proposal.md
- 04_language_expansion.proposal.md

**Medium Priority (5):**
- 00_actionable_suggestions_integration.proposal.md
- 01_xtask_pattern_adoption.proposal.md
- 01b_cargo_deny_integration.proposal.md
- 06_workspace_consolidation.proposal.md
- 07_plugin_architecture_decoupling.proposal.md

**Low Priority (3):**
- 01a_rust_directory_structure.proposal.md
- 05_rename_to_typemill.proposal.md
- 07_expose_consolidate_in_rename_command.md

---

## Recommended Next Steps

Based on earlier discussion about sequential execution with testing between phases:

**Option 1: Continue with Phase 1 (Build Infrastructure)**
- Start with 01_xtask_pattern_adoption.proposal.md
- Improves developer experience
- Lower risk, good incremental progress

**Option 2: Focus on High-Priority Architectural Work**
- Start with 03_single_language_builds.proposal.md
- Enables 04_language_expansion.proposal.md
- Higher impact but more complex

**Option 3: Complete Phase 0 (Actionable Suggestions)**
- Finish 00_actionable_suggestions_integration.proposal.md
- Close out partially-complete work
- Adds user-facing value

---

## Recently Completed (Reference)

**Phase 2: Code Quality (All Complete ✅)**
- 02d: LSP zombie process fixes
- 02c: Workspace apply handler split
- 02f: Comprehensive rename updates
- 02g: Cargo package rename coverage

**Architecture (Complete ✅)**
- 09: God crate decomposition (codebuddy-core split)

**Test Status:** 822 passing, 2 skipped, 0 failures
