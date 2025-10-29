# Actionable Suggestions

Analysis tools return **actionable suggestions** with exact commands to fix issues.

## What You Get

Every analysis finding includes:
- **Tool name** - Which MCP tool to run
- **Arguments** - Exact parameters ready to execute
- **Confidence** - 0.0-1.0 score
- **Reason** - Human explanation
- **Safety level** - Risk assessment

## Safety Levels

- `safe` - No logic changes, preserves semantics
- `requires_review` - Logic changes, needs verification
- `requires_validation` - Significant changes, thorough testing

## Example Output

```json
{
  "findings": [{
    "kind": "complexity_hotspot",
    "message": "Function too complex (cyclomatic: 25)",
    "suggestions": [{
      "tool": "extract",
      "arguments": {
        "kind": "function",
        "source": {
          "file_path": "src/app.rs",
          "range": {"start": {"line": 60, "character": 4}, "end": {"line": 75, "character": 5}},
          "name": "validateOrder"
        }
      },
      "confidence": 0.85,
      "reason": "Extract nested conditional to reduce complexity",
      "safety_level": "requires_review"
    }]
  }]
}
```text
## Workflow

1. **Analyze** - `mill analyze quality src/app.rs`
2. **Review** - Check suggestions and confidence scores
3. **Execute** - Run suggested commands (high confidence first)
4. **Verify** - Re-analyze to confirm improvements

## Configuration

Customize in `.typemill/analysis.toml`:

```toml
[suggestions]
min_confidence = 0.7
include_safety_levels = ["safe", "requires_review"]
max_per_finding = 3
generate_refactor_calls = true
```text
## Which Tools Generate Suggestions?

All analysis tools:
- `analyze.quality` - Extract functions, simplify code
- `analyze.dead_code` - Remove unused imports/symbols
- `analyze.documentation` - Add missing docs
- `analyze.tests` - Add test coverage

See [Analysis Tools](../tools/analysis.md) for complete documentation.