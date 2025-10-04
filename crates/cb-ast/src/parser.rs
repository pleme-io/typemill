//! AST parsing functionality
use crate::error::AstError;
use crate::error::AstResult;
use cb_protocol::{
    ImportGraph, ImportGraphMetadata, ImportInfo, ImportType, NamedImport, SourceLocation,
};
use petgraph::graph::NodeIndex;
use petgraph::{Direction, Graph};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use swc_common::{sync::Lrc, FileName, FilePathMapping, SourceMap};
use swc_ecma_ast::{CallExpr, ExportDecl, Expr, ImportDecl, Lit, Str};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};
/// Build import graph for a source file
pub fn build_import_graph(source: &str, path: &Path) -> AstResult<ImportGraph> {
    let language = match path.extension().and_then(|ext| ext.to_str()) {
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("rs") => "rust",
        Some("go") => "go",
        _ => "unknown",
    };
    let imports = match language {
        "typescript" | "javascript" => match parse_js_ts_imports_swc(source, path) {
            Ok(swc_imports) => swc_imports,
            Err(_) => {
                tracing::debug!(
                    file_path = % path.display(),
                    "SWC parsing failed, falling back to regex"
                );
                parse_js_ts_imports_enhanced(source)?
            }
        },
        "python" => match crate::python_parser::parse_python_imports_ast(source) {
            Ok(ast_imports) => ast_imports,
            Err(_) => {
                tracing::debug!(
                    file_path = % path.display(),
                    "Python AST parsing failed, falling back to regex"
                );
                parse_python_imports(source)?
            }
        },
        "rust" => {
            // Rust import parsing is handled by cb-lang-rust plugin
            // Cannot be called here due to circular dependency (cb-lang-rust depends on cb-ast)
            // Use cb_lang_rust::parse_imports() directly when needed
            tracing::debug!("Rust import parsing should use cb-lang-rust plugin directly");
            Vec::new()
        },
        "go" => match parse_go_imports_ast(source) {
            Ok(ast_imports) => ast_imports,
            Err(_) => {
                tracing::debug!("Go AST parsing failed, falling back to regex");
                parse_go_imports(source)?
            }
        },
        _ => parse_imports_basic(source)?,
    };
    let external_dependencies = imports
        .iter()
        .filter_map(|imp| {
            if is_external_dependency(&imp.module_path) {
                Some(imp.module_path.clone())
            } else {
                None
            }
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    Ok(ImportGraph {
        source_file: path.to_string_lossy().to_string(),
        imports,
        importers: Vec::new(),
        metadata: ImportGraphMetadata {
            language: language.to_string(),
            parsed_at: chrono::Utc::now(),
            parser_version: "0.3.0-swc".to_string(),
            circular_dependencies: Vec::new(),
            external_dependencies,
        },
    })
}
/// Parse JavaScript/TypeScript imports using SWC AST
fn parse_js_ts_imports_swc(source: &str, path: &Path) -> AstResult<Vec<ImportInfo>> {
    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let file_name = Lrc::new(FileName::Real(path.to_path_buf()));
    let source_file = cm.new_source_file(file_name, source.to_string());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "tsx")
                .unwrap_or(false),
            decorators: false,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: true,
        }),
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|e| AstError::parse(format!("SWC parsing failed: {:?}", e)))?;
    let mut visitor = ImportVisitor::new();
    module.visit_with(&mut visitor);
    Ok(visitor.imports)
}
/// Visitor that extracts import information from SWC AST
struct ImportVisitor {
    imports: Vec<ImportInfo>,
    current_line: u32,
}
impl ImportVisitor {
    fn new() -> Self {
        Self {
            imports: Vec::new(),
            current_line: 0,
        }
    }
    fn extract_string_literal(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Lit(Lit::Str(Str { value, .. })) => Some(value.to_string()),
            _ => None,
        }
    }
}
impl Visit for ImportVisitor {
    fn visit_import_decl(&mut self, n: &ImportDecl) {
        let module_path = n.src.value.to_string();
        let type_only = n.type_only;
        let mut named_imports = Vec::new();
        let mut default_import = None;
        let mut namespace_import = None;
        for spec in &n.specifiers {
            match spec {
                swc_ecma_ast::ImportSpecifier::Named(named) => {
                    let import_name = match &named.imported {
                        Some(name) => match name {
                            swc_ecma_ast::ModuleExportName::Ident(ident) => ident.sym.to_string(),
                            swc_ecma_ast::ModuleExportName::Str(str_lit) => {
                                str_lit.value.to_string()
                            }
                        },
                        None => named.local.sym.to_string(),
                    };
                    let alias = if named.local.sym != import_name {
                        Some(named.local.sym.to_string())
                    } else {
                        None
                    };
                    named_imports.push(NamedImport {
                        name: import_name,
                        alias,
                        type_only: named.is_type_only,
                    });
                }
                swc_ecma_ast::ImportSpecifier::Default(default) => {
                    default_import = Some(default.local.sym.to_string());
                }
                swc_ecma_ast::ImportSpecifier::Namespace(namespace) => {
                    namespace_import = Some(namespace.local.sym.to_string());
                }
            }
        }
        self.imports.push(ImportInfo {
            module_path,
            import_type: if type_only {
                ImportType::TypeOnly
            } else {
                ImportType::EsModule
            },
            named_imports,
            default_import,
            namespace_import,
            type_only,
            location: SourceLocation {
                start_line: self.current_line,
                start_column: 0,
                end_line: self.current_line,
                end_column: 0,
            },
        });
    }
    fn visit_call_expr(&mut self, n: &CallExpr) {
        if let swc_ecma_ast::Callee::Expr(callee_expr) = &n.callee {
            if let Expr::Ident(ident) = &**callee_expr {
                if ident.sym == "import" && n.args.len() == 1 {
                    if let Some(module_path) = Self::extract_string_literal(&n.args[0].expr) {
                        self.imports.push(ImportInfo {
                            module_path,
                            import_type: ImportType::Dynamic,
                            named_imports: Vec::new(),
                            default_import: None,
                            namespace_import: None,
                            type_only: false,
                            location: SourceLocation {
                                start_line: self.current_line,
                                start_column: 0,
                                end_line: self.current_line,
                                end_column: 0,
                            },
                        });
                    }
                }
                if ident.sym == "require" && n.args.len() == 1 {
                    if let Some(module_path) = Self::extract_string_literal(&n.args[0].expr) {
                        self.imports.push(ImportInfo {
                            module_path,
                            import_type: ImportType::CommonJs,
                            named_imports: Vec::new(),
                            default_import: None,
                            namespace_import: None,
                            type_only: false,
                            location: SourceLocation {
                                start_line: self.current_line,
                                start_column: 0,
                                end_line: self.current_line,
                                end_column: 0,
                            },
                        });
                    }
                }
            }
        }
        n.visit_children_with(self);
    }
    fn visit_export_decl(&mut self, n: &ExportDecl) {
        n.visit_children_with(self);
    }
}
/// Parse JavaScript/TypeScript imports using enhanced regex patterns
pub fn parse_js_ts_imports_enhanced(source: &str) -> AstResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    let es_import_re = Regex::new(
            r#"^\s*import\s+(?:(type)\s+)?(?:(?:(\*)\s+as\s+(\w+))|(?:(\w+)(?:\s*,\s*\{([^}]+)\})?)|(?:\{([^}]+)\}))\s+from\s+['"]([^'"]+)['"]"#,
        )
        .expect("ES import regex pattern should be valid");
    let dynamic_import_re = Regex::new(r#"import\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
        .expect("Dynamic import regex pattern should be valid");
    let require_re = Regex::new(
        r#"(?:const|let|var)\s+(?:\{([^}]+)\}|(\w+))\s*=\s*require\s*\(\s*['"]([^'"]+)['"]\s*\)"#,
    )
    .expect("Require regex pattern should be valid");
    let direct_require_re = Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
        .expect("Direct require regex pattern should be valid");
    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.starts_with("//") || line.starts_with("/*") || line.is_empty() {
            continue;
        }
        if let Some(captures) = es_import_re.captures(line) {
            let type_only = captures.get(1).is_some();
            let is_namespace = captures.get(2).is_some();
            let namespace_name = captures.get(3).map(|m| m.as_str().to_string());
            let default_import = captures.get(4).map(|m| m.as_str().to_string());
            let mixed_named = captures.get(5).map(|m| m.as_str());
            let named_only = captures.get(6).map(|m| m.as_str());
            let module_path = captures
                .get(7)
                .expect("ES import regex should always capture module path at index 7")
                .as_str()
                .to_string();
            let named_imports = if let Some(named_str) = mixed_named.or(named_only) {
                parse_named_imports_enhanced(named_str)?
            } else {
                Vec::new()
            };
            imports.push(ImportInfo {
                module_path,
                import_type: if type_only {
                    ImportType::TypeOnly
                } else {
                    ImportType::EsModule
                },
                named_imports,
                default_import,
                namespace_import: if is_namespace { namespace_name } else { None },
                type_only,
                location: SourceLocation {
                    start_line: line_num as u32,
                    start_column: 0,
                    end_line: line_num as u32,
                    end_column: line.len() as u32,
                },
            });
        }
        for captures in dynamic_import_re.captures_iter(line) {
            let module_path = captures
                .get(1)
                .expect("Dynamic import regex should always capture module path at index 1")
                .as_str()
                .to_string();
            let full_match = captures
                .get(0)
                .expect("Regex match should always have capture group 0");
            imports.push(ImportInfo {
                module_path,
                import_type: ImportType::Dynamic,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num as u32,
                    start_column: full_match.start() as u32,
                    end_line: line_num as u32,
                    end_column: full_match.end() as u32,
                },
            });
        }
        if let Some(captures) = require_re.captures(line) {
            let module_path = captures
                .get(3)
                .expect("Require regex should always capture module path at index 3")
                .as_str()
                .to_string();
            let named_imports = if let Some(destructured) = captures.get(1) {
                parse_named_imports_enhanced(destructured.as_str())?
            } else {
                Vec::new()
            };
            let default_import = captures.get(2).map(|m| m.as_str().to_string());
            imports.push(ImportInfo {
                module_path,
                import_type: ImportType::CommonJs,
                named_imports,
                default_import,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num as u32,
                    start_column: 0,
                    end_line: line_num as u32,
                    end_column: line.len() as u32,
                },
            });
        } else if let Some(captures) = direct_require_re.captures(line) {
            let module_path = captures
                .get(1)
                .expect("Direct require regex should always capture module path at index 1")
                .as_str()
                .to_string();
            let full_match = captures
                .get(0)
                .expect("Regex match should always have capture group 0");
            imports.push(ImportInfo {
                module_path,
                import_type: ImportType::CommonJs,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num as u32,
                    start_column: full_match.start() as u32,
                    end_line: line_num as u32,
                    end_column: full_match.end() as u32,
                },
            });
        }
    }
    Ok(imports)
}
/// Parse named imports with enhanced regex support
fn parse_named_imports_enhanced(named_str: &str) -> AstResult<Vec<NamedImport>> {
    let mut named_imports = Vec::new();
    let import_re = Regex::new(r#"(?:(type)\s+)?(\w+)(?:\s+as\s+(\w+))?"#)
        .expect("Named import regex pattern should be valid");
    for captures in import_re.captures_iter(named_str) {
        let type_only = captures.get(1).is_some();
        let name = captures
            .get(2)
            .expect("Named import regex should always capture name at index 2")
            .as_str()
            .to_string();
        let alias = captures.get(3).map(|m| m.as_str().to_string());
        named_imports.push(NamedImport {
            name,
            alias,
            type_only,
        });
    }
    Ok(named_imports)
}
/// Basic import parsing (simplified for foundation)
fn parse_imports_basic(source: &str) -> AstResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.starts_with("import ") && line.contains(" from ") {
            if let Some(import_info) = parse_es_import(line, line_num as u32)? {
                imports.push(import_info);
            }
        } else if line.contains("require(") {
            if let Some(import_info) = parse_commonjs_require(line, line_num as u32)? {
                imports.push(import_info);
            }
        } else if line.contains("import(") {
            if let Some(import_info) = parse_dynamic_import(line, line_num as u32)? {
                imports.push(import_info);
            }
        }
    }
    Ok(imports)
}
/// Parse ES module import statement (simplified)
fn parse_es_import(line: &str, line_num: u32) -> AstResult<Option<ImportInfo>> {
    if let Some(from_pos) = line.find(" from ") {
        let import_part = &line[6..from_pos].trim();
        let module_part = &line[from_pos + 6..].trim();
        let module_path = module_part
            .trim_matches('"')
            .trim_matches('\'')
            .trim_end_matches(';');
        let type_only = line.contains("import type");
        let (default_import, named_imports, namespace_import) =
            parse_import_specifiers(import_part)?;
        return Ok(Some(ImportInfo {
            module_path: module_path.to_string(),
            import_type: if type_only {
                ImportType::TypeOnly
            } else {
                ImportType::EsModule
            },
            named_imports,
            default_import,
            namespace_import,
            type_only,
            location: SourceLocation {
                start_line: line_num,
                start_column: 0,
                end_line: line_num,
                end_column: line.len() as u32,
            },
        }));
    }
    Ok(None)
}
/// Parse CommonJS require (simplified)
fn parse_commonjs_require(line: &str, line_num: u32) -> AstResult<Option<ImportInfo>> {
    if let Some(require_start) = line.find("require(") {
        let require_part = &line[require_start + 8..];
        if let Some(end_paren) = require_part.find(')') {
            let module_path = &require_part[..end_paren]
                .trim_matches('"')
                .trim_matches('\'');
            return Ok(Some(ImportInfo {
                module_path: module_path.to_string(),
                import_type: ImportType::CommonJs,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: require_start as u32,
                    end_line: line_num,
                    end_column: (require_start + 8 + end_paren + 1) as u32,
                },
            }));
        }
    }
    Ok(None)
}
/// Parse dynamic import (simplified)
fn parse_dynamic_import(line: &str, line_num: u32) -> AstResult<Option<ImportInfo>> {
    if let Some(import_start) = line.find("import(") {
        let import_part = &line[import_start + 7..];
        if let Some(end_paren) = import_part.find(')') {
            let module_path = &import_part[..end_paren]
                .trim_matches('"')
                .trim_matches('\'');
            return Ok(Some(ImportInfo {
                module_path: module_path.to_string(),
                import_type: ImportType::Dynamic,
                named_imports: Vec::new(),
                default_import: None,
                namespace_import: None,
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: import_start as u32,
                    end_line: line_num,
                    end_column: (import_start + 7 + end_paren + 1) as u32,
                },
            }));
        }
    }
    Ok(None)
}
/// Parse import specifiers (simplified)
fn parse_import_specifiers(
    import_part: &str,
) -> AstResult<(Option<String>, Vec<NamedImport>, Option<String>)> {
    let import_part = import_part.trim();
    if let Some(stripped) = import_part.strip_prefix("* as ") {
        let namespace = stripped.trim().to_string();
        return Ok((None, Vec::new(), Some(namespace)));
    }
    if import_part.starts_with('{') && import_part.ends_with('}') {
        let inner = &import_part[1..import_part.len() - 1];
        let named_imports = parse_named_imports(inner)?;
        return Ok((None, named_imports, None));
    }
    if let Some(comma_pos) = import_part.find(',') {
        let default_part = import_part[..comma_pos].trim();
        let rest_part = import_part[comma_pos + 1..].trim();
        let default_import = if !default_part.is_empty() {
            Some(default_part.to_string())
        } else {
            None
        };
        let named_imports = if rest_part.starts_with('{') && rest_part.ends_with('}') {
            let inner = &rest_part[1..rest_part.len() - 1];
            parse_named_imports(inner)?
        } else {
            Vec::new()
        };
        return Ok((default_import, named_imports, None));
    }
    if !import_part.is_empty() && !import_part.starts_with('{') {
        return Ok((Some(import_part.to_string()), Vec::new(), None));
    }
    Ok((None, Vec::new(), None))
}
/// Parse named imports from braces content
fn parse_named_imports(inner: &str) -> AstResult<Vec<NamedImport>> {
    let mut named_imports = Vec::new();
    for item in inner.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let type_only = item.starts_with("type ");
        let item = if type_only { &item[5..] } else { item };
        if let Some(as_pos) = item.find(" as ") {
            let name = item[..as_pos].trim().to_string();
            let alias = item[as_pos + 4..].trim().to_string();
            named_imports.push(NamedImport {
                name,
                alias: Some(alias),
                type_only,
            });
        } else {
            named_imports.push(NamedImport {
                name: item.to_string(),
                alias: None,
                type_only,
            });
        }
    }
    Ok(named_imports)
}
/// Parse Python imports (basic implementation)
fn parse_python_imports(source: &str) -> AstResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.starts_with("import ") && !line.contains("from ") {
            let import_part = &line[7..];
            for module in import_part.split(',') {
                let module = module.trim();
                if let Some(as_pos) = module.find(" as ") {
                    let module_name = module[..as_pos].trim();
                    let alias = module[as_pos + 4..].trim();
                    imports.push(ImportInfo {
                        module_path: module_name.to_string(),
                        import_type: ImportType::EsModule,
                        named_imports: vec![NamedImport {
                            name: alias.to_string(),
                            alias: None,
                            type_only: false,
                        }],
                        default_import: None,
                        namespace_import: None,
                        type_only: false,
                        location: SourceLocation {
                            start_line: line_num as u32,
                            start_column: 0,
                            end_line: line_num as u32,
                            end_column: line.len() as u32,
                        },
                    });
                } else {
                    imports.push(ImportInfo {
                        module_path: module.to_string(),
                        import_type: ImportType::EsModule,
                        named_imports: Vec::new(),
                        default_import: None,
                        namespace_import: Some(module.to_string()),
                        type_only: false,
                        location: SourceLocation {
                            start_line: line_num as u32,
                            start_column: 0,
                            end_line: line_num as u32,
                            end_column: line.len() as u32,
                        },
                    });
                }
            }
        } else if line.starts_with("from ") && line.contains(" import ") {
            if let Some(import_pos) = line.find(" import ") {
                let module_part = &line[5..import_pos];
                let import_part = &line[import_pos + 8..];
                let named_imports = if import_part.trim() == "*" {
                    Vec::new()
                } else {
                    import_part
                        .split(',')
                        .map(|name| {
                            let name = name.trim();
                            if let Some(as_pos) = name.find(" as ") {
                                let original = name[..as_pos].trim();
                                let alias = name[as_pos + 4..].trim();
                                NamedImport {
                                    name: original.to_string(),
                                    alias: Some(alias.to_string()),
                                    type_only: false,
                                }
                            } else {
                                NamedImport {
                                    name: name.to_string(),
                                    alias: None,
                                    type_only: false,
                                }
                            }
                        })
                        .collect()
                };
                let namespace_import = if import_part.trim() == "*" {
                    Some(module_part.to_string())
                } else {
                    None
                };
                imports.push(ImportInfo {
                    module_path: module_part.to_string(),
                    import_type: ImportType::EsModule,
                    named_imports,
                    default_import: None,
                    namespace_import,
                    type_only: false,
                    location: SourceLocation {
                        start_line: line_num as u32,
                        start_column: 0,
                        end_line: line_num as u32,
                        end_column: line.len() as u32,
                    },
                });
            }
        }
    }
    Ok(imports)
}
/// Parse Rust imports using AST (syn crate)
///
/// This provides accurate parsing of complex Rust import statements including:
/// - Nested module paths: `use std::collections::HashMap;`
/// - Grouped imports: `use std::{sync::Arc, collections::HashMap};`
/// - Glob imports: `use module::*;`
/// - Aliased imports: `use std::collections::HashMap as Map;`
/// - Nested groups: `use std::{io::{self, Read}, collections::*};`
/// Parse Go imports using AST (go/parser via subprocess)
///
/// This provides accurate parsing of complex Go import statements including:
/// - Single imports: `import "fmt"`
/// - Grouped imports: `import ( "fmt"; "io" )`
/// - Aliased imports: `import f "fmt"`
/// - Dot imports: `import . "fmt"`
/// - Blank imports: `import _ "database/sql/driver"`
fn parse_go_imports_ast(source: &str) -> AstResult<Vec<ImportInfo>> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let ast_tool_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("ast_tool.go");
    if !ast_tool_path.exists() {
        return Err(AstError::analysis(format!(
            "Go AST tool not found at: {}",
            ast_tool_path.display()
        )));
    }
    let mut child = Command::new("go")
        .arg("run")
        .arg(&ast_tool_path)
        .arg("analyze-imports")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AstError::analysis(format!("Failed to spawn Go AST tool: {}", e)))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| AstError::analysis(format!("Failed to write to Go AST tool: {}", e)))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| AstError::analysis(format!("Failed to wait for Go AST tool: {}", e)))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AstError::analysis(format!(
            "Go AST tool failed: {}",
            stderr
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let imports: Vec<ImportInfo> = serde_json::from_str(&stdout)
        .map_err(|e| AstError::analysis(format!("Failed to parse Go AST tool output: {}", e)))?;
    Ok(imports)
}
/// Parse Go imports using regex (fallback implementation)
fn parse_go_imports(source: &str) -> AstResult<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("//") || line.starts_with("/*") || line.is_empty() {
            i += 1;
            continue;
        }
        if line.starts_with("import ") && line.contains('"') && !line.contains("(") {
            if let Some(import_info) = parse_go_single_import(line, i as u32)? {
                imports.push(import_info);
            }
            i += 1;
        } else if line.starts_with("import (") || line == "import (" {
            i += 1;
            while i < lines.len() {
                let block_line = lines[i].trim();
                if block_line == ")" || block_line.starts_with(")") {
                    i += 1;
                    break;
                }
                if block_line.contains('"') && !block_line.is_empty() {
                    if let Some(import_info) = parse_go_block_import(block_line, i as u32)? {
                        imports.push(import_info);
                    }
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    Ok(imports)
}
/// Parse a single Go import statement
fn parse_go_single_import(line: &str, line_num: u32) -> AstResult<Option<ImportInfo>> {
    let import_part = &line[6..];
    let import_part = import_part.trim();
    if let Some(start_quote) = import_part.find('"') {
        if let Some(end_quote) = import_part[start_quote + 1..].find('"') {
            let package_path = &import_part[start_quote + 1..start_quote + 1 + end_quote];
            let alias = if start_quote > 0 {
                let alias_part = import_part[..start_quote].trim();
                if alias_part == "." {
                    Some(".".to_string())
                } else if alias_part == "_" {
                    Some("_".to_string())
                } else if !alias_part.is_empty() {
                    Some(alias_part.to_string())
                } else {
                    None
                }
            } else {
                None
            };
            return Ok(Some(ImportInfo {
                module_path: package_path.to_string(),
                import_type: ImportType::EsModule,
                named_imports: Vec::new(),
                default_import: alias.clone(),
                namespace_import: if alias.is_some() {
                    None
                } else {
                    Some(
                        package_path
                            .split('/')
                            .next_back()
                            .unwrap_or(package_path)
                            .to_string(),
                    )
                },
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: 0,
                    end_line: line_num,
                    end_column: line.len() as u32,
                },
            }));
        }
    }
    Ok(None)
}
/// Parse Go import from within an import block
fn parse_go_block_import(line: &str, line_num: u32) -> AstResult<Option<ImportInfo>> {
    let line = line.trim();
    if let Some(start_quote) = line.find('"') {
        if let Some(end_quote) = line[start_quote + 1..].find('"') {
            let package_path = &line[start_quote + 1..start_quote + 1 + end_quote];
            let alias = if start_quote > 0 {
                let alias_part = line[..start_quote].trim();
                if alias_part == "." {
                    Some(".".to_string())
                } else if alias_part == "_" {
                    Some("_".to_string())
                } else if !alias_part.is_empty() {
                    Some(alias_part.to_string())
                } else {
                    None
                }
            } else {
                None
            };
            return Ok(Some(ImportInfo {
                module_path: package_path.to_string(),
                import_type: ImportType::EsModule,
                named_imports: Vec::new(),
                default_import: alias.clone(),
                namespace_import: if alias.is_some() {
                    None
                } else {
                    Some(
                        package_path
                            .split('/')
                            .next_back()
                            .unwrap_or(package_path)
                            .to_string(),
                    )
                },
                type_only: false,
                location: SourceLocation {
                    start_line: line_num,
                    start_column: 0,
                    end_line: line_num,
                    end_column: line.len() as u32,
                },
            }));
        }
    }
    Ok(None)
}
/// Check if a module path represents an external dependency
fn is_external_dependency(module_path: &str) -> bool {
    if module_path.starts_with("./") || module_path.starts_with("../") {
        return false;
    }
    if module_path.starts_with("/") || module_path.starts_with("src/") {
        return false;
    }
    if module_path.starts_with("@") {
        return true;
    }
    !module_path.contains("/")
        || module_path.contains("node_modules")
        || !module_path.starts_with(".")
}
/// Build a dependency graph for a collection of files
pub fn build_dependency_graph(import_graphs: &[ImportGraph]) -> DependencyGraph {
    let mut graph = Graph::new();
    let mut file_nodes = HashMap::new();
    let mut path_to_node = HashMap::new();
    for import_graph in import_graphs {
        let node = graph.add_node(import_graph.source_file.clone());
        file_nodes.insert(import_graph.source_file.clone(), node);
        path_to_node.insert(import_graph.source_file.clone(), node);
    }
    for import_graph in import_graphs {
        if let Some(&source_node) = file_nodes.get(&import_graph.source_file) {
            for import in &import_graph.imports {
                if let Some(target_file) = resolve_import_path(
                    &import.module_path,
                    &import_graph.source_file,
                    import_graphs,
                ) {
                    if let Some(&target_node) = file_nodes.get(&target_file) {
                        graph.add_edge(source_node, target_node, import.clone());
                    }
                }
            }
        }
    }
    let circular_dependencies = detect_cycles(&graph, &path_to_node);
    DependencyGraph {
        graph,
        file_nodes,
        circular_dependencies,
    }
}
/// Dependency graph structure
pub struct DependencyGraph {
    pub graph: Graph<String, ImportInfo>,
    pub file_nodes: HashMap<String, NodeIndex>,
    pub circular_dependencies: Vec<Vec<String>>,
}
impl DependencyGraph {
    /// Get all files that import the given file
    pub fn get_importers(&self, file_path: &str) -> Vec<String> {
        if let Some(&node) = self.file_nodes.get(file_path) {
            self.graph
                .neighbors_directed(node, Direction::Incoming)
                .map(|n| self.graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
    /// Get all files imported by the given file
    pub fn get_imports(&self, file_path: &str) -> Vec<String> {
        if let Some(&node) = self.file_nodes.get(file_path) {
            self.graph
                .neighbors_directed(node, Direction::Outgoing)
                .map(|n| self.graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
    /// Check if there's a dependency path between two files
    pub fn has_dependency_path(&self, from: &str, to: &str) -> bool {
        if let (Some(&from_node), Some(&to_node)) =
            (self.file_nodes.get(from), self.file_nodes.get(to))
        {
            petgraph::algo::has_path_connecting(&self.graph, from_node, to_node, None)
        } else {
            false
        }
    }
}
/// Resolve an import path to an actual file path
fn resolve_import_path(
    import_path: &str,
    source_file: &str,
    graphs: &[ImportGraph],
) -> Option<String> {
    if import_path.starts_with("./") || import_path.starts_with("../") {
        let source_dir = Path::new(source_file).parent()?;
        let resolved = source_dir.join(import_path);
        for ext in &["", ".ts", ".tsx", ".js", ".jsx", ".json"] {
            let with_ext = format!("{}{}", resolved.to_string_lossy(), ext);
            if graphs.iter().any(|g| g.source_file == with_ext) {
                return Some(with_ext);
            }
        }
    }
    for graph in graphs {
        if graph.source_file.ends_with(import_path)
            || graph.source_file.contains(&format!("/{}", import_path))
        {
            return Some(graph.source_file.clone());
        }
    }
    None
}
/// Detect circular dependencies in the graph
fn detect_cycles(
    graph: &Graph<String, ImportInfo>,
    path_to_node: &HashMap<String, NodeIndex>,
) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();
    for &node in path_to_node.values() {
        if !visited.contains(&node) {
            find_cycles_dfs(
                graph,
                node,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }
    cycles
}
/// DFS helper for cycle detection
fn find_cycles_dfs(
    graph: &Graph<String, ImportInfo>,
    node: NodeIndex,
    visited: &mut HashSet<NodeIndex>,
    rec_stack: &mut HashSet<NodeIndex>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node);
    rec_stack.insert(node);
    path.push(graph[node].clone());
    for neighbor in graph.neighbors(node) {
        if !visited.contains(&neighbor) {
            find_cycles_dfs(graph, neighbor, visited, rec_stack, path, cycles);
        } else if rec_stack.contains(&neighbor) {
            let cycle_start = path.iter().position(|p| p == &graph[neighbor]).unwrap_or(0);
            let cycle = path[cycle_start..].to_vec();
            cycles.push(cycle);
        }
    }
    path.pop();
    rec_stack.remove(&node);
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    #[test]
    fn test_parse_es_module_imports() {
        let source = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import type { User } from './types';
import * as utils from './utils';
import { Button as CustomButton } from '@ui/components';
"#;
        let imports = parse_js_ts_imports_enhanced(source).unwrap();
        assert_eq!(imports.len(), 5);
        assert_eq!(imports[0].module_path, "react");
        assert_eq!(imports[0].default_import, Some("React".to_string()));
        assert_eq!(imports[0].import_type, ImportType::EsModule);
        assert_eq!(imports[1].module_path, "react");
        assert_eq!(imports[1].named_imports.len(), 2);
        assert_eq!(imports[1].named_imports[0].name, "useState");
        assert_eq!(imports[1].named_imports[1].name, "useEffect");
        assert_eq!(imports[2].module_path, "./types");
        assert_eq!(imports[2].import_type, ImportType::TypeOnly);
        assert!(imports[2].type_only);
        assert_eq!(imports[3].module_path, "./utils");
        assert_eq!(imports[3].namespace_import, Some("utils".to_string()));
        assert_eq!(imports[4].module_path, "@ui/components");
        assert_eq!(imports[4].named_imports[0].name, "Button");
        assert_eq!(
            imports[4].named_imports[0].alias,
            Some("CustomButton".to_string())
        );
    }
    #[test]
    fn test_parse_commonjs_requires() {
        let source = r#"
const fs = require('fs');
const { readFile, writeFile } = require('fs/promises');
const express = require('express');
require('dotenv/config');
"#;
        let imports = parse_js_ts_imports_enhanced(source).unwrap();
        assert_eq!(imports.len(), 4);
        assert_eq!(imports[0].module_path, "fs");
        assert_eq!(imports[0].default_import, Some("fs".to_string()));
        assert_eq!(imports[0].import_type, ImportType::CommonJs);
        assert_eq!(imports[1].module_path, "fs/promises");
        assert_eq!(imports[1].named_imports.len(), 2);
        assert_eq!(imports[1].named_imports[0].name, "readFile");
        assert_eq!(imports[1].named_imports[1].name, "writeFile");
        assert_eq!(imports[3].module_path, "dotenv/config");
        assert_eq!(imports[3].import_type, ImportType::CommonJs);
    }
    #[test]
    fn test_parse_dynamic_imports() {
        let source = r#"
const module = await import('./dynamic-module');
import('./another-module').then(mod => console.log(mod));
"#;
        let imports = parse_js_ts_imports_enhanced(source).unwrap();
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].module_path, "./dynamic-module");
        assert_eq!(imports[0].import_type, ImportType::Dynamic);
        assert_eq!(imports[1].module_path, "./another-module");
        assert_eq!(imports[1].import_type, ImportType::Dynamic);
    }
    #[test]
    fn test_parse_python_imports() {
        let source = r#"
import os
import sys as system
from pathlib import Path
from typing import Dict, List as ArrayList
from . import utils
from ..config import settings
"#;
        let imports = parse_python_imports(source).unwrap();
        assert_eq!(imports.len(), 6);
        assert_eq!(imports[0].module_path, "os");
        assert_eq!(imports[0].namespace_import, Some("os".to_string()));
        assert_eq!(imports[1].module_path, "sys");
        assert_eq!(imports[1].named_imports[0].name, "system");
        assert_eq!(imports[2].module_path, "pathlib");
        assert_eq!(imports[2].named_imports[0].name, "Path");
        assert_eq!(imports[3].module_path, "typing");
        assert_eq!(imports[3].named_imports.len(), 2);
        assert_eq!(imports[3].named_imports[0].name, "Dict");
        assert_eq!(imports[3].named_imports[1].name, "List");
        assert_eq!(
            imports[3].named_imports[1].alias,
            Some("ArrayList".to_string())
        );
    }
    #[test]
    fn test_build_import_graph() {
        let source = r#"
import React from 'react';
import { Component } from './component';
import * as utils from '@shared/utils';
require('dotenv/config');
"#;
        let path = PathBuf::from("src/index.ts");
        let graph = build_import_graph(source, &path).unwrap();
        assert_eq!(graph.source_file, "src/index.ts");
        assert_eq!(graph.imports.len(), 4);
        assert_eq!(graph.metadata.language, "typescript");
        assert!(graph
            .metadata
            .external_dependencies
            .contains(&"react".to_string()));
        assert!(graph
            .metadata
            .external_dependencies
            .contains(&"@shared/utils".to_string()));
        assert!(graph
            .metadata
            .external_dependencies
            .contains(&"dotenv/config".to_string()));
    }
    #[test]
    fn test_is_external_dependency() {
        assert!(is_external_dependency("react"));
        assert!(is_external_dependency("@types/node"));
        assert!(is_external_dependency("lodash"));
        assert!(!is_external_dependency("./component"));
        assert!(!is_external_dependency("../utils"));
        assert!(!is_external_dependency("src/types"));
    }
    #[test]
    fn test_dependency_graph() {
        let graphs = vec![
            ImportGraph {
                source_file: "a.ts".to_string(),
                imports: vec![ImportInfo {
                    module_path: "./b".to_string(),
                    import_type: ImportType::EsModule,
                    named_imports: vec![],
                    default_import: None,
                    namespace_import: None,
                    type_only: false,
                    location: SourceLocation {
                        start_line: 0,
                        start_column: 0,
                        end_line: 0,
                        end_column: 20,
                    },
                }],
                importers: vec![],
                metadata: ImportGraphMetadata {
                    language: "typescript".to_string(),
                    parsed_at: chrono::Utc::now(),
                    parser_version: "0.2.0".to_string(),
                    circular_dependencies: vec![],
                    external_dependencies: vec![],
                },
            },
            ImportGraph {
                source_file: "b.ts".to_string(),
                imports: vec![],
                importers: vec![],
                metadata: ImportGraphMetadata {
                    language: "typescript".to_string(),
                    parsed_at: chrono::Utc::now(),
                    parser_version: "0.2.0".to_string(),
                    circular_dependencies: vec![],
                    external_dependencies: vec![],
                },
            },
        ];
        let dep_graph = build_dependency_graph(&graphs);
        assert!(dep_graph.file_nodes.contains_key("a.ts"));
        assert!(dep_graph.file_nodes.contains_key("b.ts"));
        let imports = dep_graph.get_imports("a.ts");
        assert_eq!(imports.len(), 0);
        let importers = dep_graph.get_importers("b.ts");
        assert_eq!(importers.len(), 0);
    }
    #[test]
    fn test_parse_named_imports_enhanced() {
        let named_str = "useState, useEffect, type User, Button as CustomButton";
        let imports = parse_named_imports_enhanced(named_str).unwrap();
        assert_eq!(imports.len(), 4);
        assert_eq!(imports[0].name, "useState");
        assert_eq!(imports[0].alias, None);
        assert!(!imports[0].type_only);
        assert_eq!(imports[1].name, "useEffect");
        assert!(!imports[1].type_only);
        assert_eq!(imports[2].name, "User");
        assert!(imports[2].type_only);
        assert_eq!(imports[3].name, "Button");
        assert_eq!(imports[3].alias, Some("CustomButton".to_string()));
        assert!(!imports[3].type_only);
    }
    #[test]
    fn test_parse_go_imports() {
        let source = r#"package main

import "fmt"
import alias "github.com/user/repo"
import (
    "os"
    "path/filepath"
    . "net/http"
    _ "database/sql/driver"
    json "encoding/json"
    "github.com/external/lib"
)

func main() {
    fmt.Println("Hello")
}"#;
        let imports = parse_go_imports(source).unwrap();
        println!("Found {} imports:", imports.len());
        for (i, import) in imports.iter().enumerate() {
            println!(
                "  {}: {} -> {:?}",
                i, import.module_path, import.default_import
            );
        }
        assert_eq!(imports.len(), 8);
        assert_eq!(imports[0].module_path, "fmt");
        assert_eq!(imports[0].namespace_import, Some("fmt".to_string()));
        assert_eq!(imports[0].default_import, None);
        assert_eq!(imports[1].module_path, "github.com/user/repo");
        assert_eq!(imports[1].default_import, Some("alias".to_string()));
        assert_eq!(imports[1].namespace_import, None);
        assert_eq!(imports[2].module_path, "os");
        assert_eq!(imports[2].namespace_import, Some("os".to_string()));
        assert_eq!(imports[3].module_path, "path/filepath");
        assert_eq!(imports[3].namespace_import, Some("filepath".to_string()));
        assert_eq!(imports[4].module_path, "net/http");
        assert_eq!(imports[4].default_import, Some(".".to_string()));
        assert_eq!(imports[5].module_path, "database/sql/driver");
        assert_eq!(imports[5].default_import, Some("_".to_string()));
        assert_eq!(imports[6].module_path, "encoding/json");
        assert_eq!(imports[6].default_import, Some("json".to_string()));
        assert_eq!(imports[7].module_path, "github.com/external/lib");
        assert_eq!(imports[7].namespace_import, Some("lib".to_string()));
        assert_eq!(imports[7].default_import, None);
    }
}
