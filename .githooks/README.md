# Git Hooks

Custom git hooks for Typemill development.

## Installation

To enable these hooks, run:

```bash
git config core.hooksPath .githooks
```

This tells git to use the hooks in this directory instead of `.git/hooks/`.

## Available Hooks

### pre-commit

Checks for missing external parser artifacts before allowing commits.

**What it checks:**
- Java parser JAR file (`mill-lang-java`)
- C# parser build output (`mill-lang-csharp`)
- TypeScript parser dependencies (`mill-lang-typescript`)
- Swift SourceKitten installation (optional warning)

**If artifacts are missing:**
- Commit is blocked
- Instructions are shown to run `make build-parsers`

**To bypass** (not recommended):
```bash
git commit --no-verify
```

## Why Git Hooks?

These hooks prevent common issues:
- Committing code that won't build on CI due to missing parser artifacts
- Forgetting to run `make build-parsers` after pulling changes
- Breaking other developers' builds

## Manual Installation

If you prefer to copy hooks to `.git/hooks/`:

```bash
cp .githooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

**Note:** This must be done by each developer and is not tracked by git.
Using `git config core.hooksPath .githooks` is the recommended approach.
