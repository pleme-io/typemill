# Proposal: Auto-Fix Markdown Quality Issues

## 🎯 Goal
Enable automatic fixing of safe, deterministic markdown quality issues found by `analyze.quality`.

## 📊 Current Findings

From workspace analysis of 330 markdown files:

### Highly Auto-Fixable (100% Safe) - **825 issues**
1. ✅ **trailing_whitespace** (469) - Just trim end of lines
2. ✅ **missing_code_language_tag** (183) - Add "text" as default
3. ✅ **malformed_heading** (172) - Add space after `#`
4. ✅ **reversed_link_syntax** (1) - Swap `()` and `[]`

### Moderately Auto-Fixable - **3,525 issues**
5. ⚠️ **bare_url** (3,445) - Wrap in `<>` (might affect prose URLs)
6. ⚠️ **table_column_inconsistency** (80) - Add empty cells

### Not Auto-Fixable - **1,466 issues**
- ❌ **empty_section** (863) - Needs content or design decision
- ❌ **heading_level_skip** (359) - Needs structure decision
- ❌ **multiple_h1_headings** (96) - Needs hierarchy decision
- ❌ **duplicate_heading** (148) - Needs renaming

## 💡 Proposed Solution

### Option A: Add `fix` Parameter to analyze.quality (RECOMMENDED)

Extend `analyze.quality` to support auto-fixing in a single pass.

**API:**
```json
{
  "kind": "markdown_formatting",
  "scope": {"type": "workspace", "path": "/workspace"},
  "options": {
    "fix": ["trailing_whitespace", "missing_code_language_tag", "malformed_heading"]
  }
}
```text
**CLI:**
```bash
# Preview what would be fixed (dry run - default)
mill tool analyze.quality \
  --kind markdown_formatting \
  --scope workspace \
  --path /workspace \
  --fix trailing_whitespace,missing_code_language_tag

# Apply fixes (--apply flag)
mill tool analyze.quality \
  --kind markdown_formatting \
  --scope workspace \
  --path /workspace \
  --fix trailing_whitespace,missing_code_language_tag \
  --apply
```text
**Output (dry run):**
```json
{
  "findings": [...],
  "fixes": {
    "would_fix": 652,
    "by_kind": {
      "trailing_whitespace": 469,
      "missing_code_language_tag": 183
    },
    "files_affected": [
      "/workspace/README.md",
      "/workspace/CLAUDE.md"
    ]
  },
  "summary": {...}
}
```text
**Output (applied):**
```json
{
  "findings": [...],
  "fixes": {
    "applied": 652,
    "by_kind": {
      "trailing_whitespace": 469,
      "missing_code_language_tag": 183
    },
    "files_modified": 45,
    "files_affected": [...]
  },
  "summary": {...}
}
```text
### Implementation Plan

#### Phase 1: Core Auto-Fix Infrastructure (3-4 hours)

**1. Add `MarkdownFixer` struct** (`quality.rs`):
```rust
struct MarkdownFixer {
    fixes_to_apply: HashSet<String>,
    dry_run: bool,
}

impl MarkdownFixer {
    fn fix_trailing_whitespace(&self, content: &str) -> String {
        content.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn fix_missing_code_lang(&self, content: &str) -> String {
        let re = Regex::new(r"(?m)^```\s*$").unwrap();
        re.replace_all(content, "```text").to_string()
    }

    fn fix_malformed_heading(&self, content: &str) -> String {
        let re = Regex::new(r"(?m)^(#{1,6})([^\s#])").unwrap();
        re.replace_all(content, "$1 $2").to_string()
    }

    fn fix_reversed_link(&self, content: &str) -> String {
        let re = Regex::new(r"\(([^)]+)\)\[([^\]]+)\]").unwrap();
        re.replace_all(content, "[$2]($1)").to_string()
    }

    fn apply_fixes(&self, content: &str, finding_kinds: &HashSet<String>) -> String {
        let mut result = content.to_string();

        if finding_kinds.contains("trailing_whitespace") {
            result = self.fix_trailing_whitespace(&result);
        }
        if finding_kinds.contains("missing_code_language_tag") {
            result = self.fix_missing_code_lang(&result);
        }
        if finding_kinds.contains("malformed_heading") {
            result = self.fix_malformed_heading(&result);
        }
        if finding_kinds.contains("reversed_link_syntax") {
            result = self.fix_reversed_link(&result);
        }

        result
    }
}
```text
**2. Extend QualityOptions**:
```rust
# [derive(Deserialize, Debug)]
struct QualityOptions {
    // ... existing fields ...

