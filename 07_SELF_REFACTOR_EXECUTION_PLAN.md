# Self-Refactoring Execution Plan: MCP API Cleanup Phase 1

**Using `rename_symbol` + `write_file` + `batch_execute`**

---

## 🎯 Major Goals

| Goal | Target | Tool(s) | Success Metric | Why Important |
|------|--------|---------|----------------|---------------|
| **Test LSP Integration** | Rename 3 Rust methods | `rename_symbol` | All references updated automatically | ⭐⭐⭐ Core capability |
| **Rename MCP Tools** | `apply_edits` → `execute_edits`, `batch_execute` → `execute_batch` | `rename_symbol` + `write_file` | Tool names updated everywhere | Verb-noun consistency |
| **Remove Tool** | Delete `web_fetch` completely | `write_file` | 54 lines removed, no references | Security improvement |
| **Internalize Tools** | Hide `get_completions`, `get_signature_help` | `write_file` | Not in `tools/list` output | API cleanup |
| **Batch Efficiency** | Update 4 docs at once | `batch_execute` | Single operation for all docs | Prove batch works |
| **Production Readiness** | Full test suite passes | All tools | `cargo test --workspace` ✅ | Ready for self-modification |

**Bottom Line:** Prove Codebuddy can refactor itself using only its own MCP tools. If this works, the tools are production-ready! 🚀

---

## 📊 Quick Stats

- **30 changes** across **12 files**
- **3 tools** used (`rename_symbol`, `write_file`, `batch_execute`)
- **~1-2 hours** estimated time
- **44 → 34 tools** (10 fewer public tools)

---

## 📊 Complete Change Table

| # | File | Change | Old | New | Tool | Why Interesting |
|---|------|--------|-----|-----|------|-----------------|
| **Symbol Renames** (⭐ Tests LSP integration) |
| 1 | `workflow_handler.rs` | Method name | `handle_apply_edits` | `handle_execute_edits` | `rename_symbol` | Cross-file ref tracking |
| 2 | `file_service/edit_plan.rs` | Method name | `apply_edits_with_coordination` | `execute_edits_with_coordination` | `rename_symbol` | Internal method rename |
| 3 | `file_service/edit_plan.rs` | Method name | `apply_edits_to_content` | `execute_edits_to_content` | `rename_symbol` | Private helper |
| **String Literals** (write_file required) |
| 4 | `advanced.rs` | Tool array | `&["apply_edits", "batch_execute"]` | `&["execute_edits", "execute_batch"]` | `write_file` | String literals |
| 5 | `advanced.rs` | Match arm | `"apply_edits" =>` | `"execute_edits" =>` | `write_file` | Pattern match |
| 6 | `advanced.rs` | Match arm | `"batch_execute" =>` | `"execute_batch" =>` | `write_file` | Pattern match |
| 7 | `advanced.rs` | Comment | `//! Handles: apply_edits...` | `//! Handles: execute_edits...` | `write_file` | Doc comment |
| 8 | `advanced.rs` | String literal | `"batch_execute".to_string()` | `"execute_batch".to_string()` | `write_file` | Runtime string |
| 9 | `advanced.rs` | Error message | `"Failed to parse batch_execute"` | `"Failed to parse execute_batch"` | `write_file` | Error msg |
| 10 | `workflow_handler.rs` | Tool array | `vec!["achieve_intent", "apply_edits"]` | `vec!["achieve_intent", "execute_edits"]` | `write_file` | String array |
| 11 | `workflow_handler.rs` | Match arm | `"apply_edits" =>` | `"execute_edits" =>` | `write_file` | Pattern match |
| 12 | `.codebuddy/workflows.json` | JSON field | `"apply_edits"` | `"execute_edits"` | `write_file` | Config file |
| **Create Files** |
| 13 | `internal_intelligence.rs` | New file | N/A | +49 lines | `write_file` | New handler |
| 14 | `tools/mod.rs` | Add module | N/A | `pub mod internal_intelligence;` | `write_file` | Registration |
| **Delete web_fetch** |
| 15 | `system_tools_plugin.rs` | Delete method | `handle_web_fetch` | -38 lines | `write_file` | Remove handler |
| 16 | `system_tools_plugin.rs` | Delete tool def | JSON tool def | -13 lines | `write_file` | Remove from list |
| 17 | `system_tools_plugin.rs` | Delete match arm | `"web_fetch" =>` | -3 lines | `write_file` | Remove dispatch |
| 18 | `system.rs` | Clean imports | Unused imports | -2 lines | `write_file` | Cleanup |
| **Mark Internal** |
| 19 | `navigation.rs` | Add trait impl | N/A | `fn is_internal() -> bool { true }` | `write_file` | Override trait |
| 20 | `plugin_dispatcher.rs` | Filter logic | N/A | Filter internal from list | `write_file` | Hide from MCP |
| **Update Tests** |
| 21 | `tool_registration_test.rs` | Test count | 44 tools | 34 tools | `write_file` | Update assertion |
| 22 | `tool_registration_test.rs` | Remove tools | List has 3 tools | Remove from array | `write_file` | Test data |
| 23 | `tool_registration_test.rs` | Add ignore | No ignore | `#[ignore = "..."]` | `write_file` | Track progress |
| 24 | `e2e_workflow_execution.rs` | Test assertions | Old tool names | New tool names | `write_file` | Update tests |
| 25 | `client.rs` (harness) | Fix references | Old names | New names | `write_file` | Test helper |
| 26 | `Cargo.toml` (cb-client) | Version | Hardcoded | Inherit from workspace | `write_file` | Fix version |
| **Documentation** (batch_execute) |
| 27-30 | Docs (batch) | Multiple files | Old tool names | New tool names | `batch_execute` | 4 docs at once |

