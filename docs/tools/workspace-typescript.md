# Workspace Tools - TypeScript

Language-specific details for `workspace` with `action: "create_package"` in TypeScript/npm projects.

**See [workspace.md](workspace.md) for shared API documentation.**

## Language: TypeScript

**Manifest file:** `package.json`
**Workspace configs:**
- npm/Yarn: `"workspaces"` field in root `package.json`
- pnpm: `pnpm-workspace.yaml`

## Template Structure

### Minimal Template
Creates baseline project structure:
- `package.json` - Package manifest with scripts
- `tsconfig.json` - TypeScript config (ES2022, strict mode, CommonJS)
- `src/index.ts` (library) or `src/main.ts` (binary)
- `README.md` - Basic project documentation
- `.gitignore` - Node/TypeScript ignore patterns
- `tests/index.test.ts` - Starter test file

### Full Template
Minimal template + extras:
- `.eslintrc.json` - ESLint config with TypeScript rules (ES2022)

## Package Types

| Type | Entry Point | package.json Field |
|------|-------------|-------------------|
| `library` | `src/index.ts` | `"main": "dist/index.js"` |
| `binary` | `src/main.ts` | `"bin": {"name": "dist/main.js"}` |

## Generated package.json

**Library:**
```json
{
  "name": "my-lib",
  "version": "0.1.0",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "test": "echo \"Error: no test specified\" && exit 1",
    "lint": "eslint src --ext .ts"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}
```
**Binary:**
```json
{
  "name": "my-cli",
  "version": "0.1.0",
  "bin": {
    "my-cli": "dist/main.js"
  },
  "scripts": {
    "build": "tsc",
    "start": "node dist/main.js",
    "test": "echo \"Error: no test specified\" && exit 1",
    "lint": "eslint src --ext .ts"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }
}
```
## tsconfig.json

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "commonjs",
    "lib": ["ES2022"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "declaration": true,
    "sourceMap": true
  }
}
```
## Workspace Integration

### npm/Yarn Workspaces
When `addToWorkspace: true`:
- Adds to `"workspaces"` array in root `package.json`
- Example: `"workspaces": ["packages/*", "packages/my-lib"]`

### pnpm Workspaces
When `addToWorkspace: true`:
- Adds to `packages:` array in `pnpm-workspace.yaml`
- Example:
  ```yaml
  packages:
    - 'packages/*'
    - 'packages/my-lib'
  ```

**Cross-platform:** Paths normalized to forward slashes on Windows.

## Example Usage

```bash
# Create library package
mill tool workspace '{
  "action": "create_package",
  "params": {
    "packagePath": "packages/utils",
    "packageType": "library"
  },
  "options": {
    "template": "minimal",
    "addToWorkspace": true
  }
}'

# Creates:
# - packages/utils/package.json
# - packages/utils/tsconfig.json
# - packages/utils/src/index.ts
# - packages/utils/README.md
# - packages/utils/.gitignore
# - packages/utils/tests/index.test.ts

# Create CLI with full template
mill tool workspace '{
  "action": "create_package",
  "params": {
    "packagePath": "packages/my-cli",
    "packageType": "binary"
  },
  "options": {
    "template": "full"
  }
}'

# Creates minimal files + .eslintrc.json
```
## Notes

- Package names support scopes: `@myorg/package-name`
- Binary name matches package name (strips scope if present)
- ES2022 enables: top-level await, private fields, class static blocks
- TypeScript `^5.0.0` is minimum for ES2022 support
- Lint script requires ESLint installation (not auto-installed)
- pnpm workspaces require both `pnpm-workspace.yaml` and root `package.json`
