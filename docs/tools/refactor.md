# refactor

Semantic refactoring operations (extract, inline, reorder, transform).

## Parameters (extract example)
```json
{
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
  "options": { "dryRun": true }
}
```

## Notes
- All coordinates are **0-based**.
- Use `options.dryRun: false` to apply changes.