**Total: 30 changes across 12 files**

---

## 🚀 Execution Commands

### Step 1: Symbol Renames (⭐ THE INTERESTING PART)

```bash
# Test with dry_run first!
./target/release/codebuddy tool rename_symbol \
  '{"file_path":"crates/cb-handlers/src/handlers/workflow_handler.rs",
    "symbol_name":"handle_apply_edits",
    "new_name":"handle_execute_edits",
    "symbol_kind":"method",
    "dry_run":true}'

# If looks good, execute
./target/release/codebuddy tool rename_symbol \
  '{"file_path":"crates/cb-handlers/src/handlers/workflow_handler.rs",
    "symbol_name":"handle_apply_edits",
    "new_name":"handle_execute_edits",
    "symbol_kind":"method"}'

# Repeat for other 2 symbols...
```

### Step 2-5: String Literals (write_file)

```bash
# For each file, use read_file → sed → write_file pattern
./target/release/codebuddy tool read_file \
  '{"file_path":"crates/cb-handlers/src/handlers/tools/advanced.rs"}' \
  | jq -r '.content' \
  | sed 's/"apply_edits"/"execute_edits"/g; s/"batch_execute"/"execute_batch"/g' \
  > /tmp/advanced.rs.new

./target/release/codebuddy tool write_file \
  "{\"file_path\":\"crates/cb-handlers/src/handlers/tools/advanced.rs\",
    \"content\":$(cat /tmp/advanced.rs.new | jq -Rs .)}"
```

### Step 6: Batch Documentation

```bash
./target/release/codebuddy tool batch_execute \
  '{"operations":[
    {"type":"write_file","path":"API_REFERENCE.md","content":"<NEW>"},
    {"type":"write_file","path":"CLAUDE.md","content":"<NEW>"},
    {"type":"write_file","path":"CONTRIBUTING.md","content":"<NEW>"},
    {"type":"write_file","path":"TOOLS_QUICK_REFERENCE.md","content":"<NEW>"}
  ]}'
```

---

## ✅ Success Criteria

1. **`rename_symbol` works:** ✅ 3 methods renamed, all refs updated
2. **`write_file` handles rest:** ✅ 24 changes applied
3. **`batch_execute` efficient:** ✅ 4 docs updated at once
4. **Tests pass:** ✅ `cargo test --workspace`
5. **Tool count correct:** ✅ 44 → 34 tools

---

## 📈 Why This Tests Production Readiness

| Feature | Test | Pass Criteria |
|---------|------|---------------|
| **LSP Integration** | `rename_symbol` on methods | All refs updated |
| **Cross-file tracking** | Method called from multiple files | No broken refs |
| **Pattern matching** | Match arms updated | Compiles correctly |
| **JSON handling** | Large file contents | No escaping errors |
| **Batch efficiency** | 4 docs at once | Single operation |
| **Dry-run mode** | Preview before apply | Accurate preview |

---

## 🔥 TL;DR for Execution

**3 tools, 30 changes, 12 files:**

1. **`rename_symbol`** (3 uses) - Tests LSP ⭐
2. **`write_file`** (24 uses) - Handles everything else
3. **`batch_execute`** (1 use) - Groups docs efficiently

**Time:** ~1-2 hours
**Value:** Proves MCP tools are production-ready for self-modification

**Key Learning:** Can Codebuddy refactor itself using only its own tools? Let's find out! 🚀
