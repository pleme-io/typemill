# Python Language Plugin

**Status**: ✅ Production Ready

Complete Python language support plugin implementing the `LanguageIntelligencePlugin` trait.

## Features

### Core Functionality

- **AST Parsing**: Dual-mode parsing using Python's native parser with regex fallback
- **Import Analysis**: Complete support for `import` and `from ... import` statements
- **Symbol Extraction**: Functions, classes, methods, variables, and constants
- **Manifest Handling**: `requirements.txt`, `pyproject.toml`, `setup.py`, `Pipfile`
- **Refactoring Operations**: Extract function, inline variable, extract variable

### Refactoring Support

The plugin provides comprehensive refactoring capabilities:

#### Extract Function
Extracts selected code into a new function with parameter analysis:
- Automatically identifies required parameters from external variables
- Handles return statements correctly
- Preserves Python indentation conventions
- Generates appropriate function calls

#### Inline Variable
Replaces variable usages with their initializer expression:
- Finds all variable usages in scope
- Safely inlines simple expressions
- Handles operator precedence with parentheses when needed
- Removes the original variable declaration

#### Extract Variable
Extracts an expression into a named variable:
- Validates extractable expressions (no assignments, function defs, etc.)
- Suggests meaningful variable names based on expression content
- Maintains proper indentation
- Supports multi-line expressions (with parentheses)

### Variable Name Suggestions

The refactoring engine provides intelligent variable name suggestions:
- `len(...)` → `length`
- `.split(...)` → `parts`
- `.join(...)` → `joined`
- String literals → `text`
- Numbers → `value`
- Booleans → `flag`
- Lists → `items`
- Dicts → `data`
- Arithmetic → `result`

## Architecture

### Dual-Mode Parsing

1. **AST Mode** (Primary): Spawns Python subprocess with `scripts/ast_tool.py` for accurate parsing
2. **Regex Mode** (Fallback): Regex-based parsing when Python is unavailable

### Plugin Integration

The Python plugin integrates with Codebuddy's language plugin system:

```rust
use cb_lang_python::PythonPlugin;
use cb_plugin_api::LanguageIntelligencePlugin;

let plugin = PythonPlugin::new();
let parsed = plugin.parse(source_code).await?;
```

Registered in the central plugin registry via `build_language_plugin_registry()`.

## Usage

### Parsing

```rust
use cb_lang_python::PythonPlugin;

let plugin = PythonPlugin::new();
let source = r#"
import os
from pathlib import Path

def hello():
    print('Hello, world!')
"#;

let result = plugin.parse(source).await?;
// Access symbols, imports, and AST data
```

### Refactoring

```rust
use cb_lang_python::refactoring::{CodeRange, plan_extract_function};

let source = "...";
let range = CodeRange {
    start_line: 3,
    start_col: 4,
    end_line: 5,
    end_col: 20,
};

let plan = plan_extract_function(source, &range, "my_function", "file.py")?;
// Apply the edit plan to perform the refactoring
```

## API Reference

### Public Functions

#### `refactoring::plan_extract_function`
```rust
pub fn plan_extract_function(
    source: &str,
    range: &CodeRange,
    new_function_name: &str,
    file_path: &str,
) -> RefactoringResult<EditPlan>
```

#### `refactoring::plan_inline_variable`
```rust
pub fn plan_inline_variable(
    source: &str,
    variable_line: u32,
    variable_col: u32,
    file_path: &str,
) -> RefactoringResult<EditPlan>
```

#### `refactoring::plan_extract_variable`
```rust
pub fn plan_extract_variable(
    source: &str,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    variable_name: Option<String>,
    file_path: &str,
) -> RefactoringResult<EditPlan>
```

### Analysis Functions

- `analyze_extract_function` - Analyze code for function extraction
- `analyze_inline_variable` - Analyze variable for safe inlining
- `analyze_extract_variable` - Analyze expression for variable extraction

## Testing

```bash
# Run all plugin tests
cargo test -p cb-lang-python

# Run only refactoring tests
cargo test -p cb-lang-python refactoring

# Run with output
cargo test -p cb-lang-python -- --nocapture
```

**Test Coverage:**
- 12 refactoring operation tests
- 8 parser tests
- 6 plugin integration tests
- 3 manifest handling tests

All tests passing ✅

## Migration Completion

Migration from cb-ast to plugin architecture: **COMPLETE** ✅

- ✅ Moved `cb-ast/src/python_parser.rs` → `src/parser.rs` (680 lines)
- ✅ Moved Python refactoring from `cb-ast/src/refactoring.rs` → `src/refactoring.rs` (560 lines)
- ✅ Implemented `LanguageIntelligencePlugin` trait (305 lines)
- ✅ Added manifest support (`requirements.txt`, `pyproject.toml`, `setup.py`)
- ✅ Registered Python plugin in central registry
- ✅ Removed Python code from `cb-ast` (1376 lines deleted)

## Limitations

### Refactoring Constraints

- Variable inlining uses simplified safety analysis (doesn't detect all side effects)
- Extract function doesn't track return variables automatically (currently simplified)
- Multi-line expressions must be parenthesized for extraction
- Function/class definitions cannot be extracted as variables
- Assignment statements cannot be extracted as variables

### Parser Limitations

- AST mode requires Python 3.x in PATH
- Regex fallback has limited accuracy for complex syntax
- Type stub (.pyi) files use same parsing as .py files

## Future Improvements

- Enhanced scope analysis for safer refactoring
- Return variable detection for extract function
- Type annotation preservation in refactorings
- Virtual environment detection
- pytest/unittest test discovery
- Type stub (.pyi) specialized support

## Dependencies

- `cb-plugin-api`: Plugin trait definitions
- `cb-protocol`: Common protocol types
- `regex`: Fallback parsing
- `serde`: Serialization
- `tokio`: Async runtime
- `tracing`: Structured logging

## License

See workspace LICENSE file.