    /// List of issue kinds to auto-fix
    #[serde(default)]
    fix: Vec<String>,

    /// Apply fixes (false = dry run, true = write files)
    #[serde(default)]
    apply: bool,
}
```text
**3. Modify analyze_workspace_markdown()**:
```rust
async fn analyze_workspace_markdown(
    &self,
    context: &ToolHandlerContext,
    scope_param: &ScopeParam,
    category: &str,
    kind: &str,
    analysis_fn: MarkdownAnalysisFn,
    options: &QualityOptions,
) -> ServerResult<Value> {
    // ... existing analysis code ...

    // NEW: Auto-fix phase
    let mut fixes_by_file = HashMap::new();
    let mut files_modified = 0;

    if !options.fix.is_empty() {
        let fixer = MarkdownFixer {
            fixes_to_apply: options.fix.iter().cloned().collect(),
            dry_run: !options.apply,
        };

        for file_path in &md_files {
            // Read original content
            let content = context.file_service.read_file(file_path).await?;

            // Get findings for this file
            let file_findings: HashSet<_> = all_findings
                .iter()
                .filter(|f| f.location.file_path == file_path.to_string_lossy())
                .map(|f| f.kind.clone())
                .collect();

            // Apply fixes
            let fixed_content = fixer.apply_fixes(&content, &file_findings);

            if fixed_content != content {
                fixes_by_file.insert(
                    file_path.to_string_lossy().to_string(),
                    (content.len(), fixed_content.len())
                );

                if options.apply {
                    // Write fixed content back
                    context.file_service
                        .write_file(file_path, &fixed_content)
                        .await?;
                    files_modified += 1;
                }
            }
        }
    }

    // Add fix metadata to result
    if !options.fix.is_empty() {
        result["fixes"] = json!({
            "would_fix": fixes_by_file.len(),
            "files_affected": fixes_by_file.keys().collect::<Vec<_>>(),
            "applied": options.apply,
            "files_modified": files_modified,
        });
    }

    Ok(result)
}
```text
#### Phase 2: Safety & Validation (1-2 hours)

**1. Pre-fix validation:**
- Check files are writable
- Create backup hashes for rollback
- Validate markdown still parses after fix

**2. Post-fix verification:**
- Re-parse fixed markdown
- Verify no new errors introduced
- Confirm targeted issues are resolved

**3. Rollback mechanism:**
```rust
struct FixTransaction {
    original_contents: HashMap<PathBuf, String>,
    dry_run: bool,
}

impl FixTransaction {
    fn commit(&self) -> Result<()> { /* Write all changes */ }
    fn rollback(&self) -> Result<()> { /* Restore originals */ }
}
```text
#### Phase 3: CLI Flags (1 hour)

Add to `flag_parser.rs`:
```rust
--fix <KINDS>         Auto-fix specific issue kinds (comma-separated)
                      Examples: trailing_whitespace,missing_code_language_tag

--fix-safe            Auto-fix all safe issues (default set)

--apply               Apply fixes (default is dry-run preview)

