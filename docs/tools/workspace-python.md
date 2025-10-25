# Workspace Tools - Python

Language-specific details for `workspace.create_package` with Python projects.

**See [workspace.md](workspace.md) for shared API documentation.**

## Language: Python

**Manifest file:** `pyproject.toml`
**Workspace configs:**
- PDM: `[tool.pdm.workspace]` in root `pyproject.toml`
- Poetry: `[tool.poetry.workspace]` in root `pyproject.toml`
- Hatch: `[tool.hatch.workspace]` in root `pyproject.toml`

## Template Structure

### Minimal Template
Creates baseline project structure:
- `pyproject.toml` - Package manifest (PEP 621 format)
- `src/<package>/__init__.py` (library) or `src/<package>/main.py` (binary)
- `README.md` - Basic project documentation
- `.gitignore` - Python-specific ignore patterns
- `tests/test_basic.py` - Starter pytest test

### Full Template
Minimal template + extras:
- `setup.py` - Legacy setuptools wrapper (backwards compatibility)

## Package Types

| Type | Entry Point | pyproject.toml Config |
|------|-------------|----------------------|
| `library` | `src/<pkg>/__init__.py` | None |
| `binary` | `src/<pkg>/main.py` | `[project.scripts]` console entry point |

## Generated pyproject.toml

**Library:**
```toml
[project]
name = "my-lib"
version = "0.1.0"
description = ""
requires-python = ">=3.8"
dependencies = []

[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[tool.setuptools.packages.find]
where = ["src"]

[tool.setuptools.package-dir]
"" = "src"
```

**Binary:**
```toml
[project]
name = "my-cli"
version = "0.1.0"
description = ""
requires-python = ">=3.8"
dependencies = []

[project.scripts]
my-cli = "my_cli:main"

[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[tool.setuptools.packages.find]
where = ["src"]

[tool.setuptools.package-dir]
"" = "src"
```

## Naming Conventions

- **Package path:** `packages/my-lib` (kebab-case or snake_case)
- **Package name:** `my-lib` (kebab-case for PyPI, converted to `my_lib` import)
- **Module name:** `my_lib` (underscores, used in `src/my_lib/`)
- **Script name:** `my-cli` (hyphens preserved for CLI command)

## Workspace Integration

When `addToWorkspace: true`:

**PDM:**
```toml
[tool.pdm.workspace]
members = ["packages/*", "packages/my-lib"]
```

**Poetry:**
```toml
[tool.poetry.workspace]
members = ["packages/*", "packages/my-lib"]
```

**Hatch:**
```toml
[tool.hatch.workspace]
members = ["packages/*", "packages/my-lib"]
```

**Cross-platform:** Paths normalized to forward slashes on Windows.

## Example Usage

```bash
# Create library package
mill tool workspace.create_package '{
  "packagePath": "packages/utils",
  "package_type": "library",
  "options": {
    "template": "minimal",
    "addToWorkspace": true
  }
}'

# Creates:
# - packages/utils/pyproject.toml
# - packages/utils/src/utils/__init__.py
# - packages/utils/README.md
# - packages/utils/.gitignore
# - packages/utils/tests/test_basic.py

# Create CLI with full template
mill tool workspace.create_package '{
  "packagePath": "packages/my-cli",
  "package_type": "binary",
  "options": {
    "template": "full"
  }
}'

# Creates minimal files + setup.py
```

## Notes

- Uses `src/` layout (best practice for avoiding import shadowing)
- `[tool.setuptools.packages.find]` required for src layout to work
- Script entry point format: `module:function` (e.g., `my_cli:main`)
- Package name allows hyphens, but module name uses underscores
- Python >=3.8 is minimum (for PEP 621 `pyproject.toml` support)
- `setup.py` in Full template provides backwards compatibility for older tools
- Workspace support varies by tool (PDM/Poetry/Hatch have different syntaxes)
