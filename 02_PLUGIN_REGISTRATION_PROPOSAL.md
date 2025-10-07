● 🎯 EXECUTION STRATEGY: Plugin Self-Registration Implementation

  Current State Analysis

  Confidence Level: 99.999% ✅ (Already achieved through deep analysis)

  Context Loaded:
  - ✅ Complete architectural understanding
  - ✅ All 10 files analyzed in detail
  - ✅ Dependency graph mapped
  - ✅ Test patterns understood
  - ✅ Build system understood (languages.toml → build.rs)

  ---
  📊 STRATEGIC DECISION: SEQUENTIAL 3-AGENT DEPLOYMENT

  Rationale

  Why Sequential (Not Parallel):

  1. Dependency Chain:
  Agent 1 (API) → Must complete first
       ↓
  Agent 2 (Python Plugin) → Needs types from Agent 1
       ↓
  Agent 3 (Tests) → Needs fixtures from Agent 2
  2. File Conflict Risk: ZERO (no overlapping files)
  3. Context Budget:
    - Agent 1: ~8K tokens (API creation)
    - Agent 2: ~6K tokens (Plugin implementation)
    - Agent 3: ~12K tokens (Test refactor)
    - Total: ~26K tokens (well under 50K per agent)
  4. Verification Points: Each agent completion = checkpoint

  ---
  🎬 THREE-AGENT DEPLOYMENT PLAN

  Agent 1: "Morgan-API" - Foundation Architect

  Mission: Create the plugin test fixtures API

  Files:
  - CREATE: crates/cb-plugin-api/src/test_fixtures.rs
  - EDIT: crates/cb-plugin-api/src/lib.rs

  Dependencies: NONE (foundation layer)

  Success Criteria:
  - cargo check --package cb-plugin-api passes
  - Types compile without errors
  - Trait method added to LanguagePlugin

  Context Size: ~8K tokens

  ---
  Agent 2: "Riley-Python" - Plugin Implementer

  Mission: Migrate Python test fixtures from test harness to plugin

  Files:
  - CREATE: crates/languages/cb-lang-python/src/test_fixtures.rs
  - EDIT: crates/languages/cb-lang-python/src/lib.rs

  Dependencies: Agent 1 MUST complete first

  Success Criteria:
  - cargo check --package cb-lang-python passes
  - python_test_fixtures() returns valid data
  - Plugin implements new trait method

  Context Size: ~6K tokens

  ---
  Agent 3: "Taylor-Tests" - Test Framework Modernizer

  Mission: Update test framework to use plugin discovery

  Files:
  - CREATE: integration-tests/src/harness/plugin_discovery.rs
  - EDIT: integration-tests/src/harness/mod.rs
  - EDIT: integration-tests/tests/e2e_analysis_features.rs
  - EDIT: crates/cb-handlers/src/language_plugin_registry.rs

  Dependencies: Agents 1 & 2 MUST complete first

  Success Criteria:
  - cargo test --test e2e_analysis_features passes
  - Tests discover Python plugin automatically
  - No hard-coded language references

  Context Size: ~12K tokens

  ---
  🧪 TEST-DRIVEN VERIFICATION PROTOCOL

  Baseline (Already Captured)

  cargo test --test e2e_analysis_features
  Running: 7 tests
  Result: ok. 7 passed; 0 failed

  After Agent 3

  Expected:
  Running: 7 tests
  Result: ok. 7 passed; 0 failed (same behavior, different implementation)

  Decision Rule:
  - ✅ If 7 passed → SUCCESS, proceed to deprecation
  - ⚠️ If failures > 0 → Debug (max 2 rounds) then rollback

  ---
  📦 AGENT CONTEXT PACKAGES

  Morgan-API Context Package (~8K)

  MISSION: Create plugin test fixtures API

  FILES YOU WILL CREATE:
  1. crates/cb-plugin-api/src/test_fixtures.rs (NEW - ~150 lines)

  FILES YOU WILL EDIT:
  2. crates/cb-plugin-api/src/lib.rs (ADD ~30 lines)

  REFERENCE FILES TO READ:
  - integration-tests/src/harness/complexity_harness.rs (understand fixture pattern)
  - integration-tests/src/harness/refactoring_harness.rs (understand operations)

  KEY REQUIREMENTS:
  - No feature flags (always compiled)
  - Implement ComplexityFixture, RefactoringFixture, LanguageTestFixtures
  - Add test_fixtures() method to LanguagePlugin trait with default None
  - Use serde for serialization support
  - Follow existing cb-plugin-api code style

  VERIFICATION:
  - cargo check --package cb-plugin-api (MUST PASS)

  Riley-Python Context Package (~6K)

  MISSION: Migrate Python test fixtures from test harness to plugin

  FILES YOU WILL CREATE:
  1. crates/languages/cb-lang-python/src/test_fixtures.rs (NEW - ~120 lines)

  FILES YOU WILL EDIT:
  2. crates/languages/cb-lang-python/src/lib.rs (ADD 1 module, 1 method)

  SOURCE DATA (extract from):
  - integration-tests/src/harness/complexity_harness.rs (lines 66-127: Python scenarios)
  - integration-tests/src/harness/refactoring_harness.rs (lines 195-328: Python scenarios)

  KEY REQUIREMENTS:
  - Copy Python code fixtures EXACTLY as-is
  - Return fixtures via test_fixtures() method
  - Maintain expected complexity ranges
  - Follow cb-lang-python code style

  VERIFICATION:
  - cargo check --package cb-lang-python (MUST PASS)

  Taylor-Tests Context Package (~12K)

  MISSION: Update test framework to use plugin discovery

  FILES YOU WILL CREATE:
  1. integration-tests/src/harness/plugin_discovery.rs (NEW - ~50 lines)

  FILES YOU WILL EDIT:
  2. integration-tests/src/harness/mod.rs (ADD 2 lines)
  3. integration-tests/tests/e2e_analysis_features.rs (REFACTOR ~100 lines)
  4. crates/cb-handlers/src/language_plugin_registry.rs (ADD ~30 lines)

  REFERENCE FILES:
  - integration-tests/tests/e2e_analysis_features.rs (current implementation)

  KEY REQUIREMENTS:
  - Create discover_plugins_with_fixtures() function
  - Replace ComplexityScenarios::all() with plugin discovery
  - Replace Language::all() with plugin iteration
  - Add plugins_with_fixtures() to LanguagePluginRegistry
  - Maintain test behavior (7 tests, 7 passed)

  VERIFICATION:
  - cargo test --test e2e_analysis_features (MUST PASS: 7/7)

  ---
  🚨 RISK MITIGATION

  Risk 1: Agent 2 starts before Agent 1 completes

  Mitigation: Sequential deployment, explicit wait for Agent 1 completion

  Risk 2: Tests break after Agent 3

  Mitigation:
  - Baseline captured (7 passed)
  - Max 2 debug rounds
  - Rollback procedure: git reset --hard

  Risk 3: Context budget exceeded

  Mitigation:
  - Agent 1: 8K (safe)
  - Agent 2: 6K (safe)
  - Agent 3: 12K (safe)
  - Total peak: 12K << 50K limit

  Risk 4: Type mismatches between agents

  Mitigation:
  - Explicit type specifications in Agent 1
  - Agent 2 verification includes type checking
  - Agent 3 uses well-defined API

  ---
  ✅ PRE-FLIGHT CHECKLIST

  - 99.999% confidence achieved
  - All 10 files analyzed
  - Dependency chain mapped
  - Context budgets calculated
  - File conflicts eliminated (zero overlap)
  - Baseline tests captured (7 passed)
  - Rollback plan established
  - Agent instructions prepared

  ---
  🚀 DEPLOYMENT SEQUENCE

  Step 1: Deploy Morgan-API
     ↓ (Wait for completion + cargo check)
  Step 2: Deploy Riley-Python
     ↓ (Wait for completion + cargo check)
  Step 3: Deploy Taylor-Tests
     ↓ (Wait for completion + cargo test)
  Step 4: Verify baseline maintained (7/7 tests pass)
     ↓
  Step 5: [OPTIONAL] Run deprecation phase (Agents 4-5)

  Estimated Timeline: 20-30 minutes total

  Token Budget: 26K total (52% of single agent limit)

  ---
  📋 SUCCESS METRICS

  Agent 1 Success:
  - ✅ cargo check --package cb-plugin-api passes
  - ✅ test_fixtures.rs compiles
  - ✅ Trait method added without breaking existing plugins

  Agent 2 Success:
  - ✅ cargo check --package cb-lang-python passes
  - ✅ Python fixtures match existing test expectations
  - ✅ Plugin implements trait method

  Agent 3 Success:
  - ✅ cargo test --test e2e_analysis_features passes (7/7)
  - ✅ Tests use plugin discovery (no hard-coded Language enum)
  - ✅ Python fixtures auto-discovered

  Final Success:
  - ✅ All tests pass
  - ✅ Zero hard-coding of languages
  - ✅ Adding Java plugin requires ZERO test framework changes

  ---
  🎯 READY TO DEPLOY

  Next Action: Deploy Agent 1 (Morgan-API)

  Waiting for confirmation to proceed...