--backup              Create .bak files before fixing
```text
#### Phase 4: Testing (2 hours)

**Unit tests:**
```rust
# [test]
fn test_fix_trailing_whitespace() {
    let input = "hello world  \n  test  \n";
    let expected = "hello world\n  test\n";
    assert_eq!(fix_trailing_whitespace(input), expected);
}

# [test]
fn test_fix_missing_code_lang() {
    let input = "```\ncode\n```";
    let expected = "```text\ncode\n```";
    assert_eq!(fix_missing_code_lang(input), expected);
}
```text
**Integration tests:**
```rust
# [tokio::test]
async fn test_workspace_autofix_dry_run() {
    let result = analyze_quality(..., options: QualityOptions {
        fix: vec!["trailing_whitespace".into()],
        apply: false,
    }).await?;

    assert_eq!(result["fixes"]["would_fix"], 10);
    assert_eq!(result["fixes"]["applied"], false);
}
```text
## 📐 Implementation Estimate

**Phase 1: Core auto-fix** (3-4 hours)
- MarkdownFixer implementation
- Integration with workspace analysis
- Basic fix application

**Phase 2: Safety & validation** (1-2 hours)
- Pre-fix validation
- Post-fix verification
- Rollback mechanism

**Phase 3: CLI flags** (1 hour)
- --fix, --apply flags
- --fix-safe preset

**Phase 4: Testing** (2 hours)
- Unit tests for each fixer
- Integration tests

**Total: 7-9 hours**

## 🧪 Usage Examples

### Example 1: Preview Safe Fixes
```bash
mill tool analyze.quality \
  --kind markdown_formatting \
  --scope workspace \
  --path /workspace \
  --fix-safe
```text
**Output:**
```text
Would fix 652 issues across 45 files:
  - trailing_whitespace: 469
  - missing_code_language_tag: 183

Run with --apply to make changes.
```text
### Example 2: Apply Specific Fixes
```bash
mill tool analyze.quality \
  --kind markdown_formatting \
  --scope workspace \
  --path /workspace \
  --fix trailing_whitespace \
  --apply
```text
**Output:**
```text
Fixed 469 issues across 32 files:
  ✓ README.md: 15 fixes
  ✓ CLAUDE.md: 23 fixes
  ✓ contributing.md: 8 fixes
  ...
```text
### Example 3: Fix + Verify
```bash
# Fix issues
mill tool analyze.quality \
  --kind markdown_formatting \
  --scope workspace \
  --path /workspace \
  --fix-safe \
  --apply

# Re-analyze to confirm
mill tool analyze.quality \
  --kind markdown_formatting \
  --scope workspace \
  --path /workspace
```text
## ✅ Success Criteria

1. **Dry run works** - Preview without modifying files
2. **Safe fixes only** - No breaking changes to markdown
3. **Atomic operations** - All fixes succeed or rollback
4. **Fast execution** - Fix 100+ files in <10 seconds
5. **Verification** - Re-analysis confirms fixes applied

## 🚀 Alternative: Quick Win (30 minutes)

If we want **immediate value** without full infrastructure:

**Create `mill fix-markdown` helper command:**
```bash
# !/bin/bash
# Simple auto-fixer for common issues

for file in "$@"; do
  # Fix trailing whitespace
  sed -i 's/[[:space:]]*$//' "$file"

  # Fix missing code language tags
  sed -i 's/^```$/```text/g' "$file"

  # Fix malformed headings (add space after #)
  sed -i 's/^\(#\{1,6\}\)\([^[:space:]#]\)/\1 \2/' "$file"
done
```text
But this doesn't leverage our analysis infrastructure and isn't as robust.

## 🎯 Recommendation

**Go with Option A** - Full auto-fix integration

**Why:**
1. ✅ Leverages existing finding detection
2. ✅ Safe by default (dry-run mode)
3. ✅ Atomic operations with rollback
4. ✅ Consistent with mill tool philosophy
5. ✅ Can fix **825 issues** safely across workspace

**Next Step:** Get approval and implement Phase 1 (core functionality)