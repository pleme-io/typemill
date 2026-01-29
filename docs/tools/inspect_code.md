# inspect_code

Aggregate code intelligence for a symbol or position in a single request.

## Purpose
- Definition, type info, references, implementations, call hierarchy, diagnostics
- Reduce tool selection overhead for agents

## Parameters
```json
{
  "filePath": "src/app.ts",
  "line": 9,
  "character": 5,
  "include": ["definition", "typeInfo"],
  "detailLevel": "basic",
  "limit": 50,
  "offset": 0
}
```

### Required (one of)
- `filePath` + `line` + `character`
- `filePath` + `symbolName`

### Notes
- All coordinates are **0-based**.
- `include` overrides `detailLevel`.

## Examples
```json
{ "name": "inspect_code", "arguments": { "filePath": "src/app.ts", "line": 9, "character": 5, "include": ["definition", "typeInfo"] } }
```

```json
{ "name": "inspect_code", "arguments": { "filePath": "src/app.ts", "symbolName": "Config", "include": ["references"] } }
```
