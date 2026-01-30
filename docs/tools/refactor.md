# refactor

**Purpose:** Semantic refactoring operations (extract, inline, reorder, transform) with preview support.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| action | string | Yes | Operation to perform: `"extract"` or `"inline"` |
| kind | string | Yes | Target kind (e.g., `"function"`, `"variable"`) |
| source | object | Yes (extract) | Source range to extract `{ filePath, startLine, ... }` |
| target | object | Yes (inline) | Target symbol to inline `{ filePath, position }` |
| name | string | Yes (extract) | Name for the extracted symbol |
| options | object | No | Configuration options (including `dryRun`) |

**Actions & Kinds:**

- **extract**: `function`, `variable`, `module`, `interface`, `class`, `constant`, `type_alias`
- **inline**: `variable`, `function`, `constant`, `type_alias`

**Returns:**

Returns an `EditPlan` (preview) or `ExecutionResult` (applied).

```json
{
  "plan_type": "RefactorPlan", // or EditPlan
  "edits": [ ... ],
  "summary": { ... },
  "file_checksums": { ... }
}
```

**Example (Extract):**

```json
// MCP request
{
  "name": "refactor",
  "arguments": {
    "action": "extract",
    "kind": "function",
    "source": {
      "filePath": "src/app.ts",
      "startLine": 9,
      "startCharacter": 0,
      "endLine": 19,
      "endCharacter": 0
    },
    "name": "extractedFn",
    "options": {
      "dryRun": true,
      "visibility": "private"
    }
  }
}
```

**Example (Inline):**

```json
// MCP request
{
  "name": "refactor",
  "arguments": {
    "action": "inline",
    "kind": "variable",
    "target": {
      "filePath": "src/app.ts",
      "position": {
        "line": 15,
        "character": 10
      }
    },
    "options": {
      "dryRun": true,
      "inline_all": false
    }
  }
}
```

**Notes:**

- **Dry Run**: Defaults to `true`. Use `options: { "dryRun": false }` to apply changes.
- **Coordinates**: All line/character coordinates are **0-based**.
- **Visibility**: For extraction, `options.visibility` defaults to `"private"`.
- **Validation**: Ensures extracted code is valid and doesn't break references.
