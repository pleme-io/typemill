# relocate

Move symbols, files, or directories with import updates.

## Parameters
```json
{
  "target": { "kind": "symbol", "filePath": "src/app.ts", "line": 9, "character": 5 },
  "destination": { "filePath": "src/utils.ts" },
  "options": { "dryRun": true }
}
```

## Notes
- All coordinates are **0-based**.
- Use `options.dryRun: false` to apply changes.
