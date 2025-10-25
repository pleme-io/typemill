// analysis/mill-analysis-dead-code/src/utils.rs

/// Convert LSP SymbolKind number to human-readable string
pub fn lsp_kind_to_string(kind: u64) -> String {
    match kind {
        1 => "file",
        2 => "module",
        3 => "namespace",
        4 => "package",
        5 => "class",
        6 => "method",
        7 => "property",
        8 => "field",
        9 => "constructor",
        10 => "enum",
        11 => "interface",
        12 => "function",
        13 => "variable",
        14 => "constant",
        15 => "string",
        16 => "number",
        17 => "boolean",
        18 => "array",
        19 => "object",
        20 => "key",
        21 => "null",
        22 => "enum_member",
        23 => "struct",
        24 => "event",
        25 => "operator",
        26 => "type_parameter",
        _ => "unknown",
    }
    .to_string()
}

/// Convert string symbol kind name to LSP SymbolKind number
pub fn parse_symbol_kind(kind_str: &str) -> Option<u64> {
    match kind_str.to_lowercase().as_str() {
        "file" | "files" => Some(1),
        "module" | "modules" => Some(2),
        "namespace" | "namespaces" => Some(3),
        "package" | "packages" => Some(4),
        "class" | "classes" => Some(5),
        "method" | "methods" => Some(6),
        "property" | "properties" => Some(7),
        "field" | "fields" => Some(8),
        "constructor" | "constructors" => Some(9),
        "enum" | "enums" => Some(10),
        "interface" | "interfaces" => Some(11),
        "function" | "functions" => Some(12),
        "variable" | "variables" => Some(13),
        "constant" | "constants" => Some(14),
        "string" | "strings" => Some(15),
        "number" | "numbers" => Some(16),
        "boolean" | "booleans" => Some(17),
        "array" | "arrays" => Some(18),
        "object" | "objects" => Some(19),
        "key" | "keys" => Some(20),
        "null" => Some(21),
        "enum_member" | "enum_members" | "enummember" | "enummembers" => Some(22),
        "struct" | "structs" => Some(23),
        "event" | "events" => Some(24),
        "operator" | "operators" => Some(25),
        "type_parameter" | "type_parameters" | "typeparameter" | "typeparameters" => Some(26),
        _ => None,
    }
}

/// Check if a symbol appears to be exported based on heuristic analysis.
pub fn is_symbol_exported(file_path: &str, line: u32) -> bool {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let reader = BufReader::new(file);
    if let Some(Ok(line_content)) = reader.lines().nth(line as usize) {
        let line_lower = line_content.to_lowercase();
        return line_lower.contains("export ")
            || line_lower.contains("pub ")
            || line_lower.contains("public ");
    }

    false
}
