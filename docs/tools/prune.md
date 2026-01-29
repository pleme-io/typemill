# prune

Delete symbols, files, or directories with cleanup.

## Parameters
```json
{
  "target": { "kind": "symbol", "filePath": "src/app.ts", "line": 9, "character": 5 },
  "options": { "dryRun": true, "cleanupImports": true, "force": false }
}
```

## Notes
- All coordinates are **0-based**.
- Use `options.dryRun: false` to apply changes.
