# Bug Report & Known Issues

This document tracks known bugs, limitations, and areas for improvement in Codebuddy.

## 🐛 Active Issues

**None** - All known issues resolved!

<!--
Template for reporting new issues:

### Issue Title
**Severity:** Low | Medium | High | Critical
**Affected Component:** [component name]
**First Observed:** [date]

**Problem Description:**
- Symptom 1
- Symptom 2

**Reproduction Steps:**
1. Step 1
2. Step 2

**Expected Behavior:**
[What should happen]

**Actual Behavior:**
[What actually happens]

**Workaround (if any):**
[Temporary solution]

**Environment:**
- OS: [Linux/macOS/Windows]
- Rust version: [version]
- Codebuddy version: [version]
-->

---

## 📋 Enhancement Requests

**None** - All planned enhancements implemented!

<!--
Template for enhancement requests:

### Enhancement Title
**Priority:** Low | Medium | High
**Effort Estimate:** [hours/days/weeks]

**Description:**
[What feature or improvement is needed]

**Use Case:**
[Why this is valuable]

**Proposed Implementation:**
- Approach 1
- Approach 2

**Files to Modify:**
- `path/to/file1.rs`
- `path/to/file2.rs`

**Acceptance Criteria:**
- [ ] Criterion 1
- [ ] Criterion 2
-->

---

## 📝 Best Practices

### Large Refactorings

1. **Always use dry_run first:**
   ```bash
   codebuddy tool rename_directory '{"old_path":"...","new_path":"...","dry_run":true}'
   ```

2. **For package renames:**
   ```bash
   # 1. Move files
   codebuddy tool rename_directory ...

   # 2. Update dependencies
   codebuddy tool update_dependency '{"manifest_path":"...","old_dep_name":"...","new_dep_name":"..."}'

   # 3. Fix imports
   cargo check --workspace 2>&1 | grep error

   # 4. Stage all changes
   git add -A  # Let git detect renames
   ```

3. **Validate continuously:**
   - Run `cargo check` after each major step
   - Run tests before committing
   - Check git diff to ensure renames are detected

### Reporting Issues

When reporting a bug or enhancement:
1. Use the templates above (in HTML comments)
2. Include reproduction steps for bugs
3. Add relevant logs or error messages
4. Tag with severity/priority
5. Reference related issues if applicable

---

## 📚 Historical Issues

See [CHANGELOG.md](CHANGELOG.md) for resolved issues and their solutions.

---

**Last Updated:** 2025-10-03
**Tool Count:** 44 MCP tools
**Test Coverage:** 244 library tests, 13 CLI integration tests
