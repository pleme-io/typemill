# CLI Flag Support - Examples

This document shows example commands that will work once Agent 1 implements the flag_parser.

## Status

- ✅ **Agent 2 Complete**: Convention parsers, CLI structure, flag handling
- ⏳ **Agent 1 In Progress**: Generic flag_parser implementation
- 📝 **Integration Point**: `flag_parser::parse_flags_to_json()`

## Convention Parsers (Implemented)

### Target Convention
```bash
# Simple directory target
--target directory:../../crates/mill-client

# Simple file target
--target file:src/app.rs

# Symbol target with position
--target symbol:src/app.rs:10:5
```

### Source Convention
```bash
# Position in a file
--source src/app.rs:45:8
```

### Destination Convention
```bash
# Simple path
--destination src/utils.rs

# Path with position
--destination src/utils.rs:10:0
```

## Example Commands (Will work after Agent 1 completes)

### Rename Operations

```bash
# Rename a directory
codebuddy tool rename.plan \
  --target directory:crates/cb-types \
  --new-name crates/codebuddy-core/src/types

# Rename a file
codebuddy tool rename.plan \
  --target file:src/utils.rs \
  --new-name src/helpers.rs

# Rename a symbol
codebuddy tool rename.plan \
  --target symbol:src/app.rs:10:5 \
  --new-name newFunctionName
```

### Extract Operations

```bash
# Extract a function
codebuddy tool extract.plan \
  --source src/app.rs:45:8 \
  --name extractedFunction \
  --kind function

# Extract a variable
codebuddy tool extract.plan \
  --source src/app.rs:20:4 \
  --name extractedVar \
  --kind variable
```

### Move Operations

```bash
# Move code to a new location
codebuddy tool move.plan \
  --target symbol:src/app.rs:10:5 \
  --destination src/utils.rs:20:0
```

### Delete Operations

```bash
# Delete unused imports
codebuddy tool delete.plan \
  --target file:src/app.rs \
  --kind imports

# Delete a specific symbol
codebuddy tool delete.plan \
  --target symbol:src/app.rs:10:5
```

### Transform Operations

```bash
# Transform to async
codebuddy tool transform.plan \
  --target symbol:src/app.rs:10:5 \
  --kind to_async
```

### Scope Control

```bash
# Rename with custom scope
codebuddy tool rename.plan \
  --target directory:old-dir \
  --new-name new-dir \
  --scope code-only
```

## Flag vs JSON Comparison

### Using Flags (New)
```bash
codebuddy tool rename.plan \
  --target directory:../../crates/mill-client \
  --new-name crates/cb-core/src/client
```

### Using JSON (Original)
```bash
codebuddy tool rename.plan '{
  "target": {
    "kind": "directory",
    "path": "../../crates/mill-client"
  },
  "newName": "crates/cb-core/src/client"
}'
```

Both approaches work! Flags are ergonomic for simple cases, JSON for complex ones.

## How It Works

1. **CLI Parsing** (Implemented)
   - Clap parses flags into Option<String> values
   - Flags conflict with `--args` (mutual exclusion)

2. **Convention Parsing** (Implemented)
   - `parse_target_convention()` - Parses "kind:path" format
   - `parse_source_convention()` - Parses "path:line:char" format
   - `parse_destination_convention()` - Parses path with optional position

3. **Flag Parser** (Agent 1 - In Progress)
   - `parse_flags_to_json()` - Uses convention parsers
   - Builds complete JSON argument structure
   - Returns serde_json::Value ready for MCP

4. **MCP Dispatch** (Existing)
   - Arguments passed to tool handler
   - Normal MCP execution flow

## Testing

```bash
# Test convention parsers (working now)
cargo test --bin codebuddy cli::conventions

# Test full integration (after Agent 1 completes)
codebuddy tool rename.plan --target file:test.rs --new-name test2.rs
```

## Notes for Agent 1

The convention parsers are ready to use:

```rust
use crate::cli::conventions::{
    parse_target_convention,
    parse_source_convention,
    parse_destination_convention,
};

// Your flag_parser can call these:
pub fn parse_flags_to_json(
    tool_name: &str,
    flags: HashMap<String, String>,
) -> Result<Value, String> {
    let mut args = json!({});

    if let Some(target) = flags.get("target") {
        let parsed = parse_target_convention(target)
            .map_err(|e| e.to_string())?;
        args["target"] = parsed;
    }

    // ... handle other flags

    Ok(args)
}
```

## Edge Cases Handled

1. **Paths with spaces**: `directory:path with spaces/subdir` ✅
2. **Windows paths**: Must use JSON or relative paths (drive letters have colons)
3. **Missing fields**: Convention parsers return descriptive errors
4. **Invalid numbers**: Line/char must be valid u32 integers
5. **Format validation**: Each convention has specific format requirements
