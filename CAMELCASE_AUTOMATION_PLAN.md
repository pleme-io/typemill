# camelCase Conversion - Automated Execution Plan

## Summary
Use `workspace.find_replace` tool to automate 90% of the conversion work.

---

## Phase 1: Test JSON Conversion (Highest ROI)
**Effort:** 30 minutes | **Automation:** 100% | **Risk:** 🟢 LOW

Convert 367+ test JSON instances across 30+ files.

### Execution Steps:

```bash
# 1. dry_run → dryRun (110 occurrences)
codebuddy tool workspace.find_replace '{
  "pattern": "\"dry_run\"",
  "replacement": "\"dryRun\"",
  "options": {
    "wholeWord": false,
    "scope": {
      "includePatterns": ["tests/**/*.rs"]
    },
    "dryRun": true
  }
}'

# 2. whole_word → wholeWord (18 occurrences)
codebuddy tool workspace.find_replace '{
  "pattern": "\"whole_word\"",
  "replacement": "\"wholeWord\"",
  "options": {
    "scope": {"includePatterns": ["tests/**/*.rs"]},
    "dryRun": true
  }
}'

# 3. preserve_case → preserveCase (14 occurrences)
# 4. new_name → newName (56 occurrences)
# 5. file_path → filePath (24 occurrences)
# ... (continue for all 82 fields)
```

**Validation:** Run `cargo nextest run --workspace` after each batch

---

## Phase 2: Documentation Updates (Easy Win)
**Effort:** 15 minutes | **Automation:** 100% | **Risk:** 🟢 LOW

Update 116 occurrences across 18 doc files.

### Files affected:
- `/workspace/docs/tools/workspace.md` (38 occurrences)
- `/workspace/docs/examples/find_replace_examples.md` (16 occurrences)
- `/workspace/CLAUDE.md` (3 occurrences)
- etc.

### Execution:
Same find/replace commands as Phase 1, but with scope:
```json
{
  "scope": {
    "includePatterns": ["**/*.md"]
  }
}
```

**Validation:** Review via git diff, no compilation needed

---

## Phase 3: Adding Serde Annotations (Regex Magic)
**Effort:** 45 minutes | **Automation:** 80% | **Risk:** 🟡 MEDIUM

Add `#[serde(rename_all = "camelCase")]` to 45 structs.

### Strategy: Multiple Targeted Passes

**Pass 1: Simple Params Structs (No existing serde attrs)**

```bash
# Find structs with ONLY #[derive] and no #[serde]
codebuddy tool workspace.find_replace '{
  "pattern": "(#\\[derive\\([^)]*Deserialize[^)]*\\)\\])\\n(pub struct \\w+Params \\{)",
  "replacement": "$1\\n#[serde(rename_all = \"camelCase\")]\\n$2",
  "options": {
    "pattern_type": "regex",
    "scope": {
      "includePatterns": ["crates/mill-handlers/src/handlers/**/*.rs"]
    },
    "dryRun": true
  }
}'
```

**Pass 2: Options Structs**
Same regex, but match `\w+Options` instead of `\w+Params`

**Pass 3: Config Structs**
Match `\w+Config` pattern

**Pass 4: Manual Review for Edge Cases**
- Structs with existing `#[serde(...)]` attributes
- Enums needing conversion
- Multi-line derives

### Validation After Each Pass:
```bash
cargo check --workspace
git diff  # Review changes before committing
```

---

## Phase 4: Enum Conversions (Manual)
**Effort:** 15 minutes | **Automation:** 0% | **Risk:** 🟢 LOW

Update 5 enums from snake_case to camelCase:
- `SearchMode`
- `ImportType`
- `ChangeCategory`
- etc.

**Why manual?** Only 5 items, requires understanding of enum usage patterns.

---

## Validation Strategy

### After Each Phase:
```bash
# 1. Compile check
cargo check --workspace

# 2. Run relevant tests
cargo nextest run --workspace <module>

# 3. Review diff
git diff --stat
git diff <specific-file>

# 4. Checkpoint
git add .
git commit -m "feat(phase-N): convert <description> to camelCase"
```

### Final Validation:
```bash
# Full test suite
cargo nextest run --workspace --all-features --status-level skip

# LSP tests
cargo nextest run --workspace --features lsp-tests

# Manual smoke test
codebuddy tool rename.plan '{"target": {"kind": "file", "path": "test.rs"}, "newName": "test2.rs", "options": {"dryRun": true}}'
```

---

## Rollback Strategy

Each phase is independently committable:
```bash
# Rollback last phase
git reset --hard HEAD^

# Rollback specific files
git checkout HEAD -- <file>

# Nuclear option
git checkout HEAD -- .
```

---

## Timeline Estimate

| Phase | Automation | Manual | Total |
|-------|-----------|--------|-------|
| Phase 1: Test JSON | 5 min | 25 min | 30 min |
| Phase 2: Docs | 5 min | 10 min | 15 min |
| Phase 3: Annotations | 15 min | 30 min | 45 min |
| Phase 4: Enums | 0 min | 15 min | 15 min |
| **Validation** | - | 30 min | 30 min |
| **TOTAL** | - | - | **2h 15min** |

---

## Risk Mitigation

### High Risk Areas:
1. **Regex annotations** - Could add duplicates or break formatting
   - **Mitigation:** Always use `dry_run: true` first, review diff

2. **Field name collisions** - Might convert unrelated "dryRun" strings
   - **Mitigation:** Use `whole_word: true` and scope filtering

3. **Breaking API contracts** - External consumers might expect snake_case
   - **Mitigation:** Confirmed zero external users

### Safety Checklist:
- ✅ Always run with `"dryRun": true` first
- ✅ Review full diff before applying
- ✅ Compile check after each phase
- ✅ Run tests after each phase
- ✅ Git commit after each phase (easy rollback)

---

## Automation vs Manual Decision Matrix

| Task | Automation Level | Justification |
|------|-----------------|---------------|
| Test JSON conversion | **100%** | Safe, testable, high volume (367+ instances) |
| Documentation updates | **100%** | Safe, no runtime impact, high volume (116 instances) |
| Simple struct annotations | **80%** | Regex-based, needs dry-run validation |
| Complex struct annotations | **Manual** | Edge cases, existing attrs, needs review |
| Enum conversions | **Manual** | Low volume (5 items), context-dependent |

---

## Expected Outcomes

**Before:**
- Mixed snake_case/camelCase (93% already camelCase)
- 50 failing tests due to inconsistency
- Confusing API for users

**After:**
- 100% camelCase consistency
- All tests passing
- Matches MCP/LSP protocol standards
- Clear API contracts

**Automation Savings:**
- Manual effort: ~6 hours without automation
- With automation: ~2.25 hours
- **Saved: ~4 hours (67% time reduction)**
