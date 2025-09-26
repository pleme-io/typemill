# Proposal: Intent-Based Workflow Engine

## 1. Executive Summary

To elevate the MCP API from a powerful toolbox to a transformative codebase manipulation engine, this proposal outlines the evolution from the current "Workflow" system to an **Intent-Based Workflow Engine**. This shift will enable AI agents and users to declare high-level goals (e.g., "move a function") rather than executing a complex sequence of low-level commands. The engine will be responsible for intelligently planning, executing, and verifying these operations, dramatically increasing reliability, safety, and the scope of possible automated refactorings.

## 2. The Proposal: An Intent-Based Workflow Engine

The core idea is to shift the responsibility of *planning* from the AI agent to the MCP. The AI specifies **what** it wants to do (the Intent), and the MCP figures out **how** to do it. This moves MCP from a command executor to a goal-oriented partner.

This engine will be built on three key pillars and a collaboration model that lets multiple implementers (or agents) advance the work in parallel without tripping over each other.

### Pillar 1: A High-Level "Intent" API

We will introduce a new top-level tool, `achieve_intent`, which takes a goal-oriented input. This abstracts away the complex orchestration logic from the client.

**Example:** Instead of a multi-step process to move a function, an agent would make a single call:

```json
{
  "tool": "achieve_intent",
  "intent": "refactor_move_function",
  "args": {
    "source_file": "src/services/old_service.ts",
    "destination_file": "src/services/new_service.ts",
    "function_name": "calculateTotals"
  }
}
```

Other potential intents include:
*   `refactor_extract_interface_from_class`
*   `refactor_convert_to_async_await`
*   `debug_find_and_fix_null_reference`

### Pillar 2: A Dynamic "Workflow Planner"

The "brain" of the engine, the Workflow Planner, receives an `intent` and dynamically generates a multi-step execution plan using existing and new MCP tools.

**Example Plan for `refactor_move_function`:**

1.  **Analyze:** Use `get_document_symbols` to find the exact code block for the target function.
2.  **Analyze:** Use a new `find_dependent_symbols` tool to identify all other functions/imports the target function needs.
3.  **Analyze:** Use `find_references` to locate every call site across the workspace.
4.  **Execute:** Use `batch_execute` with `atomic: true` to perform the file modifications:
    *   Create the destination file.
    *   Write the function and its dependencies to the new file (adding `export`).
    *   Remove the function from the old file.
    *   Add necessary `import` statements to all files with references.
5.  **Verify:** Use `get_diagnostics` on all modified files to check for new errors.
6.  **Commit/Rollback:** If verification passes, the operation is committed. If it fails, the atomic transaction is automatically rolled back, ensuring the codebase remains in a valid state.

### Pillar 3: New Semantic Tools

To empower the Workflow Planner, we must introduce a new class of tools that understand code's meaning (semantics), not just its text.

*   **AST (Abstract Syntax Tree) Tools:**
    *   `get_ast_node`: Get the raw syntax tree node for precise analysis and manipulation.
    *   `query_ast`: Find all nodes matching a specific pattern (e.g., find all legacy promise chains).
*   **Dependency & Scope Tools:**
    *   `find_dependent_symbols`: Find everything a function needs to run.
    *   `find_symbols_in_scope`: Identify all variables and functions available at a specific line.
*   **Code Quality Tools:**
    *   `get_code_smells`: Proactively identify refactoring opportunities like high cyclomatic complexity or duplicated logic.

### Pillar 4: Contract-First, Parallel-Friendly Architecture

To support N contributors working simultaneously, every pathway in the engine is exposed through a versioned API contract and mapped to a self-contained package with its own tests.

**Package layout.**

| Package | Responsibility | Consumes | Publishes |
|---------|----------------|----------|-----------|
| `intent-planner` | Convert intents into executable plans | `intent-schema` | `plan-schema` |
| `semantic-tools` | AST queries, dependency graph, code metrics | Source files | `semantic-schema` |
| `plan-runner` | Execute plans using MCP tools & transactions | `plan-schema`, MCP clients | `run-report` |
| `verification-suite` | Diagnostics, diff checks, rollback logic | `run-report` | `verification-report` |
| `contract-tests` | Shared mocks and schema validation | All public schemas | ✅ (pass/fail) |

Each package is a separate workspace entry (mirroring a Cargo-style crate) and must expose:

1. **Types & Schemas** – JSON Schema + TypeScript types in `schemas/<name>.ts`.
2. **Service Interface** – Stable entry points in `src/index.ts` (factories or classes).
3. **Acceptance Tests** – Located under `tests/acceptance`, using only public interfaces.

Versioning rules are tracked in `docs/contracts/CHANGELOG.md`. Breaking changes require a major bump and a migration note.

**API contracts.**

- `intent-schema` defines the allowed intents, required arguments, and error codes.
- `plan-schema` describes planner output (steps, tool calls, metadata) and must serialize cleanly to JSON for inspection.
- `run-report` captures execution details including per-step status, emitted diffs, and rollback markers.
- `verification-report` expresses diagnostics, test summaries, and overall disposition (`passed`, `rolled_back`).

Each schema has a golden snapshot committed under `tests/__snapshots__`. Implementers update these snapshots only when contracts evolve, guaranteeing consumers detect drift early.

**Coordination workflow.**

1. Freeze schema versions before work begins; mark them as `stable` in the proposal table above.
2. Assign packages to teams/agents. They work locally with the shared mocks in `contract-tests` to simulate peer behavior.
3. Upon completion, teams run `bun test --filter contracts` to ensure their implementation conforms to the locked schemas.
4. CI runs an integration suite that stitches packages together strictly via exported interfaces, verifying “it just works” once all packages land.

## 3. Why This Makes the Most Difference

This approach represents a paradigm shift with three major benefits:

1.  **Massive Reduction in Complexity:** The AI agent's role is simplified. It can focus on high-level strategy, offloading complex, error-prone execution details to the MCP engine.
2.  **Increased Reliability and Safety:** By codifying refactoring logic within the MCP, we ensure it is performed correctly and safely (with automatic rollbacks) every time.
3.  **Unlocks Transformative Refactorings:** This engine makes it possible to safely perform complex, workspace-wide changes that are currently infeasible, such as "Extract interface from class" or "Convert an entire module to use async/await."
4.  **Scales Team Execution:** The contract-first, package-per-pathway structure lets multiple contributors deliver features simultaneously. As long as their code passes the contract test suite, their work integrates cleanly without manual coordination.

## 4. Conclusion

The future of AI-driven development lies in systems that understand intent and manage complexity. By evolving the MCP into an Intent-Based Workflow Engine, we position it at the forefront of this trend, creating a more powerful, reliable, and intelligent system for both AI agents and human developers.
