# workspace.find_replace Examples

Real-world usage examples for workspace-wide find and replace operations.

## Table of Contents

- [Basic Examples](#basic-examples)
  - [Simple Literal Replacement](#simple-literal-replacement)
  - [Whole Word Matching](#whole-word-matching)
  - [Case-Preserving Replacement](#case-preserving-replacement)
- [Regex Examples](#regex-examples)
  - [Environment Variable Prefix Change](#environment-variable-prefix-change)
  - [Path Updates](#path-updates)
  - [Named Capture Groups](#named-capture-groups)
  - [Version Number Updates](#version-number-updates)
- [Scoped Replacements](#scoped-replacements)
  - [Rust Files Only](#rust-files-only)
  - [Documentation Updates](#documentation-updates)
  - [Configuration Files](#configuration-files)
- [Project Refactoring Scenarios](#project-refactoring-scenarios)
  - [Project Rename](#project-rename)
  - [API Endpoint Migration](#api-endpoint-migration)
  - [Dependency Rename](#dependency-rename)
  - [Error Type Consolidation](#error-type-consolidation)
- [Advanced Patterns](#advanced-patterns)
  - [Multi-Stage Replacement](#multi-stage-replacement)
  - [Conditional Replacement](#conditional-replacement)
  - [Function Signature Updates](#function-signature-updates)
- [Common Issues and Solutions](#common-issues-and-solutions)

---

## Basic Examples

### Simple Literal Replacement

Replace all occurrences of a simple string:

```bash
# Preview replacement
codebuddy tool workspace.find_replace '{
  "pattern": "username",
  "replacement": "user_id"
}'

# Execute replacement
codebuddy tool workspace.find_replace '{
  "pattern": "username",
  "replacement": "user_id",
  "dryRun": false
}'
```

**Before:**
```rust
fn authenticate(username: &str) -> Result<User> {
    let user = db.find_by_username(username)?;
    Ok(user)
}
```

**After:**
```rust
fn authenticate(user_id: &str) -> Result<User> {
    let user = db.find_by_user_id(user_id)?;
    Ok(user)
}
```

### Whole Word Matching

Avoid partial matches by using whole word mode:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "user",
  "replacement": "account",
  "wholeWord": true
}'
```

**Before:**
```rust
let user = get_user();        // Matches
let username = user.name;     // Does NOT match "user" in "username"
let user_info = load_info();  // Does NOT match "user" in "user_info"
```

**After:**
```rust
let account = get_account();  // Changed
let username = account.name;  // "username" unchanged
let user_info = load_info();  // "user_info" unchanged
```

### Case-Preserving Replacement

Automatically preserve case styles across different naming conventions:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "user_name",
  "replacement": "account_id",
  "preserveCase": true
}'
```

**Before:**
```rust
// snake_case
let user_name = "alice";
const DEFAULT_USER_NAME: &str = "guest";

// camelCase
let userName = request.userName;

// PascalCase
struct UserName {
    value: String
}
```

**After:**
```rust
// snake_case preserved
let account_id = "alice";
const DEFAULT_ACCOUNT_ID: &str = "guest";

// camelCase preserved
let accountId = request.accountId;

// PascalCase preserved
struct AccountId {
    value: String
}
```

---

## Regex Examples

### Environment Variable Prefix Change

Change environment variable prefix across entire codebase:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "CODEBUDDY_([A-Z_]+)",
  "replacement": "TYPEMILL_$1",
  "mode": "regex",
  "scope": {
    "includePatterns": ["**/*.rs", "**/*.toml", "**/*.md", "**/*.sh"]
  }
}'
```

**Before:**
```rust
// src/config.rs
let log_level = env::var("CODEBUDDY_LOG_LEVEL")?;
let debug = env::var("CODEBUDDY_DEBUG_MODE").is_ok();
let cache = env::var("CODEBUDDY_ENABLE_CACHE")?;
```

**After:**
```rust
// src/config.rs
let log_level = env::var("TYPEMILL_LOG_LEVEL")?;
let debug = env::var("TYPEMILL_DEBUG_MODE").is_ok();
let cache = env::var("TYPEMILL_ENABLE_CACHE")?;
```

### Path Updates

Update configuration directory paths:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "\\.codebuddy/([a-z_]+\\.toml)",
  "replacement": ".typemill/$1",
  "mode": "regex"
}'
```

**Before:**
```rust
const CONFIG_PATH: &str = ".codebuddy/config.toml";
const ANALYSIS_CONFIG: &str = ".codebuddy/analysis.toml";
```

**After:**
```rust
const CONFIG_PATH: &str = ".typemill/config.toml";
const ANALYSIS_CONFIG: &str = ".typemill/analysis.toml";
```

### Named Capture Groups

Use named capture groups for clarity:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "(?P<module>\\w+)::(?P<function>\\w+)\\(\\)",
  "replacement": "${function}_from_${module}()",
  "mode": "regex",
  "scope": {
    "includePatterns": ["**/*.rs"]
  }
}'
```

**Before:**
```rust
let formatted = utils::format();
let parsed = parser::parse();
let validated = validator::validate();
```

**After:**
```rust
let formatted = format_from_utils();
let parsed = parse_from_parser();
let validated = validate_from_validator();
```

### Version Number Updates

Update version numbers in manifests:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "version = \"0\\.1\\.([0-9]+)\"",
  "replacement": "version = \"0.2.$1\"",
  "mode": "regex",
  "scope": {
    "includePatterns": ["**/Cargo.toml"]
  }
}'
```

**Before:**
```toml
[package]
name = "my-crate"
version = "0.1.5"

[dependencies]
other-crate = { version = "0.1.2", path = "../other" }
```

**After:**
```toml
[package]
name = "my-crate"
version = "0.2.5"

[dependencies]
other-crate = { version = "0.2.2", path = "../other" }
```

---

## Scoped Replacements

### Rust Files Only

Target only Rust source files:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "OldType",
  "replacement": "NewType",
  "scope": {
    "includePatterns": ["**/*.rs"],
    "excludePatterns": ["**/target/**", "**/examples/**"]
  }
}'
```

### Documentation Updates

Update CLI command references in documentation:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "codebuddy",
  "replacement": "typemill",
  "scope": {
    "includePatterns": ["**/*.md", "**/README*"]
  }
}'
```

**Before (README.md):**
```markdown
## Installation

Install codebuddy with cargo:

\`\`\`bash
cargo install codebuddy
\`\`\`

Run codebuddy server:

\`\`\`bash
codebuddy serve
\`\`\`
```

**After (README.md):**
```markdown
## Installation

Install typemill with cargo:

\`\`\`bash
cargo install typemill
\`\`\`

Run typemill server:

\`\`\`bash
typemill serve
\`\`\`
```

### Configuration Files

Update only TOML and YAML configuration files:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "old_setting",
  "replacement": "new_setting",
  "scope": {
    "includePatterns": ["**/*.toml", "**/*.yaml", "**/*.yml"]
  }
}'
```

---

## Project Refactoring Scenarios

### Project Rename

Complete project rename from "codebuddy" to "typemill":

```bash
# Step 1: Preview all changes
codebuddy tool workspace.find_replace '{
  "pattern": "codebuddy",
  "replacement": "typemill",
  "preserveCase": true,
  "scope": {
    "includePatterns": ["**/*.rs", "**/*.toml", "**/*.md", "**/*.json"]
  }
}'

# Step 2: Review the EditPlan carefully

# Step 3: Execute the replacement
codebuddy tool workspace.find_replace '{
  "pattern": "codebuddy",
  "replacement": "typemill",
  "preserveCase": true,
  "scope": {
    "includePatterns": ["**/*.rs", "**/*.toml", "**/*.md", "**/*.json"]
  },
  "dryRun": false
}'
```

**Changes applied:**
- `codebuddy` → `typemill` (lowercase)
- `Codebuddy` → `Typemill` (title case)
- `CODEBUDDY` → `TYPEMILL` (uppercase)
- `CodeBuddy` → `TypeMill` (PascalCase)

### API Endpoint Migration

Update API endpoint paths:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "/api/v1/([a-z_]+)",
  "replacement": "/api/v2/$1",
  "mode": "regex",
  "scope": {
    "includePatterns": ["**/*.rs", "**/*.ts", "**/*.md"]
  }
}'
```

**Before:**
```rust
const USERS_ENDPOINT: &str = "/api/v1/users";
const POSTS_ENDPOINT: &str = "/api/v1/posts";
```

**After:**
```rust
const USERS_ENDPOINT: &str = "/api/v2/users";
const POSTS_ENDPOINT: &str = "/api/v2/posts";
```

### Dependency Rename

Rename dependency imports after upstream package rename:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "use old_crate::",
  "replacement": "use new_crate::",
  "scope": {
    "includePatterns": ["**/*.rs"]
  }
}'
```

**Before:**
```rust
use old_crate::Parser;
use old_crate::config::Config;
use old_crate::utils::{helper, formatter};
```

**After:**
```rust
use new_crate::Parser;
use new_crate::config::Config;
use new_crate::utils::{helper, formatter};
```

### Error Type Consolidation

Consolidate multiple error types into one:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "(FileError|ParseError|ValidationError)",
  "replacement": "AppError",
  "mode": "regex",
  "scope": {
    "includePatterns": ["**/*.rs"],
    "excludePatterns": ["**/tests/**"]
  }
}'
```

**Before:**
```rust
fn read_config() -> Result<Config, FileError> { ... }
fn parse_data() -> Result<Data, ParseError> { ... }
fn validate_input() -> Result<(), ValidationError> { ... }
```

**After:**
```rust
fn read_config() -> Result<Config, AppError> { ... }
fn parse_data() -> Result<Data, AppError> { ... }
fn validate_input() -> Result<(), AppError> { ... }
```

---

## Advanced Patterns

### Multi-Stage Replacement

For complex refactorings, perform staged replacements:

```bash
# Stage 1: Rename the core type
codebuddy tool workspace.find_replace '{
  "pattern": "OldServiceImpl",
  "replacement": "NewServiceImpl",
  "dryRun": false
}'

# Stage 2: Update trait references
codebuddy tool workspace.find_replace '{
  "pattern": "OldService",
  "replacement": "NewService",
  "wholeWord": true,
  "dryRun": false
}'

# Stage 3: Update module paths
codebuddy tool workspace.find_replace '{
  "pattern": "use services::old_service",
  "replacement": "use services::new_service",
  "dryRun": false
}'
```

### Conditional Replacement

Replace only in specific contexts using regex lookahead/lookbehind:

```bash
# Replace "config" only when it appears as a function parameter
codebuddy tool workspace.find_replace '{
  "pattern": "\\(config: ",
  "replacement": "(settings: ",
  "mode": "regex",
  "scope": {
    "includePatterns": ["**/*.rs"]
  }
}'
```

**Before:**
```rust
fn init(config: Config) -> Result<App> {
    let config_path = config.path();  // Not changed
    load_app(config)
}
```

**After:**
```rust
fn init(settings: Config) -> Result<App> {
    let config_path = settings.path();  // Not changed (not a parameter)
    load_app(settings)
}
```

### Function Signature Updates

Update function signatures with capture groups:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "fn (\\w+)\\(\\&self\\) -> (\\w+)",
  "replacement": "fn $1(&self) -> Result<$2>",
  "mode": "regex",
  "scope": {
    "includePatterns": ["src/**/*.rs"]
  }
}'
```

**Before:**
```rust
fn get_name(&self) -> String { ... }
fn get_id(&self) -> u64 { ... }
```

**After:**
```rust
fn get_name(&self) -> Result<String> { ... }
fn get_id(&self) -> Result<u64> { ... }
```

---

## Common Issues and Solutions

### Issue: Too Many Matches

**Problem:** Replacement finds thousands of matches, making preview difficult.

**Solution:** Use scoped patterns to narrow down the search:

```bash
# Instead of:
codebuddy tool workspace.find_replace '{
  "pattern": "data"
}'

# Use:
codebuddy tool workspace.find_replace '{
  "pattern": "data",
  "wholeWord": true,
  "scope": {
    "includePatterns": ["src/models/**/*.rs"]
  }
}'
```

### Issue: Regex Backslash Escaping

**Problem:** Regex patterns with backslashes not working.

**Solution:** Double-escape backslashes in JSON strings:

```bash
# Wrong:
"pattern": "\w+"

# Correct:
"pattern": "\\w+"

# Also correct (raw string in some contexts):
"pattern": "\\\\w+"
```

### Issue: Case Preservation Not Working

**Problem:** Case preservation doesn't work with regex mode.

**Solution:** Case preservation only works in literal mode. Use literal mode:

```bash
# Won't preserve case:
codebuddy tool workspace.find_replace '{
  "pattern": "user.*",
  "replacement": "account",
  "mode": "regex",
  "preserveCase": true
}'

# Will preserve case:
codebuddy tool workspace.find_replace '{
  "pattern": "user_name",
  "replacement": "account_id",
  "mode": "literal",
  "preserveCase": true
}'
```

### Issue: Capture Groups Not Expanding

**Problem:** `$1`, `$2` appearing literally in replacement text.

**Solution:** Ensure you're using `mode: "regex"`:

```bash
# Won't expand $1:
codebuddy tool workspace.find_replace '{
  "pattern": "CODEBUDDY_([A-Z_]+)",
  "replacement": "TYPEMILL_$1"
}' # mode defaults to "literal"

# Will expand $1:
codebuddy tool workspace.find_replace '{
  "pattern": "CODEBUDDY_([A-Z_]+)",
  "replacement": "TYPEMILL_$1",
  "mode": "regex"
}'
```

### Issue: Unintended Replacements in Generated Files

**Problem:** Replacement modifies files in `target/` or `node_modules/`.

**Solution:** These are excluded by default, but verify exclude patterns:

```bash
codebuddy tool workspace.find_replace '{
  "pattern": "something",
  "replacement": "other",
  "scope": {
    "excludePatterns": [
      "**/target/**",
      "**/node_modules/**",
      "**/.git/**",
      "**/build/**",
      "**/dist/**"
    ]
  }
}'
```

### Issue: Preview Shows Wrong Line Numbers

**Problem:** EditPlan line numbers don't match current file state.

**Solution:** Ensure files are saved and not modified externally during operation. The tool reads files fresh each time.

### Issue: Performance Slow on Large Codebase

**Problem:** Replacement takes a long time on large projects.

**Solution:** Use more specific include patterns:

```bash
# Slow - scans entire workspace:
codebuddy tool workspace.find_replace '{
  "pattern": "old_name"
}'

# Fast - only scans specific directories:
codebuddy tool workspace.find_replace '{
  "pattern": "old_name",
  "scope": {
    "includePatterns": ["crates/my-crate/**/*.rs"]
  }
}'
```

---

## Best Practices Summary

1. **Always preview first** - Use default `dry_run: true` to review changes
2. **Use whole_word for identifiers** - Avoid partial matches in variable names
3. **Scope narrowly** - Use include/exclude patterns to target specific files
4. **Test regex on small scope** - Validate pattern on single directory first
5. **Staged replacements** - Break complex refactorings into multiple steps
6. **Backup before large changes** - Commit to git before 100+ file modifications
7. **Double-escape regex** - Remember to escape backslashes in JSON strings
8. **Case preservation limits** - Review changes with preserve_case for acronyms
9. **Check exclusions** - Verify default excludes cover your build artifacts
10. **Monitor performance** - Use include_patterns for large workspaces

---

**Related Documentation:**
- [workspace.md](../tools/workspace.md#workspacefind_replace) - Complete API reference
- [refactoring.md](../tools/refactoring.md) - Semantic refactoring tools
- [CLAUDE.md](../../CLAUDE.md) - AI assistant usage guide
