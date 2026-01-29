# rename_all

Rename symbols, files, or directories with reference updates.

## Parameters
```json
{
  "target": { "kind": "symbol", "filePath": "src/app.ts", "line": 9, "character": 5 },
  "newName": "NewName",
  "options": { "dryRun": true, "scope": "standard" }
}
```

## Notes
- All coordinates are **0-based**.
- Use `options.dryRun: false` to apply changes.

## Examples
```json
{ "name": "rename_all", "arguments": { "target": { "kind": "file", "filePath": "src/old.ts" }, "newName": "src/new.ts" } }
```
